"""A statechart's answers leave by `<send>`, and the check has to look there.

Every other document kind here answers with a datamodel value: the binding
names an output, the document declares `<data sce:direction="out">`, and the
two are compared. A statechart does not work that way. What it produces for
anything outside itself leaves as a `<send>` to a host-served processor, which
is the W3C channel for reaching out of a machine (W3C SCXML 6.2) and the one a
real platform receives.

⚠ THE POINT IS WHAT HAPPENED WITHOUT IT. Asking the datamodel question of a
statechart answered `the document does not compute it` for an output the
document plainly writes -- so a correct binding was refused for being correct,
and the only way past the refusal was to bind something other than the real
channel. A check that can be satisfied only by misdescribing the document is
worse than no check: it teaches the shape it was built to prevent.

⚠⚠ The peer defect is on the input side and is quieter. A statechart is driven
by events, so a case that sends one nothing listens for leaves the machine
exactly where it was -- and every reading taken afterwards is a reading of a
machine that was never driven. Each case is still judged, against whatever the
document happened to hold, and the run reports a verdict about nothing.

Why this reads the SENDS rather than the configuration: a state is not an
output. Binding one would make a verification break when a state is renamed or
split while the behaviour is unchanged, which is an assertion about the
document's insides wearing the clothes of an assertion about its behaviour.
"""

from __future__ import annotations

import pathlib
import shutil
import tempfile
import unittest

import yaml

from sce_author.check import check, read_document
from sce_author.errors import PackError
from sce_author.pack import load_pack

HERE = pathlib.Path(__file__).resolve().parent
CROSSING = HERE / "fixtures" / "crossing"

# The same subject matter as the rest of this fixture -- its addresses and
# value spaces are the ones the interface model already declares -- written as
# a statechart instead of a computation. Held here rather than in `fixtures/`
# because it exists to be read by `check`, never generated from.
STATECHART = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" datamodel="ecmascript" initial="dark" sce:kind="statechart">
  <state id="dark">
    <transition event="train.approaching" target="flashing"/>
  </state>
  <state id="flashing">
    <onentry>
      <send event="signal.flashing" type="x-sce-host"/>
    </onentry>
    <transition event="train.cleared" target="dark"/>
  </state>
