"""`check` says what the product would refuse, not only what the binding gets wrong.

`check` judges a binding against a pack, in this package. The product judges the document -- its
structure and its design-time lints -- and answers through another tool (`validate_scxml`) that a
writer is not required to call. So the tool a writer iterates on until it says "no refusals" said so
for a document whose ten-second wait was armed as `delay="10000"`: no unit, so every backend raises
the argument error at the send and nothing is scheduled, and the position it guarded was written at
0 ms (measured 2026-10-12, a Qwen draft of `Washer_Low_Warning`, two cases of four).

Asserted with the real generator, each refusal beside the nearest document that must be accepted:

  refused      a document the product's lint refuses is refused by `check`, by the tool and by the
               command, with the product's code and the line it names
  accepted     the same document with the unit written says "no refusals", as before
  unasked      a product that cannot be asked is a NOTICE beside the answer, never silence and
               never a refusal: a tree with no generator still checks bindings
"""

from __future__ import annotations

import io
import json
import os
import pathlib
import shutil
import tempfile
import unittest
from contextlib import redirect_stdout
from unittest import mock

import yaml

from sce_author import __main__ as cli
from sce_author import mcp
from sce_author.pack import load_pack
from sce_author.verify import _default_codegen, product_findings

from tests.test_a_statechart_is_driven_not_called import BINDING, CROSSING, DELAYED, EXAMPLES

CODE = "scxml/send-delay-invalid"
NOT_RUN = "was not run"


def call(name: str, **arguments) -> dict:
    request = {"jsonrpc": "2.0", "id": 1, "method": "tools/call",
               "params": {"name": name, "arguments": arguments}}
    output = io.StringIO()
    mcp.serve(io.StringIO(json.dumps(request) + "\n"), output)
    return json.loads(output.getvalue())["result"]


class _Folder:
    """A pack, a statechart that waits and the binding that names it, in a folder of their own."""

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.tmp = pathlib.Path(self._tmp.name)
        for name in ("interface-model.yaml", "conventions.yaml"):
            shutil.copy(CROSSING / name, self.tmp / name)
        (self.tmp / "examples.yaml").write_text(yaml.safe_dump(EXAMPLES), encoding="utf-8")
        occupied = {"address": "plant/in/train-approach", "becomes": "OCCUPIED",
                    "event": "train.occupied"}
        # The position is written only when the wait ends, which `check` asks a binding to say
        # with `hold_last` and a `when_nothing_sent` its map leaves out.
        road = {**BINDING["outputs"]["roadSignal"], "hold_last": True,
                "when_nothing_sent": "signal.none"}
        (self.tmp / "b.yaml").write_text(yaml.safe_dump(
            {**BINDING, "inputs": {**BINDING["inputs"], "occupied": occupied},
             "outputs": {"roadSignal": road}}), encoding="utf-8")

    def document(self, delay: str) -> None:
        text = DELAYED.replace('delay="500ms"', f'delay="{delay}"')
        self.assertIn(f'delay="{delay}"', text)
        (self.tmp / "signal.scxml").write_text(text, encoding="utf-8")

    def tool(self) -> dict:
        return call("check", pack=str(self.tmp), binding=str(self.tmp / "b.yaml"))

    def command(self, *more: str) -> tuple[int, str]:
        output = io.StringIO()
        with redirect_stdout(output):
            rc = cli.main(["check", "--pack", str(self.tmp), "--binding", str(self.tmp / "b.yaml"),
                           *more])
        return rc, output.getvalue()


@unittest.skipUnless(_default_codegen().exists(), "the product's code generator is not built")
class CheckCarriesTheProductsRefusals(_Folder, unittest.TestCase):
    def test_a_delay_with_no_unit_is_refused_by_the_tool_with_the_products_code_and_line(self):
        self.document("500")
        sent = next(n for n, line in enumerate((self.tmp / "signal.scxml").read_text().splitlines(), 1)
                    if "<send" in line)
        result = self.tool()
        text = result["content"][0]["text"]
        self.assertTrue(result.get("isError"), text)
        self.assertIn(f"document signal.scxml line {sent}: {CODE}", text)

    def test_the_command_refuses_it_the_same_way(self):
        self.document("500")
        rc, text = self.command()
        self.assertEqual(1, rc, text)
        self.assertIn(CODE, text)
        self.assertIn("1 refusal(s)", text)

    # The nearest document that must be accepted: the same wait, written so the product reads it.
    def test_the_same_wait_with_a_unit_is_accepted_by_both(self):
        self.document("500ms")
        result = self.tool()
        self.assertFalse(result.get("isError"), result["content"][0]["text"])
        self.assertTrue(result["content"][0]["text"].startswith("no refusals"))
        rc, text = self.command()
        self.assertEqual((0, True), (rc, "0 refusal(s)" in text), text)

    def test_what_the_bindings_own_check_refuses_is_still_refused(self):
        """The product's answer is added to the binding's, not put in its place."""
        self.document("500ms")
        (self.tmp / "b.yaml").write_text(yaml.safe_dump(
            {**BINDING, "inputs": {"ghost": {"address": "plant/in/nowhere", "event": "x"}}}),
            encoding="utf-8")
        rc, text = self.command()
        self.assertEqual(1, rc, text)
        self.assertIn("not in the interface model", text)


class AProductThatCannotBeAskedIsSaidSo(_Folder, unittest.TestCase):
    """No generator is a notice. These run without one, and must not need one."""

    def setUp(self):
        super().setUp()
        self.document("500ms")
        missing = self.tmp / "no-such-generator"
        patched = mock.patch.dict(os.environ, {"SCE_CODEGEN": str(missing)})
        patched.start()
        self.addCleanup(patched.stop)

    def test_the_tool_still_answers_and_says_the_product_was_not_asked(self):
        result = self.tool()
        text = result["content"][0]["text"]
        self.assertFalse(result.get("isError"), text)
        self.assertTrue(text.startswith("no refusals"), text)
        self.assertIn(NOT_RUN, text)

    def test_the_command_exits_clean_and_says_so_beside_the_count(self):
        rc, text = self.command()
        self.assertEqual(0, rc, text)
        self.assertIn("0 refusal(s)", text)
        self.assertIn(NOT_RUN, text)

    def test_the_finding_function_returns_a_notice_and_no_finding(self):
        findings, notice = product_findings(self.tmp / "signal.scxml")
        self.assertEqual([], findings)
        self.assertIn(NOT_RUN, notice)

    def test_a_named_generator_is_the_one_asked(self):
        findings, notice = product_findings(self.tmp / "signal.scxml",
                                            self.tmp / "another-missing-generator")
        self.assertEqual(([], True), (findings, "another-missing-generator" in notice))


if __name__ == "__main__":
    unittest.main()
