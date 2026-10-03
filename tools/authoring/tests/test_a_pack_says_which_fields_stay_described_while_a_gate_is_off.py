"""A pack can say which fields of an output keep their description while its gate is off.

A specification shows what an output describes (an identifier, a label, a count) where it
SHOWS the output, and says nothing of what the same fields hold while the output is off.
A writer fills that silence with the emptiest value there is: an empty text, NONE, zero.
Measured on one platform's own cases, every case that reports an output off still reports
its description, so the emptiest value was wrong in every one of them -- and the pack had
no way to say so, because `gate_off` is about the VALUE of the gate and a text field has
no value space to propose a value from.

So a pack may state it: `held_when_off`, a rule per kind of output (an address pattern),
naming the gate field and the fields that stay described. The core knows no platform and
no field name; it reads the rule and says it, beside the field it is about, in the brief.

Asserted here, in the order a reader meets it:

  the pack   a rule is read; one that cannot be read is refused naming the file and the key
  the brief  the sentence is printed for exactly the output fields the rule names, on the
             outputs whose address it matches, and says whose claim it is
  absence    a pack without the key prints nothing new
"""

from __future__ import annotations

import copy
import unittest

from sce_author.brief import assemble
from sce_author.errors import PackError
from sce_author.pack import load_pack
from tests.test_refusals_actually_fire import CONVENTIONS, MODEL, Fixture

PROSE = "Fig_Lamp_stat is ON when In_SupplyMode == HIGH, and OFF otherwise.\n"

# One output kind that describes itself (a label and a count) beside its gate.
DESCRIBED = {
    "address": "Plant.Out.Lamp",
    "role": "output",
    "names": ["Fig_Lamp_stat"],
    "fields": {
        "Stat": {"values": {"NONE": 0, "OFF": 1, "ON": 2, "MAX": 3}},
        "Label": {"type": "text"},
        "Count": {"type": "number"},
        "Value": {"type": "number"},
    },
}


def model_with_described_output():
    model = copy.deepcopy(MODEL)
    model["entries"] = [DESCRIBED if e["address"] == "Plant.Out.Lamp" else e for e in model["entries"]]
    return model


def rule(**over):
    return {
        "address_pattern": r"\.Out\.Lamp$",
        "gate": "Stat",
        "fields": ["Label", "Count"],
        **over,
    }


class APackSaysWhichFieldsStayDescribed(Fixture):
    def setUp(self):
        super().setUp()
        self.write_pack(model_with_described_output(), {**CONVENTIONS, "held_when_off": [rule()]})

    def section_two(self):
        page = assemble(self.prose(PROSE), self.pack())
        return page.split("## 2.")[1].split("## 3.")[0]

    def lines_under(self, section, field):
        """The sentences printed under one field of the Lamp output."""
        out, current = [], None
        for line in section.splitlines():
            if line.startswith("- `"):
                current = line
            elif current is not None and "Out.Lamp" in current and line.startswith("  - `"):
                inside = line.startswith(f"  - `.{field}`")
            elif current is not None and "Out.Lamp" in current and line.startswith("    -"):
                if inside:
                    out.append(line)
        return out

    def test_the_rule_is_read_into_the_conventions(self):
        conventions = self.pack().conventions
        self.assertEqual(1, len(conventions.held_when_off))
        self.assertEqual("Stat", conventions.held_when_off[0].gate)
        self.assertEqual(("Label", "Count"), conventions.held_when_off[0].fields)

    def test_the_brief_says_it_beside_each_field_the_rule_names(self):
        section = self.section_two()
        for field in ("Label", "Count"):
            said = " ".join(self.lines_under(section, field))
            self.assertIn("stays described", said, f"{field}: {section}")
            self.assertIn("`Stat` is off", said)
            self.assertIn("not emptied, zeroed or replaced by a placeholder", said)

    def test_the_brief_does_not_say_it_of_the_gate_or_of_a_field_the_rule_does_not_name(self):
        section = self.section_two()
        for field in ("Stat", "Value"):
            said = " ".join(self.lines_under(section, field))
            self.assertNotIn("is off", said.replace("precondition is false", ""), f"{field}: {said}")
            self.assertNotIn("stays", said)

    def test_the_brief_says_whose_claim_it_is_and_what_it_rests_on(self):
        self.write_pack(
            model_with_described_output(),
            {**CONVENTIONS, "held_when_off": [rule(measured="930 of 930 cases")]},
        )
        said = " ".join(self.lines_under(self.section_two(), "Label"))
        self.assertIn("the pack's convention", said)
        self.assertIn("930 of 930 cases", said)
        self.assertIn("confirm it against the platform", said)

    def test_it_is_not_said_of_an_output_the_pattern_does_not_match(self):
        self.write_pack(
            model_with_described_output(),
            {**CONVENTIONS, "held_when_off": [rule(address_pattern=r"\.Elsewhere\.")]},
        )
        for field in ("Label", "Count"):
            self.assertEqual([], [l for l in self.lines_under(self.section_two(), field) if "is off" in l])

    def test_it_is_not_said_of_an_input(self):
        model = model_with_described_output()
        model["entries"].append({
            "address": "Plant.Out.LampIn", "role": "input", "names": ["In_Label"],
            "values": {"NONE": 0, "A": 1},
        })
        self.write_pack(model, {**CONVENTIONS, "held_when_off": [rule(address_pattern=r"Plant\.")]})
        page = assemble(self.prose(PROSE + "In_Label is read.\n"), self.pack())
        section = page.split("## 2.")[1].split("## 3.")[0]
        owner = None
        for line in section.splitlines():
            if line.startswith("- `"):
                owner = line
            elif "is off" in line and "stays" in line:
                self.assertIn("(output)", owner)


class APackWithoutTheKeyPrintsNothingNew(Fixture):
    def test_the_brief_has_no_such_sentence(self):
        self.write_pack(model_with_described_output(), CONVENTIONS)
        page = assemble(self.prose(PROSE), self.pack())
        self.assertNotIn("stays described", page)
        self.assertEqual((), tuple(self.pack().conventions.held_when_off))


class ARuleThatCannotBeReadIsRefused(Fixture):
    def refused(self, held):
        self.write_pack(model_with_described_output(), {**CONVENTIONS, "held_when_off": held})
        with self.assertRaises(PackError) as caught:
            load_pack(self.pack_dir)
        return str(caught.exception)

    def test_a_pattern_that_is_not_a_regular_expression_names_the_file_and_the_key(self):
        message = self.refused([rule(address_pattern="(")])
        self.assertIn("conventions.yaml", message)
        self.assertIn("held_when_off", message)

    def test_a_rule_that_names_no_field_is_refused(self):
        self.assertTrue(self.refused([rule(fields=[])]))

    def test_a_rule_that_holds_its_own_gate_is_refused(self):
        message = self.refused([rule(fields=["Stat", "Label"])])
        self.assertIn("gate", message)

    def test_a_key_the_core_does_not_know_is_refused(self):
        self.assertTrue(self.refused([rule(unknown="x")]))


if __name__ == "__main__":
    unittest.main()
