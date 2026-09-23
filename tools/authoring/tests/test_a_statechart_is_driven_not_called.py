"""A statechart is verified by driving it, and reading what it sent.

`verify` could run one shape: a document whose generated form is one function
per output, taking this round's inputs by name. That shape is a pure
computation, and it is the minority of what SCXML is for. A statechart has no
such function to call -- it receives events, it holds states, and what it
produces for anything outside leaves as a `<send>` (W3C SCXML 6.2). So the
only command that says whether a document BEHAVES said nothing at all about
the product's main kind of document.

⚠ WHAT MAKES THIS A RUN RATHER THAN A CALL, and the thing most easily got
wrong: the machine is built ONCE and the cases are replayed through it. An
examples file is one run. What a case observes is partly the result of the
cases before it -- that is what states are -- so rebuilding between cases
would verify a machine that forgets, which is a different document.

⚠⚠ Every refusal here is a case that WOULD otherwise have been judged. Cases
that drive nothing the document listens for still produce a reading: whatever
the previous case left, attributed to this one. That reading is not a pass and
is not a failure; it is a verdict about a machine nobody drove, and the only
honest report of it is that it could not be judged.
"""

from __future__ import annotations

import pathlib
import shutil
import tempfile
import unittest
from dataclasses import dataclass, field

import yaml

from sce_author.errors import AuthoringError
from sce_author.pack import load_pack
from sce_author.verify import _default_codegen, sent_value, verify

HERE = pathlib.Path(__file__).resolve().parent
CROSSING = HERE / "fixtures" / "crossing"

# Two states, one outward channel, and two events off ONE address -- which is
# what `becomes` is for: the same reading drives a different event depending
# on the value it took.
DOCUMENT = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" datamodel="ecmascript" initial="dark"
       sce:kind="statechart">
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

# The same crossing with a lead delay: a signal does not start flashing the
# instant a train is detected, it arms and flashes 500 ms later. Nothing here
# can be judged without moving virtual time, which is the point.
DELAYED = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" datamodel="ecmascript" initial="dark"
       sce:kind="statechart">
  <state id="dark">
    <transition event="train.approaching" target="arming"/>
  </state>
  <state id="arming">
    <onentry>
      <send event="signal.flashing" type="x-sce-host" delay="500ms"/>
    </onentry>
    <transition event="train.occupied"/>
    <transition event="train.cleared" target="dark"/>
  </state>
</scxml>
"""

# A statechart whose answer is a variable it DECLARES an output, rather than
# something it sends. `sce:direction="out"` is the document saying this is part
# of its outward surface, and the generator emits a host-facing accessor for
# every such variable -- so reading one is reading the declared interface.
DECLARING = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" datamodel="ecmascript" initial="dark"
       sce:kind="statechart">
  <datamodel>
    <data id="roadSignal" sce:type="int32" sce:direction="out" expr="0"/>
  </datamodel>
  <state id="dark">
    <onentry><assign location="roadSignal" expr="0"/></onentry>
    <transition event="train.approaching" target="flashing"/>
  </state>
  <state id="flashing">
    <onentry><assign location="roadSignal" expr="1"/></onentry>
    <transition event="train.cleared" target="dark"/>
  </state>
</scxml>
"""

BINDING = {
    "version": 1,
    "document": "signal.scxml",
    "inputs": {
        "approaching": {"address": "plant/in/train-approach",
                        "becomes": "APPROACHING",
                        "event": "train.approaching"},
        "cleared": {"address": "plant/in/train-approach",
                    "becomes": "CLEAR",
                    "event": "train.cleared"},
    },
    "outputs": {
        "roadSignal": {
            "address": "plant/out/road-signal", "field": "value",
            "sent": {"processor": "x-sce-host"},
            # Nothing sent is the machine saying the position is unchanged
            # from its resting value, which for this document is dark.
            "when_nothing_sent": "signal.dark",
            "map": {"signal.dark": "DARK", "signal.flashing": "FLASHING"},
        },
    },
}

