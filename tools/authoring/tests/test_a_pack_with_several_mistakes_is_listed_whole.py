"""A pack with several mistakes is listed whole, not one round trip at a time.

Every loader refused at the first problem, so a pack with three mistakes cost
three runs to learn about. A pack is prepared by the people who build it and
handed to a specification owner already verified, so the person preparing it
needs every problem at once, and needs to be told what could not be checked
because something it depends on did not load.

The loader is one implementation and serves both callers: `load_pack` still
refuses at the first problem in the sentence it always used, and `check_pack`
keeps the same sentences and goes on. These cases hold the two to each other.
"""

from __future__ import annotations

import contextlib
import io
import pathlib
import shutil
import tempfile
import unittest

import yaml

from sce_author.__main__ import main
from sce_author.errors import PackError
from sce_author.pack import check_pack, load_pack

HERE = pathlib.Path(__file__).resolve().parent
FIXTURE = HERE / "fixtures" / "crossing"


class Pack(unittest.TestCase):
    """A copy of the fixture pack, which a case then spoils in places."""

    def setUp(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = pathlib.Path(temporary.name) / "pack"
        shutil.copytree(FIXTURE, self.root)

    def edit(self, name: str, change) -> None:
        path = self.root / name
        document = yaml.safe_load(path.read_text(encoding="utf-8"))
        change(document)
        path.write_text(yaml.safe_dump(document, sort_keys=False), encoding="utf-8")

    def spoil_conventions(self) -> None:
        """Four mistakes of four kinds in one file, and one in the examples."""
        def conventions(document):
            document["name_classes"].append({"pattern": "(", "role": "supplied"})
            pre = document["preconditions"]
            pre["phrases"]["broken"] = "poweredUp &&& ("
            pre["phrases"]["ghost"] = "ghostInput"
            pre["inputs"]["poweredUp"] = {
                "note": "the installation is energised",
                "rule": {"address": "plant/in/ghost", "equals": "OK"}}

        def examples(document):
            document["cases"][0]["elapsed_ms"] = {"min": 9, "max": 1}

        self.edit("conventions.yaml", conventions)
        self.edit("examples.yaml", examples)


class TheWholePackIsListed(Pack):
    def test_a_sound_pack_is_clean(self):
        report = check_pack(self.root)
        self.assertEqual([], [str(p) for p in report.problems])
        self.assertEqual([], report.skipped)
        self.assertTrue(report.clean)

    def test_every_mistake_in_every_file_is_listed_once(self):
        self.spoil_conventions()
        said = [str(problem) for problem in check_pack(self.root).problems]
        wanted = ("is not a regular expression",           # the pattern
                  "preconditions.phrases 'broken'",         # the phrase that is no expression
                  "'ghostInput' is not declared",           # a name nobody declares
                  "reads address 'plant/in/ghost'",         # a rule the model cannot hold
                  "elapsed_ms window has min 9.0 above max 1.0")  # the examples file
        for fragment in wanted:
            with self.subTest(fragment=fragment):
                self.assertEqual(1, sum(fragment in line for line in said), said)
        self.assertEqual(len(wanted), len(said), said)

    def test_the_loader_that_refuses_names_the_first_of_them_in_the_same_words(self):
        """One implementation: what `load_pack` raises is what the check lists
        first, word for word, so a pack author sees the same sentence either way."""
        self.spoil_conventions()
        report = check_pack(self.root)
        with self.assertRaises(PackError) as caught:
            load_pack(self.root)
        self.assertEqual(str(report.problems[0]), str(caught.exception))

    def test_every_departure_from_a_files_schema_is_listed(self):
        def conventions(document):
            document["name_classes"][0]["role"] = "no_such_role"
            document["gate_off"] = "not a list"

        self.edit("conventions.yaml", conventions)
        said = [str(p) for p in check_pack(self.root).problems]
        self.assertGreaterEqual(len(said), 2, said)
        self.assertTrue(any("role" in line for line in said), said)
        self.assertTrue(any("gate_off" in line for line in said), said)


class WhatCouldNotBeCheckedIsSaid(Pack):
    def spoil_model(self) -> None:
        extra = self.root / "interface-model.d"
        extra.mkdir()
        # A key written twice: refused, not resolved, and the file is skipped.
        (extra / "more.yaml").write_text(
            "version: 1\nentries:\n  - address: plant/in/extra\n    role: input\n"
            "    role: output\n", encoding="utf-8")

        def duplicate(document):
            document["entries"].append(dict(document["entries"][0]))

        self.edit("interface-model.yaml", duplicate)

    def test_a_model_that_did_not_load_whole_does_not_accuse_the_rules(self):
        """A rule's address may be one declared by the file that failed. Holding
        the rules to a partial model would call a good rule wrong, so the check
        says it did not make that check instead."""
        self.spoil_model()
        self.edit("conventions.yaml", lambda d: d["preconditions"]["inputs"].update(
            {"poweredUp": {"note": "energised",
                           "rule": {"address": "plant/in/ghost", "equals": "OK"}}}))
        report = check_pack(self.root)
        said = [str(p) for p in report.problems]
        self.assertTrue(any("is already declared" in line for line in said), said)
        self.assertTrue(any("written twice" in line or "twice" in line for line in said), said)
        self.assertFalse(any("plant/in/ghost" in line for line in said), said)
        self.assertEqual(1, len(report.skipped), report.skipped)
        self.assertIn("did not load cleanly", report.skipped[0])
        self.assertFalse(report.clean)

    def test_a_file_that_would_not_load_is_not_also_called_an_empty_model(self):
        (self.root / "interface-model.yaml").write_text("entries: [", encoding="utf-8")
        said = [str(p) for p in check_pack(self.root).problems]
        self.assertEqual(1, len(said), said)
        self.assertIn("not well-formed", said[0])

    def test_two_missing_files_are_two_problems(self):
        (self.root / "interface-model.yaml").unlink()
        (self.root / "conventions.yaml").unlink()
        said = [str(p) for p in check_pack(self.root).problems]
        self.assertEqual(2, len(said), said)
        self.assertIn("no interface-model file", said[0])
        self.assertIn("no conventions file", said[1])

    def test_a_directory_that_is_not_there_is_one_problem(self):
        said = [str(p) for p in check_pack(self.root / "nowhere").problems]
        self.assertEqual(1, len(said), said)
        self.assertIn("not a directory", said[0])


class TheCommand(Pack):
    def run_command(self, root: pathlib.Path) -> tuple[int, str]:
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            status = main(["check-pack", "--pack", str(root)])
        return status, out.getvalue()

    def test_a_sound_pack_exits_zero(self):
        status, printed = self.run_command(self.root)
        self.assertEqual(0, status, printed)
        self.assertIn("no problem found", printed)

    def test_a_spoiled_pack_exits_one_and_numbers_every_problem(self):
        self.spoil_conventions()
        status, printed = self.run_command(self.root)
        self.assertEqual(1, status, printed)
        numbered = [line for line in printed.splitlines() if line.startswith("problem ")]
        self.assertEqual(5, len(numbered), printed)
        self.assertEqual([f"problem {n}:" for n in range(1, 6)],
                         [line.split(" ", 2)[0] + " " + line.split(" ", 2)[1] for line in numbered])
        self.assertIn("5 problem(s)", printed)

    def test_what_was_not_checked_is_printed_and_fails_the_pack(self):
        extra = self.root / "interface-model.d"
        extra.mkdir()
        (extra / "more.yaml").write_text("entries: [", encoding="utf-8")
        status, printed = self.run_command(self.root)
        self.assertEqual(1, status, printed)
        self.assertIn("not checked: ", printed)


if __name__ == "__main__":
    unittest.main()
