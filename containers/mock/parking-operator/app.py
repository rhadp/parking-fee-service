#!/usr/bin/env python3
"""
Mock Parking Operator Service

A simple HTTP server that simulates a parking operator backend for local
development and testing of the SDV Parking Demo System.

Endpoints:
  GET  /health          - Health check endpoint
  POST /sessions/start  - Start a parking session
  POST /sessions/stop   - Stop a parking session
  GET  /sessions/{id}   - Get session status

Requirements: 3.4
"""

import json
import os
import time
import uuid
from http.server import HTTPServer, BaseHTTPRequestHandler
from urllib.parse import urlparse, parse_qs

# Configuration
PORT = int(os.environ.get('PORT', 8080))
LOG_LEVEL = os.environ.get('LOG_LEVEL', 'info')
MOCK_DELAY_MS = int(os.environ.get('MOCK_DELAY_MS', 100))

# In-memory session storage
sessions = {}


def log(level: str, message: str):
    """Simple logging function."""
    levels = {'debug': 0, 'info': 1, 'warning': 2, 'error': 3}
    current_level = levels.get(LOG_LEVEL.lower(), 1)
    msg_level = levels.get(level.lower(), 1)
    if msg_level >= current_level:
        print(f"[{level.upper()}] {message}")


class MockParkingHandler(BaseHTTPRequestHandler):
    """HTTP request handler for mock parking operator."""

    def log_message(self, format, *args):
        """Override to use custom logging."""
        log('info', f"{self.address_string()} - {format % args}")

    def send_json_response(self, status_code: int, data: dict):
        """Send a JSON response."""
        self.send_response(status_code)
        self.send_header('Content-Type', 'application/json')
        self.end_headers()
        self.wfile.write(json.dumps(data).encode('utf-8'))

    def simulate_delay(self):
        """Simulate network/processing delay."""
        if MOCK_DELAY_MS > 0:
            time.sleep(MOCK_DELAY_MS / 1000.0)

    def do_GET(self):
        """Handle GET requests."""
        parsed = urlparse(self.path)
        path = parsed.path

        self.simulate_delay()

        if path == '/health':
            self.send_json_response(200, {
                'status': 'healthy',
                'service': 'mock-parking-operator',
                'timestamp': time.time()
            })
        elif path.startswith('/sessions/'):
            session_id = path.split('/')[-1]
            if session_id in sessions:
                self.send_json_response(200, sessions[session_id])
            else:
                self.send_json_response(404, {
                    'error': 'Session not found',
                    'session_id': session_id
                })
        else:
            self.send_json_response(404, {'error': 'Not found'})

    def do_POST(self):
        """Handle POST requests."""
        parsed = urlparse(self.path)
        path = parsed.path

        # Read request body
        content_length = int(self.headers.get('Content-Length', 0))
        body = self.rfile.read(content_length).decode('utf-8') if content_length > 0 else '{}'
        
        try:
            data = json.loads(body) if body else {}
        except json.JSONDecodeError:
            self.send_json_response(400, {'error': 'Invalid JSON'})
            return

        self.simulate_delay()

        if path == '/sessions/start':
            session_id = str(uuid.uuid4())
            session = {
                'session_id': session_id,
                'vehicle_id': data.get('vehicle_id', 'unknown'),
                'zone_id': data.get('zone_id', 'ZONE-001'),
                'latitude': data.get('latitude', 0.0),
                'longitude': data.get('longitude', 0.0),
                'start_time': time.time(),
                'active': True,
                'operator_name': 'Mock Parking Co.',
                'hourly_rate': 2.50,
                'currency': 'USD'
            }
            sessions[session_id] = session
            log('info', f"Started session: {session_id}")
            self.send_json_response(201, {
                'session_id': session_id,
                'success': True,
                'operator_name': session['operator_name'],
                'hourly_rate': session['hourly_rate'],
                'currency': session['currency']
            })

        elif path == '/sessions/stop':
            session_id = data.get('session_id')
            if session_id and session_id in sessions:
                session = sessions[session_id]
                duration = time.time() - session['start_time']
                hours = duration / 3600.0
                total = hours * session['hourly_rate']
                session['active'] = False
                session['end_time'] = time.time()
                session['duration_seconds'] = int(duration)
                session['total_amount'] = round(total, 2)
                log('info', f"Stopped session: {session_id}, duration: {duration:.0f}s")
                self.send_json_response(200, {
                    'success': True,
                    'session_id': session_id,
                    'duration_seconds': session['duration_seconds'],
                    'total_amount': session['total_amount'],
                    'currency': session['currency']
                })
            else:
                self.send_json_response(404, {
                    'error': 'Session not found',
                    'session_id': session_id
                })
        else:
            self.send_json_response(404, {'error': 'Not found'})


def main():
    """Start the mock parking operator server."""
    server = HTTPServer(('0.0.0.0', PORT), MockParkingHandler)
    log('info', f"Mock Parking Operator starting on port {PORT}")
    log('info', f"Log level: {LOG_LEVEL}, Mock delay: {MOCK_DELAY_MS}ms")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        log('info', "Shutting down...")
        server.shutdown()


if __name__ == '__main__':
    main()
