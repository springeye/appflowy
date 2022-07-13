import 'package:flowy_editor/flowy_editor.dart';
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:flowy_editor/document/attributes.dart';

class ImageNodeBuilder extends NodeWidgetBuilder {
  ImageNodeBuilder.create({
    required super.node,
    required super.editorState,
  }) : super.create();

  @override
  Widget build(BuildContext buildContext) {
    return _ImageNodeWidget(
      node: node,
      editorState: editorState,
    );
  }
}

class _ImageNodeWidget extends StatelessWidget {
  final Node node;
  final EditorState editorState;

  const _ImageNodeWidget({
    Key? key,
    required this.node,
    required this.editorState,
  }) : super(key: key);

  String get src => node.attributes['image_src'] as String;

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      child: ChangeNotifierProvider.value(
        value: node,
        builder: (_, __) => Consumer<Node>(
          builder: ((context, value, child) => _build(context)),
        ),
      ),
      onTap: () {
        editorState.update(
          node,
          Attributes.from(node.attributes)
            ..addAll(
              {
                'image_src':
                    "https://images.pexels.com/photos/9995076/pexels-photo-9995076.png?cs=srgb&dl=pexels-temmuz-uzun-9995076.jpg&fm=jpg&w=640&h=400",
              },
            ),
        );
      },
    );
  }

  Widget _build(BuildContext context) {
    return Column(
      children: [
        Image.network(src),
        if (node.children.isNotEmpty)
          Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: node.children
                .map(
                  (e) => editorState.renderPlugins.buildWidget(
                    context: NodeWidgetContext(
                      buildContext: context,
                      node: e,
                      editorState: editorState,
                    ),
                  ),
                )
                .toList(),
          ),
      ],
    );
  }
}
