#!/usr/bin/env python3
"""Disposable Anthropic SSE fixture for one shell command in a container.

The model asks for one foreground Bash command (argv[1]) and then answers
"PROBE:" followed by what the command printed. No credentials or outside
calls. Prints the port it listens on, on 127.0.0.1.
"""
import http.server
import json
import sys

COMMAND = sys.argv[1]
TOOL_ID = "container-probe"


def tool_output(messages):
    for message in messages:
        content = message.get("content")
        if not isinstance(content, list):
            continue
        for block in content:
            if isinstance(block, dict) and block.get("type") == "tool_result" and block.get("tool_use_id") == TOOL_ID:
                value = block.get("content")
                if isinstance(value, list):
                    value = "".join(part.get("text", "") for part in value if isinstance(part, dict))
                return str(value)
    return None


class Handler(http.server.BaseHTTPRequestHandler):
    def log_message(self, *_):
        pass

    def do_POST(self):
        data = json.loads(self.rfile.read(int(self.headers.get("Content-Length", "0"))))
        if "count_tokens" in self.path:
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(b'{"input_tokens":17}')
            return
        tools = [tool.get("name") for tool in data.get("tools", [])]
        output = tool_output(data.get("messages", []))
        if "Bash" in tools and output is None:
            blocks = [{"type": "tool_use", "id": TOOL_ID, "name": "Bash",
                       "input": {"command": COMMAND, "description": "Probe the container"}}]
            stop = "tool_use"
        else:
            blocks = [{"type": "text", "text": "PROBE:" + (output or "none")}]
            stop = "end_turn"
        message = {"id": f"msg_{len(data.get('messages', []))}", "type": "message", "role": "assistant",
                   "model": data.get("model", "claude-opus-5"), "content": blocks,
                   "stop_reason": stop, "stop_sequence": None,
                   "usage": {"input_tokens": 17, "output_tokens": 9, "cache_read_input_tokens": 0,
                             "cache_creation_input_tokens": 0}}
        self.send_response(200)
        if not data.get("stream"):
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps(message).encode())
            return
        self.send_header("Content-Type", "text/event-stream")
        self.end_headers()

        def event(kind, value):
            self.wfile.write(f'event: {kind}\ndata: {json.dumps({"type": kind, **value})}\n\n'.encode())
            self.wfile.flush()
        event("message_start", {"message": dict(message, content=[], stop_reason=None)})
        for index, block in enumerate(blocks):
            if block["type"] == "tool_use":
                event("content_block_start", {"index": index, "content_block": dict(block, input={})})
                event("content_block_delta", {"index": index, "delta": {"type": "input_json_delta",
                                                                         "partial_json": json.dumps(block["input"])}})
            else:
                event("content_block_start", {"index": index, "content_block": {"type": "text", "text": ""}})
                event("content_block_delta", {"index": index, "delta": {"type": "text_delta", "text": block["text"]}})
            event("content_block_stop", {"index": index})
        event("message_delta", {"delta": {"stop_reason": stop, "stop_sequence": None}, "usage": {"output_tokens": 9}})
        event("message_stop", {})


server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
print(server.server_address[1], flush=True)
server.serve_forever()
