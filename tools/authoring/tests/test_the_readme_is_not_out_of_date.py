"""The README's examples are validated, not proofread.

A contract document that drifts from the contract is worse than none: a reader
follows it, the loader refuses them, and the refusal looks like their mistake.
This package's vocabulary grew a lot in one sitting -- `ordered`, `drove`,
`clock`, `elapsed_ms`, `when_absent`, `initial`, and a protocol definition --
and the README described none of it until the gap was measured by grepping for
the words.

So the examples in it are extracted and put through the same loaders a pack
goes through. Nothing here checks prose; it checks that what a reader is told
to write is what the tool accepts.
"""

from __future__ import annotations

import json
import pathlib
import re
import unittest

import yaml

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parent
README = ROOT / "README.md"
SCHEMA_DIR = ROOT / "schema"

try:
    import jsonschema
except ImportError:  # pragma: no cover - the suite says so rather than passing
    jsonschema = None


def indented_blocks(text: str) -> list[str]:
    """Every four-space-indented block in the README, as written."""
    blocks, current = [], []
    for line in text.splitlines():
        if line.startswith("    ") or (not line.strip() and current):
            current.append(line[4:] if line.startswith("    ") else "")
        elif current:
            blocks.append("\n".join(current).strip("\n"))
            current = []
    if current:
        blocks.append("\n".join(current).strip("\n"))
    return blocks


def parses_as_yaml(block: str):
    try:
        doc = yaml.safe_load(block)
    except yaml.YAMLError:
        return None
    return doc if isinstance(doc, dict) else None


