#!/usr/bin/env python3
"""
Cross-Origin Isolation enabled HTTP server for SharedArrayBuffer support.
Required for Emscripten pthread/SharedArrayBuffer in browsers.
"""

import http.server
import socketserver
import json
import tempfile
import subprocess
import os
from pathlib import Path

PORT = 8000

class COIRequestHandler(http.server.SimpleHTTPRequestHandler):
    def end_headers(self):
        # Enable Cross-Origin Isolation (required for SharedArrayBuffer)
        self.send_header('Cross-Origin-Opener-Policy', 'same-origin')
        self.send_header('Cross-Origin-Embedder-Policy', 'require-corp')
        super().end_headers()

if __name__ == '__main__':
    # Allow immediate port reuse (prevent "Address already in use" error)
    socketserver.TCPServer.allow_reuse_address = True

    with socketserver.TCPServer(("", PORT), COIRequestHandler) as httpd:
        print(f"✅ Cross-Origin Isolation enabled server running at http://localhost:{PORT}/")
        print(f"📦 SharedArrayBuffer support: ENABLED")
        print(f"🧵 pthread support: ENABLED")
        print(f"\n🌐 Open: http://localhost:{PORT}/tools/web/visualizer.html#test=144")
        print(f"\nPress Ctrl+C to stop...")
        httpd.serve_forever()
