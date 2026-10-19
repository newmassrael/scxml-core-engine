"""The schema files of a scope and a ledger, and the code that makes them, say the same.

A scope and a ledger are wire forms (`SCHEMA_STATUS`): a host writes the ledger a screen built and the specs
folder reads a scope back, so something outside this tool may read them by their schemas alone. A schema that
is looser than the code lets a producer write what this tool would refuse, and one that is stricter refuses
what this tool writes. Each rule the code holds is put to the schema as well, with the nearest input that must
be accepted, and what the code writes is put to the schema.
"""

from __future__ import annotations

import json
import pathlib
import unittest

import jsonschema

from sce_author import ledger, scope
from sce_author.errors import IngestError
from sce_author.extraction import Block, Extraction, Place
from sce_author.ledger import LEFT_OUT, NEW, SPEC, Allotment, Destination, Ledger
from sce_author.scope import Scope

SCHEMAS = pathlib.Path(__file__).resolve().parents[1] / "schema"
SCOPE = json.loads((SCHEMAS / "scope.v1.schema.json").read_text(encoding="utf-8"))
LEDGER = json.loads((SCHEMAS / "ledger.v1.schema.json").read_text(encoding="utf-8"))
REVISION = "a" * 64


def valid(schema: dict, wire: object) -> bool:
    try:
        jsonschema.validate(wire, schema)
    except jsonschema.ValidationError:
        return False
    return True


def code_accepts(parse, wire: object) -> bool:
    try:
        parse(wire)
    except IngestError:
        return False
    return True


CASES = json.loads(
    (pathlib.Path(__file__).resolve().parent / "fixtures" / "ledger_wire_cases.json").read_text(encoding="utf-8")
)
WORDS = {
    "$SPEC80": "a" * 80,
    "$SPEC81": "a" * 81,
    "$TITLE200": "t" * 200,
    "$TITLE201": "t" * 201,
    "$REASON1000": "r" * 1000,
    "$REASON1001": "r" * 1001,
}


def extraction_of_cases() -> Extraction:
    return Extraction(
        "plain",
        "1",
        "spec.md",
        tuple(
            Block(identifier, "paragraph", f"words of {identifier}", Place("spec.md", ordinal=number))
            for number, identifier in enumerate(CASES["blocks"], 1)
        ),
    )


def filled(value, revision: str):
    """A case with the words it stands for put in: the revision of the extraction, and text of a length."""
    if isinstance(value, str):
        return revision if value == "$EXTRACTION" else WORDS.get(value, value)
    if isinstance(value, list):
        return [filled(item, revision) for item in value]
    if isinstance(value, dict):
        return {key: filled(item, revision) for key, item in value.items()}
    return value


class TheToolsAndTheSpecsFolderReadALedgerTheSame(unittest.TestCase):
    """`tools/authoring/tests/fixtures/ledger_wire_cases.json` is read by the specs folder's tests too
    (`sce-specs/tests/ledger_cases.rs`): a ledger one side refuses and the other keeps is a ledger the
    owner's screen can save and no tool can count."""

    def setUp(self) -> None:
        self.extraction = extraction_of_cases()
        self.revision = self.extraction.revision()

    def read(self, wire: dict) -> Ledger:
        return Ledger.from_dict(wire).held_by(self.extraction)

    def test_every_good_ledger_is_read_and_the_schema_takes_it(self):
        for case in CASES["good"]:
            wire = filled(case["wire"], self.revision)
            with self.subTest(case["name"]):
                self.read(wire)
                self.assertTrue(valid(LEDGER, wire))

    def test_every_bad_ledger_is_refused_by_the_code_and_a_form_fault_by_the_schema_too(self):
        for case in CASES["bad"]:
            wire = filled(case["wire"], self.revision)
            with self.subTest(case["name"]):
                with self.assertRaises(IngestError):
                    self.read(wire)
                if case["layer"] == "form":
                    # A fault of the form is the form's own: the parse refuses it before any extraction is asked.
                    with self.assertRaises(IngestError):
                        Ledger.from_dict(wire)
                    self.assertFalse(valid(LEDGER, wire), "the schema accepts what the code refuses")
                elif case["layer"] == "list":
                    # A fault of the allotments against each other is refused as the ledger is read, and is one
                    # a schema cannot say (it has no way to say that two items are of one destination).
                    with self.assertRaises(IngestError):
                        Ledger.from_dict(wire)
                    self.assertTrue(valid(LEDGER, wire))
                else:
                    # A fault against the blocks is one a schema cannot say, so it takes the ledger.
                    Ledger.from_dict(wire)
                    self.assertTrue(valid(LEDGER, wire))

    def test_the_cases_cover_what_the_schema_says_each_way(self):
        # A rule of the schema with no case in the file is a rule only one side is held to.
        layers = {case["layer"] for case in CASES["bad"]}
        self.assertEqual(layers, {"form", "list", "extraction"})
        self.assertGreaterEqual(len(CASES["good"]), 5)


