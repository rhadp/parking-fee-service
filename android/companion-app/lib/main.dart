import 'package:flutter/material.dart';

void main() {
  runApp(const CompanionApp());
}

/// COMPANION_APP entry point.
///
/// Placeholder skeleton — screens, providers, and services will be added
/// in subsequent task groups.
class CompanionApp extends StatelessWidget {
  const CompanionApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Companion App',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: Colors.blue),
        useMaterial3: true,
      ),
      home: const Scaffold(
        body: Center(
          child: Text('Companion App'),
        ),
      ),
    );
  }
}
