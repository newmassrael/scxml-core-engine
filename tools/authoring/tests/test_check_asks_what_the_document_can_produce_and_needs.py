"""`check` answers two things a writer could not hear before `verify`.

Measured 2026-09-26, one document passed `check` with no refusal and then left
every case unjudged, for two reasons nothing the writer can call named:

  - each output was a table of literals (`status == 1 ? 'E1' : ''`) and its
    map was keyed by the INPUT's symbol, so no value it produced had an entry;
  - the output was computed from the status input alone, and the supply
    condition the specification writes in the same table row was gone.

The first is now read off the expression (`landing.expression_values`, used by
`landing.produces`); the second is asked when `check` is handed the
specification (`unread_preconditions`).
"""

from __future__ import annotations

import pathlib
import shutil
import tempfile
import unittest

from sce_author import mcp
from sce_author.check import check
from sce_author.landing import expression_values
from sce_author.pack import load_pack
from sce_author.prose import load_prose

HERE = pathlib.Path(__file__).resolve().parent
PACK = HERE / "fixtures" / "crossing"
SPEC = PACK / "specification.md"


class AnExpressionOfLiteralsSaysWhatItCanProduce(unittest.TestCase):
    def test_a_literal_and_a_conditional_of_literals_are_known(self):
        self.assertEqual(["E1"], expression_values("'E1'"))
        self.assertEqual([2], expression_values("2"))
        self.assertEqual(["E1", ""], expression_values("status === 1 ? 'E1' : ''"))
        self.assertEqual([2, 1], expression_values("(a && b) ? 2 : 1"))

    def test_nesting_brackets_and_strings_that_look_like_operators(self):
        self.assertEqual([1, 2, 3], expression_values("a ? b ? 1 : 2 : 3"))
        self.assertEqual([1, 2, 3], expression_values("a ? (b ? 1 : 2) : (3)"))
        self.assertEqual([1, 0], expression_values("((a ? 1 : 0))"))
        self.assertEqual(["q?:", "r"], expression_values("a ? 'q?:' : 'r'"))

    def test_anything_computed_claims_nothing(self):
        for expr in ("a ? x : 1", "a + 1", "f(a ? 1 : 2)", "previous(x)", None):
            self.assertIsNone(expression_values(expr), expr)


class ACheckedPair(unittest.TestCase):
    def setUp(self):
        self.pack = load_pack(PACK)
        self.dir = pathlib.Path(tempfile.mkdtemp(prefix="sce_check_produce_"))
        for name in ("controller.scxml", "controller.binding.yaml"):
            shutil.copy(PACK / name, self.dir / name)
        self.document = self.dir / "controller.scxml"
        self.binding = self.dir / "controller.binding.yaml"

    def tearDown(self):
        shutil.rmtree(self.dir, ignore_errors=True)

    def replace(self, path, old, new):
        text = path.read_text(encoding="utf-8")
        self.assertIn(old, text)
        path.write_text(text.replace(old, new, 1), encoding="utf-8")

    def details(self, prose=None):
        return [f.detail for f in check(self.pack, self.binding, prose)]


class AMapKeyedByWhatTheDocumentNeverProduces(ACheckedPair):
    def setUp(self):
        super().setUp()
        # The road signal as a value table: DARK, FLASHING or STEADY by number.
        text = self.document.read_text(encoding="utf-8")
        start = text.index('<data id="roadSignal"')
        end = text.index("/>", start) + 2
        self.document.write_text(
            text[:start] + '<data id="roadSignal" sce:type="int32" sce:direction="out" '
            'expr="approaching ? 1 : 0"/>' + text[end:], encoding="utf-8")

    def test_a_map_with_an_entry_for_every_literal_passes(self):
        self.assertFalse([d for d in self.details() if "map has no entry" in d])

    def test_keyed_by_the_inputs_symbol_it_is_refused_and_the_stray_key_named(self):
        self.replace(self.binding, "map: {0: DARK, 1: FLASHING, 2: STEADY}",
                     "map: {APPROACHING: FLASHING}")
        refused = [d for d in self.details() if "map has no entry" in d]
        self.assertEqual(1, len(refused), refused)
        self.assertIn("1, 0", refused[0])
        self.assertIn("'APPROACHING' name no value the document produces", refused[0])

    def test_a_literal_with_no_entry_is_refused_without_blaming_a_good_key(self):
        self.replace(self.binding, "map: {0: DARK, 1: FLASHING, 2: STEADY}",
                     "map: {1: FLASHING}")
        refused = [d for d in self.details() if "map has no entry" in d]
        self.assertEqual(1, len(refused), refused)
        self.assertIn("no entry for 0", refused[0])
        self.assertNotIn("name no value", refused[0])


