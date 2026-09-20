"""Each refusal and each question class, made to happen.

A guard that has never fired is a guard nobody has measured. Every case below
builds the situation the guard exists for and asserts that it is caught, and
pairs it with the nearest situation that must NOT be caught — because a check
that refuses everything passes the first half of this file and is worthless.

The fixture subject matter is deliberately about nothing: a supply, a lamp and
a count. The core may not know what those are either.
"""

import collections
import pathlib
import tempfile
import unittest

import yaml

from sce_author.check import check
from sce_author.pack import PackError, load_pack
from sce_author.prose import load_prose
from sce_author.questions import ask

MODEL = {
    "version": 1,
    "entries": [
        {
            "address": "Plant.Input.SupplyMode",
            "role": "input",
            "names": ["In_SupplyMode"],
            "values": {"NONE": 0, "LOW": 1, "HIGH": 2, "MAX": 3},
        },
        {
            "address": "Plant.Input.Count",
            "role": "input",
            "names": ["In_Count"],
            "type": "number",
        },
        {
            "address": "Plant.Out.Lamp",
            "role": "output",
            "names": ["Fig_Lamp_stat"],
            "fields": {
                "Stat": {"values": {"NONE": 0, "OFF": 1, "ON": 2, "MAX": 3}},
                "Value": {"type": "number"},
            },
        },
        {
            "address": "Plant.Out.Unwritten",
            "role": "output",
            "names": ["Fig_Unwritten_stat"],
            "fields": {"Stat": {"values": {"NONE": 0, "OFF": 1, "ON": 2}}},
        },
    ],
}

CONVENTIONS = {
    "version": 1,
    "name_classes": [
        {"pattern": r"\bIn_[A-Za-z0-9_]+\b", "role": "supplied"},
        {"pattern": r"\bOwn_[A-Za-z0-9_]+\b", "role": "own_internal"},
    ],
    "preconditions": {
        "inputs": {"supplyOn": "the supply is present"},
        "phrases": {"supply on": "supplyOn"},
    },
    "gate_off": [
        {"when": "has_symbol", "symbol": "OFF", "use": "OFF"},
        {"when": "always", "use": "first"},
    ],
    "neutral_symbols": ["NONE", "MAX"],
    "duration_pattern": r"\b\d+\s*ms\b",
    "protocols": {"paired-counter": {"parameters": ["up", "down"]}},
}

# The second expectation source. Everything else here compares a text against
# what EXISTS; only these can notice what the text fails to mention.
EXAMPLES = {
    "version": 1,
    "origin": "shipped with the fixture",
    # This one case drives the whole input, so it is independent by definition.
    # Saying so is what lets the checks that compare cases run at all.
    "independent_cases": True,
    "cases": [
        {
            "name": "the lamp follows the supply",
            "given": {"Plant.Input.SupplyMode": 2, "Plant.Input.Count": 1},
            "expect": {"Plant.Out.Lamp.Stat": 2, "Plant.Out.Unwritten.Stat": 2},
        },
        # ⚠ The same inputs a second time, with the same answer. Without a
        # repeat there is nothing to compare, and the memory check would
        # decline rather than pass -- a set of examples that never drives one
        # input twice cannot show that anything is a function of its inputs.
        {
            "name": "and again, to the same answer",
            "given": {"Plant.Input.SupplyMode": 2, "Plant.Input.Count": 1},
            "expect": {"Plant.Out.Lamp.Stat": 2, "Plant.Out.Unwritten.Stat": 2},
        },
    ],
}

DOCUMENT = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="transform" name="fixture">
  <datamodel>
    <data id="mode" sce:type="bool" sce:direction="in"/>
    <data id="lamp" sce:type="int32" sce:direction="out" expr="mode ? 1 : 0"/>
  </datamodel>
