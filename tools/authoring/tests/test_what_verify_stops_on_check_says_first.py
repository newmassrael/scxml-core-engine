"""What `verify` would stop on, `check` says first -- in the same words.

`check` answers whether a binding is well formed and `verify` runs it. Where
the two disagree, the author meets a defect of the BINDING only after a build
and a run, and meets it as a run that could not be judged rather than as a
defect. Three such disagreements were found one at a time on one corpus -- a
boolean map written `true`/`false`, a document with no `sce:kind`, an event
matched by its exact name -- so every form `verify` refuses was then written
into a binding this fixture otherwise accepts, measured 2026-09-22:

  check passed, verify stopped    an output with neither `map` nor
                                  `passthrough`; `hold_last` with no map; a
                                  map missing a value the document can produce;
                                  a document keeping values with no
                                  `activation`; an input the document does not
                                  declare; a statechart binding driving nothing
  check refused, verify ran       `clock` and `variant_is`, which name no
                                  address because they are read off the case

⚠⚠ And one worse than a late refusal. A statechart's binding could compute a
value -- `equals`, `protocol`, an address read as it is -- and the run DROPPED
it and sent the event bare, so a guard comparing it compared the value it was
declared with, and a correct document FAILED. Nothing refused it anywhere.

Each is now one judgement both commands make (`landing`,
`check.driving_refusals`, `check.activation_unsaid`). Every refusal below is
paired with a binding that differs from it in one thing and is not refused, so
neither a check that refuses everything nor one that refuses nothing passes.
"""

from __future__ import annotations

import copy
import json
import pathlib
import tempfile
import unittest

import yaml

from sce_author.check import (STATECHART_KINDS, _ANNOTATIONS,
                              _STATECHART_DRIVER_READS, Document,
                              driving_refusals, read_document)
from sce_author.pack import SCHEMA_DIR
from sce_author.verify import _default_codegen, generate, verify
from tests.test_a_document_keeps_what_it_declares import FORGETS, KEPT
from tests.test_refusals_actually_fire import (BINDING, CONVENTIONS, DOCUMENT,
                                               MODEL, Fixture)

HAVE_CODEGEN = _default_codegen().exists()

# Two rounds, the lamp following the supply. Ordered, so a document keeping
# values can be run on them too.
EXAMPLES = {
    "version": 1, "origin": "written for this test",
    "independent_cases": True, "ordered": True,
    "cases": [
        {"name": "high", "given": {"Plant.Input.SupplyMode": 2, "Plant.Input.Count": 1},
         "drove": ["Plant.Input.SupplyMode"], "expect": {"Plant.Out.Lamp.Stat": 2}},
        {"name": "low", "given": {"Plant.Input.SupplyMode": 1, "Plant.Input.Count": 1},
         "drove": ["Plant.Input.SupplyMode"], "expect": {"Plant.Out.Lamp.Stat": 1}},
    ],
}

# The lamp as a truth value rather than a number.
BOOL_OUT = DOCUMENT.replace(
    'sce:type="int32" sce:direction="out" expr="mode ? 1 : 0"',
    'sce:type="bool" sce:direction="out" expr="mode"')

# A crossing signal that flashes only while a level it READS is above 3 --
# the shape a statechart cannot be handed from a binding.
SIGNAL = """<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" datamodel="ecmascript" initial="dark" sce:kind="statechart">
  <state id="dark">
    <transition event="train.approaching" target="flashing"/>
  </state>
  <state id="flashing">
    <onentry><send event="signal.flashing" type="x-sce-host"/></onentry>
    <transition event="train.cleared" target="dark"/>
  </state>
</scxml>
"""
LEVEL = SIGNAL.replace(
    'sce:kind="statechart">',
    'sce:kind="statechart">\n  <datamodel><data id="level" sce:type="int32" '
    'sce:direction="in" expr="0"/></datamodel>').replace(
    '<transition event="train.approaching" target="flashing"/>',
    '<transition event="train.approaching" cond="level &gt; 3" target="flashing"/>')

SIGNAL_MODEL = {"version": 1, "entries": [
    {"address": "plant/in/approach", "role": "input", "names": ["In_Approach"],
     "values": {"CLEAR": 0, "APPROACHING": 1}},
    {"address": "plant/in/level", "role": "input", "names": ["In_Level"],
     "type": "number"},
    {"address": "plant/out/signal", "role": "output", "names": ["Out_Signal"],
     "fields": {"value": {"values": {"DARK": 0, "FLASHING": 1}}}},
]}

