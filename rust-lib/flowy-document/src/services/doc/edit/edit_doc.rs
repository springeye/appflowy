use crate::{
    entities::{
        doc::{Doc, RevId, RevType, Revision, RevisionRange},
        ws::{WsDataType, WsDocumentData},
    },
    errors::{internal_error, DocError, DocResult},
    module::DocumentUser,
    services::{
        doc::{
            edit::{edit_actor::DocumentEditActor, message::EditMsg},
            revision::{DocRevision, RevisionCmd, RevisionManager, RevisionServer, RevisionStoreActor},
            UndoResult,
        },
        ws::{DocumentWebSocket, WsDocumentHandler},
    },
};
use bytes::Bytes;
use flowy_database::ConnectionPool;
use flowy_ot::core::{Attribute, Delta, Interval};
use flowy_ws::WsState;
use std::{convert::TryFrom, sync::Arc};
use tokio::sync::{mpsc, mpsc::UnboundedSender, oneshot};

pub type DocId = String;

pub struct ClientEditDoc {
    pub doc_id: DocId,
    rev_manager: Arc<RevisionManager>,
    document: UnboundedSender<EditMsg>,
    pool: Arc<ConnectionPool>,
}

impl ClientEditDoc {
    pub(crate) async fn new(
        doc_id: &str,
        pool: Arc<ConnectionPool>,
        ws: Arc<dyn DocumentWebSocket>,
        server: Arc<dyn RevisionServer>,
        user: Arc<dyn DocumentUser>,
    ) -> DocResult<Self> {
        let user_id = user.user_id()?;
        let rev_store = spawn_rev_store_actor(doc_id, pool.clone(), server.clone());
        let DocRevision { rev_id, delta } = fetch_document(rev_store.clone()).await?;

        log::info!("😁 Document delta: {:?}", delta);

        let rev_manager = Arc::new(RevisionManager::new(doc_id, &user_id, rev_id, ws, rev_store));
        let document = spawn_doc_edit_actor(doc_id, delta, pool.clone());
        let doc_id = doc_id.to_string();
        Ok(Self {
            doc_id,
            rev_manager,
            document,
            pool,
        })
    }

    pub async fn insert<T: ToString>(&self, index: usize, data: T) -> Result<(), DocError> {
        let (ret, rx) = oneshot::channel::<DocResult<Delta>>();
        let msg = EditMsg::Insert {
            index,
            data: data.to_string(),
            ret,
        };
        let _ = self.document.send(msg);
        let delta_data = rx.await.map_err(internal_error)??.to_bytes();
        let rev_id = self.mk_revision(&delta_data).await?;
        save_document(self.document.clone(), rev_id.into()).await
    }

    pub async fn delete(&self, interval: Interval) -> Result<(), DocError> {
        let (ret, rx) = oneshot::channel::<DocResult<Delta>>();
        let msg = EditMsg::Delete { interval, ret };
        let _ = self.document.send(msg);
        let delta_data = rx.await.map_err(internal_error)??.to_bytes();
        let _ = self.mk_revision(&delta_data).await?;
        Ok(())
    }

    pub async fn format(&self, interval: Interval, attribute: Attribute) -> Result<(), DocError> {
        let (ret, rx) = oneshot::channel::<DocResult<Delta>>();
        let msg = EditMsg::Format {
            interval,
            attribute,
            ret,
        };
        let _ = self.document.send(msg);
        let delta_data = rx.await.map_err(internal_error)??.to_bytes();
        let _ = self.mk_revision(&delta_data).await?;
        Ok(())
    }

    pub async fn replace<T: ToString>(&mut self, interval: Interval, data: T) -> Result<(), DocError> {
        let (ret, rx) = oneshot::channel::<DocResult<Delta>>();
        let msg = EditMsg::Replace {
            interval,
            data: data.to_string(),
            ret,
        };
        let _ = self.document.send(msg);
        let delta_data = rx.await.map_err(internal_error)??.to_bytes();
        let _ = self.mk_revision(&delta_data).await?;
        Ok(())
    }

    pub async fn can_undo(&self) -> bool {
        let (ret, rx) = oneshot::channel::<bool>();
        let msg = EditMsg::CanUndo { ret };
        let _ = self.document.send(msg);
        rx.await.unwrap_or(false)
    }

    pub async fn can_redo(&self) -> bool {
        let (ret, rx) = oneshot::channel::<bool>();
        let msg = EditMsg::CanRedo { ret };
        let _ = self.document.send(msg);
        rx.await.unwrap_or(false)
    }

    pub async fn undo(&self) -> Result<UndoResult, DocError> {
        let (ret, rx) = oneshot::channel::<DocResult<UndoResult>>();
        let msg = EditMsg::Undo { ret };
        let _ = self.document.send(msg);
        rx.await.map_err(internal_error)?
    }

    pub async fn redo(&self) -> Result<UndoResult, DocError> {
        let (ret, rx) = oneshot::channel::<DocResult<UndoResult>>();
        let msg = EditMsg::Redo { ret };
        let _ = self.document.send(msg);
        rx.await.map_err(internal_error)?
    }

