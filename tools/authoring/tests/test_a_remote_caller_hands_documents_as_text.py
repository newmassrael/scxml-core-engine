"""A caller that is not on this machine hands its documents over as text.

The pack-free tools took nothing but paths, so the only client that could
use them was one that started this server beside the owner's files. An
assistant reached through an API, or a hosted one, holds the specification
and the drafts somewhere else. These cases hold the text form to what the
path form answers, the HTTP transport to the stdio one, and the remote
server to its refusals: a path from a remote caller names this machine's
files, and it is refused rather than read.
"""

from __future__ import annotations

import http.client
import io
import json
import pathlib
import tempfile
import threading
import unittest

from sce_author import mcp, mcp_http
from sce_author.verify import _default_codegen

ROOT = pathlib.Path(__file__).resolve().parents[3]
SCHEMAS = ROOT / "sce-build" / "tests" / "fixtures" / "event_schema"
BUILT = unittest.skipUnless(_default_codegen().exists(),
                            "the product's code generator is not built")

DOOR = ('<scxml xmlns="http://www.w3.org/2005/07/scxml" '
        'xmlns:sce="http://sce.dev/ext" sce:kind="statechart" version="1.0" '
        'initial="closed">\n'
        '  <state id="closed"><transition event="open" target="opened"/></state>\n'
        '  <state id="opened"><transition event="close" target="closed"/></state>\n'
        '</scxml>\n')


def call(name: str, remote: bool = False, **arguments) -> dict:
    return mcp.call_tool(name, arguments, remote=remote)


def body(answer: dict) -> str:
    return answer["content"][0]["text"]


class TheTextForm(unittest.TestCase):
    @BUILT
    def test_a_document_as_text_is_read_under_the_name_it_was_given(self):
        checked = call("validate_scxml", document_text=DOOR, document_name="door.scxml")
        self.assertFalse(checked.get("isError"), body(checked))
        self.assertEqual("statechart",
                         json.loads(body(checked))["manifest"]["document_kind"]["name"])
        page = call("render_scxml_pseudocode", document_text=DOOR, document_name="door.scxml")
        self.assertFalse(page.get("isError"), body(page))
        # The document's name is its stem, not a temporary file's.
        self.assertTrue(body(page).startswith("machine door "), body(page))

    @BUILT
    def test_a_refusal_names_the_document_not_where_it_was_staged(self):
        broken = DOOR.replace('target="opened"', 'target="nowhere"')
        refused = call("validate_scxml", document_text=broken, document_name="door.scxml")
        self.assertTrue(refused.get("isError"))
        records = json.loads(body(refused))["diagnostics"]
        self.assertEqual("door.scxml", records[0]["location"]["file"], records[0])

    @BUILT
    def test_figures_without_a_directory_come_back_as_svg(self):
        drawn = call("render_scxml_diagram", document_text=DOOR, document_name="door.scxml")
        self.assertFalse(drawn.get("isError"), body(drawn))
        figures = json.loads(body(drawn))["figures"]
        self.assertEqual(["document.svg"], [f["name"] for f in figures])
        self.assertTrue(figures[0]["svg"].lstrip().startswith("<"), figures[0]["svg"][:80])

    def test_a_name_that_leaves_the_staging_directory_is_refused(self):
        for name in ("../door.scxml", "sub/door.scxml", "..", ""):
            with self.subTest(name=name):
                refused = call("validate_scxml", document_text=DOOR, document_name=name)
                self.assertTrue(refused.get("isError"))
                self.assertIn("plain file name", body(refused))

    def test_both_forms_at_once_is_refused(self):
        refused = call("validate_scxml", document="x.scxml", document_text=DOOR)
        self.assertTrue(refused.get("isError"))
        self.assertIn("not both", body(refused))


class ASet(unittest.TestCase):
    """Documents that import one another, checked as one set."""

    def texts(self, *names):
        return [{"name": n, "text": (SCHEMAS / n).read_text(encoding="utf-8")} for n in names]

    @BUILT
    def test_an_import_resolves_among_the_documents_handed_over(self):
        docs = self.texts("mesh_receiver_matched.scxml", "schema_job_completed_minimal.scxml")
        checked = call("validate_scxml_set", documents_text=docs)
        self.assertFalse(checked.get("isError"), body(checked))
        self.assertEqual("accepted", json.loads(body(checked))["verdict"])

        # The schema is READ, not merely present: a send whose payload
        # contradicts it is refused with the schema beside it. (Without a
        # schema a statechart keeps the schemaless `_event.data`, so its
        # absence alone would prove nothing.)
        wrong = self.texts("negative_statechart_send_type_mismatch.scxml",
                           "schema_job_completed_minimal.scxml")
        refused = call("validate_scxml_set", documents_text=wrong)
        self.assertTrue(refused.get("isError"), body(refused))
        self.assertEqual("validation/cross-kind-type-mismatch",
                         json.loads(body(refused))["diagnostics"][0]["code"])

    @BUILT
    def test_the_set_by_path_answers_as_the_set_by_text(self):
        names = ("mesh_receiver_matched.scxml", "schema_job_completed_minimal.scxml")
        by_path = json.loads(body(call("validate_scxml_set",
                                       documents=[str(SCHEMAS / n) for n in names])))
        by_text = json.loads(body(call("validate_scxml_set", documents_text=self.texts(*names))))
        self.assertEqual(by_path["verdict"], by_text["verdict"])
        self.assertEqual(by_path["manifest"]["languages"], by_text["manifest"]["languages"])


