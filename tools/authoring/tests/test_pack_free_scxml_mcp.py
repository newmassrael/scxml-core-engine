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
        self.assertEqual(("accepted", "check"),
                         (report["verdict"], report["manifest"]["kind"]))

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
        self.assertEqual([str(pathlib.Path(sys.executable)), "--error-format", "json",
                          "check", str(self.document), "--lint"],
                         seen["argv"])
        report = json.loads(result["content"][0]["text"])
        self.assertEqual(("accepted", {"kind": "check"}, []),
                         (report["verdict"], report["manifest"], report["diagnostics"]))

    def test_every_record_reaches_the_caller_as_data(self):
        # The product reports every lint finding in one run; a caller that
        # got the first line of prose saw one problem and fixed one.
        records = [{"code": "scxml/unreachable-state"},
                   {"code": "scxml/always-false-guard"}]
        stderr = "".join(json.dumps(r) + "\n" for r in records)

        def fake_run(argv, **kwargs):
            return subprocess.CompletedProcess(argv, 20, stdout="", stderr=stderr)

        with mock.patch.object(verify.subprocess, "run", fake_run), \
             mock.patch.object(verify, "_default_codegen",
                               lambda: pathlib.Path(sys.executable)):
            result = call("validate_scxml", document=str(self.document))
        self.assertTrue(result.get("isError"))
        report = json.loads(result["content"][0]["text"])
        self.assertEqual(("refused", None, records),
                         (report["verdict"], report["manifest"], report["diagnostics"]))

    def test_a_record_on_an_accepted_run_is_not_dropped(self):
        # SCE_ERROR_CONTRACT.md §1: records, exit 0, manifest.
        record = {"code": "validation/refused-expression"}

        def fake_run(argv, **kwargs):
            return subprocess.CompletedProcess(argv, 0, stdout='{"kind":"check"}\n',
                                               stderr=json.dumps(record) + "\n")

        with mock.patch.object(verify.subprocess, "run", fake_run), \
             mock.patch.object(verify, "_default_codegen",
                               lambda: pathlib.Path(sys.executable)):
            result = call("validate_scxml", document=str(self.document))
        self.assertFalse(result.get("isError"))
        self.assertEqual([record],
                         json.loads(result["content"][0]["text"])["diagnostics"])

    def _argv_of(self, name: str, stdout: str = "", **arguments):
        """The generator command line a tool builds, and its answer."""
        seen = {}

        def fake_run(argv, **kwargs):
            seen["argv"] = argv
            return subprocess.CompletedProcess(argv, 0, stdout=stdout, stderr="")

        with mock.patch.object(verify.subprocess, "run", fake_run), \
             mock.patch.object(verify, "_default_codegen",
                               lambda: pathlib.Path(sys.executable)):
            result = call(name, **arguments)
        self.assertFalse(result.get("isError"), result["content"][0]["text"])
        return seen["argv"][1:], result["content"][0]["text"]

    def test_each_report_tool_reaches_its_product_command(self):
        # Every one runs under --error-format json, so its refusals come
        # back as records, and passes its names through unchecked.
        doc, manifest = str(self.document), str(self.document)
        cases = [
            ("render_scxml_diagram",
             dict(document=doc, out="figs", page="a3-landscape", min_pt=8, lexicon="ko",
                  manifest=manifest),
             ["diagram", doc, "-o", "figs", "--page", "a3-landscape",
              "--min-pt", "8.0", "--lexicon", "ko", "--manifest", manifest]),
            ("scxml_unresolved", dict(document=doc), ["unresolved", doc]),
            ("scxml_requirements", dict(document=doc, manifest=manifest),
             ["requirements", doc, "--manifest", manifest]),
            ("scxml_acceptance_report",
             dict(document=doc, manifest=manifest, variant="base"),
             ["acceptance-report", doc, "--manifest", manifest, "--variant", "base"]),
            ("scxml_accept",
             dict(document=doc, manifest=manifest, variant="base", root=".", out="acc.json"),
             ["accept", doc, "--manifest", manifest, "--variant", "base",
              "--root", ".", "--out", "acc.json"]),
            ("scxml_acceptance_check", dict(record=doc, variant="base", root="."),
             ["acceptance-check", doc, "--variant", "base", "--root", "."]),
        ]
        for name, arguments, expected in cases:
            with self.subTest(name=name):
                argv, _ = self._argv_of(name, **arguments)
                self.assertEqual(["--error-format", "json", *expected], argv)

    def test_ndjson_output_comes_back_as_records(self):
        marker = {"kind": "unresolved", "id": "T_Idle"}
        _, text = self._argv_of("scxml_unresolved", stdout=json.dumps(marker) + "\n",
                                document=str(self.document))
        self.assertEqual({"verdict": "done", "markers": [marker], "diagnostics": []},
                         json.loads(text))

    def test_a_lapsed_acceptance_is_an_answer_not_an_error(self):
        record = {"code": "cli/acceptance-lapsed", "message": "variant moved"}

        def fake_run(argv, **kwargs):
            return subprocess.CompletedProcess(argv, 20, stdout="",
                                               stderr=json.dumps(record) + "\n")

        with mock.patch.object(verify.subprocess, "run", fake_run), \
             mock.patch.object(verify, "_default_codegen",
                               lambda: pathlib.Path(sys.executable)):
            result = call("scxml_acceptance_check", record=str(self.document),
                          variant="other", root=".")
        self.assertFalse(result.get("isError"))
        report = json.loads(result["content"][0]["text"])
        self.assertEqual(("lapsed", [record]), (report["verdict"], report["diagnostics"]))

    def test_the_acceptance_tool_says_it_needs_the_owner(self):
        tool = next(t for t in mcp.TOOLS if t["name"] == "scxml_accept")
        self.assertIn("owner", tool["description"])

    @unittest.skipUnless(_default_codegen().exists(),
                         "the product's code generator is not built")
    def test_real_product_draws_the_figures(self):
        out = self.document.parent / "figures"
        result = call("render_scxml_diagram", document=str(self.document), out=str(out))
        self.assertFalse(result.get("isError"), result["content"][0]["text"])
        report = json.loads(result["content"][0]["text"])
        self.assertEqual([str(out / "document.svg")], report["figures"])
        self.assertTrue((out / "document.svg").is_file())

    @unittest.skipUnless(_default_codegen().exists(),
                         "the product's code generator is not built")
    def test_real_product_accepts_and_the_acceptance_holds(self):
        root = pathlib.Path(__file__).resolve().parents[3]
        fixtures = root / "sce-build" / "tests" / "fixtures" / "requirement_closure"
        document = fixtures / "doip_nl_connection_states.scxml"
        manifest = fixtures / "iso13400_2_nl_socket_handling.manifest.json"
        record = self.document.parent / "acceptance.json"
        accepted = call("scxml_accept", document=str(document), manifest=str(manifest),
                        variant="base", root=str(root), out=str(record))
        self.assertFalse(accepted.get("isError"), accepted["content"][0]["text"])
        self.assertTrue(record.is_file())
        for variant, verdict in (("base", "holds"), ("other", "lapsed")):
            with self.subTest(variant=variant):
                checked = call("scxml_acceptance_check", record=str(record),
                               variant=variant, root=str(root))
                self.assertFalse(checked.get("isError"), checked["content"][0]["text"])
                self.assertEqual(verdict, json.loads(checked["content"][0]["text"])["verdict"])

    def test_missing_document_is_a_refusal(self):
        for name in ("validate_scxml", "render_scxml_pseudocode", "scxml_unresolved"):
            with self.subTest(name=name):
                result = call(name, document=str(self.document.parent / "missing.scxml"))
                self.assertTrue(result.get("isError"))
                self.assertIn("missing.scxml", result["content"][0]["text"])


if __name__ == "__main__":
    unittest.main()
