"""A subject matter is several documents, and nothing could ask about the set.

`check` and `verify` are each handed ONE binding. Each is right about the
document it was given, and neither can see the shape of the set that document
belongs to. So a conversion that needed five components and produced three
reports green: the three that exist check, run and pass, and the two that were
never written are not absent from any list -- there was no list.

⚠ THE POINT IS THE SILENCE, NOT THE FIGURE. A missing document has no
binding, so no command runs against it, so nothing reports it. The failure is
invisible in the one way that matters: it looks exactly like success.

⚠⚠ The two things this reports are different in kind, and the exit status
tells them apart. An unwritten position is a STATUS -- an unfinished
conversion looks precisely like that -- and alarming on it would be an alarm
every piece of work in progress trips. A position TWO documents write cannot
be right whatever the platform is: one field would take two answers, and
which stood would be decided by whichever component ran last.
"""

from __future__ import annotations

import pathlib
import shutil
import tempfile
import unittest

import yaml

from sce_author.coverage import coverage
from sce_author.pack import load_pack
from sce_author.verify import written_positions

HERE = pathlib.Path(__file__).resolve().parent
CROSSING = HERE / "fixtures" / "crossing"
WHOLE = CROSSING / "controller_resolved.binding.yaml"


class TheSetOfDocumentsIsMeasuredToo(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = pathlib.Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)
        for name in ("interface-model.yaml", "conventions.yaml"):
            shutil.copy(CROSSING / name, self.tmp / name)
        self.pack = load_pack(self.tmp)
        self.whole = yaml.safe_load(WHOLE.read_text(encoding="utf-8"))

    def write(self, name: str, outputs: dict) -> pathlib.Path:
        path = self.tmp / name
        path.write_text(yaml.safe_dump({
            "version": 1, "document": "d.scxml", "outputs": outputs,
        }), encoding="utf-8")
        return path

    def only(self, *names) -> dict:
        return {n: self.whole["outputs"][n] for n in names}

    def test_one_document_covering_everything_leaves_nothing_unwritten(self):
        got = coverage(self.pack, [WHOLE])
        self.assertEqual([], got.unwritten)
        self.assertEqual([], got.alarms())
        self.assertEqual(len(got.positions), got.covered)

    def test_the_positions_no_document_writes_are_named(self):
        """The defect: three of five written, and every command still green."""
        part = self.write("part.yaml", self.only("roadSignal", "bell"))
        got = coverage(self.pack, [part])
        self.assertEqual(2, got.covered)
        self.assertEqual(["plant/out/barrier.value", "plant/out/train-signal.value"],
                         got.unwritten)
        self.assertEqual([], got.alarms(),
                         "unfinished work is a status, not a shape that "
                         "cannot be right -- an alarm here would fire on "
                         "every conversion in progress")

    def test_two_documents_writing_one_position_is_an_alarm(self):
        """One field cannot take two answers, whatever the platform is."""
        a = self.write("a.yaml", self.only("roadSignal", "bell"))
        b = self.write("b.yaml", self.only("bell", "barrier"))
        got = coverage(self.pack, [a, b])
        self.assertEqual([("plant/out/bell.value", ["a.yaml", "b.yaml"])],
                         got.contested())
        self.assertEqual(1, len(got.alarms()))
        self.assertIn("a.yaml, b.yaml", got.alarms()[0])

    def test_two_documents_between_them_cover_what_neither_does(self):
        a = self.write("a.yaml", self.only("roadSignal", "bell"))
        b = self.write("b.yaml", self.only("barrier", "trainSignal"))
        got = coverage(self.pack, [a, b])
        self.assertEqual([], got.unwritten)
        self.assertEqual([], got.alarms())

    def test_an_internal_or_unresolved_output_reaches_no_position(self):
        """Neither is a document reaching an address, so neither covers one.

        ⚠ BOTH RULES CARRY AN ADDRESS HERE, AND THAT IS THE WHOLE TEST. The
        schema does not forbid it -- `internal` and `unresolved` are the
        binding saying "not published" and "nobody has said where yet", and
        an address written beside either is a contradiction it accepts. With
        the addresses left off, a rule falls out one guard earlier for having
        no address at all, and a version of this case that omitted them
        passed against a build with the check deleted: it was measuring the
        wrong line.
        """
        part = self.write("part.yaml", {
            **self.only("roadSignal"),
            "held": {"address": "plant/out/bell", "field": "value",
                     "internal": True},
            "later": {"address": "plant/out/barrier", "field": "value",
                      "unresolved": "the platform list is not available yet"},
        })
        got = coverage(self.pack, [part])
        self.assertEqual(1, got.covered)
        self.assertIn("plant/out/bell.value", got.unwritten)
        self.assertIn("plant/out/barrier.value", got.unwritten)

    def test_a_position_the_model_does_not_declare_is_kept_apart(self):
        """Otherwise a mistyped field arrives as a document that is missing.

        `check` refuses this against the binding that wrote it. Counting it
        here as coverage would hide it, and counting it as unwritten would
        send a reader looking for a component nobody ever needed.
        """
        part = self.write("part.yaml", {
            **self.only("roadSignal"),
            "typo": {"address": "plant/out/bell", "field": "valeu",
                     "map": {0: "SILENT"}},
        })
        got = coverage(self.pack, [part])
        self.assertEqual(["plant/out/bell.valeu"], got.undeclared)
        self.assertIn("plant/out/bell.value", got.unwritten)
        self.assertEqual(1, got.covered)