</scxml>
"""

BINDING = {
    "version": 1,
    "document": "fixture.scxml",
    "inputs": {"mode": {"address": "Plant.Input.SupplyMode", "equals": "HIGH"}},
    "outputs": {
        "lamp": {
            "address": "Plant.Out.Lamp",
            "field": "Stat",
            "map": {0: "OFF", 1: "ON"},
        }
    },
}


class Fixture(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = pathlib.Path(self._tmp.name)
        self.pack_dir = self.root / "pack"
        self.pack_dir.mkdir()
        self.write_pack(MODEL, CONVENTIONS)
        (self.root / "fixture.scxml").write_text(DOCUMENT, encoding="utf-8")

    def tearDown(self):
        self._tmp.cleanup()

    def write_pack(self, model, conventions, examples=EXAMPLES):
        (self.pack_dir / "interface-model.yaml").write_text(yaml.safe_dump(model), encoding="utf-8")
        (self.pack_dir / "conventions.yaml").write_text(yaml.safe_dump(conventions), encoding="utf-8")
        path = self.pack_dir / "examples.yaml"
        if examples is None:
            path.unlink(missing_ok=True)
        else:
            path.write_text(yaml.safe_dump(examples), encoding="utf-8")

    def pack(self):
        return load_pack(self.pack_dir)

    def bind(self, binding):
        path = self.root / "fixture.binding.yaml"
        path.write_text(yaml.safe_dump(binding), encoding="utf-8")
        return check(self.pack(), path)

    def prose(self, text):
        path = self.root / "spec.md"
        path.write_text(text, encoding="utf-8")
        return load_prose([path])


class TheCheckRefuses(Fixture):
    def test_a_sound_binding_is_not_refused(self):
        """The discriminator. Without it every case below is satisfied by a
        check that refuses unconditionally."""
        self.assertEqual([], self.bind(BINDING))

    def test_an_address_the_model_does_not_declare(self):
        b = {**BINDING, "inputs": {"mode": {"address": "Plant.Input.Ghost", "equals": "HIGH"}}}
        self.assertIn("not in the interface model", str(self.bind(b)[0]))

    def test_a_field_the_address_does_not_have(self):
        b = {**BINDING, "outputs": {"lamp": {"address": "Plant.Out.Lamp", "field": "Ghost", "map": {1: "ON"}}}}
        self.assertIn("has no field", str(self.bind(b)[0]))

    def test_a_symbol_outside_the_value_space(self):
        b = {**BINDING, "outputs": {"lamp": {"address": "Plant.Out.Lamp", "field": "Stat", "map": {1: "BLINKING"}}}}
        self.assertIn("does not admit 'BLINKING'", str(self.bind(b)[0]))

    def test_a_negation_naming_a_symbol_the_address_cannot_take(self):
        """⚠ `not_equals` reached the evaluation path with NO test at all.

        Measured 2026-09-20 over the whole vocabulary: it was the one key that
        the core reads -- in `check` and again in `verify` -- and that no
        test, no pack and no binding anywhere had ever exercised. Live code on
        the path a verdict comes down, with nothing holding it.

        A negation is wrong in the same way a positive comparison is: a symbol
        the address cannot take never matches, so the rule is always true and
        says nothing. The check already folds the two together on purpose, and
        this is what says so.
        """
        b = {**BINDING, "inputs": {
            "mode": {"address": "Plant.Input.SupplyMode",
                     "not_equals": "BLINKING"}}}
        self.assertIn("does not admit 'BLINKING'", str(self.bind(b)[0]))

    def test_a_negation_against_a_symbol_the_address_does_take(self):
        """The discriminator. Without it the case above is satisfied by a
        check that refuses every negation."""
        b = {**BINDING, "inputs": {
            "mode": {"address": "Plant.Input.SupplyMode",
                     "not_equals": "HIGH"}}}
        self.assertEqual([], self.bind(b))

    def test_a_document_that_calls_itself_pure_and_needs_memory(self):
        """The mis-naming neither the document nor the platform can see.

        A transform says its answer comes from this round's inputs alone. The
        feedback that breaks that lives in the binding, so the document stays
        syntactically pure and the product accepts it. Eleven of twenty-nine
        documents in one corpus were named this way, and all of them worked --
        because a driver was remembering for them.
        """
        b = {**BINDING, "inputs": {**BINDING["inputs"], "was": {"state_of": "lamp"}}}
        found = str(self.bind(b))
        self.assertIn("feeds its own output back", found)
        self.assertIn("'transform'", found)

    def test_a_kind_that_is_allowed_to_remember_is_left_alone(self):
        """The discriminator. A procedure keeps state by definition, so the
        same binding must not be refused there."""
        document = DOCUMENT.replace('sce:kind="transform"', 'sce:kind="procedure"')
        (self.root / "fixture.scxml").write_text(document, encoding="utf-8")
        b = {**BINDING, "inputs": {**BINDING["inputs"], "was": {"state_of": "lamp"}}}
        self.assertNotIn("feeds its own output back", str(self.bind(b)))

    def test_a_latched_protocol_is_memory_too(self):
        """⚠ The same lie arriving through a key that did not exist when the
        guard was written.

        A latch answers by which parameter changed most RECENTLY, which is a
        fact about the round before. A document declaring itself a pure
        computation while its binding reads one is exactly what this check
        exists to refuse, and the check did not know the key.
        """
        self.write_pack(MODEL, {**CONVENTIONS, "protocols": {
            "paired-counter": {
                "parameters": ["up", "down"],
                "latch": {"set_when_changed": "up", "clear_when_changed": "down"},
            }}})
        b = {**BINDING, "inputs": {**BINDING["inputs"], "gate": {
            "protocol": "paired-counter",
            "parameters": {"up": "Plant.Input.Count",
                           "down": "Plant.Input.SupplyMode"}}}}
        found = str(self.bind(b))
        self.assertIn("feeds its own output back", found)
        self.assertIn("'transform'", found)

    def test_a_latch_over_the_platforms_own_plumbing_is_not_the_authors_memory(self):
        """⚠ The discriminator that keeps the finding worth reading.

        A supply ladder is handed to every document alike; the author did not
        choose to remember it. Without this, one true sentence was said once
        per document -- sixteen of sixteen on one corpus -- and a finding
        every document carries is one nobody reads.

        The pack already declares which addresses are plumbing. The core
        simply was not reading the list, which is the same defect as the
        gate-off check never reading `absence_tokens`.
        """
        self.write_pack(MODEL, {
            **CONVENTIONS,
            "infrastructure": ["Plant.Supply."],
            "protocols": {"paired-counter": {
                "parameters": ["up", "down"],
                "latch": {"set_when_changed": "up", "clear_when_changed": "down"},
            }}})
        over_plumbing = {**BINDING, "inputs": {**BINDING["inputs"], "gate": {
            "protocol": "paired-counter",
            "parameters": {"up": "Plant.Supply.On", "down": "Plant.Supply.Off"}}}}
        self.assertNotIn("feeds its own output back",
                         str(self.bind(over_plumbing)))

        # One parameter outside the plumbing and it is the document's again.
        mixed = {**BINDING, "inputs": {**BINDING["inputs"], "gate": {
            "protocol": "paired-counter",
            "parameters": {"up": "Plant.Input.Count",
                           "down": "Plant.Supply.Off"}}}}
        self.assertIn("feeds its own output back", str(self.bind(mixed)))

    def test_an_undefined_protocol_is_undecidable_not_innocent(self):
        """⚠ The discriminator, and the honest third answer.

        A protocol the pack NAMES and never DEFINES gives the core nothing to
        read. Calling the document pure would be a guess in its favour and
        calling it impure a guess against it; saying so is neither.
        """
        self.write_pack(MODEL, CONVENTIONS)   # `paired-counter`, no latch
        b = {**BINDING, "inputs": {**BINDING["inputs"], "gate": {
            "protocol": "paired-counter",
            "parameters": {"up": "Plant.Input.Count",
                           "down": "Plant.Input.SupplyMode"}}}}
        found = str(self.bind(b))
        self.assertIn("cannot be decided", found)
        self.assertNotIn("feeds its own output back", found)

    def test_a_kind_allowed_to_remember_is_left_alone_by_both_arms(self):
        document = DOCUMENT.replace('sce:kind="transform"', 'sce:kind="procedure"')
        (self.root / "fixture.scxml").write_text(document, encoding="utf-8")
        self.write_pack(MODEL, {**CONVENTIONS, "protocols": {
            "paired-counter": {
                "parameters": ["up", "down"],
                "latch": {"set_when_changed": "up", "clear_when_changed": "down"},
            }}})
        b = {**BINDING, "inputs": {**BINDING["inputs"], "gate": {
            "protocol": "paired-counter",
            "parameters": {"up": "Plant.Input.Count",
                           "down": "Plant.Input.SupplyMode"}}}}
        found = str(self.bind(b))
        self.assertNotIn("feeds its own output back", found)
        self.assertNotIn("cannot be decided", found)

    def test_a_document_value_with_no_binding_at_all(self):
        b = {**BINDING, "outputs": {}}
        self.assertIn("computed and dropped", str(self.bind(b)[0]))

    def test_carried_state_that_names_no_output(self):
        b = {**BINDING, "inputs": {**BINDING["inputs"], "was": {"state_of": "ghost"}}}
        self.assertIn("read a default every round", str(self.bind(b)[0]))

    def test_a_protocol_the_pack_never_declared(self):
        b = {**BINDING, "inputs": {"mode": {"protocol": "telepathy", "parameters": {}}}}
        self.assertIn("is not declared by the pack", str(self.bind(b)[0]))

    def test_a_declared_protocol_missing_a_parameter(self):
        b = {**BINDING, "inputs": {"mode": {"protocol": "paired-counter",
                                            "parameters": {"up": "Plant.Input.Count"}}}}
        self.assertIn("needs parameter 'down'", str(self.bind(b)[0]))


class TheQuestionsFire(Fixture):
    def kinds(self, text):
        prose = self.prose(text)
        pack = self.pack()
        return sorted({q.kind
                       for q in ask(prose, pack.model, pack.conventions, pack.examples)})

    def test_a_specification_that_answers_everything_asks_nothing(self):
        """The discriminator for the whole class list."""
        text = (
            "Fig_Lamp_stat is ON when In_SupplyMode == HIGH, and OFF otherwise.\n"
            "Fig_Unwritten_stat is ON when In_Count == 1, and OFF otherwise.\n"
        )
        self.assertEqual([], self.kinds(text))

    def test_a_name_that_must_come_from_outside_and_is_not_in_the_model(self):
        text = "Fig_Lamp_stat is ON when In_Ghost == HIGH, and OFF otherwise.\nFig_Unwritten_stat is OFF.\n"
        self.assertIn("unknown-name", self.kinds(text))

    def test_a_name_the_specification_defines_itself_is_not_a_question(self):
        text = "Fig_Lamp_stat is ON when Own_Scratch == HIGH, and OFF otherwise.\nFig_Unwritten_stat is OFF.\n"
        self.assertNotIn("unknown-name", self.kinds(text))

    def test_a_name_the_platform_publishes_as_a_numbered_family(self):
        """⚠ "The platform has not got this" is the wrong sentence when the
        platform has fifteen of it.

        A specification writes one subscripted name where the platform
        publishes a numbered family, and telling the author their name does
        not exist sends them to ask for something they already have. The
        question worth asking is which member an index picks -- and neither
        side's document answers it, which is why this is an error.

        ⚠⚠ The members reach the pack through the EXAMPLES as often as through
        the interface model: the model is built from what the prose names, the
        prose names the subscripted form, so the members never get in. Both
        witnesses are consulted.
        """
        model = {**MODEL, "entries": [
            *MODEL["entries"],
            {"address": "Plant.Param.Setting_Tolerance1", "role": "input",
             "names": ["Setting_Tolerance1"], "type": "number"},
            {"address": "Plant.Param.Setting_Tolerance2", "role": "input",
             "names": ["Setting_Tolerance2"], "type": "number"},
            {"address": "Plant.Param.Setting_Tolerance10", "role": "input",
             "names": ["Setting_Tolerance10"], "type": "number"},
        ]}
        self.write_pack(model, {
            **CONVENTIONS,
            "name_classes": [*CONVENTIONS["name_classes"],
                             {"pattern": r"\bSetting_[A-Za-z0-9_]+\b",
                              "role": "supplied"}],
        })
        text = ("Fig_Lamp_stat is ON when In_SupplyMode == HIGH, "
                "and OFF otherwise, within Setting_Tolerance.\n"
                "Fig_Unwritten_stat is ON when In_Count == 1, and OFF otherwise.\n")
        found = self.kinds(text)
        self.assertIn("name-is-indexed", found)
        self.assertNotIn("unknown-name", found,
                         "a family is not an absence, and saying both is worse "
                         "than saying either")

    def test_a_family_is_reported_once_not_once_per_member(self):
        """⚠ One fact reported once is what makes the OTHER facts visible.

        A specification writing `Setting_X[i]` against a platform holding
        `Setting_X1..X15` produced fifteen copies of "the examples drive an
        address you never declared" -- all true, all the same fact, and on one
        component the 25 copies buried the four addresses that were the real
        finding: that its main input is another unit's output. Removing the
        duplicates did not need a new class; it needed the old one to stop
        saying the same thing twenty-five times.
        """
        model = {**MODEL, "entries": [
            *MODEL["entries"],
            {"address": "Plant.Param.Setting_Tolerance1", "role": "input",
             "names": ["Setting_Tolerance1"], "type": "number"},
            {"address": "Plant.Param.Setting_Tolerance2", "role": "input",
             "names": ["Setting_Tolerance2"], "type": "number"},
        ]}
        examples = {
            **EXAMPLES,
            "cases": [{
                "name": "the family, and one address from somewhere else",
                "given": {"Plant.Input.SupplyMode": 2,
                          "Plant.Param.Setting_Tolerance1": 3,
                          "Plant.Param.Setting_Tolerance2": 4,
                          "Plant.Param.Setting_Tolerance3": 5,
                          "Elsewhere.Upstream.Out.Reading": 9},
                "expect": {"Plant.Out.Lamp.Stat": 2},
            }],
        }
        self.write_pack(model, {
            **CONVENTIONS,
            "name_classes": [*CONVENTIONS["name_classes"],
                             {"pattern": r"\bSetting_[A-Za-z0-9_]+\b",
                              "role": "supplied"}],
        }, examples=examples)
        text = ("Fig_Lamp_stat is ON when In_SupplyMode == HIGH, "
                "and OFF otherwise, within Setting_Tolerance.\n"
                "Fig_Unwritten_stat is ON when In_Count == 1, and OFF otherwise.\n")
        prose = self.prose(text)
        pack = self.pack()
        from sce_author.questions import ask
        found = ask(prose, pack.model, pack.conventions, pack.examples)
        undeclared = [q for q in found
                      if q.kind == "example-drives-undeclared-address"]
        # ⚠ The addresses moved from `subject` into `detail` when this class
        # became one question per pack rather than one per address. What the
        # case is about did not move: the family member the model does not
        # declare is NOT repeated here, and the address from outside the
        # subject matter still is.
        self.assertEqual(1, len(undeclared))
        self.assertIn("Elsewhere.Upstream.Out.Reading", undeclared[0].detail)
        self.assertNotIn("Setting_Tolerance", undeclared[0].detail)
        self.assertIn("drive 1 address(es)", undeclared[0].detail)
        self.assertIn("name-is-indexed", {q.kind for q in found})

    # ------------------------ one specification is not one program

    def outside_pack(self, upstream: bool):
        """A pack whose examples drive two addresses it does not decide."""
        elsewhere = [
            {"address": "Elsewhere.Unit.Out.Reading", "role": "upstream",
             "names": ["Up_Reading"], "type": "number"},
            {"address": "Elsewhere.Unit.Out.Mode", "role": "upstream",
             "names": ["Up_Mode"], "values": {"OFF": 0, "ON": 1}},
        ] if upstream else []
        examples = {
            **EXAMPLES,
            "cases": [{
                "name": "the lamp follows a reading this document does not make",
                "given": {"Plant.Input.SupplyMode": 2,
                          "Elsewhere.Unit.Out.Reading": 9,
                          "Elsewhere.Unit.Out.Mode": 1},
                "expect": {"Plant.Out.Lamp.Stat": 2},
            }],
        }
        self.write_pack({**MODEL, "entries": [*MODEL["entries"], *elsewhere]},
                        CONVENTIONS, examples=examples)
        text = "Fig_Lamp_stat is ON when In_SupplyMode == HIGH, and OFF otherwise.\n"
        from sce_author.questions import ask
        pack = self.pack()
        return ask(self.prose(text), pack.model, pack.conventions,
                   pack.examples)

    def test_an_address_another_unit_writes_is_said_once_as_a_dependency(self):
        """⚠ A DIFFERENT CLAIM FROM "UNDECLARED", and the only one an author
        can act on. "Reaching outside what it declares" sends them looking for
        something missing from this document; this sends them to the document
        that decides it.

        Measured over 129 subject packs against a 244-component platform: 45
        of 180 undeclared addresses are another component's output, and they
        concentrate -- 19 of one pack's 20, 9 of another's 9. For those, the
        report is not a defect, it is what the system is.
        """
        found = self.outside_pack(upstream=True)
        kinds = collections.Counter(q.kind for q in found)
        self.assertEqual(1, kinds["depends-on-another-component"],
                         "said once, or not at all")
        self.assertEqual(0, kinds["example-drives-undeclared-address"])
        said = next(q.detail for q in found
                    if q.kind == "depends-on-another-component")
        self.assertIn("2 address(es)", said)
        self.assertIn("Elsewhere.Unit.Out.Reading", said)
        self.assertIn("one of several", said)

    def test_the_same_addresses_undeclared_are_a_different_sentence(self):
        """The discriminator. Identical examples, identical prose; the model
        simply does not say who writes them. Without this, a class that always
        said "dependency" would pass the case above.
        """
        found = self.outside_pack(upstream=False)
        kinds = collections.Counter(q.kind for q in found)
        self.assertEqual(0, kinds["depends-on-another-component"])
        self.assertEqual(1, kinds["example-drives-undeclared-address"],
                         "two addresses, one sentence")
        said = next(q.detail for q in found
                    if q.kind == "example-drives-undeclared-address")
        self.assertIn("2 address(es)", said)
        self.assertIn("reaching outside what it declares", said)

    def test_a_name_with_no_family_is_still_simply_unknown(self):
        """The discriminator. One numbered sibling is not a family."""
        model = {**MODEL, "entries": [
            *MODEL["entries"],
            {"address": "Plant.Param.Setting_Tolerance1", "role": "input",
             "names": ["Setting_Tolerance1"], "type": "number"},
        ]}
        self.write_pack(model, {
            **CONVENTIONS,
            "name_classes": [*CONVENTIONS["name_classes"],
                             {"pattern": r"\bSetting_[A-Za-z0-9_]+\b",
                              "role": "supplied"}],
        })
        text = ("Fig_Lamp_stat is ON when In_SupplyMode == HIGH, "
                "and OFF otherwise, within Setting_Tolerance.\n"
                "Fig_Unwritten_stat is ON when In_Count == 1, and OFF otherwise.\n")
        found = self.kinds(text)
        self.assertIn("unknown-name", found)
        self.assertNotIn("name-is-indexed", found)

    def test_a_value_the_address_cannot_take(self):
        text = "Fig_Lamp_stat is ON when In_SupplyMode == BLINKING, and OFF otherwise.\nFig_Unwritten_stat is OFF.\n"
        self.assertIn("value-not-in-space", self.kinds(text))

    def test_an_output_nothing_in_the_prose_decides(self):
        text = "Fig_Lamp_stat is ON when In_SupplyMode == HIGH, and OFF otherwise.\n"
        self.assertIn("no-decision-logic", self.kinds(text))

    def test_an_output_whose_off_value_is_never_stated(self):
        text = "Fig_Lamp_stat is ON when In_SupplyMode == HIGH.\nFig_Unwritten_stat is ON when In_Count == 1, and OFF otherwise.\n"
        self.assertIn("gate-off-unstated", self.kinds(text))

    def test_a_longer_symbol_that_merely_contains_the_off_value(self):
        """⚠ The class went quiet on eleven real positions because of this.

        The off value was looked for as a SUBSTRING, so `DISPLAY_OFF`,
        `ACT_PRESSURE_OFF` and an image identifier holding `HEV` all read as
        the document having stated it. `\\b` does not help: the boundary
        between `_` and a letter is not a word boundary, so `\\bOFF\\b` matches
        inside `DISPLAY_OFF` too.

        Found by deleting the closing case from documents that state it and
        checking the class then fires -- 38 of 49 did, and all eleven that did
        not were this.
        """
        text = ("Fig_Lamp_stat is ON when In_SupplyMode == HIGH, "
                "and the panel shows DISPLAY_OFF.\n"
                "Fig_Unwritten_stat is ON when In_Count == 1, and OFF otherwise.\n")
        self.assertIn("gate-off-unstated", self.kinds(text))

    def test_this_silence_does_not_change_with_what_the_examples_read(self):
        """⚠ Deliberate: the class is UNGROUNDED and is meant to stay so.

        Grounding it on whether `examples.expected` reads the position back
        was tried and measured on a 129-pack corpus: 3272 of 3600 output
        fields are asserted by some case, so the split moved 732 of 757
        findings into one half and separated nothing. The predicate worth
        having -- does any case exercise the precondition-FALSE branch --
        needs case values and a binding, and neither exists when questions
        are asked. This case exists so a reader who tries it again finds the
        answer before the corpus does.
        """
        text = "Fig_Lamp_stat is ON when In_SupplyMode == HIGH.\nFig_Unwritten_stat is ON when In_Count == 1, and OFF otherwise.\n"
        read_back = self.kinds(text)
        self.write_pack(MODEL, CONVENTIONS, examples={
            **EXAMPLES,
            "cases": [{
                "name": "nothing reads the lamp",
                "given": {"Plant.Input.SupplyMode": 2, "Plant.Input.Count": 1},
                "expect": {"Plant.Out.Unwritten.Stat": 2},
            }],
        })
        self.assertIn("gate-off-unstated", read_back)
        self.assertIn("gate-off-unstated", self.kinds(text))

    def test_a_duration_with_no_input_that_observes_time(self):
        text = "Fig_Lamp_stat is ON for 500 ms when In_SupplyMode == HIGH, and OFF otherwise.\nFig_Unwritten_stat is ON when In_Count == 1, and OFF otherwise.\n"
        self.assertIn("no-time-input", self.kinds(text))

    def test_a_duration_is_not_a_question_when_the_pack_has_a_clock(self):
        self.write_pack(MODEL, {**CONVENTIONS, "time_inputs": ["elapsed"]})
        text = "Fig_Lamp_stat is ON for 500 ms when In_SupplyMode == HIGH, and OFF otherwise.\nFig_Unwritten_stat is ON when In_Count == 1, and OFF otherwise.\n"
        self.assertNotIn("no-time-input", self.kinds(text))

    def test_a_token_the_pack_calls_an_absence_is_not_a_value(self):
        """Absence is not a member of a value space, and adding it to one would
        invent a state the platform has not got."""
        self.write_pack(MODEL, {**CONVENTIONS, "absence_tokens": ["NOT_REPORTING"]})
        text = "Fig_Lamp_stat is ON when In_SupplyMode == NOT_REPORTING, and OFF otherwise.\nFig_Unwritten_stat is ON when In_Count == 1, and OFF otherwise.\n"
        self.assertNotIn("value-not-in-space", self.kinds(text))

    def test_the_same_token_without_that_declaration_is_a_value(self):
        """The discriminator: the pack's declaration is what silences it, not
        the shape of the word."""
        text = "Fig_Lamp_stat is ON when In_SupplyMode == NOT_REPORTING, and OFF otherwise.\nFig_Unwritten_stat is ON when In_Count == 1, and OFF otherwise.\n"
        self.assertIn("value-not-in-space", self.kinds(text))

    def test_prose_after_an_operator_is_not_read_as_a_value(self):
        """`excluding` is a word, and the value space is what says so: every
        symbol in it is upper case, so a lower-case token is prose. No list of
        words to ignore, which would be a guess about a language."""
        text = "Fig_Lamp_stat is ON when In_SupplyMode == HIGH, excluding low modes.\nFig_Unwritten_stat is ON when In_Count == 1, and OFF otherwise.\n"
        self.assertNotIn("value-not-in-space", self.kinds(text))

    def test_a_symbol_spelled_in_another_case_is_its_own_question(self):
        """The author wrote a symbol that exists and spelled it their way. That
        is a different conversation from writing one that does not exist, so it
        is reported separately rather than as the stronger claim."""
        text = "Fig_Lamp_stat is ON when In_SupplyMode == High, and OFF otherwise.\nFig_Unwritten_stat is ON when In_Count == 1, and OFF otherwise.\n"
        kinds = self.kinds(text)
        self.assertIn("value-cased-differently", kinds)
        self.assertNotIn("value-not-in-space", kinds)

    def test_a_document_that_writes_comparisons_in_words(self):
        """The one thing about the SHAPE of prose the core would assume.

        A specification that never writes `==` is invisible to the default
        pattern, and this is the case that says so: the same sentence is
        missed with the default and caught once the pack declares how this
        kind of document writes a comparison.
        """
        text = ("Fig_Lamp_stat is ON when In_SupplyMode is BLINKING, and OFF otherwise.\n"
                "Fig_Unwritten_stat is ON when In_Count is 1, and OFF otherwise.\n")
        self.assertNotIn("value-not-in-space", self.kinds(text))

        self.write_pack(MODEL, {
            **CONVENTIONS,
            "comparison_pattern":
                r"(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s+(?P<op>is|equals)\s+(?P<token>[A-Za-z_][A-Za-z0-9_]*)",
        })
        self.assertIn("value-not-in-space", self.kinds(text))

    def test_a_comparison_pattern_without_its_groups_is_refused(self):
        self.write_pack(MODEL, {**CONVENTIONS, "comparison_pattern": r"\w+ is \w+"})
        with self.assertRaises(PackError) as caught:
            self.pack()
        self.assertIn("named group", str(caught.exception))

    def test_a_signal_the_examples_drive_and_the_prose_never_names(self):
        """The class the other seven cannot reach.

        A model can only say what exists, so a specification that simply fails
        to mention a signal the product uses reads as complete. Nothing is
        missing until something expects it.
        """
        text = ("Fig_Lamp_stat is ON when In_SupplyMode == HIGH, and OFF otherwise.\n"
                "Fig_Unwritten_stat is ON when In_Count == 1, and OFF otherwise.\n")
        self.assertNotIn("example-drives-unnamed-signal", self.kinds(text))

        # The same prose, with the count never mentioned.
        text = "Fig_Lamp_stat is ON when In_SupplyMode == HIGH, and OFF otherwise.\nFig_Unwritten_stat is OFF.\n"
        found = self.kinds(text)
        self.assertIn("example-drives-unnamed-signal", found)

    def test_an_output_the_examples_read_back_and_the_prose_never_decides(self):
        """Grounded silence. `no-decision-logic` asks this of every output in
        the model; this one only fires where something actually requires a
        value, which is what makes it worth waking someone for."""
        text = "Fig_Lamp_stat is ON when In_SupplyMode == HIGH, and OFF otherwise.\n"
        self.assertIn("example-expects-undecided-output", self.kinds(text))

    # The whole text is correct about each case on its own; what it never says
    # is that the two are told apart by history. Measured as the cause of both
    # components that would not convert cleanly on one corpus.
    MEMORY = (
        "Fig_Lamp_stat is ON when In_SupplyMode == HIGH, and OFF otherwise.\n"
        "Fig_Unwritten_stat is ON when In_Count == 1, and OFF otherwise.\n"
    )

    def valued(self, **over):
        """Examples that carry values, which is what a comparison needs."""
        return {
            "version": 1,
            "origin": "shipped with the fixture",
            "independent_cases": True,
            "cases": [
                {"name": "the lamp is on",
                 "given": {"Plant.Input.SupplyMode": 2, "Plant.Input.Count": 1},
                 "expect": {"Plant.Out.Lamp.Stat": 2, "Plant.Out.Unwritten.Stat": 2}},
                {"name": "the same inputs, a different result",
                 "given": {"Plant.Input.SupplyMode": 2, "Plant.Input.Count": 1},
                 "expect": {"Plant.Out.Lamp.Stat": 1, "Plant.Out.Unwritten.Stat": 2}},
            ],
            **over,
        }

    def test_two_cases_with_one_input_and_two_answers(self):
        self.write_pack(MODEL, CONVENTIONS, examples=self.valued())
        self.assertIn("example-shows-memory", self.kinds(self.MEMORY))

    def test_an_address_the_model_does_not_declare_does_not_hide_memory(self):
        """⚠ The case that made the check useless before it was written this way.

        A case may carry signals belonging to a whole installation. Comparing
        whole `given` maps found NOTHING on a corpus of 18 components -- with
        tens of accumulated keys two cases are never equal, so the check could
        not fire and the empty result read as clean. What the component is a
        function of is what the model declares.
        """
        ex = self.valued()
        ex["cases"][1]["given"]["Plant.Somewhere.Else"] = 99
        self.write_pack(MODEL, CONVENTIONS, examples=ex)
        self.assertIn("example-shows-memory", self.kinds(self.MEMORY))

    def test_cases_that_differ_in_their_inputs_are_not_memory(self):
        """The discriminator. Without it, every example set would look stateful.

        ⚠ And the answer is that the check could not run, NOT that the
        component is stateless -- see the case below, which is the same set-up
        read for what it does not prove.
        """
        ex = self.valued()
        ex["cases"][1]["given"] = {"Plant.Input.SupplyMode": 1,
                                   "Plant.Input.Count": 1}
        self.write_pack(MODEL, CONVENTIONS, examples=ex)
        self.assertNotIn("example-shows-memory", self.kinds(self.MEMORY))

    def test_the_check_says_how_much_of_the_pack_it_judged(self):
        """⚠ Comparability is per ADDRESS; saying it per pack hides most of it.

        Measured over the 65 packs with any comparable pair: of 2702 addresses
        the cases read back, only 410 (15%) were ever read back twice under
        the same inputs. The other 85% got no answer and no notice, which
        reads exactly like a clean one.
        """
        ex = self.valued()
        # A third case, under different inputs, reading back an address the
        # comparable pair never touches. That address cannot be judged.
        ex["cases"].append({
            "name": "a lone case touching something else",
            "given": {"Plant.Input.SupplyMode": 1, "Plant.Input.Count": 9},
            "expect": {"Plant.Out.Lamp.Value": 7},
        })
        self.write_pack(MODEL, CONVENTIONS, examples=ex)
        found = self.kinds(self.MEMORY)
        self.assertIn("memory-check-partial", found)
        self.assertNotIn("no-comparable-cases", found)

    def test_full_coverage_says_nothing_about_coverage(self):
        self.write_pack(MODEL, CONVENTIONS, examples=self.valued())
        self.assertNotIn("memory-check-partial", self.kinds(self.MEMORY))

    def test_cases_that_never_repeat_an_input_prove_nothing(self):
        """⚠ The silence that looks exactly like a clean answer.

        Measured over 127 packs, 62 -- 49% -- have no two cases driving the
        same inputs, so this check could never speak for half a corpus. The
        component that prompted the class is one of them: its own document
        holds the previous answer, so it demonstrably remembers, and the check
        returned nothing. Reading that as "nothing is remembered" is the
        mistake this module exists to make impossible.
        """
        ex = self.valued()
        ex["cases"][1]["given"] = {"Plant.Input.SupplyMode": 1,
                                   "Plant.Input.Count": 1}
        self.write_pack(MODEL, CONVENTIONS, examples=ex)
        self.assertIn("no-comparable-cases", self.kinds(self.MEMORY))

    def test_the_check_declines_when_the_cases_are_deltas(self):
        """⚠ A case that is a delta on the one before it was not run under the
        inputs it lists, so two cases that look alike were not alike. The
        check says it did not run rather than answering from that.
        """
        ex = self.valued()
        del ex["independent_cases"]
        self.write_pack(MODEL, CONVENTIONS, examples=ex)
        found = self.kinds(self.MEMORY)
        self.assertIn("cases-not-independent", found)
        self.assertNotIn("example-shows-memory", found)

    def test_the_check_declines_when_the_cases_withhold_values(self):
        ex = self.valued()
        for case in ex["cases"]:
            case["given"] = {k: None for k in case["given"]}
            case["expect"] = {k: None for k in case["expect"]}
        self.write_pack(MODEL, CONVENTIONS, examples=ex)
        found = self.kinds(self.MEMORY)
        self.assertIn("cases-carry-no-values", found)
        self.assertNotIn("example-shows-memory", found)

    def test_a_pack_with_no_examples_says_so(self):
        """⚠ Without examples the two classes above cannot fire, and a short
        list would be read as a clean specification. An empty result from a
        check that never ran is the shape this tool exists to refuse."""
        self.write_pack(MODEL, CONVENTIONS, examples=None)
        text = ("Fig_Lamp_stat is ON when In_SupplyMode == HIGH, and OFF otherwise.\n"
                "Fig_Unwritten_stat is ON when In_Count == 1, and OFF otherwise.\n")
        found = self.kinds(text)
        self.assertEqual(["no-examples"], found)

    def test_a_name_that_reaches_two_addresses(self):
        model = {**MODEL, "entries": MODEL["entries"] + [
            {"address": "Plant.Memory.SupplyMode", "role": "input",
             "names": ["In_SupplyMode"], "values": {"NONE": 0, "LOW": 1, "HIGH": 2}},
        ]}
        self.write_pack(model, CONVENTIONS)
        text = "Fig_Lamp_stat is ON when In_SupplyMode == HIGH, and OFF otherwise.\nFig_Unwritten_stat is ON when In_Count == 1, and OFF otherwise.\n"
        self.assertIn("ambiguous-name", self.kinds(text))


class TheBriefAgreesWithItsOwnQuestions(Fixture):
    def test_an_output_the_prose_writes_is_not_called_unwritten(self):
        """The brief and the questions have to read the same text.

        They did not: section 3 asked whether the name was among the ones the
        prose RECEIVES, so every output it decides was reported as untouched
        while the question class beside it correctly said nothing was wrong.
        """
        from sce_author.brief import assemble

        text = ("Fig_Lamp_stat is ON when In_SupplyMode == HIGH, and OFF otherwise.\n"
                "Fig_Unwritten_stat is ON when In_Count == 1, and OFF otherwise.\n")
        page = assemble(self.prose(text), self.pack())
        section = page.split("## 3.")[1].split("## 4.")[0]
        self.assertNotIn("NOT WRITTEN", section)

    def test_a_cascade_proposal_never_reads_as_a_fact(self):
        """The cascade is a procedure with a hit rate, not an oracle.

        Stated flatly, its answer reads as something the core knows, and a
        reader acts on it. Two of the proposals it made against a real pack
        were wrong, and neither raised anything.
        """
        from sce_author.brief import assemble

        self.write_pack(MODEL, {**CONVENTIONS, "gate_off_note": "measured 87% over 174 spaces"})
        page = assemble(self.prose("Fig_Lamp_stat is ON when In_SupplyMode == HIGH.\n"), self.pack())
        self.assertIn("PROPOSES", page)
        self.assertIn("measured 87% over 174 spaces", page)
        self.assertIn("confirm it against the platform", page)

    def test_no_off_value_is_proposed_for_an_input(self):
        """The question has no meaning there, and the sentence it produced --
        'when the precondition is false it is HIGH' -- is not about anything."""
        from sce_author.brief import assemble

        page = assemble(self.prose("Fig_Lamp_stat is ON when In_SupplyMode == HIGH.\n"), self.pack())
        section = page.split("## 2.")[1].split("## 3.")[0]

        # Walk the section and attribute each proposal to the entry above it.
        owner, proposed_for = None, []
        for line in section.splitlines():
            if line.startswith("- `"):
                owner = line
            elif "PROPOSES" in line:
                proposed_for.append(owner)

        self.assertTrue(proposed_for, "the case proves nothing if nothing was proposed")
        for owner in proposed_for:
            self.assertIn("(output)", owner)
        # And the input IS in the section -- it is listed, just not proposed for.
        self.assertIn("Plant.Input.SupplyMode` (input)", section)

    def test_an_output_the_prose_never_writes_is_still_called_unwritten(self):
        from sce_author.brief import assemble

        text = "Fig_Lamp_stat is ON when In_SupplyMode == HIGH, and OFF otherwise.\n"
        page = assemble(self.prose(text), self.pack())
        section = page.split("## 3.")[1].split("## 4.")[0]
        self.assertIn("NOT WRITTEN", section)


class TheReaderSaysWhatItCouldNotCarry(Fixture):
    """A format is not a subject matter, so readers are general -- but what a
    reader loses is invisible downstream, which is what these cases hold."""

    def docx(self, name, body_xml, extra=None):
        import zipfile

        path = self.root / name
        with zipfile.ZipFile(path, "w") as zf:
            zf.writestr("word/document.xml",
                        '<?xml version="1.0"?>'
                        '<w:document xmlns:w="http://schemas.openxmlformats.org/'
                        'wordprocessingml/2006/main"><w:body>' + body_xml
                        + "</w:body></w:document>")
            for k, v in (extra or {}).items():
                zf.writestr(k, v)
        return path

    def para(self, text):
        return f"<w:p><w:r><w:t>{text}</w:t></w:r></w:p>"

    def test_a_table_survives_as_rows(self):
        """A table flattened into a paragraph loses which value went with
        which condition, which in these documents is the entire content."""
        from sce_author.ingest import ingest

        body = ("<w:tbl><w:tr>"
                f"<w:tc>{self.para('Fig_Lamp_stat')}</w:tc>"
                f"<w:tc>{self.para('ON')}</w:tc>"
                "</w:tr></w:tbl>")
        got = ingest(self.docx("spec.docx", body))
        self.assertIn("| Fig_Lamp_stat | ON |", got.text)
        self.assertEqual([], got.notes)

    def test_an_embedded_object_is_named_not_swallowed(self):
        from sce_author.ingest import ingest

        got = ingest(self.docx(
            "spec.docx", self.para("see the attached sheet"),
            {"word/embeddings/oleObject1.bin": "not really a sheet"}))
        self.assertTrue(got.notes)
        self.assertIn("oleObject1.bin", got.notes[0])

    def test_what_a_reader_lost_becomes_a_question(self):
        """Every other class asks what the text does not say, and all of them
        are wrong if the text is not all there."""
        path = self.docx("spec.docx", self.para("Fig_Lamp_stat is ON when In_SupplyMode == HIGH, and OFF otherwise."),
                         {"word/embeddings/oleObject1.bin": "x"})
        prose = load_prose([path])
        pack = self.pack()
        kinds = [q.kind for q in ask(prose, pack.model, pack.conventions)]
        self.assertIn("source-not-fully-read", kinds)

    def test_a_picture_and_an_embedded_object_are_reported_apart(self):
        """They are not equally dangerous and a warning that says so is the
        one that gets read. A picture is usually layout; an attached
        spreadsheet is usually the requirement itself."""
        from sce_author.ingest import ingest

        got = ingest(self.docx(
            "spec.docx", self.para("see below"),
            {"word/media/image1.png": "x", "word/embeddings/oleObject1.bin": "y"}))
        joined = " ".join(got.notes)
        self.assertIn("oleObject1.bin", joined)
        self.assertIn("picture(s) were not read", joined)
        self.assertEqual(2, len(got.notes))

    def test_a_document_that_reads_as_empty_is_refused(self):
        from sce_author.ingest import IngestError, ingest

        with self.assertRaises(IngestError) as caught:
            ingest(self.docx("spec.docx", ""))
        self.assertIn("indistinguishable", str(caught.exception))

    def test_a_format_with_no_reader_names_what_to_do(self):
        from sce_author.ingest import IngestError, ingest

        path = self.root / "spec.pdf"
        path.write_bytes(b"%PDF-1.4 not really")
        with self.assertRaises(IngestError) as caught:
            ingest(path)
        self.assertIn("convert it first", str(caught.exception))

    def test_html_table_rows_survive(self):
        from sce_author.ingest import ingest

        path = self.root / "spec.html"
        path.write_text(
            "<html><body><table><tr><td>Fig_Lamp_stat</td><td>ON</td></tr>"
            "</table></body></html>", encoding="utf-8")
        self.assertIn("| Fig_Lamp_stat | ON |", ingest(path).text)


class ThePackRefuses(Fixture):
    def test_an_empty_model_is_not_a_clean_one(self):
        self.write_pack({"version": 1, "entries": []}, CONVENTIONS)
        with self.assertRaises(PackError) as caught:
            self.pack()
        self.assertIn("empty", str(caught.exception))

    def test_one_address_declared_twice(self):
        model = {**MODEL, "entries": MODEL["entries"] + [MODEL["entries"][0]]}
        self.write_pack(model, CONVENTIONS)
        with self.assertRaises(PackError) as caught:
            self.pack()
        self.assertIn("already declared", str(caught.exception))

    def test_a_model_that_does_not_match_the_schema(self):
        self.write_pack({"version": 1, "entries": [{"address": "X", "role": "sideways"}]}, CONVENTIONS)
        with self.assertRaises(PackError):
            self.pack()


if __name__ == "__main__":
    unittest.main()
