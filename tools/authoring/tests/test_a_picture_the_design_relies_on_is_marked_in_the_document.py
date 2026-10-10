"""A reading of a picture is marked in the document, not only somewhere beside it.

Measured 2026-10-10: five writers were given one specification and the one picture its timing rule
lives in. All five called `picture` and all five read the same rule, and the reading was kept in
three different places. Three documents carried it as `sce:assumed`. One kept it in the binding's
`assumed:` and in the question list, with a comment in the document pointing at the question list.
One cited the picture as `sce:evidence` and wrote "assumed" in its report alone. Whoever is handed
only the document -- the next reader, `verify`, `gaps` -- was told nothing in those two.

Asserted here:

    a picture the specification shows and the document cites, with no `sce:assumed` naming it, is refused
    the same when only the binding cites it, and the refusal says so
    a document whose `sce:assumed` names the picture is not refused
    a picture the design never names is not asked about          (a drawing may bear on nothing here)
    a guess that names a different picture does not stand in for it   (the discriminator)
"""

from __future__ import annotations

import unittest

import yaml

from sce_author.check import check
from tests.test_refusals_actually_fire import BINDING, DOCUMENT, Fixture

PICTURE = "image3.png"
SPECIFICATION = f"""\
The lamp follows the supply.

An example of how the two inputs combine in time: [picture: {PICTURE}]
"""

COMMENT = f"<!-- the order of the two inputs is read off {PICTURE} -->"
MARKED = (f'sce:assumed="ORDER" sce:assumed-reason="read from {PICTURE}: the later input decides" '
          'sce:assumed-candidates="1 0"')


class APictureTheDesignReliesOn(Fixture):
    def findings(self, document: str = DOCUMENT, binding: dict | None = None) -> list:
        (self.root / "fixture.scxml").write_text(document, encoding="utf-8")
        path = self.root / "fixture.binding.yaml"
        path.write_text(yaml.safe_dump(binding or BINDING), encoding="utf-8")
        found = check(self.pack(), path, self.prose(SPECIFICATION))
        return [f for f in found if f.where.startswith("picture ")]

    def cited(self) -> str:
        return DOCUMENT.replace("<datamodel>", f"{COMMENT}\n  <datamodel>")

    def test_a_document_that_cites_it_and_marks_nothing_is_refused(self):
        found = self.findings(self.cited())
        self.assertEqual([f"picture {PICTURE}"], [f.where for f in found])
        self.assertIn("the document names it", found[0].detail)
        self.assertIn("`sce:assumed`", found[0].detail)

    def test_a_binding_that_cites_it_is_refused_and_called_the_binding(self):
        binding = yaml.safe_load(yaml.safe_dump(BINDING))
        binding["inputs"]["mode"]["assumed"] = f"the order is read off {PICTURE}"
        found = self.findings(binding=binding)
        self.assertEqual([f"picture {PICTURE}"], [f.where for f in found])
        self.assertIn("the binding names it", found[0].detail)
        self.assertNotIn("the document names it", found[0].detail)

    def test_a_document_whose_guess_names_it_is_not_refused(self):
        marked = self.cited().replace(
            'sce:direction="out"', f'sce:direction="out" {MARKED}')
        self.assertEqual([], self.findings(marked))

    def test_a_picture_the_design_never_names_is_not_asked_about(self):
        self.assertEqual([], self.findings())

    def test_a_guess_about_something_else_does_not_stand_in_for_it(self):
        elsewhere = self.cited().replace(
            'sce:direction="out"',
            'sce:direction="out" sce:assumed="ABSENT" '
            'sce:assumed-reason="nobody said what no input reads as" '
            'sce:assumed-candidates="1 0"')
        found = self.findings(elsewhere)
        self.assertEqual([f"picture {PICTURE}"], [f.where for f in found])


if __name__ == "__main__":
    unittest.main()