class ARemoteCaller(unittest.TestCase):
    def test_a_path_is_refused(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = pathlib.Path(tmp) / "door.scxml"
            path.write_text(DOOR, encoding="utf-8")
            for name, args in (("validate_scxml", {"document": str(path)}),
                               ("validate_scxml_set", {"documents": [str(path)]}),
                               ("render_scxml_diagram", {"document_text": DOOR, "out": tmp})):
                with self.subTest(name=name):
                    refused = call(name, remote=True, **args)
                    self.assertTrue(refused.get("isError"))
                    self.assertIn("cannot reach", body(refused))

    def test_a_pack_tool_is_not_offered(self):
        refused = call("brief", remote=True, pack="p", prose=["x"])
        self.assertTrue(refused.get("isError"))
        self.assertIn("not offered to a remote caller", body(refused))

    @BUILT
    def test_the_text_form_is_what_a_remote_caller_uses(self):
        checked = call("validate_scxml", remote=True, document_text=DOOR,
                       document_name="door.scxml")
        self.assertFalse(checked.get("isError"), body(checked))


class OverHttp(unittest.TestCase):
    """The same answers over HTTP, behind its three guards."""

    def serve(self, token):
        server = mcp_http.make_server("127.0.0.1", 0, token)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        self.addCleanup(server.server_close)
        self.addCleanup(server.shutdown)
        return server.server_address[1]

    def post(self, port, payload, headers=None):
        connection = http.client.HTTPConnection("127.0.0.1", port, timeout=60)
        self.addCleanup(connection.close)
        data = json.dumps(payload).encode()
        connection.request("POST", "/mcp", data,
                           {"Content-Type": "application/json", **(headers or {})})
        response = connection.getresponse()
        return response.status, response.read().decode()

    def test_it_speaks_the_protocol_and_answers_as_stdio_does(self):
        port = self.serve(None)
        status, text = self.post(port, {"jsonrpc": "2.0", "id": 1, "method": "initialize",
                                        "params": {"protocolVersion": "2025-06-18"}})
        self.assertEqual(200, status)
        self.assertEqual("2025-06-18", json.loads(text)["result"]["protocolVersion"])

        status, text = self.post(port, {"jsonrpc": "2.0", "method": "notifications/initialized"})
        self.assertEqual((202, ""), (status, text))

        request = {"jsonrpc": "2.0", "id": 2, "method": "tools/list"}
        status, text = self.post(port, request)
        output = io.StringIO()
        mcp.serve(io.StringIO(json.dumps(request) + "\n"), output)
        self.assertEqual(json.loads(output.getvalue()), json.loads(text))

    def test_a_remote_request_may_not_name_a_path(self):
        port = self.serve(None)
        status, text = self.post(port, {
            "jsonrpc": "2.0", "id": 3, "method": "tools/call",
            "params": {"name": "validate_scxml", "arguments": {"document": "/etc/hostname"}}})
        self.assertEqual(200, status)
        result = json.loads(text)["result"]
        self.assertTrue(result["isError"])
        self.assertIn("cannot reach", result["content"][0]["text"])

    def test_a_token_is_required_when_one_is_set(self):
        port = self.serve("s3cret")
        ping = {"jsonrpc": "2.0", "id": 4, "method": "ping"}
        self.assertEqual(401, self.post(port, ping)[0])
        self.assertEqual(401, self.post(port, ping, {"Authorization": "Bearer wrong"})[0])
        self.assertEqual(200, self.post(port, ping, {"Authorization": "Bearer s3cret"})[0])

    def test_a_foreign_origin_is_refused(self):
        port = self.serve(None)
        ping = {"jsonrpc": "2.0", "id": 5, "method": "ping"}
        self.assertEqual(403, self.post(port, ping, {"Origin": "http://evil.example"})[0])
        self.assertEqual(200, self.post(port, ping, {"Origin": f"http://127.0.0.1:{port}"})[0])

    def test_a_network_address_without_a_token_is_refused(self):
        with self.assertRaises(SystemExit):
            mcp_http.make_server("0.0.0.0", 0, None)


if __name__ == "__main__":
    unittest.main()
