import 'package:flutter_test/flutter_test.dart';

import 'package:companion_app/main.dart';

void main() {
  testWidgets('CompanionApp smoke test', (WidgetTester tester) async {
    await tester.pumpWidget(const CompanionApp());
    expect(find.text('Companion App'), findsOneWidget);
  });
}
