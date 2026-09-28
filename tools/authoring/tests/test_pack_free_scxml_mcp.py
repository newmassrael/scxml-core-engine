"""MCP can check and show an SCXML document without platform data."""

from __future__ import annotations

import io
import json
import pathlib
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

from sce_author import mcp, verify
from sce_author.verify import _default_codegen


def call(name: str, **arguments) -> dict:
    request = {"jsonrpc": "2.0", "id": 1, "method": "tools/call",
               "params": {"name": name, "arguments": arguments}}
    output = io.StringIO()
    mcp.serve(io.StringIO(json.dumps(request) + "\n"), output)
    return json.loads(output.getvalue())["result"]


class PackFreeScxmlMcp(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.document = pathlib.Path(temporary.name) / "client.scxml"
        self.document.write_text(
            '<scxml xmlns="http://www.w3.org/2005/07/scxml" '
            'version="1.0" initial="Idle">\n'
            '  <state id="Idle"/>\n'
            '</scxml>\n', encoding="utf-8")

    def test_tools_require_only_the_document(self):
        for name in ("validate_scxml", "render_scxml_pseudocode"):
            with self.subTest(name=name):
                tool = next(t for t in mcp.TOOLS if t["name"] == name)
                self.assertEqual(["document"], tool["inputSchema"]["required"])
                self.assertNotIn("pack", tool["inputSchema"]["properties"])
                self.assertNotIn("binding", tool["inputSchema"]["properties"])

    @unittest.skipUnless(_default_codegen().exists(),
                         "the product's code generator is not built")
    def test_real_product_checks_and_renders_without_a_pack(self):
        checked = call("validate_scxml", document=str(self.document))
        self.assertFalse(checked.get("isError"), checked["content"][0]["text"])
        report = json.loads(checked["content"][0]["text"])
        self.assertEqual("check", report["kind"])

        shown = call("render_scxml_pseudocode", document=str(self.document))
        self.assertFalse(shown.get("isError"), shown["content"][0]["text"])
        self.assertIn("state Idle:", shown["content"][0]["text"])

    @unittest.skipUnless(_default_codegen().exists(),
                         "the product's code generator is not built")
    def test_a_bad_state_reference_returns_the_generator_diagnostic(self):
        self.document.write_text(
            '<scxml xmlns="http://www.w3.org/2005/07/scxml" '
            'version="1.0" initial="Idle">\n'
            '  <state id="Idle"><transition target="MissingState"/></state>\n'
            '</scxml>\n', encoding="utf-8")
        result = call("validate_scxml", document=str(self.document))
        self.assertTrue(result.get("isError"))
        self.assertIn("MissingState", result["content"][0]["text"])

    def test_pseudo_options_reach_the_generator_without_a_binding(self):
        seen = {}

        def fake_run(argv, **kwargs):
            seen["argv"] = argv
            return subprocess.CompletedProcess(argv, 0, stdout="page\n", stderr="")

        with mock.patch.object(verify.subprocess, "run", fake_run), \
             mock.patch.object(verify, "_default_codegen",
                               lambda: pathlib.Path(sys.executable)):
            result = call("render_scxml_pseudocode", document=str(self.document),
                          shape="endmark", lexicon="ko")
        self.assertFalse(result.get("isError"))
        self.assertEqual("page\n", result["content"][0]["text"])
        self.assertEqual("pseudo", seen["argv"][1])
        self.assertEqual(str(self.document), seen["argv"][2])
        self.assertEqual(["--shape", "endmark", "--lexicon", "ko"],
                         seen["argv"][3:])

    def test_validation_reaches_the_generator_without_a_pack(self):
        seen = {}

        def fake_run(argv, **kwargs):
            seen["argv"] = argv
            return subprocess.CompletedProcess(argv, 0, stdout='{"kind":"check"}\n', stderr="")

        with mock.patch.object(verify.subprocess, "run", fake_run), \
             mock.patch.object(verify, "_default_codegen",
                               lambda: pathlib.Path(sys.executable)):
            result = call("validate_scxml", document=str(self.document))
        self.assertFalse(result.get("isError"))
        self.assertEqual([str(pathlib.Path(sys.executable)), "check", str(self.document)],
                         seen["argv"])

    def test_missing_document_is_a_refusal(self):
        for name in ("validate_scxml", "render_scxml_pseudocode"):
            with self.subTest(name=name):
                result = call(name, document=str(self.document.parent / "missing.scxml"))
                self.assertTrue(result.get("isError"))
                self.assertIn("missing.scxml", result["content"][0]["text"])


if __name__ == "__main__":
    unittest.main()