    pub async fn doc(&self) -> DocResult<Doc> {
        let (ret, rx) = oneshot::channel::<DocResult<String>>();
        let msg = EditMsg::Doc { ret };
        let _ = self.document.send(msg);
        let data = rx.await.map_err(internal_error)??;
        let rev_id = self.rev_manager.rev_id();
        let id = self.doc_id.clone();

        Ok(Doc { id, data, rev_id })
    }

    async fn mk_revision(&self, delta_data: &Bytes) -> Result<RevId, DocError> {
        let (base_rev_id, rev_id) = self.rev_manager.next_rev_id();
        let delta_data = delta_data.to_vec();
        let revision = Revision::new(base_rev_id, rev_id, delta_data, &self.doc_id, RevType::Local);
        let _ = self.rev_manager.add_revision(revision).await?;
        Ok(rev_id.into())
    }

    #[tracing::instrument(level = "debug", skip(self, data), err)]
    pub(crate) async fn compose_local_delta(&self, data: Bytes) -> Result<(), DocError> {
        let delta = Delta::from_bytes(&data)?;
        let (ret, rx) = oneshot::channel::<DocResult<()>>();
        let msg = EditMsg::Delta { delta, ret };
        let _ = self.document.send(msg);
        let _ = rx.await.map_err(internal_error)??;

        let rev_id = self.mk_revision(&data).await?;
        save_document(self.document.clone(), rev_id).await
    }

    #[cfg(feature = "flowy_test")]
    pub async fn doc_json(&self) -> DocResult<String> {
        let (ret, rx) = oneshot::channel::<DocResult<String>>();
        let msg = EditMsg::Doc { ret };
        let _ = self.document.send(msg);
        rx.await.map_err(internal_error)?
    }
}

impl WsDocumentHandler for ClientEditDoc {
    fn receive(&self, doc_data: WsDocumentData) {
        let document = self.document.clone();
        let rev_manager = self.rev_manager.clone();
        let handle_ws_message = |doc_data: WsDocumentData| async move {
            let bytes = Bytes::from(doc_data.data);
            match doc_data.ty {
                WsDataType::PushRev => {
                    let _ = handle_push_rev(bytes, rev_manager, document).await?;
                },
                WsDataType::PullRev => {
                    let range = RevisionRange::try_from(bytes)?;
                    let _ = rev_manager.send_revisions(range).await?;
                },
                WsDataType::NewDocUser => {},
                WsDataType::Acked => {
                    let rev_id = RevId::try_from(bytes)?;
                    let _ = rev_manager.ack_rev(rev_id);
                },
                WsDataType::Conflict => {},
            }
            Result::<(), DocError>::Ok(())
        };

        tokio::spawn(async move {
            if let Err(e) = handle_ws_message(doc_data).await {
                log::error!("{:?}", e);
            }
        });
    }
    fn state_changed(&self, state: &WsState) { let _ = self.rev_manager.handle_ws_state_changed(state); }
}

async fn save_document(document: UnboundedSender<EditMsg>, rev_id: RevId) -> DocResult<()> {
    let (ret, rx) = oneshot::channel::<DocResult<()>>();
    let _ = document.send(EditMsg::SaveDocument { rev_id, ret });
    let result = rx.await.map_err(internal_error)?;
    result
}

async fn handle_push_rev(
    rev_bytes: Bytes,
    rev_manager: Arc<RevisionManager>,
    document: UnboundedSender<EditMsg>,
) -> DocResult<()> {
    let revision = Revision::try_from(rev_bytes)?;
    let _ = rev_manager.add_revision(revision.clone()).await?;

    let delta = Delta::from_bytes(&revision.delta_data)?;
    let (ret, rx) = oneshot::channel::<DocResult<()>>();
    let msg = EditMsg::Delta { delta, ret };
    let _ = document.send(msg);
    let _ = rx.await.map_err(internal_error)??;

    save_document(document, revision.rev_id.into()).await;
    Ok(())
}

fn spawn_rev_store_actor(
    doc_id: &str,
    pool: Arc<ConnectionPool>,
    server: Arc<dyn RevisionServer>,
) -> mpsc::Sender<RevisionCmd> {
    let (sender, receiver) = mpsc::channel::<RevisionCmd>(50);
    let actor = RevisionStoreActor::new(doc_id, pool, receiver, server);
    tokio::spawn(actor.run());
    sender
}

fn spawn_doc_edit_actor(doc_id: &str, delta: Delta, pool: Arc<ConnectionPool>) -> UnboundedSender<EditMsg> {
    let (sender, receiver) = mpsc::unbounded_channel::<EditMsg>();
    let actor = DocumentEditActor::new(&doc_id, delta, pool.clone(), receiver);
    tokio::spawn(actor.run());
    sender
}

async fn fetch_document(sender: mpsc::Sender<RevisionCmd>) -> DocResult<DocRevision> {
    let (ret, rx) = oneshot::channel();
    let _ = sender.send(RevisionCmd::DocumentDelta { ret }).await;

    match rx.await {
        Ok(result) => Ok(result?),
        Err(e) => {
            log::error!("fetch_document: {}", e);
            Err(DocError::internal().context(format!("fetch_document: {}", e)))
        },
    }
}