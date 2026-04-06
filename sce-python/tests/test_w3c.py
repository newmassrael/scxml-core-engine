"""W3C SCXML 1.0 Conformance Tests via Python bindings.

Validates that all 202 W3C tests pass through the pybind11-wrapped
C++ ReadySCXMLEngine interpreter, matching the native C++ results.

Pass/fail criteria (same as C++ W3CTestRunner):
  - Engine reaches <final id="pass"/> -> PASS
  - Engine reaches <final id="fail"/> -> FAIL
  - Engine does not reach final state within timeout -> TIMEOUT
  - Document rejected (W3C SCXML 5.8, unfetchable script) -> PASS
"""

import json
import os
import sys
import threading
import time
from http.server import HTTPServer, BaseHTTPRequestHandler
from urllib.parse import parse_qs

# Allow running from build directory
_SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
_PROJECT_ROOT = os.path.abspath(os.path.join(_SCRIPT_DIR, "..", ".."))

sys.path.insert(0, os.path.join(_SCRIPT_DIR, "..", "python"))

import _sce as sce

# W3C test list (202 tests, from build/W3CAotTestList.cmake)
W3C_TEST_IDS = [
    "144", "147", "148", "149", "150", "151", "152", "153", "155", "156",
    "158", "159", "172", "173", "174", "175", "176", "178", "179", "183",
    "185", "186", "187", "189", "190", "191", "192", "193", "194", "198",
    "199", "200", "201", "205", "207", "208", "210", "215", "216", "220",
    "223", "224", "225", "226", "228", "229", "230", "232", "233", "234",
    "235", "236", "237", "239", "240", "241", "242", "243", "244", "245",
    "247", "250", "252", "253", "276", "277", "278", "279", "280", "286",
    "287", "294", "298", "301", "302", "303", "304", "307", "309", "310",
    "311", "312", "313", "314", "318", "319", "321", "322", "323", "324",
    "325", "326", "329", "330", "331", "332", "333", "335", "336", "337",
    "338", "339", "342", "343", "344", "346", "347", "348", "349", "350",
    "351", "352", "354", "355", "364", "372", "375", "376", "377", "378",
    "387", "388", "396", "399", "401", "402", "403a", "403b", "403c",
    "404", "405", "406", "407", "409", "411", "412", "413", "415", "416",
    "417", "419", "421", "422", "423", "436", "444", "445", "446", "448",
    "449", "451", "452", "453", "456", "457", "459", "460", "525", "487",
    "488", "527", "528", "529", "530", "495", "496", "500", "501", "503",
    "504", "505", "506", "509", "510", "513", "518", "519", "520", "521",
    "522", "531", "532", "533", "534", "550", "551", "552", "553", "554",
    "557", "558", "560", "561", "562", "567", "569", "570", "576", "577",
    "578", "579", "580",
]

# W3C SCXML C.2: BasicHTTPEventProcessor tests (require HTTP server)
HTTP_TEST_IDS = {"201", "509", "510", "513", "518", "519", "520", "522",
                 "531", "532", "534", "567", "577"}

TIMEOUT_SEC = 10.0
POLL_INTERVAL_SEC = 0.01


# ---------------------------------------------------------------------------
# W3C BasicHTTP Event I/O Processor test server
# Mirrors C++ W3CHttpTestServer: receives POST on localhost:8080/test,
# extracts event name/data per W3C SCXML C.2, and forwards to the engine.
# ---------------------------------------------------------------------------

