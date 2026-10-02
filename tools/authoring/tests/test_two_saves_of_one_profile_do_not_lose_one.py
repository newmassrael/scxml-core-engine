"""Two saves into one profile never both say `saved` while one of them is lost.

A profile that every drafter under it reads is saved by reading it, adding rules to
what was read, checking that nobody changed it meanwhile (its digest), and renaming
the result over it. Measured 2026-10-02 with two real processes held between the
check and the rename: both passed the check, both answered `saved`, and the file
kept the later one's rules only. The check and the rename now happen together under
a lock on the directory, so the second save checks the file the first one wrote and
is refused like any save from a profile that has changed.

The other half is the read: the rules are added to the bytes read and the save is
held to the digest of the same bytes. Read twice, a change between the two reads
put the new file's digest beside rules added to the old one, and the save passed.
"""

from __future__ import annotations

import hashlib
import json
import multiprocessing
import os
import pathlib
import tempfile
import time
import unittest
from unittest import mock

from sce_author import house_rule, mcp
from sce_author.verify import _default_codegen

# The tool holds the profile it would write to the product's own reader before it
# saves it, so the case that goes through the tool needs the generator. The domain-
# free job builds nothing on purpose and skips it there.
needs_the_generator = unittest.skipUnless(_default_codegen().exists(),
                                          "the product's generator is not built")

#: How long the first save waits, inside its rename, for the second to have asked.
#: Long enough for a save with no lock to finish its whole check and rename.
GRACE_SECONDS = 0.6
WAIT_SECONDS = 20.0


def _profile(*ids: str) -> str:
    return json.dumps({"record": "sce-authoring-profile", "v": 1, "name": "shared",
                       "house_rules": [{"id": i, "rule": f"rule {i}"} for i in ids]}, indent=2)


def _wait_for(path: pathlib.Path) -> None:
    deadline = time.monotonic() + WAIT_SECONDS
    while not path.exists():
        if time.monotonic() > deadline:
            raise RuntimeError(f"{path.name} never appeared")
        time.sleep(0.01)


def _first(out, text, base, control, results) -> None:
    """The save that is already past its check when the second one asks."""
    real = os.replace

    def held_at_the_rename(source, target):
        (control / "first-at-rename").write_text("x")
        _wait_for(control / "second-asked")
        time.sleep(GRACE_SECONDS)
        real(source, target)

    os.replace = held_at_the_rename
    try:
        house_rule.save(out, text, base)
        results.put(("first", "saved", ""))
    except house_rule.HouseRuleError as error:
        results.put(("first", "refused", str(error)))


def _second(out, text, base, control, results) -> None:
    _wait_for(control / "first-at-rename")
    (control / "second-asked").write_text("x")
    try:
        house_rule.save(out, text, base)
        results.put(("second", "saved", ""))
    except house_rule.HouseRuleError as error:
        results.put(("second", "refused", str(error)))


class TwoSavesFromOneRevisionTest(unittest.TestCase):
    @unittest.skipUnless(hasattr(os, "fork"), "the two saves are separate forked processes")
    def test_the_second_save_is_refused_and_the_first_one_is_kept(self):
        with tempfile.TemporaryDirectory() as name:
            work = pathlib.Path(name)
            out = work / "shared-profile.json"
            base = _profile("H1")
            out.write_text(base, encoding="utf-8")
            digest = hashlib.sha256(base.encode()).hexdigest()
            control = work / "control"
            control.mkdir()
            context = multiprocessing.get_context("fork")
            results = context.Queue()
            processes = [
                context.Process(target=_first, args=(
                    out, _profile("H1", "H2"), digest, control, results)),
                context.Process(target=_second, args=(
                    out, _profile("H1", "H3"), digest, control, results)),
            ]
            for process in processes:
                process.start()
            for process in processes:
                process.join(60)
                self.assertEqual(0, process.exitcode)
            answers = {who: (said, why) for who, said, why in
                       (results.get(timeout=5) for _ in processes)}

            self.assertEqual("saved", answers["first"][0], answers)
            self.assertEqual("refused", answers["second"][0], answers)
            self.assertIn("changed since you read it", answers["second"][1])
            kept = [rule["id"] for rule in json.loads(out.read_text())["house_rules"]]
            self.assertEqual(["H1", "H2"], kept, "the first save's rule is not lost")
            self.assertEqual([], [p.name for p in work.iterdir()
                                  if p.name.startswith(".")], "nothing is left beside it")

    def test_a_lock_that_cannot_be_taken_refuses_the_save_and_writes_nothing(self):
        with tempfile.TemporaryDirectory() as name:
            work = pathlib.Path(name)
            out = work / "profile.json"
            from sce_author import filelock

            def unavailable(directory, wait_seconds=0):
                raise filelock.LockUnavailable("no lock here")

            with mock.patch.object(filelock, "exclusive", unavailable):
                with self.assertRaises(house_rule.HouseRuleError) as raised:
                    house_rule.save(out, _profile("H1"), None)
            self.assertIn("nothing was written", str(raised.exception))
            self.assertFalse(out.exists())


@needs_the_generator
class OneReadOfTheProfileTest(unittest.TestCase):
    def test_a_change_between_two_reads_cannot_be_overwritten(self):
        """The profile changes right after the tool first reads it. Rules were
        added to what was read, so the save is held to what was read: it finds the
        file changed and refuses. Two reads would hold it to the changed file's
        digest and write over the change."""
        with tempfile.TemporaryDirectory() as name:
            work = pathlib.Path(name)
            profile = work / "profile.json"
            profile.write_text(_profile("H1"), encoding="utf-8")
            changed = _profile("H1", "H9")
            reads = []
            real_bytes, real_text = pathlib.Path.read_bytes, pathlib.Path.read_text

            def after_the_first_read(path):
                if path == profile and not reads:
                    reads.append(path)
                    pathlib.Path.write_text(profile, changed, encoding="utf-8")

            def read_bytes(self):
                data = real_bytes(self)
                after_the_first_read(self)
                return data

            def read_text(self, *args, **kwargs):
                data = real_text(self, *args, **kwargs)
                after_the_first_read(self)
                return data

            with mock.patch.object(pathlib.Path, "read_bytes", read_bytes), \
                    mock.patch.object(pathlib.Path, "read_text", read_text):
                result = mcp.call_tool("scxml_house_rule", {
                    "rules": [{"quote": "just ignore it", "rule": "Unknown events are ignored."}],
                    "owner_words_text": "When the screen shows nothing, just ignore it.",
                    "owner_confirmed": True, "profile": str(profile), "out": str(profile)})
            self.assertTrue(result.get("isError"), result)
            self.assertIn("changed since you read it", result["content"][0]["text"])
            self.assertEqual(changed, profile.read_text(encoding="utf-8"),
                             "the change made after the read is still there")


if __name__ == "__main__":
    unittest.main()
