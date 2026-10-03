"""The brief tells a writer the attribute the generator's schema declares on `sce:evidence`.

The brief once said an evidence carries "its provenance anchor", and a writer read that as an
attribute named `anchor`. The authoring check let the document through; the generator's schema
declares only `provenance` on that element, so the same document was refused one stage later.
The attribute is read from the schema here rather than listed, so a rename there reaches this
test and not the next writer.
"""

from __future__ import annotations

import pathlib
import unittest
import xml.etree.ElementTree as ET

from sce_author import mcp

XS = "{http://www.w3.org/2001/XMLSchema}"
SCHEMA = pathlib.Path(__file__).resolve().parents[3] / "schemas" / "sce-forge-ext.xsd"


def evidence_attributes() -> list[str]:
    for element in ET.parse(SCHEMA).getroot().iter(XS + "element"):
        if element.get("name") == "evidence":
            return [a.get("name") for a in element.iter(XS + "attribute")]
    raise AssertionError("the schema declares no `evidence` element")


class TheBriefNamesTheAttributeTheGeneratorReadsOnEvidence(unittest.TestCase):
    def test_the_schema_declares_an_attribute_on_evidence(self):
        self.assertTrue(evidence_attributes())

    def test_the_brief_names_each_attribute_the_schema_declares(self):
        for name in evidence_attributes():
            self.assertIn(f"`{name}` attribute", mcp.SERVER_INSTRUCTIONS)

    def test_the_brief_does_not_call_the_attribute_an_anchor_of_its_own(self):
        # "its provenance anchor" was read as an attribute named `anchor`.
        self.assertNotIn("provenance anchor", mcp.SERVER_INSTRUCTIONS)


if __name__ == "__main__":
    unittest.main()