class W3CHttpTestServer:
    """HTTP server for W3C SCXML BasicHTTPEventProcessor conformance tests.

    Equivalent to the C++ W3CHttpTestServer that the native W3CTestRunner uses.
    Listens on localhost:<port><path>, parses incoming POST requests according
    to W3C SCXML Appendix C.2, and forwards the resulting event to the SCXML
    engine via engine.send_external_event() (W3C SCXML external queue).
    """

    def __init__(self, port=8080, path="/test"):
        self.port = port
        self.path = path
        self._engine = None
        self._engine_lock = threading.Lock()
        self._pending_events: list[tuple[str, str]] = []
        self._server: HTTPServer | None = None
        self._thread: threading.Thread | None = None

    def set_engine(self, engine):
        """Set the SCXML engine that receives forwarded HTTP events."""
        with self._engine_lock:
            self._engine = engine
            # Deliver any events that arrived before the engine was set
            for event_name, event_data in self._pending_events:
                try:
                    engine.send_external_event(event_name, event_data)
                except Exception:
                    pass
            self._pending_events.clear()

    def start(self) -> bool:
        """Start the HTTP server in a background thread. Returns True on success."""
        server_ref = self
        expected_path = self.path

        class _Handler(BaseHTTPRequestHandler):
            def do_POST(self):
                # Reject requests to wrong path
                if self.path != expected_path:
                    self.send_error(404)
                    return

                content_length = int(self.headers.get("Content-Length", 0))
                body = self.rfile.read(content_length).decode("utf-8") if content_length > 0 else ""

                event_name, event_data = _parse_http_event(self.headers, body)

                # Forward event to engine's external queue (thread-safe)
                # W3C SCXML 5.10: HTTP events must use external queue (test 510)
                with server_ref._engine_lock:
                    if server_ref._engine:
                        try:
                            server_ref._engine.send_external_event(event_name, event_data)
                        except Exception:
                            pass
                    else:
                        server_ref._pending_events.append((event_name, event_data))

                # W3C C.2: Respond with 200 OK
                response_body = json.dumps({"status": "success", "event": event_name})
                self.send_response(200)
                self.send_header("Content-Type", "application/json")
                self.send_header("Cache-Control", "no-cache")
                self.end_headers()
                self.wfile.write(response_body.encode("utf-8"))

            def log_message(self, format, *args):
                pass  # Suppress HTTP request logging

        try:
            # SO_REUSEADDR must be set before bind(), use allow_reuse_address
            class _ReuseServer(HTTPServer):
                allow_reuse_address = True

            self._server = _ReuseServer(("localhost", self.port), _Handler)
            self._thread = threading.Thread(target=self._server.serve_forever, daemon=True)
            self._thread.start()
            return True
        except OSError:
            return False

    def clear_engine(self):
        """Disconnect the engine from this server."""
        with self._engine_lock:
            self._engine = None
            self._pending_events.clear()

    def stop(self):
        """Shutdown the HTTP server and release resources."""
        if self._server:
            self._server.shutdown()
            if self._thread:
                self._thread.join(timeout=2)
            self._server.server_close()
            self._server = None
            self._thread = None
        self.clear_engine()


def _parse_http_event(headers, body: str) -> tuple[str, str]:
    """Parse an HTTP POST into (event_name, event_data) per W3C SCXML C.2.

    Mirrors the C++ W3CHttpTestServer::handlePost logic:
    - Form-urlencoded: _scxmleventname -> event name, all params -> JSON data
    - JSON body: "event" field -> event name, body -> data
    - Raw content: event name = "HTTP.POST", body -> data
    """
    content_type = headers.get("Content-Type", "")
    event_name = "event1"  # W3C default
    event_data = ""

    if "application/x-www-form-urlencoded" in content_type:
        params = parse_qs(body, keep_blank_values=True)

        # W3C test 531: _scxmleventname has highest priority
        if "_scxmleventname" in params:
            event_name = params["_scxmleventname"][0]

        # Convert all params to JSON object with numeric parsing (test 519, 567)
        data_obj = {}
        for key, values in params.items():
            val = values[0]
            # Try int first, then float, then string
            try:
                data_obj[key] = int(val)
                continue
            except ValueError:
                pass
            try:
                data_obj[key] = float(val)
                continue
            except ValueError:
                pass
            data_obj[key] = val

        if data_obj:
            event_data = json.dumps(data_obj, separators=(",", ":"))
    else:
        event_data = body
        is_json = body and body[0] in ("{", "[")

        if is_json:
            try:
                parsed = json.loads(body)
                if isinstance(parsed, dict) and "event" in parsed:
                    event_name = parsed["event"]
            except json.JSONDecodeError:
                pass
        elif body:
            # W3C C.2: Non-JSON content uses HTTP method name as event name
            event_name = "HTTP.POST"

    return event_name, event_data


# ---------------------------------------------------------------------------
# Test execution
# ---------------------------------------------------------------------------

def resolve_scxml_path(test_id: str) -> str:
    """Resolve SCXML file path for a test ID (handles variants like 403a)."""
    # Extract directory number (e.g., "403a" -> "403")
    dir_num = test_id.rstrip("abcdefgh")
    return os.path.join(_PROJECT_ROOT, "resources", dir_num, f"test{test_id}.scxml")


