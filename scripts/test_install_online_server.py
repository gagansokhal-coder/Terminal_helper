#!/usr/bin/env python3
"""Test double for api.github.com and the release-asset host.

Used by scripts/test_install_online.sh to exercise scripts/install-online.sh
without touching the real GitHub API or downloading a real release.

Modes (env MODE):
  ok         -> 200 release JSON + valid tarball + valid checksums.txt
  ratelimit  -> 403 with GitHub's real rate-limit body on the API call
  dlfail     -> 200 release JSON, but the asset response is truncated
                (models an interrupted download)
"""
import hashlib
import io
import json
import os
import tarfile
from http.server import BaseHTTPRequestHandler, HTTPServer

MODE = os.environ.get("MODE", "ok")
PORT = int(os.environ.get("PORT", "8940"))
BASE = f"http://127.0.0.1:{PORT}"
ASSET = "ggnmem-linux-x86_64.tar.gz"


def build_bundle() -> bytes:
    """A minimal but *valid* release bundle with every file the script requires."""
    files = {
        "ggnmem": b"#!/bin/sh\necho 'ggnmem 0.0.0-test'\n",
        "ggnmem-daemon": b"#!/bin/sh\necho daemon\n",
        "install.sh": b"#!/usr/bin/env bash\nset -euo pipefail\necho '[stub install.sh]'\nexit 0\n",
        "VERSION": b"version=v0.0.0-test\n",
        "checksums.txt": b"placeholder\n",
    }
    buf = io.BytesIO()
    with tarfile.open(fileobj=buf, mode="w:gz") as tar:
        for name, data in files.items():
            info = tarfile.TarInfo(name)
            info.size = len(data)
            info.mode = 0o755
            tar.addfile(info, io.BytesIO(data))
    return buf.getvalue()


TARBALL = build_bundle()
TARBALL_SHA = hashlib.sha256(TARBALL).hexdigest()
CHECKSUMS = f"{TARBALL_SHA}  {ASSET}\n".encode()

RELEASE_JSON = json.dumps(
    [
        {
            "tag_name": "v0.0.0-test",
            "prerelease": True,
            "assets": [
                {"name": "checksums.txt", "browser_download_url": BASE + "/dl/checksums.txt"},
                {"name": ASSET, "browser_download_url": BASE + "/dl/" + ASSET},
                {
                    "name": "ggnmem-windows-x86_64.zip",
                    "browser_download_url": BASE + "/dl/ggnmem-windows-x86_64.zip",
                },
            ],
        }
    ]
).encode()

# What api.github.com actually returns when the unauthenticated quota is spent.
RATE_LIMIT_BODY = (
    b'{"message":"API rate limit exceeded for 203.0.113.9. '
    b"(But here's the good news: Authenticated requests get a higher rate limit. "
    b'Check out the documentation for more details.)",'
    b'"documentation_url":"https://docs.github.com/rest/overview/rate-limits"}'
)


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.0"

    def log_message(self, *args, **kwargs):
        pass

    def _send(self, code, body, ctype="application/json"):
        self.send_response(code)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        path = self.path.split("?")[0]
        if path == "/releases":
            if MODE == "ratelimit":
                self._send(403, RATE_LIMIT_BODY)
            else:
                self._send(200, RELEASE_JSON)
        elif path == f"/dl/{ASSET}":
            if MODE == "dlfail":
                # Announce the full length, then hang up early: this is what an
                # interrupted download looks like to curl.
                self.send_response(200)
                self.send_header("Content-Type", "application/gzip")
                self.send_header("Content-Length", str(len(TARBALL)))
                self.end_headers()
                self.wfile.write(TARBALL[: len(TARBALL) // 3])
                self.close_connection = True
            else:
                self._send(200, TARBALL, "application/gzip")
        elif path == "/dl/checksums.txt":
            self._send(200, CHECKSUMS, "text/plain")
        else:
            self._send(404, b'{"message":"Not Found"}')


if __name__ == "__main__":
    print(f"listening {BASE} mode={MODE}", flush=True)
    HTTPServer(("127.0.0.1", PORT), Handler).serve_forever()