# A machine that answers only once it has heard BOTH of two inputs. Each event
# only records what it heard; the answer waits for the pair.
BOTH = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" datamodel="ecmascript" initial="waiting" sce:kind="statechart">
  <datamodel>
    <data id="train" expr="false"/>
    <data id="clear" expr="false"/>
  </datamodel>
  <state id="waiting">
    <transition event="train.approaching">
      <assign location="train" expr="true"/>
      <raise event="heard"/>
    </transition>
    <transition event="obstacle.none">
      <assign location="clear" expr="true"/>
      <raise event="heard"/>
    </transition>
    <transition event="heard" cond="train &amp;&amp; clear" target="flashing"/>
  </state>
  <state id="flashing">
    <onentry>
      <send event="signal.flashing" type="x-sce-host"/>
    </onentry>
  </state>
</scxml>
"""

EXAMPLES = {
    "version": 1,
    "origin": "written for this test",
    "independent_cases": True,
    "ordered": True,
    "cases": [
        {"name": "a train approaches",
         "given": {"plant/in/train-approach": "APPROACHING"},
         "drove": ["plant/in/train-approach"],
         "expect": {"plant/out/road-signal.value": "FLASHING"}},
        {"name": "the train clears",
         "given": {"plant/in/train-approach": "CLEAR"},
         "drove": ["plant/in/train-approach"],
         "expect": {"plant/out/road-signal.value": "DARK"}},
    ],
}


@dataclass
class FakeSend:
    """What the runtime hands a host handler, reduced to what is read here."""

    processor_type: str = "x-sce-host"
    event_name: str = ""
    content: str = ""
    params: dict = field(default_factory=dict)


def codegen_is_built() -> bool:
    return _default_codegen().exists()


class ReadingWhatWasSent(unittest.TestCase):
    """`sent_value` alone -- the part that needs no machine to exercise."""

    RULE = {"sent": {}, "when_nothing_sent": "nothing"}

    def test_with_no_send_the_declared_resting_value_stands(self):
        self.assertEqual("nothing", sent_value("out", self.RULE, []))

    def test_without_a_param_the_value_is_the_event_name(self):
        got = sent_value("out", self.RULE, [FakeSend(event_name="signal.on")])
        self.assertEqual("signal.on", got)

    def test_the_last_send_of_a_case_is_what_the_position_holds(self):
        """A machine crossing two states in one case sends twice."""
        got = sent_value("out", self.RULE, [FakeSend(event_name="a"),
                                            FakeSend(event_name="b")])
        self.assertEqual("b", got, "the first is a moment inside the case that "
                                   "no record claimed to be about")

    def test_a_send_of_another_processor_does_not_answer(self):
        rule = {"sent": {"processor": "x-sce-host"},
                "when_nothing_sent": "nothing"}
        got = sent_value("out", rule,
                         [FakeSend(processor_type="x-other", event_name="a")])
        self.assertEqual("nothing", got)

    def test_a_param_carries_the_value_when_the_rule_names_one(self):
        rule = {"sent": {"param": "lamp"}, "when_nothing_sent": "nothing"}
        got = sent_value("out", rule,
                         [FakeSend(params={"lamp": ["FLASHING"]})])
        self.assertEqual("FLASHING", got)

    def test_a_param_that_repeated_is_a_refusal_not_a_choice(self):
        """`<param>` may repeat under one name, so the value is a LIST."""
        rule = {"sent": {"param": "lamp"}, "when_nothing_sent": "nothing"}
        with self.assertRaises(AuthoringError) as caught:
            sent_value("out", rule, [FakeSend(params={"lamp": ["A", "B"]})])
        self.assertIn("2 time(s)", str(caught.exception))

    def test_content_is_read_when_the_rule_asks_for_it(self):
        rule = {"sent": {"content": True}, "when_nothing_sent": "nothing"}
        got = sent_value("out", rule, [FakeSend(content="42")])
        self.assertEqual("42", got)


@unittest.skipUnless(codegen_is_built(),
                     "the product's generator is not built; nothing can be run")
class AStatechartIsDrivenNotCalled(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = pathlib.Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)
        for name in ("interface-model.yaml", "conventions.yaml"):
            shutil.copy(CROSSING / name, self.tmp / name)
        (self.tmp / "signal.scxml").write_text(DOCUMENT, encoding="utf-8")

    def run_with(self, examples=None, binding=None):
        (self.tmp / "examples.yaml").write_text(
            yaml.safe_dump(examples or EXAMPLES), encoding="utf-8")
        path = self.tmp / "b.yaml"
        path.write_text(yaml.safe_dump(binding or BINDING), encoding="utf-8")
        return verify(load_pack(self.tmp), path)

    def test_the_cases_are_replayed_through_one_machine(self):
        """The whole point: driven, and judged on what it sent."""
        result = self.run_with()
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((2, 0, 0),
                         (result.passed, result.failed, result.unjudged),
                         [(r.name, r.refusal, r.failures) for r in result.results])

    def test_plain_scxml_with_no_kind_is_a_statechart(self):
        """⚠ A forge kind is opt-in, and the product reads a document that
        names none as a statechart. The core read the absent kind as '' and
        refused to drive it -- measured when an author wrote plain SCXML from
        a brief, `check` passed it and `verify` then refused it."""
        plain = DOCUMENT.replace('\n       sce:kind="statechart"', "")
        self.assertNotIn("sce:kind", plain)
        (self.tmp / "signal.scxml").write_text(plain, encoding="utf-8")
        result = self.run_with()
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((2, 0, 0),
                         (result.passed, result.failed, result.unjudged),
                         [(r.name, r.refusal, r.failures) for r in result.results])

    def test_a_record_that_writes_codes_drives_the_same_events(self):
        """⚠ The spelling the record keeps is not the binding's business.

        A platform's records write the CODE (`1`) where the binding names the
        symbol (`APPROACHING`). `becomes` compared the two spellings, so no
        event was ever sent and every case of such a document came back
        "could not be judged" -- first met on a component whose every input is
        an enumeration, which is the ordinary case. The model says which code
        a symbol is, as it already does for `equals`.
        """
        examples = {**EXAMPLES, "cases": [
            {**case, "given": {"plant/in/train-approach":
                               {"APPROACHING": "1", "CLEAR": "0"}[
                                   case["given"]["plant/in/train-approach"]]}}
            for case in EXAMPLES["cases"]]}
        result = self.run_with(examples)
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((2, 0, 0),
                         (result.passed, result.failed, result.unjudged),
                         [(r.name, r.refusal, r.failures) for r in result.results])

    def test_a_case_that_drove_two_addresses_sends_both_events(self):
        """⚠ One event per driven address, in the order the record drove them.

        The driver stopped at the first rule that sent, so a case that moved
        two inputs told the machine about one. This document answers only once
        BOTH have been heard, which is the shape of any rule written over two
        signals moving together -- a pair of switches, a pair of sensors.
        """
        (self.tmp / "signal.scxml").write_text(BOTH, encoding="utf-8")
        binding = {**BINDING, "inputs": {
            "approaching": {"address": "plant/in/train-approach",
                            "becomes": "APPROACHING",
                            "event": "train.approaching"},
            "clearOfObstacles": {"address": "plant/in/obstacle",
                                 "becomes": "NONE",
                                 "event": "obstacle.none"},
        }}
        examples = {**EXAMPLES, "cases": [
            {"name": "a train, and nothing on the crossing",
             "given": {"plant/in/train-approach": "APPROACHING",
                       "plant/in/obstacle": "NONE"},
             "drove": ["plant/in/obstacle", "plant/in/train-approach"],
             "expect": {"plant/out/road-signal.value": "FLASHING"}},
        ]}
        result = self.run_with(examples, binding)
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((1, 0, 0),
                         (result.passed, result.failed, result.unjudged),
                         [(r.name, r.refusal, r.failures) for r in result.results])

    def test_the_second_case_depends_on_what_the_first_one_left(self):
        """Rebuilding per case would hide this; replaying cannot.

        Driving `train.cleared` first leaves the machine in `dark`, where the
        second drive can reach `flashing`. Order is the subject, so the
        expectations are the ones that order produces.
        """
        examples = {**EXAMPLES, "cases": [
            {"name": "a clear reading first",
             "given": {"plant/in/train-approach": "CLEAR"},
             "drove": ["plant/in/train-approach"],
             "expect": {"plant/out/road-signal.value": "DARK"}},
            {"name": "then a train",
             "given": {"plant/in/train-approach": "APPROACHING"},
             "drove": ["plant/in/train-approach"],
             "expect": {"plant/out/road-signal.value": "FLASHING"}},
            {"name": "and it is still flashing until it clears",
             "given": {"plant/in/train-approach": "APPROACHING"},
             "drove": ["plant/in/train-approach"],
             # ⚠ Re-driving `train.approaching` in `flashing` selects no
             # transition, so nothing is sent and the resting value stands.
             # That is the document's answer, and it is what a machine with
             # memory looks like from outside.
             "expect": {"plant/out/road-signal.value": "DARK"}},
        ]}
        result = self.run_with(examples)
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual(3, result.passed,
                         [(r.name, r.refusal, r.failures) for r in result.results])

    def test_unordered_cases_are_refused_rather_than_replayed(self):
        examples = {**EXAMPLES, "ordered": False}
        result = self.run_with(examples)
        self.assertFalse(result.ran)
        self.assertIn("ordered", result.refusal)

    def test_a_case_that_drives_nothing_is_unjudged_not_passed(self):
        """Otherwise it reads whatever the case before it left."""
        examples = {**EXAMPLES, "cases": EXAMPLES["cases"] + [
            {"name": "a reading nothing turns into an event",
             "given": {"plant/in/obstacle": "DETECTED"},
             "drove": ["plant/in/obstacle"],
             "expect": {"plant/out/road-signal.value": "DARK"}},
        ]}
        result = self.run_with(examples)
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual(1, result.unjudged,
                         [(r.name, r.refusal) for r in result.results])
        self.assertEqual(2, result.passed)

    def test_an_output_on_neither_declared_channel_is_refused(self):
        """This document sends, and declares no output variable either.

        So the only place left to look is the active configuration, and a
        state is not an output: asserting on one would break when a state is
        renamed or split while the document went on doing the same thing.
        """
        binding = {**BINDING, "outputs": {"roadSignal": {
            "address": "plant/out/road-signal", "field": "value",
            "map": {"0": "DARK", "1": "FLASHING"}}}}
        result = self.run_with(binding=binding)
        self.assertTrue(result.ran, result.refusal)
        self.assertTrue(
            all("not an output" in r.refusal for r in result.results),
            [(r.name, r.refusal) for r in result.results])

    def test_no_input_naming_an_event_is_refused_up_front(self):
        """Nothing could drive the machine, so nothing could be judged."""
        binding = {**BINDING, "inputs": {"approaching": {
            "address": "plant/in/train-approach", "equals": "APPROACHING"}}}
        result = self.run_with(binding=binding)
        self.assertFalse(result.ran)
        self.assertIn("no input rule names an `event`", result.refusal)


@unittest.skipUnless(codegen_is_built(),
                     "the product's generator is not built; nothing can be run")
class ADelayedActNeedsTimeToBeDrivenThrough(unittest.TestCase):
    """A document with a delay does half its work between the cases.

    ⚠ `elapsed_ms` is the age of the SITUATION, not a moment on a timeline
    and not a delta between two records. So the observation sits that far
    after the drive that started the situation, and nothing is subtracted
    from anything -- a subtraction would be wrong twice over, because the
    field restarts whenever the situation does and its own schema calls a
    record that goes backwards ordinary.
    """

    OCCUPIED = {"address": "plant/in/train-approach",
                "becomes": "OCCUPIED", "event": "train.occupied"}

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = pathlib.Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)
        for name in ("interface-model.yaml", "conventions.yaml"):
            shutil.copy(CROSSING / name, self.tmp / name)
        (self.tmp / "signal.scxml").write_text(DELAYED, encoding="utf-8")
        self.binding = {**BINDING,
                        "inputs": {**BINDING["inputs"], "occupied": self.OCCUPIED}}

    def run_cases(self, cases):
        (self.tmp / "examples.yaml").write_text(
            yaml.safe_dump({**EXAMPLES, "cases": cases}), encoding="utf-8")
        path = self.tmp / "b.yaml"
        path.write_text(yaml.safe_dump(self.binding), encoding="utf-8")
        return verify(load_pack(self.tmp), path)

    def test_the_act_fires_once_the_age_the_record_states_has_passed(self):
        result = self.run_cases([
            {"name": "a train is detected",
             "given": {"plant/in/train-approach": "APPROACHING"},
             "drove": ["plant/in/train-approach"], "elapsed_ms": 100,
             # The arming delay has 400 ms left to run.
             "expect": {"plant/out/road-signal.value": "DARK"}},
            {"name": "it reaches the crossing",
             "given": {"plant/in/train-approach": "OCCUPIED"},
             "drove": ["plant/in/train-approach"], "elapsed_ms": 600,
             "expect": {"plant/out/road-signal.value": "FLASHING"}},
        ])
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual(2, result.passed,
                         [(r.name, r.refusal, r.failures) for r in result.results])

    def test_without_the_age_nothing_says_whether_the_wait_was_over(self):
        """A reading taken at a moment no record names is not a verdict."""
        result = self.run_cases([
            {"name": "a train is detected",
             "given": {"plant/in/train-approach": "APPROACHING"},
             "drove": ["plant/in/train-approach"], "elapsed_ms": 100,
             "expect": {"plant/out/road-signal.value": "DARK"}},
            {"name": "and then a reading with no age on it",
             "given": {"plant/in/train-approach": "OCCUPIED"},
             "drove": ["plant/in/train-approach"],
             "expect": {"plant/out/road-signal.value": "FLASHING"}},
        ])
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual(1, result.unjudged,
                         [(r.name, r.refusal) for r in result.results])
        self.assertIn("no `elapsed_ms`", result.results[1].refusal)

    def test_a_restated_value_with_an_age_is_refused_not_guessed(self):
        """The one anchor the record genuinely does not settle.

        Driving the same address to the same value again is a real
        assertion -- the schema is explicit that a restatement is not
        nothing happening. But nothing says whether the age that comes with
        it runs from this assertion or from the one that began the
        situation, and a delayed act can fall between the two.
        """
        result = self.run_cases([
            {"name": "a train is detected",
             "given": {"plant/in/train-approach": "APPROACHING"},
             "drove": ["plant/in/train-approach"], "elapsed_ms": 100,
             "expect": {"plant/out/road-signal.value": "DARK"}},
            {"name": "the same reading, asserted again",
             "given": {"plant/in/train-approach": "APPROACHING"},
             "drove": ["plant/in/train-approach"], "elapsed_ms": 600,
             "expect": {"plant/out/road-signal.value": "FLASHING"}},
        ])
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual(1, result.unjudged,
                         [(r.name, r.refusal) for r in result.results])
        self.assertIn("started the situation", result.results[1].refusal)

    def test_a_document_with_delays_and_no_ages_at_all_is_refused(self):
        result = self.run_cases([
            {"name": "a train is detected",
             "given": {"plant/in/train-approach": "APPROACHING"},
             "drove": ["plant/in/train-approach"],
             "expect": {"plant/out/road-signal.value": "DARK"}},
        ])
        self.assertFalse(result.ran)
        self.assertIn("never fired", result.refusal)


@unittest.skipUnless(codegen_is_built(),
                     "the product's generator is not built; nothing can be run")
class AnOutputTheDocumentDeclaresIsReadFromIt(unittest.TestCase):
    """Not everything a statechart answers with leaves as a send.

    A document may declare a datamodel variable `sce:direction="out"`, which
    is it saying that variable is part of its outward surface: the generator
    emits a host-facing accessor for every one, and `check` refuses a binding
    that leaves one uncovered. Reading it is therefore reading the declared
    interface, and a verification that breaks when it is renamed is right to.

    ⚠ The active CONFIGURATION is the opposite case and stays unreadable. No
    state is declared an output anywhere, so an assertion on one would break
    when a state is renamed or split while the document went on doing exactly
    the same thing -- an assertion about the insides wearing the clothes of
    one about behaviour.
    """

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = pathlib.Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)
        for name in ("interface-model.yaml", "conventions.yaml"):
            shutil.copy(CROSSING / name, self.tmp / name)
        (self.tmp / "signal.scxml").write_text(DECLARING, encoding="utf-8")
        (self.tmp / "examples.yaml").write_text(
            yaml.safe_dump(EXAMPLES), encoding="utf-8")

    def run_with(self, output):
        binding = {**BINDING, "outputs": {"roadSignal": output}}
        path = self.tmp / "b.yaml"
        path.write_text(yaml.safe_dump(binding), encoding="utf-8")
        return verify(load_pack(self.tmp), path)

    def test_the_declared_variable_is_what_the_case_is_judged_on(self):
        result = self.run_with({
            "address": "plant/out/road-signal", "field": "value",
            "map": {0: "DARK", 1: "FLASHING"}})
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual(2, result.passed,
                         [(r.name, r.refusal, r.failures) for r in result.results])

    def test_an_output_the_document_declares_nowhere_is_refused(self):
        """The configuration is what is left, and it is not an output."""
        binding = {**BINDING, "outputs": {"bell": {
            "address": "plant/out/bell", "field": "value",
            "map": {0: "SILENT", 1: "RINGING"}}}}
        path = self.tmp / "b.yaml"
        path.write_text(yaml.safe_dump(binding), encoding="utf-8")
        result = verify(load_pack(self.tmp), path)
        self.assertTrue(result.ran, result.refusal)
        self.assertTrue(
            all("not an output" in r.refusal for r in result.results),
            [(r.name, r.refusal) for r in result.results])


# A machine that speaks only when something CHANGES: entering `flashing` says
# so, leaving it says so, and a round that changes nothing sends nothing. The
# position it writes keeps its last value between those rounds -- which is
# what `hold_last` says of the address, not of the document.
SPEAKS_ON_CHANGE = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       version="1.0" datamodel="null" initial="dark" sce:kind="statechart">
  <state id="dark">
    <transition event="train.approaching" target="flashing"/>
  </state>
  <state id="flashing">
    <onentry>
      <send event="signal.flashing" type="x-sce-host"/>
    </onentry>
    <transition event="train.cleared" target="dark">
      <send event="signal.dark" type="x-sce-host"/>
    </transition>
  </state>
</scxml>
"""