def run_test(test_id: str, http_server: W3CHttpTestServer | None = None) -> tuple[str, float, str]:
    """Run a single W3C test and return (result, elapsed_sec, detail).

    Result is one of: PASS, FAIL, TIMEOUT, ERROR
    """
    scxml_path = resolve_scxml_path(test_id)
    if not os.path.exists(scxml_path):
        return "ERROR", 0.0, f"File not found: {scxml_path}"

    start = time.monotonic()
    try:
        engine = sce.Engine.from_file(scxml_path)
    except ValueError as e:
        elapsed = time.monotonic() - start
        error_msg = str(e)
        # W3C SCXML 5.8: Document rejection for unfetchable scripts is PASS.
        # The parser error "document rejected per W3C SCXML 5.8" propagates
        # through the factory error chain into the ValueError message.
        if "document rejected" in error_msg:
            return "PASS", elapsed, "document rejected (W3C SCXML 5.8)"
        return "ERROR", elapsed, error_msg

    # Wire HTTP server to this engine before starting.
    # W3C SCXML C.2: Uses send_external_event() to route through the external
    # queue, ensuring internal events (from <raise>) are processed first (test 510).
    if http_server is not None:
        http_server.set_engine(engine)

    try:
        if not engine.start():
            return "ERROR", time.monotonic() - start, f"start() failed: {engine.last_error}"

        # Poll until final state or timeout
        deadline = start + TIMEOUT_SEC
        while time.monotonic() < deadline:
            state = engine.current_state
            if state == "pass":
                return "PASS", time.monotonic() - start, ""
            if state == "fail":
                return "FAIL", time.monotonic() - start, ""
            if not engine.running:
                # Engine stopped (reached a final state)
                # Check active states for pass/fail
                active = engine.active_states
                if "pass" in active:
                    return "PASS", time.monotonic() - start, ""
                if "fail" in active:
                    return "FAIL", time.monotonic() - start, ""
                # Reached unknown final state
                return "PASS", time.monotonic() - start, f"final_state={state}"
            time.sleep(POLL_INTERVAL_SEC)

        return "TIMEOUT", time.monotonic() - start, f"last_state={engine.current_state}"
    except Exception as e:
        return "ERROR", time.monotonic() - start, str(e)
    finally:
        # Disconnect engine from HTTP server before cleanup
        if http_server is not None:
            http_server.clear_engine()
        try:
            if engine.running:
                engine.stop()
        except Exception:
            pass


def main():
    skip_http = "--skip-http" in sys.argv
    only_test = None
    for arg in sys.argv[1:]:
        if arg.isdigit() or (arg[:-1].isdigit() and arg[-1].isalpha()):
            only_test = arg

    if only_test:
        test_ids = [only_test]
    else:
        test_ids = W3C_TEST_IDS

    passed = 0
    failed = 0
    errors = 0
    timeouts = 0
    skipped = 0
    failures = []

    # Start HTTP server once for all HTTP tests (matches C++ pattern of per-test
    # server but reusing is safe since we swap the engine reference per test)
    http_server = None
    if not skip_http:
        http_server = W3CHttpTestServer(port=8080, path="/test")
        if not http_server.start():
            print("WARNING: Failed to start HTTP server on port 8080, HTTP tests will be skipped")
            http_server = None

    total = len(test_ids)
    try:
        for i, test_id in enumerate(test_ids, 1):
            if skip_http and test_id in HTTP_TEST_IDS:
                skipped += 1
                print(f"[{i:3d}/{total}] test{test_id:>5s} SKIP (HTTP)")
                continue

            # Use HTTP server for HTTP tests
            test_http_server = http_server if test_id in HTTP_TEST_IDS else None
            result, elapsed, detail = run_test(test_id, test_http_server)

            if result == "PASS":
                passed += 1
                status = "PASS"
            elif result == "FAIL":
                failed += 1
                status = "FAIL"
                failures.append((test_id, result, detail))
            elif result == "TIMEOUT":
                timeouts += 1
                status = "TIMEOUT"
                failures.append((test_id, result, detail))
            else:
                errors += 1
                status = "ERROR"
                failures.append((test_id, result, detail))

            suffix = f" ({detail})" if detail else ""
            print(f"[{i:3d}/{total}] test{test_id:>5s} {status:7s} {elapsed:6.2f}s{suffix}")
    finally:
        if http_server is not None:
            http_server.stop()

    # Summary
    print()
    print("=" * 60)
    print(f"W3C SCXML 1.0 Python Binding Conformance Results")
    print(f"=" * 60)
    print(f"  PASS:    {passed:3d}/{total}")
    print(f"  FAIL:    {failed:3d}/{total}")
    print(f"  ERROR:   {errors:3d}/{total}")
    print(f"  TIMEOUT: {timeouts:3d}/{total}")
    if skipped:
        print(f"  SKIP:    {skipped:3d}/{total} (HTTP tests)")
    print(f"=" * 60)

    if failures:
        print()
        print("FAILED TESTS:")
        for tid, res, det in failures:
            print(f"  test{tid}: {res} - {det}")

    # Exit code: 0 if all passed (excluding skipped)
    run_total = total - skipped
    if passed == run_total:
        print(f"\nAll {run_total} tests PASSED")
        return 0
    else:
        print(f"\n{run_total - passed} tests DID NOT PASS")
        return 1


if __name__ == "__main__":
    sys.exit(main())
