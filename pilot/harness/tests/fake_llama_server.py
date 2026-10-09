#!/usr/bin/env python3
"""TEST ONLY. Pretends to be llama-server (no model, no LLM): scripted replies keyed on the number of assistant messages."""
import sys, json
from http.server import BaseHTTPRequestHandler, HTTPServer
port = int(sys.argv[sys.argv.index("--port") + 1])
REPLIES = [
    "Here you go, no code block.",
    "Second try.\n```pvc\nsurface { text \"Hi\" }\n```\n",
    "Third try.\n```pvc\nsurface { text \"Hi\" button \"Go\" }\n```\n",
]
class H(BaseHTTPRequestHandler):
    def log_message(self, *a): pass
    def _send(self, o):
        b = json.dumps(o).encode(); self.send_response(200); self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(b))); self.end_headers(); self.wfile.write(b)
    def do_GET(self):
        self._send({"status": "ok"} if self.path == "/health" else {"fake": True})
    def do_POST(self):
        n = int(self.headers["Content-Length"]); body = json.loads(self.rfile.read(n))
        if self.path == "/tokenize":
            return self._send({"tokens": list(range(len(body["content"].split())))})
        k = sum(1 for m in body["messages"] if m["role"] == "assistant")
        txt = REPLIES[k]
        self._send({"choices": [{"message": {"role": "assistant", "content": txt}, "finish_reason": "stop"}],
                    "usage": {"prompt_tokens": 100 * (k + 1), "completion_tokens": len(txt.split())}, "timings": {"fake": True}})
HTTPServer(("127.0.0.1", port), H).serve_forever()
