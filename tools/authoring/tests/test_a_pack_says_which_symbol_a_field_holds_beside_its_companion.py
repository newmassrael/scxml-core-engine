"""A pack can say which symbol a field holds while another field of its output is in use.

A specification shows an output's companion field where it shows the output -- a second
channel that carries part of the display -- and says nothing of what the first field holds
beside it. A writer fills that silence with the symbol the field's name suggests, the plain
"on", where the platform keeps a different symbol that says the companion carries the
display. Measured on one platform's own cases, the field held that symbol in most of the
cases that used the companion and in none of the cases that did not, so the plain symbol
was wrong in nearly every case that mattered -- and the pack had no way to say so:
`gate_off` is about what an output becomes when its precondition is false, and
`held_when_off` is about fields that stay described, neither about a symbol that depends
on another field.

So a pack may state it: `companion_symbol`, a rule per kind of output (an address pattern)
naming the field, its companion, the symbol the field holds while the companion is in use,
and the symbol that means the companion is not. The core knows no platform and no field
name; it reads the rule and says it, beside the field it is about, in the brief, and holds
it to the interface model so that it names no symbol a document could not write.

Asserted here, in the order a reader meets it:

  the pack   a rule is read; one that cannot be read is refused naming the file and the key
  the brief  the sentence is printed for exactly the field the rule names, on the outputs
             whose address it matches and that have the companion, and says whose claim it is
  the model  a symbol the field's value space does not admit is refused, naming the address
             and what the space does admit; a rule about nothing the pack has is not refused
  absence    a pack without the key prints nothing new
"""

from __future__ import annotations

import copy
import unittest

from sce_author.brief import assemble
from sce_author.errors import PackError
from sce_author.pack import check_pack, load_pack
from tests.test_refusals_actually_fire import CONVENTIONS, MODEL, Fixture

PROSE = "Fig_Lamp_stat is ON when In_SupplyMode == HIGH, and OFF otherwise.\n"

# An output with a second channel beside its state, and one value that is not a symbol.
DISPLAYED = {
    "address": "Plant.Out.Lamp",
    "role": "output",
    "names": ["Fig_Lamp_stat"],
    "fields": {
        "Stat": {"values": {"NONE": 0, "OFF": 1, "ON": 2, "SPARE": 3, "MAX": 4}},
        "Aux": {"values": {"NONE": 0, "OFF": 1, "GREEN": 2, "MAX": 3}},
        "Value": {"type": "number"},
    },
}

# An output of the same kind that has no second channel: the rule is about nothing here.
PLAIN = {
    "address": "Plant.Out.Plain",
    "role": "output",
    "names": ["Fig_Plain_stat"],
    "fields": {"Stat": {"values": {"NONE": 0, "OFF": 1, "ON": 2, "MAX": 3}}},
}


def model_with_a_companion(*extra):
    model = copy.deepcopy(MODEL)
    model["entries"] = [DISPLAYED if e["address"] == "Plant.Out.Lamp" else e for e in model["entries"]]
    model["entries"] += list(extra)
    return model


def rule(**over):
    return {
        "address_pattern": r"\.Out\.(Lamp|Plain)$",
        "field": "Stat",
        "companion": "Aux",
        "symbol": "SPARE",
        "companion_off": "OFF",
        **over,
    }


def conventions(*rules):
    return {**CONVENTIONS, "companion_symbol": list(rules)}


