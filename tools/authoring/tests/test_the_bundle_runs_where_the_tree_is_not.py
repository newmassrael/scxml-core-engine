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
from sce_author.works import default_work_binary

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
                         "SCE_WORK", "PYTHONPATH"}}


def ask_launcher(bundle: pathlib.Path, env: dict, *requests: dict) -> list[dict]:
    """Start the bundle's launcher in an unrelated directory and read its replies.

    ⚠ Without bytecode written: a run that imports the package leaves
    `__pycache__` beside it, and the case that holds the bundle to carrying none
    would then pass or fail by which of the cases ran first."""
    with tempfile.TemporaryDirectory() as elsewhere:
        run = subprocess.run(
            [str(bundle / "bin" / "sce-author-mcp")],
            input="".join(json.dumps(r) + "\n" for r in requests),
            capture_output=True, text=True, cwd=elsewhere, timeout=300,
            env={**env, "PYTHONDONTWRITEBYTECODE": "1"})
    return [json.loads(line) for line in run.stdout.splitlines() if line.strip()]


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
                     "python/sce_author/mcp.py", "python/schema/decisions.v1.schema.json",
                     "LICENSE", "README.txt"):
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
            # ⚠ The decision record is read against a schema that sits
            # BESIDE the package, not in it; a bundle carrying only the
            # package refused every record it was handed.
            {"jsonrpc": "2.0", "id": 4, "method": "tools/call",
             "params": {"name": "decisions", "arguments": {
                 "document_text": DOOR, "document_name": "door.scxml",
                 "decisions_text": json.dumps({
                     "record": "sce-decision-record", "v": 1,
                     "specification": {"doc_id": "door"}, "decisions": []})}}},
        ]
        with tempfile.TemporaryDirectory() as elsewhere:
            run = subprocess.run(
                [str(self.bundle / "bin" / "sce-author-mcp")],
                input="".join(json.dumps(r) + "\n" for r in requests),
                capture_output=True, text=True, cwd=elsewhere, env=clean_env(),
                timeout=300)
        replies = [json.loads(line) for line in run.stdout.splitlines() if line.strip()]
        self.assertEqual([1, 2, 3, 4], [r["id"] for r in replies], run.stderr)
        held = replies[3]["result"]
        self.assertFalse(held.get("isError"), held)
        self.assertEqual("holds", json.loads(held["content"][0]["text"])["verdict"])
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

    def test_a_bundle_without_the_works_command_says_so_and_does_not_borrow_one(self):
        """The works tools run `sce-work`. A bundle packaged without one must not
        find this tree's build by walking up from where it was unpacked: it answers
        `unavailable`, naming the way to have one."""
        replies = ask_launcher(
            self.bundle, clean_env(),
            {"jsonrpc": "2.0", "id": 1, "method": "tools/call",
             "params": {"name": "works_list", "arguments": {}}})
        answer = replies[0]["result"]
        self.assertTrue(answer.get("isError"), answer)
        refusal = json.loads(answer["content"][0]["text"])
        self.assertEqual("unavailable", refusal["refused"])
        self.assertIn("SCE_WORK", refusal["message"])


@unittest.skipUnless(_default_codegen().exists() and default_work_binary().is_file(),
                     "the generator and sce-work are not both built")
class TheBundleWithTheWorksCommand(unittest.TestCase):
    def test_it_carries_sce_work_and_its_launcher_reads_the_folder_the_application_opens(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = pathlib.Path(tmp) / "dist"
            made = subprocess.run(
                ["bash", str(PACKAGE), str(out), "--codegen", str(_default_codegen()),
                 "--work", str(default_work_binary())],
                capture_output=True, text=True, env=clean_env())
            self.assertEqual(0, made.returncode, made.stderr)
            bundle = pathlib.Path(made.stdout.split()[0])
            self.assertTrue((bundle / "bin" / "sce-work").is_file())

            # The application's own command makes the work; the launcher, started
            # somewhere else, lists it from the same folder.
            env = {**clean_env(), "SCE_WORKS_DIR": str(pathlib.Path(tmp) / "works")}
            created = subprocess.run(
                [str(bundle / "bin" / "sce-work"), "call", "create_work",
                 "--args", json.dumps({"title": "Door lock"})],
                capture_output=True, text=True, env=env)
            self.assertEqual(0, created.returncode, created.stderr)
            work = json.loads(created.stdout)["id"]

            replies = ask_launcher(
                bundle, env,
                {"jsonrpc": "2.0", "id": 1, "method": "tools/call",
                 "params": {"name": "works_list", "arguments": {}}})
        listed = replies[0]["result"]
        self.assertFalse(listed.get("isError"), listed)
        self.assertEqual([work], [w["id"] for w in
                                  json.loads(listed["content"][0]["text"])["works"]])

    def test_a_named_sce_work_that_is_not_one_is_refused_before_a_bundle_is_made(self):
        with tempfile.TemporaryDirectory() as tmp:
            made = subprocess.run(
                ["bash", str(PACKAGE), str(pathlib.Path(tmp) / "dist"), "--codegen",
                 str(_default_codegen()), "--work", str(pathlib.Path(tmp) / "absent")],
                capture_output=True, text=True, env=clean_env())
        self.assertNotEqual(0, made.returncode)
        self.assertIn("is not an executable sce-work", made.stderr)


if __name__ == "__main__":
    unittest.main()