# An event slot: a status, the identifier beside it, and a sound only while it
# is on. Three positions at one address -- the shape the crossing's one-field
# outputs never had, and so the shape this counting was never shown.
ALARM = {"address": "plant/out/alarm", "role": "output", "names": ["OUT_Alarm"],
         "fields": {"Stat": {"values": {"OFF": 1, "ON": 2}},
                    "ID": {"type": "number"},
                    "Sound": {"type": "number"}}}

ALARM_RULE = {"address": "plant/out/alarm", "field": "Stat",
              "map": {True: "ON", False: "OFF"},
              "also": {"ID": 7},
              "when": {True: {"Sound": 3}}}


class WhatABindingWritesIsCountedAsVerifyCountsIt(unittest.TestCase):
    """`also` and `when` fields are positions written, here as in `verify`.

    ⚠ This count used to read `address.field` alone. A binding writing an
    event's identifier with `also` had it reported unwritten by `coverage`
    while `verify`, on the same binding, judged it -- two tools of one core
    contradicting each other about one file. Measured 2026-09-23: a binding
    with `also` and one without it were reported identically.
    """

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = pathlib.Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)
        model = yaml.safe_load((CROSSING / "interface-model.yaml").read_text(encoding="utf-8"))
        model["entries"].append(ALARM)
        (self.tmp / "interface-model.yaml").write_text(yaml.safe_dump(model), encoding="utf-8")
        shutil.copy(CROSSING / "conventions.yaml", self.tmp / "conventions.yaml")
        self.pack = load_pack(self.tmp)

    def write(self, rule: dict) -> pathlib.Path:
        path = self.tmp / "alarm.yaml"
        path.write_text(yaml.safe_dump({
            "version": 1, "document": "d.scxml", "outputs": {"alarm": rule},
        }), encoding="utf-8")
        return path

    def alarm_positions_written(self, rule: dict) -> set:
        got = coverage(self.pack, [self.write(rule)])
        return {p for p in got.written if p.startswith("plant/out/alarm.")}

    def test_the_identifier_written_with_also_is_covered(self):
        self.assertIn("plant/out/alarm.ID", self.alarm_positions_written(ALARM_RULE))

    def test_a_field_written_only_when_on_is_covered(self):
        self.assertIn("plant/out/alarm.Sound", self.alarm_positions_written(ALARM_RULE))

    def test_without_them_only_the_status_is(self):
        """The discriminator: the same rule stripped of `also` and `when`."""
        bare = {k: v for k, v in ALARM_RULE.items() if k not in ("also", "when")}
        self.assertEqual({"plant/out/alarm.Stat"}, self.alarm_positions_written(bare))

    def test_the_count_is_the_one_verify_judges_with(self):
        bound, _ = written_positions({"alarm": ALARM_RULE})
        self.assertEqual(bound, self.alarm_positions_written(ALARM_RULE))


if __name__ == "__main__":
    unittest.main()