</scxml>
"""

# The same machine with its outward channel removed: the send that reached the
# host is now a targetless one, which W3C SCXML 6.2.4 keeps inside the
# session's own world. Nothing outside it can read the result.
INWARD_ONLY = STATECHART.replace(' type="x-sce-host"', "")

BINDING = {
    "version": 1,
    "document": "signal.scxml",
    "inputs": {
        "approaching": {
            "address": "plant/in/train-approach",
            "becomes": "APPROACHING",
            "event": "train.approaching",
        },
    },
    "outputs": {
        "roadSignal": {
            "address": "plant/out/road-signal",
            "field": "value",
            "sent": {"processor": "x-sce-host"},
            "when_nothing_sent": "signal.dark",
            "map": {"signal.dark": "DARK", "signal.flashing": "FLASHING"},
        },
    },
}


def _mutate(base, path, value):
    """A copy of `base` with one position changed, or removed on `None`."""
    import copy

    doc = copy.deepcopy(base)
    node = doc
    for step in path[:-1]:
        node = node[step]
    if value is None:
        node.pop(path[-1], None)
    else:
        node[path[-1]] = value
    return doc


class AStatechartAnswersThroughItsSends(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = pathlib.Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)
        for name in ("interface-model.yaml", "conventions.yaml"):
            shutil.copy(CROSSING / name, self.tmp / name)
        self.pack = load_pack(self.tmp)

    def bind(self, binding=None, document=STATECHART):
        (self.tmp / "signal.scxml").write_text(document, encoding="utf-8")
        path = self.tmp / "b.yaml"
        path.write_text(yaml.safe_dump(binding or BINDING), encoding="utf-8")
        return path

    def test_the_document_is_read_for_what_it_sends_outward(self):
        """Only the host-served send is a position anything outside reads."""
        (self.tmp / "signal.scxml").write_text(STATECHART, encoding="utf-8")
        document = read_document(self.tmp / "signal.scxml")
        self.assertEqual(document.sends, (("signal.flashing", "x-sce-host"),))
        self.assertEqual(document.events,
                         frozenset({"train.approaching", "train.cleared"}))

        (self.tmp / "inward.scxml").write_text(INWARD_ONLY, encoding="utf-8")
        inward = read_document(self.tmp / "inward.scxml")
        self.assertEqual(inward.sends, (), (
            "a send with no `type` is the SCXML processor's own and stays in "
            "the session; counting it would report an outward channel that "
            "does not exist"))

    def test_an_output_bound_to_a_send_is_accepted(self):
        """The defect this closes: the correct binding used to be refused."""
        findings = check(self.pack, self.bind())
        self.assertEqual(
            [str(f) for f in findings], [],
            "the document writes `plant/out/road-signal` by sending to the "
            "host, and the binding says so")

    def test_an_output_bound_to_a_send_the_document_never_makes_is_refused(self):
        """A rule may not name a channel the document does not have."""
        findings = check(self.pack, self.bind(document=INWARD_ONLY))
        self.assertTrue(
            any("sends none" in str(f) for f in findings),
            f"a send nothing outside can read is not the bound channel; "
            f"got {[str(f) for f in findings]}")

    def test_a_send_of_another_processor_type_does_not_answer_for_this_one(self):
        """`processor` narrows, and a narrowing that never narrows is a lie."""
        binding = _mutate(BINDING, ["outputs", "roadSignal", "sent"],
                          {"processor": "x-sce-other"})
        findings = check(self.pack, self.bind(binding))
        self.assertTrue(
            any("x-sce-other" in str(f) for f in findings),
            f"got {[str(f) for f in findings]}")

    def test_an_event_no_transition_listens_for_is_refused(self):
        """The quiet half: the case would drive a machine that ignores it."""
        binding = _mutate(BINDING, ["inputs", "approaching", "event"],
                          "train.arriving")
        findings = check(self.pack, self.bind(binding))
        self.assertTrue(
            any("no transition listens" in str(f) for f in findings),
            f"got {[str(f) for f in findings]}")

    def test_a_descriptor_matches_by_prefix_as_the_specification_says(self):
        """W3C SCXML 3.12.1 -- `train` selects `train.approaching`.

        An equality test here would refuse a document that plainly answers the
        event, and the author's only way out would be to write the descriptor
        the checker wanted rather than the one the machine needs.
        """
        document = STATECHART.replace('event="train.approaching"',
                                      'event="train"')
        findings = check(self.pack, self.bind(document=document))
        self.assertEqual([str(f) for f in findings], [])

    def test_a_symbol_becomes_cannot_take_is_refused(self):
        """`becomes` names a platform symbol and is wrong the way `equals` is."""
        binding = _mutate(BINDING, ["inputs", "approaching", "becomes"],
                          "ARRIVING")
        findings = check(self.pack, self.bind(binding))
        self.assertTrue(
            any("ARRIVING" in str(f) for f in findings),
            f"got {[str(f) for f in findings]}")

    def test_a_sent_output_must_say_what_nothing_sent_means(self):
        """Required for the reason `when_absent` is required for a number.

        Without it the case where the machine should have signalled and did
        not produces no value at all, and an output that merely vanishes is
        reported as a position nobody looked at -- which reads as a clean run.
        """
        binding = _mutate(BINDING,
                          ["outputs", "roadSignal", "when_nothing_sent"], None)
        with self.assertRaises(PackError) as caught:
            check(self.pack, self.bind(binding))
        self.assertIn("when_nothing_sent", str(caught.exception))


if __name__ == "__main__":
    unittest.main()