class ACompanionRuleIsSaidBesideItsField(Fixture):
    def setUp(self):
        super().setUp()
        self.write_pack(model_with_a_companion(PLAIN), conventions(rule()))

    def section_two(self, prose=PROSE + "Fig_Plain_stat is ON when In_SupplyMode == LOW.\n"):
        page = assemble(self.prose(prose), self.pack())
        return page.split("## 2.")[1].split("## 3.")[0]

    def lines_under(self, section, address, field):
        """The sentences printed under one field of one output."""
        out, current, inside = [], None, False
        for line in section.splitlines():
            if line.startswith("- `"):
                current = line
            elif current is not None and address in current and line.startswith("  - `"):
                inside = line.startswith(f"  - `.{field}`")
            elif current is not None and address in current and line.startswith("    -"):
                if inside:
                    out.append(line)
        return out

    def said(self, field, address="Out.Lamp"):
        return " ".join(self.lines_under(self.section_two(), address, field))

    def test_the_rule_is_read_into_the_conventions(self):
        read = self.pack().conventions.companion_symbol
        self.assertEqual(1, len(read))
        self.assertEqual(("Stat", "Aux", "SPARE", "OFF"),
                         (read[0].field, read[0].companion, read[0].symbol, read[0].companion_off))

    def test_the_brief_says_the_symbol_the_companion_and_its_off_symbol(self):
        said = self.said("Stat")
        self.assertIn("holds `SPARE`", said)
        self.assertIn("`Aux` of this output", said)
        self.assertIn("anything but `OFF`", said)
        self.assertIn("and not otherwise", said)

    def test_the_brief_says_whose_claim_it_is(self):
        # The field's other sentences say it too, so it is read off THIS sentence only.
        ours = [line for line in self.lines_under(self.section_two(), "Out.Lamp", "Stat")
                if "holds `SPARE`" in line]
        self.assertEqual(1, len(ours), ours)
        self.assertIn("the pack's convention", ours[0])
        self.assertIn("confirm it against the platform", ours[0])
        self.assertIn("does not say", ours[0])

    def test_the_brief_says_what_it_rests_on(self):
        self.write_pack(model_with_a_companion(PLAIN),
                        conventions(rule(measured="125 of 136 while in use, 19 of 19 while off")))
        self.assertIn("125 of 136 while in use, 19 of 19 while off", self.said("Stat"))

    def test_it_is_not_said_of_the_companion_or_of_a_field_the_rule_does_not_name(self):
        for field in ("Aux", "Value"):
            said = self.said(field)
            self.assertNotIn("holds `SPARE`", said, f"{field}: {said}")
            self.assertNotIn("not otherwise", said, f"{field}: {said}")

    def test_it_is_not_said_of_an_output_that_lacks_the_companion(self):
        # The pack's other sentences about the field are still there; this one is not.
        said = self.said("Stat", address="Out.Plain")
        self.assertIn("PROPOSES", said)
        self.assertNotIn("holds `SPARE`", said)
        self.assertNotIn("not otherwise", said)

    def test_it_is_not_said_of_an_output_the_pattern_does_not_match(self):
        self.write_pack(model_with_a_companion(PLAIN),
                        conventions(rule(address_pattern=r"\.Elsewhere\.")))
        self.assertNotIn("holds `SPARE`", self.said("Stat"))

    def test_it_is_not_said_of_an_input(self):
        model = model_with_a_companion()
        model["entries"].append({
            "address": "Plant.Out.LampIn", "role": "input", "names": ["In_Label"],
            "values": {"NONE": 0, "A": 1},
        })
        self.write_pack(model, conventions(rule(address_pattern=r"Plant\.")))
        section = self.section_two(PROSE + "In_Label is read.\n")
        owner = None
        for line in section.splitlines():
            if line.startswith("- `"):
                owner = line
            elif "not otherwise" in line:
                self.assertIn("(output)", owner)

    def test_the_first_rule_that_names_the_field_decides(self):
        self.write_pack(model_with_a_companion(PLAIN),
                        conventions(rule(), rule(symbol="ON")))
        said = self.said("Stat")
        self.assertIn("holds `SPARE`", said)
        self.assertNotIn("holds `ON`", said)


class APackWithoutTheKeyPrintsNothingNew(Fixture):
    def test_the_brief_has_no_such_sentence(self):
        self.write_pack(model_with_a_companion(), CONVENTIONS)
        page = assemble(self.prose(PROSE), self.pack())
        self.assertNotIn("not otherwise", page)
        self.assertEqual((), tuple(self.pack().conventions.companion_symbol))