SIGNAL_BINDING = {
    "version": 1, "document": "fixture.scxml",
    "inputs": {
        "approaching": {"address": "plant/in/approach", "becomes": "APPROACHING",
                        "event": "train.approaching"},
        "cleared": {"address": "plant/in/approach", "becomes": "CLEAR",
                    "event": "train.cleared"},
    },
    "outputs": {
        "signal": {"address": "plant/out/signal", "field": "value",
                   "sent": {"processor": "x-sce-host"},
                   "when_nothing_sent": "signal.dark",
                   "map": {"signal.dark": "DARK", "signal.flashing": "FLASHING"}},
    },
}

# The level is 5 throughout, so a machine that received it flashes on every
# approach. One that did not receives 0 and stays dark.
SIGNAL_EXAMPLES = {
    "version": 1, "origin": "written for this test",
    "independent_cases": True, "ordered": True,
    "cases": [
        {"name": "approach", "given": {"plant/in/approach": "APPROACHING", "plant/in/level": 5},
         "drove": ["plant/in/approach"], "expect": {"plant/out/signal.value": "FLASHING"}},
        {"name": "clear", "given": {"plant/in/approach": "CLEAR", "plant/in/level": 5},
         "drove": ["plant/in/approach"], "expect": {"plant/out/signal.value": "DARK"}},
    ],
}


class Both(Fixture):
    """Ask `check` and, where the generator is built, `verify` of one binding."""

    def use(self, document, model=MODEL, examples=EXAMPLES):
        (self.root / "fixture.scxml").write_text(document, encoding="utf-8")
        self.write_pack(model, CONVENTIONS, examples)

    def found(self, binding) -> list[str]:
        return [str(f) for f in self.bind(binding)]

    def verified(self, binding):
        path = self.root / "fixture.binding.yaml"
        path.write_text(yaml.safe_dump(binding), encoding="utf-8")
        return verify(self.pack(), path)

    def refusals(self, result) -> str:
        """Every refusal a run gave, the run's own and each case's."""
        return "\n".join([result.refusal or ""]
                         + [r.refusal for r in result.results if r.refusal])

    def assertSaidByBoth(self, binding, words):
        """`check` says `words`, and so does `verify` when it can run."""
        found = self.found(binding)
        self.assertTrue(any(words in f for f in found), found)
        if HAVE_CODEGEN:
            said = self.refusals(self.verified(binding))
            self.assertIn(words, said)


# --------------------------------------------------------------- an output


class WhereAnOutputLands(Both):
    def setUp(self):
        super().setUp()
        self.use(DOCUMENT)

    def lamp(self, **rule):
        binding = copy.deepcopy(BINDING)
        binding["outputs"]["lamp"] = {"address": "Plant.Out.Lamp", "field": "Stat", **rule}
        return binding

    def test_a_rule_that_neither_maps_nor_passes_through(self):
        self.assertSaidByBoth(self.lamp(), "neither 'map' nor 'passthrough'")

    def test_a_rule_that_passes_through_is_not_refused(self):
        found = self.found(self.lamp(passthrough=True))
        self.assertFalse([f for f in found if "output lamp" in f], found)

    def test_hold_last_with_no_map(self):
        self.assertSaidByBoth(self.lamp(passthrough=True, hold_last=True),
                              "`hold_last` holds a MAPPED value")

    def test_hold_last_with_a_map_is_not_refused(self):
        found = self.found(self.lamp(map={0: "OFF", 1: "ON"}, hold_last=True))
        self.assertFalse([f for f in found if "output lamp" in f], found)