class TheSchemasAndTheCodeAgree(unittest.TestCase):
    def test_each_is_a_schema_and_says_its_status_as_the_code_does(self):
        for schema, module in ((SCOPE, scope), (LEDGER, ledger)):
            jsonschema.Draft202012Validator.check_schema(schema)
            self.assertEqual(schema["x-sce-schema-status"], module.SCHEMA_STATUS)
            self.assertEqual(schema["properties"]["v"]["const"], module.SCHEMA_VERSION)

    def test_the_kinds_of_destination_are_the_same_list(self):
        kinds = [branch["properties"]["kind"]["const"] for branch in LEDGER["$defs"]["destination"]["oneOf"]]

        self.assertEqual(tuple(kinds), ledger.KINDS)

    def test_the_limits_are_the_same_numbers(self):
        title = LEDGER["$defs"]["destination"]["oneOf"][1]["properties"]["title"]
        reason = LEDGER["$defs"]["allotment"]["properties"]["reason"]

        self.assertEqual(title["maxLength"], ledger.MAX_TITLE)
        self.assertEqual(reason["maxLength"], ledger.MAX_REASON)

    def test_what_the_code_writes_the_schemas_accept(self):
        made = Ledger(
            REVISION,
            (
                Allotment(Destination(SPEC, spec="lamp-1"), ("b1", "b2")),
                Allotment(Destination(NEW, title="Fan"), ("b2", "b3")),
                Allotment(Destination(LEFT_OUT), ("b4",), "the cover"),
                Allotment(Destination(LEFT_OUT), ("b5",)),
            ),
        )

        self.assertTrue(valid(LEDGER, made.to_dict()))
        self.assertTrue(valid(LEDGER, Ledger(REVISION).to_dict()))
        self.assertTrue(valid(SCOPE, Scope(REVISION, ("b1", "b2")).to_dict()))

    def test_a_scope_the_code_refuses_the_schema_refuses_and_the_nearest_good_one_both_take(self):
        good = {"v": 1, "extraction": REVISION, "blocks": ["b1", "b2"]}
        self.assertTrue(valid(SCOPE, good) and code_accepts(Scope.from_dict, good))
        for name, wire in {
            "another version": {**good, "v": 2},
            "an extraction that is not a revision": {**good, "extraction": "short"},
            "no block": {**good, "blocks": []},
            "a block twice": {**good, "blocks": ["b1", "b1"]},
            "a block with no id": {**good, "blocks": ["b1", ""]},
            "a field nobody defined": {**good, "colour": "red"},
            "a missing field": {"v": 1, "extraction": REVISION},
        }.items():
            with self.subTest(name):
                self.assertFalse(valid(SCOPE, wire), "the schema accepts what the code refuses")
                self.assertFalse(code_accepts(Scope.from_dict, wire))

    def test_a_ledger_the_code_refuses_the_schema_refuses_and_the_nearest_good_one_both_take(self):
        spec = {"to": {"kind": "spec", "spec": "lamp-1"}, "blocks": ["b1"]}
        out = {"to": {"kind": "left-out"}, "reason": "the cover", "blocks": ["b2"]}
        good = {"v": 1, "extraction": REVISION, "allotments": [spec, out]}
        self.assertTrue(valid(LEDGER, good) and code_accepts(Ledger.from_dict, good))

        def with_(**changes):
            return {**good, "allotments": [dict(spec, **changes), out]}

        for name, wire in {
            "another version": {**good, "v": 2},
            "an extraction that is not a revision": {**good, "extraction": "short"},
            "a field nobody defined": {**good, "colour": "red"},
            "allotments that are not a list": {**good, "allotments": "x"},
            "an allotment of no block": with_(blocks=[]),
            "a block twice in an allotment": with_(blocks=["b1", "b1"]),
            "a block with no id": with_(blocks=["b1", ""]),
            "a kind nobody defined": with_(to={"kind": "elsewhere"}),
            "a spec with no id": with_(to={"kind": "spec"}),
            "a spec id that is not one": with_(to={"kind": "spec", "spec": "Lamp/../x"}),
            "a spec id that begins with a hyphen": with_(to={"kind": "spec", "spec": "-lamp"}),
            "a spec id of more than eighty characters": with_(to={"kind": "spec", "spec": "a" * 81}),
            "a spec with a title as well": with_(to={"kind": "spec", "spec": "lamp-1", "title": "x"}),
            "a new spec with no title": with_(to={"kind": "new", "title": "  "}),
            "a new spec with a title that is too long": with_(to={"kind": "new", "title": "t" * 201}),
            "a left-out part that names a spec": with_(to={"kind": "left-out", "spec": "lamp-1"}),
            "a reason for a part that is not left out": with_(reason="why"),
            "a reason that is too long": {**good, "allotments": [spec, dict(out, reason="r" * 1001)]},
            "a field of an allotment nobody defined": with_(colour="red"),
        }.items():
            with self.subTest(name):
                self.assertFalse(valid(LEDGER, wire), "the schema accepts what the code refuses")
                self.assertFalse(code_accepts(Ledger.from_dict, wire))
        # The edges of the limits are accepted by both.
        for edge in (
            with_(to={"kind": "spec", "spec": "a" * 80}),
            with_(to={"kind": "new", "title": "t" * 200}),
            {**good, "allotments": [spec, dict(out, reason="r" * 1000)]},
            {**good, "allotments": [spec, dict(out, reason="")]},
        ):
            self.assertTrue(valid(LEDGER, edge) and code_accepts(Ledger.from_dict, edge), edge)


if __name__ == "__main__":
    unittest.main()
