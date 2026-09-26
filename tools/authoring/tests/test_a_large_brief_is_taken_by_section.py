"""A brief too large for one tool result is taken a section at a time.

Measured 2026-09-26: the brief for the largest specification in one corpus
was 227,761 characters, 72% of it section 1 -- the specification itself,
which the writer also holds as a file. The client that asked refused a result
that size and spilled it into a file; one writer read it back in pieces, a
smaller model could not. The server now returns the section index when the
whole brief does not fit, and any sections asked for by number.
"""

from __future__ import annotations

import pathlib
import unittest
from unittest import mock

from sce_author import brief, mcp
from sce_author.pack import load_pack
from sce_author.prose import load_prose

HERE = pathlib.Path(__file__).resolve().parent
PACK = HERE / "fixtures" / "crossing"
ARGS = {"pack": str(PACK), "prose": [str(PACK / "specification.md")]}


def text(result):
    return result["content"][0]["text"]


class ABriefBySection(unittest.TestCase):
    def setUp(self):
        self.prose = load_prose([PACK / "specification.md"])
        self.pack = load_pack(PACK)
        self.whole = brief.assemble(self.prose, self.pack)

    def test_a_brief_that_fits_comes_back_whole_and_unchanged(self):
        self.assertEqual(self.whole, text(mcp.call_tool("brief", ARGS)))

    def test_the_sections_rebuild_the_whole_brief(self):
        got = brief.sections(self.prose, self.pack)
        self.assertEqual([h for h in brief.HEADINGS], [f"## {h}" for h, _ in got])
        numbers = list(range(1, len(got) + 1))
        self.assertEqual(self.whole, brief.pick(self.prose, self.pack, numbers))

    def test_a_heading_inside_the_specification_does_not_split_a_section(self):
        self.assertIn("\n## ", self.prose.text, "the fixture's prose must carry its own headings")
        spec = dict(brief.sections(self.prose, self.pack))["1. Specification"]
        self.assertIn(self.prose.sources[0].text.rstrip(), "\n".join(spec))

    def test_one_too_large_comes_back_as_its_index(self):
        with mock.patch.object(mcp, "BRIEF_LIMIT", len(self.whole) - 1):
            got = text(mcp.call_tool("brief", ARGS))
        headings = [h for h, _ in brief.sections(self.prose, self.pack)]
        self.assertLess(len(got), len(self.whole))
        for heading in headings:
            self.assertIn(heading, got)
        self.assertIn("read those files directly", got)
        self.assertIn("`sections`", got)

    def test_sections_asked_for_are_returned_in_the_order_asked(self):
        got = text(mcp.call_tool("brief", {**ARGS, "sections": [4, 2]}))
        self.assertLess(got.index("## 4."), got.index("## 2."))
        self.assertNotIn("## 1.", got)
        self.assertNotIn("## 3.", got)
        self.assertTrue(got.startswith("# Conversion brief"))

    def test_a_section_the_brief_does_not_have_is_refused_by_what_it_has(self):
        result = mcp.call_tool("brief", {**ARGS, "sections": [99]})
        self.assertTrue(result.get("isError"))
        self.assertIn("no section 99", text(result))

    def test_sections_must_be_numbers(self):
        for bad in ([], ["2"], [True], "2"):
            with self.subTest(bad=bad):
                self.assertTrue(mcp.call_tool("brief", {**ARGS, "sections": bad}).get("isError"))


if __name__ == "__main__":
    unittest.main()