class AStringOutputsInitialValue(ACheckedPair):
    """What a string output holds before its first activation is the value of a case's first
    round, so the map has to land it. Measured 2026-10-14: a draft wrote `sce:initial="''"` for
    "empty" -- two apostrophes inside an attribute -- the map had `"": ""`, and every case of the
    component was unjudged."""

    def setUp(self):
        super().setUp()
        text = self.document.read_text(encoding="utf-8")
        start = text.index('<data id="roadSignal"')
        end = text.index("/>", start) + 2
        self.document.write_text(
            text[:start] + '<data id="roadSignal" sce:type="string" sce:direction="out" '
            'sce:initial="INITIAL" expr="approaching ? \'FLASHING\' : \'DARK\'"/>' + text[end:],
            encoding="utf-8")
        self.replace(self.binding, "map: {0: DARK, 1: FLASHING, 2: STEADY}",
                     "map: {DARK: DARK, FLASHING: FLASHING}")

    def refused(self):
        return [d for d in self.details() if "sce:initial" in d]

    def test_an_initial_value_the_map_has_no_entry_for_is_refused_and_explained(self):
        refused = self.refused()
        self.assertEqual(1, len(refused), refused)
        self.assertIn("'INITIAL'", refused[0])
        self.assertIn("two apostrophes", refused[0])

    def test_an_initial_value_the_map_lands_is_accepted(self):
        self.replace(self.document, 'sce:initial="INITIAL"', 'sce:initial="DARK"')
        self.assertEqual([], self.refused())

    def test_a_hold_last_output_may_start_on_a_value_its_map_leaves_out(self):
        self.replace(self.binding, "map: {DARK: DARK, FLASHING: FLASHING}",
                     "map: {DARK: DARK, FLASHING: FLASHING}\n    hold_last: true")
        self.assertEqual([], self.refused())


class APreconditionTheSpecificationStates(ACheckedPair):
    """The fixture's specification writes "while the installation is powered",
    which the pack reads as `poweredUp`; the reference pair reads no such name."""

    def unread(self, prose):
        return [d for d in self.details(prose) if "the precondition" in d]

    def test_without_the_specification_nothing_is_asked(self):
        self.assertEqual([], self.unread(None))

    def test_a_precondition_nothing_reads_is_refused_by_phrase_and_input(self):
        refused = self.unread(load_prose([SPEC]))
        self.assertEqual(1, len(refused), refused)
        self.assertIn("'powered'", refused[0])
        self.assertIn("poweredUp", refused[0])
        self.assertIn("specification.md:", refused[0])

    def test_a_document_input_under_that_name_answers_it(self):
        self.replace(self.document, "<datamodel>",
                     '<datamodel>\n    <data id="poweredUp" sce:type="bool" sce:direction="in"/>')
        self.assertEqual([], self.unread(load_prose([SPEC])))

    def test_a_binding_rule_under_that_name_answers_it(self):
        self.replace(self.binding, "inputs:\n",
                     "inputs:\n  poweredUp:\n    address: plant/in/mains-power\n"
                     "    equals: OK\n")
        self.assertEqual([], self.unread(load_prose([SPEC])))

    def test_the_server_asks_it_when_given_the_specification(self):
        result = mcp.call_tool("check", {"pack": str(PACK), "binding": str(self.binding),
                                         "prose": [str(SPEC)]})
        self.assertTrue(result.get("isError"))
        self.assertIn("poweredUp", result["content"][0]["text"])

    def prose_of(self, *lines):
        path = self.dir / "other_specification.md"
        path.write_text("\n".join(lines) + "\n", encoding="utf-8")
        return load_prose([path])

    def test_a_condition_written_only_on_a_line_about_an_output_the_pack_lacks_is_not_asked(self):
        # OUT_Phantom is classed an output of the document by the pack's name classes, and
        # the interface model has no such address: nothing a document writes can be gated.
        prose = self.prose_of("While the installation is powered, OUT_Phantom follows the approach.")
        self.assertEqual([], self.unread(prose))

    def test_a_line_naming_an_output_the_pack_declares_still_asks_for_it(self):
        prose = self.prose_of("While the installation is powered, OUT_RoadSignal follows the approach.")
        refused = self.unread(prose)
        self.assertEqual(1, len(refused), refused)
        self.assertIn("poweredUp", refused[0])

    def test_one_declared_output_among_the_names_is_enough_to_ask(self):
        prose = self.prose_of("While the installation is powered, OUT_Phantom and OUT_Bell follow it.")
        self.assertEqual(1, len(self.unread(prose)))

    def test_a_line_that_names_no_output_still_asks_for_it(self):
        prose = self.prose_of("While the installation is powered, nothing else is shown.")
        self.assertEqual(1, len(self.unread(prose)))

    def test_every_place_the_condition_is_written_has_to_be_about_a_missing_output(self):
        prose = self.prose_of("While the installation is powered, OUT_Phantom follows the approach.",
                              "While the installation is powered, the bell is silent.")
        refused = self.unread(prose)
        self.assertEqual(1, len(refused), refused)
        self.assertIn("2 time(s)", refused[0])


if __name__ == "__main__":
    unittest.main()