HOLDING = {**BINDING, "outputs": {
    "roadSignal": {
        "address": "plant/out/road-signal", "field": "value",
        "sent": {"processor": "x-sce-host"},
        # Not a value the position can take: the round said nothing, and the
        # slot keeps what it held. `check` accepts exactly this under
        # `hold_last`, because a value with no entry is the one that holds.
        "when_nothing_sent": "unchanged",
        "map": {"signal.dark": "DARK", "signal.flashing": "FLASHING"},
        "hold_last": True,
    },
}}


def approach(name: str, expect: str | None = None, **extra) -> dict:
    case = {"name": name,
            "given": {"plant/in/train-approach": "APPROACHING"},
            "drove": ["plant/in/train-approach"], **extra}
    if expect is not None:
        case["expect"] = {"plant/out/road-signal.value": expect}
    return case


@unittest.skipUnless(codegen_is_built(),
                     "the product's generator is not built; nothing can be run")
class AHeldPositionOutlivesTheRoundThatWroteIt(unittest.TestCase):
    """`hold_last` on a statechart's output is applied, on every round.

    ⚠ It was applied only on the computation path. The statechart path read
    each round's sends and mapped them, so a round that sent nothing handed
    `when_nothing_sent` to the map, found no entry, and reported the case
    "could not be judged" -- for a binding `check` had just accepted, and that
    the README describes as holding. Measured 2026-09-23 on a model-written
    document: six of eight cases unjudged, every one a round in which the
    document rightly said nothing about the held output.
    """

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = pathlib.Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)
        for name in ("interface-model.yaml", "conventions.yaml"):
            shutil.copy(CROSSING / name, self.tmp / name)
        (self.tmp / "signal.scxml").write_text(SPEAKS_ON_CHANGE, encoding="utf-8")

    def run_cases(self, cases):
        (self.tmp / "examples.yaml").write_text(
            yaml.safe_dump({**EXAMPLES, "cases": cases}), encoding="utf-8")
        path = self.tmp / "b.yaml"
        path.write_text(yaml.safe_dump(HOLDING), encoding="utf-8")
        return verify(load_pack(self.tmp), path)

    def verdict(self, result):
        """Passed, failed, unjudged -- and how many cases left a position
        unwritten. ⚠ The fourth is not decoration: an unwritten position does
        not stop a pass, so without it a hold that never happened would read
        exactly like one that did."""
        return (result.passed, result.failed, result.unjudged,
                sum(1 for r in result.results if r.unchecked))

    def details(self, result):
        return [(r.name, r.refusal, r.failures) for r in result.results]

    def test_a_round_that_sends_nothing_keeps_what_the_last_one_wrote(self):
        result = self.run_cases([
            approach("a train approaches", "FLASHING"),
            # Re-driving the approach in `flashing` selects no transition.
            approach("the reading is restated", "FLASHING"),
            {"name": "the train clears",
             "given": {"plant/in/train-approach": "CLEAR"},
             "drove": ["plant/in/train-approach"],
             "expect": {"plant/out/road-signal.value": "DARK"}},
        ])
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((3, 0, 0, 0), self.verdict(result), self.details(result))

    def test_what_the_setup_wrote_is_what_the_judged_round_holds(self):
        """⚠ The slot does not know which rounds the record calls setup. The
        approach below is sent during the case's `before`, and the judged
        round -- a restatement -- sends nothing, so FLASHING can only come
        from the setup round."""
        result = self.run_cases([
            approach("restated after an approach", "FLASHING",
                     before=[{"given": {"plant/in/train-approach": "APPROACHING"},
                              "drove": ["plant/in/train-approach"]}]),
        ])
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((1, 0, 0, 0), self.verdict(result), self.details(result))

    def test_the_held_value_is_judged_so_a_wrong_one_fails(self):
        """The hold is a reading, not a pass: expecting DARK where the slot
        holds FLASHING is a failure, not something waved through."""
        result = self.run_cases([
            approach("a train approaches", "FLASHING"),
            approach("the reading is restated", "DARK"),
        ])
        self.assertTrue(result.ran, result.refusal)
        self.assertEqual((1, 1, 0, 0), self.verdict(result), self.details(result))

    def test_before_anything_is_held_the_position_is_not_written(self):
        """A clear reading first: `dark` acts on nothing, nothing is sent, and
        nothing has been held -- so the position is not written. It is
        reported as such (`unchecked`), never landed as `unchanged` or as a
        value the map would invent."""
        result = self.run_cases([
            {"name": "a clear reading first",
             "given": {"plant/in/train-approach": "CLEAR"},
             "drove": ["plant/in/train-approach"],
             "expect": {"plant/out/road-signal.value": "DARK"}},
        ])
        self.assertTrue(result.ran, result.refusal)
        [case] = result.results
        self.assertEqual(("", [], ["plant/out/road-signal.value"]),
                         (case.refusal, case.failures, case.unchecked),
                         self.details(result))


if __name__ == "__main__":
    unittest.main()
