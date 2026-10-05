"""A pack can say which value a field holds when the specification states none.

A specification shows an output and the condition that turns it on, and is silent on a field
that is the same nearly everywhere -- how a linked sound plays, how many times. A writer fills
that silence with a guess. Measured on one platform's own cases, the same two values held in
84% and 96% of the observations across ninety-seven components, so most of the silences a
reader found on that platform were one fact the pack could have stated once. `held_when_off`
is about what stays described while an output is off and `companion_symbol` about a symbol
that depends on another field; neither says what a field holds by default.

So a pack may state it: `field_defaults`, a rule per kind of output (an address pattern)
giving, per field, the value it holds unless the specification states another. The core
knows no platform and no field name; it reads the rule, says it beside the field in the
brief with the rate the pack measured, and holds it to the interface model so that it names
no value a document could not write.

Asserted here, in the order a reader meets it:

  the pack   a rule is read; one that cannot be read is refused naming the file and the key
  the brief  the sentence is printed for exactly the fields the rule names, on the outputs
             whose address it matches and that have them, and says whose claim it is
  the model  a symbol the field does not admit, a number outside its range, or a non-number
             for a number field is refused naming the address; a rule about nothing the pack
             has is not refused
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

# An output with a symbolic field, a counted field and a field with no value space at all.
SOUNDED = {
    "address": "Plant.Out.Lamp",
    "role": "output",
    "names": ["Fig_Lamp_stat"],
    "fields": {
        "Stat": {"values": {"NONE": 0, "OFF": 1, "ON": 2, "MAX": 3}},
        "Sound.Kind": {"values": {"NONE": 0, "ONCE": 1, "REPEAT": 2, "MAX": 3}},
        "Sound.Count": {"type": "integer", "range": {"minimum": 0, "maximum": 5}},
        "Label": {"type": "text"},
    },
}

# An output of the same kind that has no sound fields: the rule is about nothing here.
PLAIN = {
    "address": "Plant.Out.Plain",
    "role": "output",
    "names": ["Fig_Plain_stat"],
    "fields": {"Stat": {"values": {"NONE": 0, "OFF": 1, "ON": 2, "MAX": 3}}},
}


def model_with_sound(*extra):
    model = copy.deepcopy(MODEL)
    model["entries"] = [SOUNDED if e["address"] == "Plant.Out.Lamp" else e for e in model["entries"]]
    model["entries"] += list(extra)
    return model


def rule(**over):
    return {
        "address_pattern": r"\.Out\.(Lamp|Plain)$",
        "fields": {"Sound.Kind": "REPEAT", "Sound.Count": 1},
        **over,
    }


def conventions(*rules):
    return {**CONVENTIONS, "field_defaults": list(rules)}


class ADefaultIsSaidBesideItsField(Fixture):
    def setUp(self):
        super().setUp()
        self.write_pack(model_with_sound(PLAIN), conventions(rule()))

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
        read = self.pack().conventions.field_defaults
        self.assertEqual(1, len(read))
        self.assertEqual({"Sound.Kind": "REPEAT", "Sound.Count": 1}, dict(read[0].fields))

    def test_the_brief_says_the_value_of_each_field_the_rule_names(self):
        self.assertIn("holds `REPEAT`", self.said("Sound.Kind"))
        self.assertIn("holds `1`", self.said("Sound.Count"))

    def test_the_brief_says_whose_claim_it_is(self):
        ours = [line for line in self.lines_under(self.section_two(), "Out.Lamp", "Sound.Kind")
                if "holds `REPEAT`" in line]
        self.assertEqual(1, len(ours), ours)
        self.assertIn("the pack's convention", ours[0])
        self.assertIn("unless the specification states another", ours[0])
        self.assertIn("confirm it against the platform", ours[0])

    def test_the_brief_says_what_it_rests_on(self):
        self.write_pack(model_with_sound(PLAIN),
                        conventions(rule(measured="1111 of 1329 cases")))
        self.assertIn("1111 of 1329 cases", self.said("Sound.Kind"))

    def test_it_is_not_said_of_a_field_the_rule_does_not_name(self):
        for field in ("Stat", "Label"):
            self.assertNotIn("unless the specification states another", self.said(field), field)

    def test_it_is_not_said_of_an_output_that_lacks_the_field(self):
        said = self.said("Stat", address="Out.Plain")
        self.assertNotIn("unless the specification states another", said)

    def test_it_is_not_said_of_an_output_the_pattern_does_not_match(self):
        self.write_pack(model_with_sound(PLAIN),
                        conventions(rule(address_pattern=r"\.Elsewhere\.")))
        self.assertNotIn("unless the specification states another", self.said("Sound.Kind"))

    def test_the_first_rule_that_names_the_field_decides(self):
        self.write_pack(model_with_sound(PLAIN),
                        conventions(rule(), rule(fields={"Sound.Kind": "ONCE"})))
        said = self.said("Sound.Kind")
        self.assertIn("holds `REPEAT`", said)
        self.assertNotIn("holds `ONCE`", said)


class APackWithoutTheKeyPrintsNothingNew(Fixture):
    def test_the_brief_has_no_such_sentence(self):
        self.write_pack(model_with_sound(), CONVENTIONS)
        page = assemble(self.prose(PROSE), self.pack())
        self.assertNotIn("unless the specification states another", page)
        self.assertEqual((), tuple(self.pack().conventions.field_defaults))


class ARuleThatCannotBeReadIsRefused(Fixture):
    def refused(self, *rules):
        self.write_pack(model_with_sound(PLAIN), conventions(*rules))
        with self.assertRaises(PackError) as caught:
            load_pack(self.pack_dir)
        return str(caught.exception)

    def test_a_pattern_that_is_not_a_regular_expression_names_the_file_and_the_key(self):
        message = self.refused(rule(address_pattern="("))
        self.assertIn("conventions.yaml", message)
        self.assertIn("field_defaults", message)

    def test_a_rule_that_leaves_out_a_part_of_the_claim_is_refused(self):
        for part in ("fields", "address_pattern"):
            claim = rule()
            del claim[part]
            self.assertTrue(self.refused(claim), part)

    def test_a_rule_that_gives_no_field_a_value_is_refused(self):
        self.assertTrue(self.refused(rule(fields={})))

    def test_a_key_the_core_does_not_know_is_refused(self):
        self.assertTrue(self.refused(rule(unknown="x")))


class ARuleIsHeldToTheInterfaceModel(Fixture):
    def refused(self, *rules):
        self.write_pack(model_with_sound(PLAIN), conventions(*rules))
        with self.assertRaises(PackError) as caught:
            load_pack(self.pack_dir)
        return str(caught.exception)

    def test_a_symbol_the_field_does_not_admit_names_the_address_and_what_it_admits(self):
        message = self.refused(rule(fields={"Sound.Kind": "NOWHERE"}))
        self.assertIn("field_defaults[1]", message)
        self.assertIn("Plant.Out.Lamp", message)
        self.assertIn("'NOWHERE'", message)
        self.assertIn("NONE, ONCE, REPEAT", message.replace("MAX, ", ""))

    def test_a_number_outside_the_range_the_field_carries_is_refused(self):
        message = self.refused(rule(fields={"Sound.Count": 9}))
        self.assertIn("Sound.Count", message)
        self.assertIn("range", message)

    def test_a_word_for_a_number_field_is_refused(self):
        message = self.refused(rule(fields={"Sound.Count": "once"}))
        self.assertIn("number field", message)

    def test_a_check_lists_both_mistakes_at_once(self):
        self.write_pack(model_with_sound(PLAIN),
                        conventions(rule(fields={"Sound.Kind": "NOWHERE", "Sound.Count": 9})))
        report = check_pack(self.pack_dir)
        self.assertEqual(2, len(report.problems), [str(p) for p in report.problems])

    def test_a_rule_about_nothing_the_pack_has_is_not_refused(self):
        self.write_pack(model_with_sound(PLAIN),
                        conventions(rule(address_pattern=r"\.Elsewhere\.",
                                         fields={"Sound.Kind": "NOWHERE"})))
        self.assertEqual(1, len(load_pack(self.pack_dir).conventions.field_defaults))

    def test_a_value_is_not_held_to_an_output_that_lacks_the_field(self):
        # The pattern finds the plain output too, and it has no sound fields: there the
        # rule is about nothing, so only the output that has them is held to it.
        self.write_pack(model_with_sound(PLAIN), conventions(rule()))
        self.assertEqual(1, len(load_pack(self.pack_dir).conventions.field_defaults))

    def test_a_pack_whose_model_did_not_load_says_the_rule_was_not_held(self):
        # An address declared twice is refused, and the model is then not held to anything.
        broken = copy.deepcopy(model_with_sound(PLAIN))
        broken["entries"].append(copy.deepcopy(broken["entries"][0]))
        self.write_pack(broken, conventions(rule()))
        report = check_pack(self.pack_dir)
        self.assertTrue(any("field_defaults" in s for s in report.skipped), report.skipped)


if __name__ == "__main__":
    unittest.main()
