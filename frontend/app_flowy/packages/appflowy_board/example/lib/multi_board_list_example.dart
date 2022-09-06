import 'package:appflowy_board/appflowy_board.dart';
import 'package:flutter/material.dart';

class MultiBoardListExample extends StatefulWidget {
  const MultiBoardListExample({Key? key}) : super(key: key);

  @override
  State<MultiBoardListExample> createState() => _MultiBoardListExampleState();
}

class _MultiBoardListExampleState extends State<MultiBoardListExample> {
  final AFBoardDataController boardDataController = AFBoardDataController(
    onMoveColumn: (fromColumnId, fromIndex, toColumnId, toIndex) {
      // debugPrint('Move column from $fromIndex to $toIndex');
    },
    onMoveColumnItem: (columnId, fromIndex, toIndex) {
      // debugPrint('Move $columnId:$fromIndex to $columnId:$toIndex');
    },
    onMoveColumnItemToColumn: (fromColumnId, fromIndex, toColumnId, toIndex) {
      // debugPrint('Move $fromColumnId:$fromIndex to $toColumnId:$toIndex');
    },
  );

  @override
  void initState() {
    List<AFColumnItem> a = [
      TextItem("Card 1"),
      TextItem("Card 2"),
      RichTextItem(title: "Card 3", subtitle: 'Aug 1, 2020 4:05 PM'),
      TextItem("Card 4"),
      TextItem("Card 5"),
      TextItem("Card 6"),
      RichTextItem(title: "Card 7", subtitle: 'Aug 1, 2020 4:05 PM'),
      RichTextItem(title: "Card 8", subtitle: 'Aug 1, 2020 4:05 PM'),
      TextItem("Card 9"),
    ];

    final column1 = AFBoardColumnData(id: "To Do", name: "To Do", items: a);
    final column2 = AFBoardColumnData(
      id: "In Progress",
      name: "In Progress",
      items: <AFColumnItem>[
        RichTextItem(title: "Card 10", subtitle: 'Aug 1, 2020 4:05 PM'),
        TextItem("Card 11"),
      ],
    );

    final column3 =
        AFBoardColumnData(id: "Done", name: "Done", items: <AFColumnItem>[]);

    boardDataController.addColumn(column1);
    boardDataController.addColumn(column2);
    boardDataController.addColumn(column3);

    super.initState();
  }

  @override
  Widget build(BuildContext context) {
    final config = AFBoardConfig(
      columnBackgroundColor: HexColor.fromHex('#F7F8FC'),
    );
    return Container(
      color: Colors.white,
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 30, horizontal: 20),
        child: AFBoard(
          dataController: boardDataController,
          footerBuilder: (context, columnData) {
            return AppFlowyColumnFooter(
              icon: const Icon(Icons.add, size: 20),
              title: const Text('New'),
              height: 50,
              margin: config.columnItemPadding,
            );
          },
          headerBuilder: (context, columnData) {
            return AppFlowyColumnHeader(
              icon: const Icon(Icons.lightbulb_circle),
              title: SizedBox(
                width: 60,
                child: TextField(
                  controller: TextEditingController()
                    ..text = columnData.headerData.columnName,
                  onSubmitted: (val) {
                    boardDataController
                        .getColumnController(columnData.headerData.columnId)!
                        .updateColumnName(val);
                  },
                ),
              ),
              addIcon: const Icon(Icons.add, size: 20),
              moreIcon: const Icon(Icons.more_horiz, size: 20),
              height: 50,
              margin: config.columnItemPadding,
            );
          },
          cardBuilder: (context, column, columnItem) {
            return AppFlowyColumnItemCard(
              key: ValueKey(columnItem.id),
              child: _buildCard(columnItem),
            );
          },
          columnConstraints: const BoxConstraints.tightFor(width: 240),
          config: AFBoardConfig(
            columnBackgroundColor: HexColor.fromHex('#F7F8FC'),
          ),
        ),
      ),
    );
  }

  Widget _buildCard(AFColumnItem item) {
    if (item is TextItem) {
      return Align(
        alignment: Alignment.centerLeft,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 30),
          child: Text(item.s),
        ),
      );
    }

    if (item is RichTextItem) {
      return RichTextCard(item: item);
    }

    throw UnimplementedError();
  }
}

class RichTextCard extends StatefulWidget {
  final RichTextItem item;
  const RichTextCard({
    required this.item,
    Key? key,
  }) : super(key: key);

  @override
  State<RichTextCard> createState() => _RichTextCardState();
}

class _RichTextCardState extends State<RichTextCard> {
  @override
  void initState() {
    super.initState();
  }

  @override
  Widget build(BuildContext context) {
    return Align(
      alignment: Alignment.centerLeft,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 20),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              widget.item.title,
              style: const TextStyle(fontSize: 14),
              textAlign: TextAlign.left,
            ),
            const SizedBox(height: 10),
            Text(
              widget.item.subtitle,
              style: const TextStyle(fontSize: 12, color: Colors.grey),
            )
          ],
        ),
      ),
    );
  }
}

class TextItem extends AFColumnItem {
  final String s;

  TextItem(this.s);

  @override
  String get id => s;
}

class RichTextItem extends AFColumnItem {
  final String title;
  final String subtitle;

  RichTextItem({required this.title, required this.subtitle});

  @override
  String get id => title;
}

extension HexColor on Color {
  static Color fromHex(String hexString) {
    final buffer = StringBuffer();
    if (hexString.length == 6 || hexString.length == 7) buffer.write('ff');
    buffer.write(hexString.replaceFirst('#', ''));
    return Color(int.parse(buffer.toString(), radix: 16));
  }
}