class ABooleanOutputOwesBothCases(Both):
    def setUp(self):
        super().setUp()
        self.use(BOOL_OUT)

    def lamp(self, table, **rule):
        binding = copy.deepcopy(BINDING)
        binding["outputs"]["lamp"] = {"address": "Plant.Out.Lamp", "field": "Stat",
                                      "map": table, **rule}
        return binding

    def test_a_map_naming_one_case(self):
        found = self.found(self.lamp({True: "ON"}))
        self.assertTrue(any("no entry for False" in f for f in found), found)
        if HAVE_CODEGEN:
            # `verify` refuses exactly the case that produced the other one.
            result = self.verified(self.lamp({True: "ON"}))
            by_name = {r.name: r for r in result.results}
            self.assertEqual("", by_name["high"].refusal)
            self.assertIn("produced False", by_name["low"].refusal)

    def test_either_spelling_of_both_cases_is_not_refused(self):
        for table in ({True: "ON", False: "OFF"}, {1: "ON", 0: "OFF"},
                      {"true": "ON", "false": "OFF"}):
            with self.subTest(table=table):
                self.assertEqual([], self.found(self.lamp(table)))

    def test_hold_last_leaves_the_other_case_unwritten_on_purpose(self):
        self.assertEqual([], self.found(self.lamp({True: "ON"}, hold_last=True)))

    def test_a_computed_number_output_is_not_judged_before_it_runs(self):
        """The boundary. A COMPUTED number has no list of values to owe
        entries for, so a map missing one is found by the case that produces
        it. (This used `mode ? 1 : 0`, which since 2026-09-26 is read as the
        two literals it can produce -- the next test.)"""
        self.use(DOCUMENT.replace('expr="mode ? 1 : 0"', 'expr="mode ? 1 + 1 : 0"'))
        self.assertEqual([], self.found(self.lamp({2: "ON"})))

    def test_a_number_output_of_literals_owes_each_an_entry(self):
        """`mode ? 1 : 0` can produce 1 and 0 and nothing else, so a map with
        no entry for 0 is refused before any case runs."""
        self.use(DOCUMENT)
        found = self.found(self.lamp({1: "ON"}))
        self.assertTrue(any("no entry for 0" in f for f in found), found)


class ASendIsOwedAnEntry(Both):
    def setUp(self):
        super().setUp()
        self.use(SIGNAL, SIGNAL_MODEL, SIGNAL_EXAMPLES)

    def signal(self, table):
        binding = copy.deepcopy(SIGNAL_BINDING)
        binding["outputs"]["signal"]["map"] = table
        return binding

    def test_the_binding_as_written_is_not_refused(self):
        self.assertEqual([], self.found(SIGNAL_BINDING))

    def test_an_event_the_document_sends(self):
        found = self.found(self.signal({"signal.dark": "DARK"}))
        self.assertTrue(any("no entry for 'signal.flashing'" in f for f in found), found)

    def test_what_nothing_sent_reads_as(self):
        found = self.found(self.signal({"signal.flashing": "FLASHING"}))
        self.assertTrue(any("no entry for 'signal.dark'" in f for f in found), found)


# ---------------------------------------------------------------- an input


class AnInputTheDocumentTakes(Both):
    def setUp(self):
        super().setUp()
        self.use(DOCUMENT)

    def test_an_input_the_document_does_not_declare(self):
        binding = copy.deepcopy(BINDING)
        binding["inputs"]["ghost"] = {"address": "Plant.Input.Count"}
        self.assertSaidByBoth(binding, "the document declares no input 'ghost'")

    def test_the_clock_and_the_variant_name_no_address(self):
        """Read off the case, as the schema says of both. `check` refused
        them as having "no address and no protocol" while `verify` ran them."""
        document = DOCUMENT.replace(
            '<data id="mode" sce:type="bool" sce:direction="in"/>',
            '<data id="mode" sce:type="bool" sce:direction="in"/>'
            '<data id="since" sce:type="int32" sce:direction="in"/>'
            '<data id="branch" sce:type="bool" sce:direction="in"/>')
        self.use(document)
        binding = copy.deepcopy(BINDING)
        binding["inputs"]["since"] = {"clock": True, "when_absent": 0}
        binding["inputs"]["branch"] = {"variant_is": ["BRANCH"]}
        self.assertEqual([], self.found(binding))
        if HAVE_CODEGEN:
            result = self.verified(binding)
            self.assertEqual((2, 0, 0),
                             (result.passed, result.failed, result.unjudged),
                             self.refusals(result))

    def test_an_address_is_still_owed_by_a_rule_that_reads_one(self):
        binding = copy.deepcopy(BINDING)
        binding["inputs"]["mode"] = {"equals": "HIGH"}
        found = self.found(binding)
        self.assertTrue(any("no address and no protocol" in f for f in found), found)


