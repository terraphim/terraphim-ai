#!/usr/bin/env python3
"""Local auth-injecting proxy for `cargo publish` to the terraphim Gitea
registry (companion to scripts/publish-via-auth-proxy.sh).

Cargo (>=1.85 builtin token provider) sends the registry token in the bare
legacy `Authorization: <token>` header, which Gitea's cargo endpoint rejects
(it accepts Basic/Bearer/token-prefixed forms). This proxy listens on
127.0.0.1:8899 and forwards every request to git.terraphim.cloud, replacing
the Authorization header with Basic auth, so `cargo publish` works
unmodified.
"""

import base64
import http.client
import http.server
import os
import sys
import threading
from urllib.parse import urlsplit

UPSTREAM_HOST = "git.terraphim.cloud"
TOKEN = os.environ.get("GITEA_TOKEN", "")
BASIC = "Basic " + base64.b64encode(f"root:{TOKEN}".encode()).decode()


class Proxy(http.server.BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"

    def _forward(self, method: str) -> None:
        length = int(self.headers.get("Content-Length") or 0)
        body = self.rfile.read(length) if length else None
        parts = urlsplit(self.path)
        headers = {
            k: v
            for k, v in self.headers.items()
            if k.lower() not in {"host", "authorization", "content-length"}
        }
        headers["Authorization"] = BASIC
        headers["Host"] = UPSTREAM_HOST
        if body is not None:
            headers["Content-Length"] = str(len(body))
        connection = http.client.HTTPSConnection(UPSTREAM_HOST, timeout=120)
        try:
            connection.request(
                method,
                parts.path + (f"?{parts.query}" if parts.query else ""),
                body=body,
                headers=headers,
            )
            response = connection.getresponse()
            payload = response.read()
            # Rewrite the sparse-index config so cargo's API calls (publish)
            # also route through this proxy instead of going upstream with
            # the bare-header auth Gitea rejects.
            if parts.path.endswith("/config.json") and response.status == 200:
                payload = payload.replace(
                    f"https://{UPSTREAM_HOST}".encode(),
                    f"http://127.0.0.1:{self.server.server_port}".encode(),
                )
            self.send_response(response.status)
            for key, value in response.getheaders():
                if key.lower() in {"transfer-encoding", "connection", "content-length"}:
                    continue
                self.send_header(key, value)
            self.send_header("Content-Length", str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)
        finally:
            connection.close()

    def do_GET(self):
        self._forward("GET")

    def do_PUT(self):
        self._forward("PUT")

    def do_HEAD(self):
        self._forward("HEAD")

    def log_message(self, fmt, *args):
        sys.stderr.write(f"cargo-registry-proxy: {fmt % args}\n")


if __name__ == "__main__":
    if not TOKEN:
        sys.exit("GITEA_TOKEN must be set in the environment")
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 8899), Proxy)
    print(
        "cargo registry proxy on http://127.0.0.1:8899 -> https://" + UPSTREAM_HOST,
        flush=True,
    )
    server.serve_forever()