class TheReadmeIsNotOutOfDate(unittest.TestCase):
    def setUp(self):
        if jsonschema is None:
            self.skipTest("jsonschema is not installed")
        self.text = README.read_text(encoding="utf-8")
        self.blocks = [b for b in indented_blocks(self.text)
                       if parses_as_yaml(b) is not None]

    def schema(self, name):
        return json.loads((SCHEMA_DIR / name).read_text(encoding="utf-8"))

    def validating(self, schema_name, marker, fragment=False):
        """The README block carrying `marker`, checked against its schema.

        ⚠ A block showing ONE section is a fragment, not a document, and
        insisting it carry `version` would push the README towards repeating
        the whole file around every snippet. A fragment is checked key by key
        against the property subschemas instead -- which is the part a reader
        is actually copying.
        """
        wanted = [parses_as_yaml(b) for b in self.blocks
                  if re.search(marker, b, re.M)]
        self.assertTrue(wanted, f"the README no longer shows an example with "
                                f"{marker!r}; this test is the thing that is "
                                f"stale, or the section was dropped")
        schema = self.schema(schema_name)
        if fragment:
            for doc in wanted:
                for key, value in doc.items():
                    subschema = schema["properties"].get(key)
                    self.assertIsNotNone(
                        subschema, f"the README shows {key!r} and "
                                   f"{schema_name} does not publish it")
                    # ⚠ The subschema ALONE, plus whatever definitions it may
                    # reference. Merging the parent in carried its `required`
                    # down and demanded the whole document of a fragment --
                    # which is the thing this branch exists to avoid.
                    errors = sorted(
                        jsonschema.Draft202012Validator(
                            {**subschema, "$defs": schema.get("$defs", {})}
                        ).iter_errors(value), key=lambda e: list(e.path))
                    self.assertEqual(
                        [], errors,
                        f"the README's {key!r} example does not validate: "
                        + "; ".join(e.message for e in errors[:3]))
            return
        validator = jsonschema.Draft202012Validator(schema)
        for doc in wanted:
            errors = sorted(validator.iter_errors(doc),
                            key=lambda e: list(e.path))
            self.assertEqual(
                [], errors,
                f"a {schema_name} example in the README does not validate: "
                + "; ".join(f"{'.'.join(str(p) for p in e.path) or '(root)'}: "
                            f"{e.message}" for e in errors[:3]))

    def test_the_interface_model_example_validates(self):
        self.validating("interface-model.v1.schema.json", r"^entries:")

    def test_the_conventions_examples_validate(self):
        self.validating("conventions.v1.schema.json", r"^name_classes:")
        self.validating("conventions.v1.schema.json", r"^protocols:",
                        fragment=True)

    def test_the_examples_example_validates(self):
        self.validating("examples.v1.schema.json", r"^cases:")

    def test_the_binding_example_validates(self):
        self.validating("binding.v1.schema.json", r"^document:")

    # ------------------------------------------------- the vocabulary itself

    def test_every_published_binding_key_is_written_down(self):
        """⚠ The gap this file was written for.

        Seven keys had been added to the binding schema and the README
        mentioned none of them. A key a reader cannot find is a key that does
        not exist for them.
        """
        schema = self.schema("binding.v1.schema.json")
        keys = set()
        for side in ("input", "output"):
            keys |= set(schema["$defs"][side]["properties"])
        missing = sorted(k for k in keys
                         if not re.search(rf"(?<![A-Za-z0-9_]){re.escape(k)}"
                                          rf"(?![A-Za-z0-9_])", self.text))
        self.assertEqual([], missing,
                         f"the binding schema publishes these and the README "
                         f"never says them: {missing}")

    def test_every_case_property_is_written_down(self):
        schema = self.schema("examples.v1.schema.json")
        keys = set(schema["properties"])
        keys |= set(schema["properties"]["cases"]["items"]["properties"])
        missing = sorted(k for k in keys
                         if not re.search(rf"(?<![A-Za-z0-9_]){re.escape(k)}"
                                          rf"(?![A-Za-z0-9_])", self.text))
        self.assertEqual([], missing,
                         f"the examples schema publishes these and the README "
                         f"never says them: {missing}")

    def test_the_brief_tells_an_author_what_to_do_when_they_must_decide(self):
        """⚠ Saying what is missing and stopping there leaves the author where
        they were: something has to go in the document and nothing says how to
        record that they guessed.

        The two markers already existed in the product -- the blocking and
        non-blocking halves of one pair -- and the brief, which is the page
        the author actually receives, mentioned neither.
        """
        from sce_author.brief import assemble
        from sce_author.pack import load_pack
        from sce_author.prose import load_prose

        crossing = HERE / "fixtures" / "crossing"
        page = assemble(load_prose([crossing / "specification.md"]),
                        load_pack(crossing))
        for marker in ("sce:unresolved", "sce:assumed"):
            self.assertIn(marker, page,
                          "the brief has to name the marker, not merely the "
                          "fact that a decision was needed")
        self.assertIn("reason", page)

    def test_the_markers_the_brief_names_are_ones_the_product_knows(self):
        """⚠ The drift that would matter most: the brief telling an author to
        write an attribute the generator refuses.

        Checked against the product's own list rather than against memory.
        """
        from sce_author.brief import assemble
        from sce_author.pack import load_pack
        from sce_author.prose import load_prose

        parser = (ROOT.parent.parent / "sce-build" / "src" / "forge"
                  / "parser.rs")
        if not parser.exists():
            self.skipTest("the product's parser is not in this tree")
        known = set(re.findall(r'^\s*"([a-z][a-z0-9-]*)",\s*$',
                               parser.read_text(encoding="utf-8"), re.M))
        crossing = HERE / "fixtures" / "crossing"
        page = assemble(load_prose([crossing / "specification.md"]),
                        load_pack(crossing))
        named = set(re.findall(r"sce:([a-z][a-z0-9-]*)", page))
        self.assertTrue(named, "the brief names no markers at all")
        unknown = sorted(n for n in named if n not in known)
        self.assertEqual([], unknown,
                         f"the brief tells an author to write attributes the "
                         f"generator does not know: {unknown}")

    def test_every_command_is_written_down(self):
        from sce_author.__main__ import main
        import argparse
        import contextlib
        import io

        help_text = io.StringIO()
        with contextlib.redirect_stdout(help_text), \
                contextlib.suppress(SystemExit):
            main(["--help"])
        commands = set(re.findall(r"\{([a-z,]+)\}", help_text.getvalue()))
        named = set()
        for group in commands:
            named |= set(group.split(","))
        self.assertTrue(named, "the command line stopped listing subcommands")
        missing = sorted(c for c in named
                         if not re.search(rf"\*\*{c}\*\*", self.text))
        self.assertEqual([], missing,
                         f"the command line has these and the README does not "
                         f"introduce them: {missing}")

    def test_every_flag_is_written_down(self):
        """⚠ THE CASE THE ONE ABOVE IS SILENT ABOUT.

        That case measures COMMANDS, so a command introduced in the README
        and then given three more flags reads as fully documented -- and
        this is the ordinary way the CLI grows. Measured the day this was
        written: seven commands were all introduced and FIVE flags across
        four of them were named nowhere, including two that decide which
        program gets run.

        The population is every flag argparse prints, asked of argparse.
        A list here would be a third copy of the command line and would go
        stale in the same direction as the README it is checking.
        """
        from sce_author.__main__ import main
        import contextlib
        import io

        def printed(argv) -> str:
            out = io.StringIO()
            with contextlib.redirect_stdout(out), \
                    contextlib.suppress(SystemExit):
                main(argv)
            return out.getvalue()

        groups = set(re.findall(r"\{([a-z,]+)\}", printed(["--help"])))
        commands = sorted({c for group in groups for c in group.split(",")})
        self.assertTrue(commands, "the command line stopped listing subcommands")

        missing = {}
        for command in commands:
            flags = sorted(set(re.findall(r"--[a-z][a-z0-9-]*",
                                          printed([command, "--help"]))))
            # ⚠ `--help` is argparse's own and belongs to no command, so
            # naming it would be documenting the library rather than this.
            absent = [f for f in flags
                      if f != "--help" and f not in self.text]
            if absent:
                missing[command] = absent
        self.assertEqual({}, missing,
                         f"these commands offer flags the README never names, "
                         f"so a reader is told the command exists and not what "
                         f"it can be asked: {missing}")


if __name__ == "__main__":
    unittest.main()
