"""The packaged server answers from its own directory, not from this tree.

`scripts/package_sce_author.sh` exists so a specification owner can use the
server without a checkout and a Rust build. What makes that true is that
the bundle carries its generator and the templates the generator renders,
and that its launcher names both -- a binary finds its templates in the tree
it was compiled in, and on an owner's machine there is none. These cases
build a bundle from the generator this tree already built, start its
launcher in an unrelated directory, and hold it to the protocol; and they
show the generator honours the bundle's templates over the tree's, by
naming an empty directory instead and watching it fail.
"""

from __future__ import annotations

import json
import os
import pathlib
import subprocess
import tempfile
import unittest

from sce_author.verify import _default_codegen

ROOT = pathlib.Path(__file__).resolve().parents[3]
PACKAGE = ROOT / "scripts" / "package_sce_author.sh"

DOOR = ('<scxml xmlns="http://www.w3.org/2005/07/scxml" '
        'xmlns:sce="http://sce.dev/ext" sce:kind="statechart" version="1.0" '
        'initial="closed">\n'
        '  <state id="closed"><transition event="open" target="opened"/></state>\n'
        '  <state id="opened"><transition event="close" target="closed"/></state>\n'
        '</scxml>\n')


def clean_env() -> dict:
    """The environment a fresh shell on the owner's machine would have: none
    of this tree's own settings."""
    return {k: v for k, v in os.environ.items()
            if k not in {"SCE_CODEGEN", "SCE_TEMPLATE_DIR", "SCE_WORKSPACE_ROOT",
                         "PYTHONPATH"}}


@unittest.skipUnless(_default_codegen().exists(),
                     "the product's code generator is not built")
class TheBundle(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        out = pathlib.Path(cls.tmp.name) / "dist"
        made = subprocess.run(["bash", str(PACKAGE), str(out), "--codegen",
                               str(_default_codegen())],
                              capture_output=True, text=True, env=clean_env())
        if made.returncode != 0:
            raise AssertionError(f"packaging failed:\n{made.stderr}")
        cls.bundle, cls.archive = (pathlib.Path(p) for p in made.stdout.split())

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def test_it_carries_what_it_runs_on(self):
        for part in ("bin/sce-author-mcp", "bin/sce-codegen", "share/sce/templates",
                     "python/sce_author/mcp.py", "LICENSE", "README.txt"):
            with self.subTest(part=part):
                self.assertTrue((self.bundle / part).exists(), part)
        self.assertTrue(self.archive.is_file())
        self.assertFalse(list((self.bundle / "python").rglob("__pycache__")))

    def test_its_launcher_answers_from_an_unrelated_directory(self):
        requests = [
            {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}},
            {"jsonrpc": "2.0", "id": 2, "method": "tools/call",
             "params": {"name": "validate_scxml",
                        "arguments": {"document_text": DOOR, "document_name": "door.scxml"}}},
            {"jsonrpc": "2.0", "id": 3, "method": "tools/call",
             "params": {"name": "scxml_kinds", "arguments": {"kind": "timer"}}},
        ]
        with tempfile.TemporaryDirectory() as elsewhere:
            run = subprocess.run(
                [str(self.bundle / "bin" / "sce-author-mcp")],
                input="".join(json.dumps(r) + "\n" for r in requests),
                capture_output=True, text=True, cwd=elsewhere, env=clean_env(),
                timeout=300)
        replies = [json.loads(line) for line in run.stdout.splitlines() if line.strip()]
        self.assertEqual([1, 2, 3], [r["id"] for r in replies], run.stderr)
        checked = replies[1]["result"]
        self.assertFalse(checked.get("isError"), checked)
        self.assertEqual("accepted", json.loads(checked["content"][0]["text"])["verdict"])
        catalog = json.loads(replies[2]["result"]["content"][0]["text"])["catalog"]
        self.assertEqual(["timer"], [k["name"] for k in catalog["kinds"]])

    def test_the_generator_renders_from_the_templates_it_is_given(self):
        """The bundle's templates are what the generator reads: named empty,
        the same check fails. Without this, the bundle could pass above by
        reading this tree's templates through the compile-time fallback."""
        with tempfile.TemporaryDirectory() as tmp:
            door = pathlib.Path(tmp) / "door.scxml"
            door.write_text(DOOR, encoding="utf-8")
            empty = pathlib.Path(tmp) / "no-templates"
            empty.mkdir()
            verdicts = {}
            for label, templates in (("bundled", self.bundle / "share/sce/templates"),
                                     ("empty", empty)):
                env = {**clean_env(), "SCE_TEMPLATE_DIR": str(templates)}
                run = subprocess.run(
                    [str(self.bundle / "bin" / "sce-codegen"), "check", str(door),
                     "-l", "rust"],
                    capture_output=True, text=True, env=env, cwd=tmp)
                verdicts[label] = run.returncode
        self.assertEqual(0, verdicts["bundled"])
        self.assertNotEqual(0, verdicts["empty"])


if __name__ == "__main__":
    unittest.main()