class AStatechartIsHandedOnlyEvents(Both):
    def setUp(self):
        super().setUp()
        self.use(SIGNAL, SIGNAL_MODEL, SIGNAL_EXAMPLES)

    def test_a_pure_driving_binding_is_not_refused(self):
        self.assertEqual([], self.found(SIGNAL_BINDING))

    def test_a_declared_input_the_machine_reads(self):
        """The one that used to FAIL a correct document. The level is 5, the
        machine flashes above 3, and a run that dropped the level reported
        "expected FLASHING, got DARK" against it."""
        self.use(LEVEL, SIGNAL_MODEL, SIGNAL_EXAMPLES)
        binding = copy.deepcopy(SIGNAL_BINDING)
        binding["inputs"]["level"] = {"address": "plant/in/level", "event": "train.approaching"}
        self.assertSaidByBoth(binding, "declares 'level' `sce:direction=\"in\"`")
        if HAVE_CODEGEN:
            self.assertEqual(0, self.verified(binding).failed)

    def test_a_value_key_beside_an_event(self):
        binding = copy.deepcopy(SIGNAL_BINDING)
        binding["inputs"]["approaching"] = {
            "address": "plant/in/approach", "equals": "APPROACHING",
            "event": "train.approaching"}
        self.assertSaidByBoth(binding, "`equals` computes a value for the machine")

    def test_a_rule_with_no_event(self):
        binding = copy.deepcopy(SIGNAL_BINDING)
        binding["inputs"]["idle"] = {"address": "plant/in/approach", "equals": "CLEAR"}
        found = self.found(binding)
        self.assertTrue(any(f.startswith("input idle: names no `event`") for f in found),
                        found)

    def test_a_binding_that_drives_nothing(self):
        binding = copy.deepcopy(SIGNAL_BINDING)
        binding["inputs"] = {"approaching": {"address": "plant/in/approach",
                                             "equals": "APPROACHING"}}
        self.assertSaidByBoth(binding, "no input rule names an `event`")

    def test_every_key_the_driver_does_not_read_is_refused(self):
        """Derived from the published schema, so a key the vocabulary grows
        is refused on a statechart until the driver reads it, rather than
        dropped as every value key was."""
        schema = json.loads((SCHEMA_DIR / "binding.v1.schema.json").read_text(
            encoding="utf-8"))
        keys = set(schema["$defs"]["input"]["properties"])
        document = Document(path=pathlib.Path("signal.scxml"), inputs=(),
                            outputs=(), kind=next(iter(STATECHART_KINDS)))
        for key in sorted(keys):
            rule = {"address": "plant/in/approach", "event": "train.approaching",
                    key: 1}
            refused = driving_refusals(document, {"approaching": rule})
            with self.subTest(key=key):
                if key in _STATECHART_DRIVER_READS | _ANNOTATIONS:
                    self.assertEqual([], refused)
                else:
                    self.assertEqual(1, len(refused), refused)
                    self.assertIn(f"`{key}`", refused[0][1])


# ------------------------------------------------------ a document that keeps


class AKeepingDocumentNeedsItsActivation(Both):
    def setUp(self):
        super().setUp()
        self.use(KEPT)
        self.binding = copy.deepcopy(BINDING)
        self.binding["inputs"]["count"] = {"address": "Plant.Input.Count"}

    def test_no_activation(self):
        self.assertSaidByBoth(self.binding, "does not say how the host runs it")

    def test_an_activation_is_not_refused(self):
        for activation in ("on-change", "periodic"):
            with self.subTest(activation=activation):
                self.assertEqual([], self.found({**self.binding,
                                                 "activation": activation}))

    def test_a_document_that_keeps_nothing_needs_none(self):
        self.use(FORGETS)
        self.assertEqual([], self.found(self.binding))


# A string output that SPELLS the call without making it.
SPELLS = DOCUMENT.replace(
    '<data id="lamp" sce:type="int32" sce:direction="out" expr="mode ? 1 : 0"/>',
    '<data id="lamp" sce:type="int32" sce:direction="out" expr="mode ? 1 : 0"/>'
    '<data id="label" sce:type="string" sce:direction="out" '
    'expr="mode ? \'previous(mode)\' : \'now\'"/>')


@unittest.skipUnless(HAVE_CODEGEN, "the product's code generator is not built")
class WhatItKeepsIsTheProductsAnswer(unittest.TestCase):
    """`check` reads what a document keeps without building it; `verify`
    asks the product, which names a holder exactly when there is something to
    keep. The two readings agree, or `check` asks for an `activation` the run
    does not need, or misses one it does."""

    def test_the_reading_agrees_with_the_holder(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = pathlib.Path(tmp)
            for name, text in (("kept", KEPT), ("forgets", FORGETS),
                               ("spells", SPELLS)):
                with self.subTest(document=name):
                    source = root / f"{name}.scxml"
                    source.write_text(text, encoding="utf-8")
                    out = root / name
                    out.mkdir()
                    build = generate(source, _default_codegen(), out, "python")
                    self.assertEqual("", build.refusal)
                    self.assertEqual(bool(build.holder),
                                     bool(read_document(source).keeps))


if __name__ == "__main__":
    unittest.main()
