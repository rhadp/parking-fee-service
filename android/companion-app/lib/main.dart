// SDV Parking Demo - Companion App
//
// Flutter/Dart mobile companion application for the SDV Parking Demo System.
// This app communicates with backend services via gRPC to manage parking sessions.

import 'package:flutter/material.dart';

void main() {
  runApp(const CompanionApp());
}

/// Root widget for the SDV Parking Companion App.
///
/// This application provides a mobile interface for:
/// - Viewing parking session status
/// - Managing parking payments
/// - Receiving notifications about parking events
class CompanionApp extends StatelessWidget {
  const CompanionApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'SDV Parking Companion',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: Colors.blue),
        useMaterial3: true,
      ),
      home: const HomePage(),
    );
  }
}

/// Home page placeholder for the companion app.
class HomePage extends StatelessWidget {
  const HomePage({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('SDV Parking Companion'),
        backgroundColor: Theme.of(context).colorScheme.inversePrimary,
      ),
      body: const Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(
              Icons.local_parking,
              size: 64,
              color: Colors.blue,
            ),
            SizedBox(height: 16),
            Text(
              'SDV Parking Demo',
              style: TextStyle(
                fontSize: 24,
                fontWeight: FontWeight.bold,
              ),
            ),
            SizedBox(height: 8),
            Text(
              'Companion App',
              style: TextStyle(
                fontSize: 16,
                color: Colors.grey,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
