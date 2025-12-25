import http.server
import json
import os
import socketserver
import sys
from urllib.parse import parse_qs, urlparse

PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 18080
FIXTURES_DIR = os.path.dirname(os.path.abspath(__file__))


class CopilotHttpRequestHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        parsed = urlparse(self.path)
        path = parsed.path
        query = parse_qs(parsed.query)

        # Handle /copilot/get/{task_id} - single task
        if path.startswith("/copilot/get/"):
            task_id = path.split("/")[-1]
            file_path = os.path.join(FIXTURES_DIR, "tasks", f"{task_id}.json")
            if os.path.exists(file_path):
                with open(file_path, "r", encoding="utf-8") as f:
                    task_content = f.read()

                # Wrap the task content in the expected API response structure
                response_data = {"status_code": 200, "data": {"content": task_content}}
                self.send_response(200)
                self.send_header("Content-Type", "application/json")
                self.send_header("Access-Control-Allow-Origin", "*")
                self.end_headers()
                self.wfile.write(
                    json.dumps(response_data, ensure_ascii=False).encode("utf-8")
                )
            else:
                self.send_response(404)
                self.send_header("Content-Type", "application/json")
                self.end_headers()
                self.wfile.write(b'{"status_code": 404, "message": "Task not found"}')

        # Handle /set/get?id={set_id} - copilot set
        elif path == "/set/get":
            set_id = query.get("id", [None])[0]
            if set_id is None:
                self.send_response(400)
                self.send_header("Content-Type", "application/json")
                self.end_headers()
                self.wfile.write(
                    b'{"status_code": 400, "message": "Missing id parameter"}'
                )
                return

            file_path = os.path.join(FIXTURES_DIR, "sets", f"{set_id}.json")
            if os.path.exists(file_path):
                with open(file_path, "r", encoding="utf-8") as f:
                    set_data = f.read()

                self.send_response(200)
                self.send_header("Content-Type", "application/json")
                self.send_header("Access-Control-Allow-Origin", "*")
                self.end_headers()
                self.wfile.write(set_data.encode("utf-8"))
            else:
                self.send_response(404)
                self.send_header("Content-Type", "application/json")
                self.end_headers()
                self.wfile.write(b'{"status_code": 404, "message": "Set not found"}')
        else:
            self.send_response(404)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(b'{"status_code": 404, "message": "Not found"}')

    def log_message(self, format, *args):
        # Suppress default logging to keep test output clean
        pass


class ReuseAddrTCPServer(socketserver.TCPServer):
    allow_reuse_address = True


if __name__ == "__main__":
    with ReuseAddrTCPServer(("127.0.0.1", PORT), CopilotHttpRequestHandler) as httpd:
        print(f"Serving copilot test API at http://127.0.0.1:{PORT}", flush=True)
        httpd.serve_forever()
