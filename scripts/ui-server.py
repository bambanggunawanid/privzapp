#!/usr/bin/env python3
"""Static file server for Playwright UI tests.

Serves the built web bundle with the MIME types browsers require
(.mjs/.wasm — ES module imports are MIME-checked), which the stock
http.server maps wrong on some systems.

Usage: ui-server.py <port> <bundle-dir>
"""

import functools
import http.server
import sys


class Handler(http.server.SimpleHTTPRequestHandler):
    extensions_map = {
        **http.server.SimpleHTTPRequestHandler.extensions_map,
        ".js": "text/javascript",
        ".mjs": "text/javascript",
        ".wasm": "application/wasm",
        ".webmanifest": "application/manifest+json",
    }

    def log_message(self, *args):
        pass  # keep test output clean


def main():
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 4173
    root = sys.argv[2] if len(sys.argv) > 2 else "target/dx/privzapp/release/web/public"
    server = http.server.ThreadingHTTPServer(
        ("127.0.0.1", port), functools.partial(Handler, directory=root)
    )
    print(f"ui-server: http://127.0.0.1:{port} ← {root}", flush=True)
    server.serve_forever()


if __name__ == "__main__":
    main()