class ARuleThatCannotBeReadIsRefused(Fixture):
    def refused(self, *rules):
        self.write_pack(model_with_a_companion(PLAIN), conventions(*rules))
        with self.assertRaises(PackError) as caught:
            load_pack(self.pack_dir)
        return str(caught.exception)

    def test_a_pattern_that_is_not_a_regular_expression_names_the_file_and_the_key(self):
        message = self.refused(rule(address_pattern="("))
        self.assertIn("conventions.yaml", message)
        self.assertIn("companion_symbol", message)

    def test_a_field_that_is_its_own_companion_is_refused(self):
        message = self.refused(rule(companion="Stat"))
        self.assertIn("companion_symbol[1]", message)
        self.assertIn("own companion", message)

    def test_a_rule_that_leaves_out_a_part_of_the_claim_is_refused(self):
        for part in ("field", "companion", "symbol", "companion_off", "address_pattern"):
            claim = rule()
            del claim[part]
            self.assertTrue(self.refused(claim), part)

    def test_an_empty_symbol_is_refused(self):
        self.assertTrue(self.refused(rule(symbol="")))

    def test_a_key_the_core_does_not_know_is_refused(self):
        self.assertTrue(self.refused(rule(unknown="x")))


class ARuleIsHeldToTheInterfaceModel(Fixture):
    def refused(self, *rules):
        self.write_pack(model_with_a_companion(PLAIN), conventions(*rules))
        with self.assertRaises(PackError) as caught:
            load_pack(self.pack_dir)
        return str(caught.exception)

    def test_a_symbol_the_field_does_not_admit_names_the_address_and_what_it_admits(self):
        message = self.refused(rule(symbol="NOWHERE"))
        self.assertIn("companion_symbol[1]", message)
        self.assertIn("Plant.Out.Lamp", message)
        self.assertIn("'NOWHERE'", message)
        self.assertIn("NONE, OFF, ON, SPARE", message.replace("MAX, ", ""))

    def test_an_off_symbol_the_companion_does_not_admit_is_refused(self):
        message = self.refused(rule(companion_off="DARK"))
        self.assertIn("'Aux'", message)
        self.assertIn("'DARK'", message)

    def test_a_field_with_no_value_space_holds_no_symbol(self):
        message = self.refused(rule(field="Value"))
        self.assertIn("no value space", message)

    def test_a_companion_with_no_value_space_has_no_off_symbol(self):
        message = self.refused(rule(companion="Value"))
        self.assertIn("no value space", message)

    def test_a_check_lists_both_mistakes_at_once(self):
        self.write_pack(model_with_a_companion(PLAIN),
                        conventions(rule(symbol="NOWHERE", companion_off="DARK")))
        report = check_pack(self.pack_dir)
        self.assertEqual(2, len(report.problems), [str(p) for p in report.problems])

    def test_a_rule_about_nothing_the_pack_has_is_not_refused(self):
        self.write_pack(model_with_a_companion(PLAIN),
                        conventions(rule(address_pattern=r"\.Elsewhere\.", symbol="NOWHERE")))
        self.assertEqual(1, len(load_pack(self.pack_dir).conventions.companion_symbol))

    def test_a_symbol_is_not_held_to_an_output_that_lacks_the_companion(self):
        # The pattern finds the plain output too, and it has no `Aux`: there the rule is
        # about nothing, so only the output that has both fields is held to it.
        self.write_pack(model_with_a_companion(PLAIN), conventions(rule()))
        self.assertEqual(1, len(load_pack(self.pack_dir).conventions.companion_symbol))

    def test_a_pack_whose_model_did_not_load_says_the_rule_was_not_held(self):
        # An address declared twice is refused, and the model is then not held to anything.
        broken = copy.deepcopy(model_with_a_companion(PLAIN))
        broken["entries"].append(copy.deepcopy(broken["entries"][0]))
        self.write_pack(broken, conventions(rule()))
        report = check_pack(self.pack_dir)
        self.assertTrue(any("companion_symbol" in s for s in report.skipped), report.skipped)


if __name__ == "__main__":
    unittest.main()
