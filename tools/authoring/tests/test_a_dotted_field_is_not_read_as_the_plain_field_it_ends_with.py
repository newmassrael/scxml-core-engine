"""A dotted field name is looked up whole before its tail is.

A record can carry a field `Stat` and, beside it, a field `Offset.Stat`. The key
`<address>.Offset.Stat` names the second. `field_at` tried the split at the LAST dot
first, and `owning` walks up to the declared ancestor, so `<address>.Offset` found the
record, `Stat` was a field of it, and the key was answered with the other field's value
space. A position was then judged against the wrong enumeration, and the glue generator
refused a correct component for the disagreement.

Two records that declare both `ID` and `Linked.ID` the same way are unaffected by this,
which is why it went unseen across most of a corpus.
"""

from __future__ import annotations

import pathlib
import shutil
import tempfile
import unittest

from sce_author.pack import load_pack

MODEL = """\
version: 1
entries:
  - address: plant.obstacle
    role: output
    names: [OBSTACLE]
    fields:
      Stat:
        values: {NONE: 0, "OFF": 1, DARK: 2, BRIGHT: 3, CONE: 4, MAX: 5}
      Offset.Stat:
        values: {NONE: 0, "OFF": 1, LEFT: 2, RIGHT: 3, CENTER: 4, MAX: 6}
      Offset.Value:
        type: integer
  - address: plant.sound
    role: output
    names: [SOUND]
    fields:
      ID:
        type: text
      Linked.ID:
        type: text
"""

CONVENTIONS = """\
version: 1
name_classes:
  - pattern: '\\\\bIN_[A-Za-z0-9_]+\\\\b'
    role: supplied
"""


class ADottedFieldIsNotReadAsThePlainFieldItEndsWith(unittest.TestCase):
    def setUp(self):
        self.dir = pathlib.Path(tempfile.mkdtemp(prefix="sce_field_at_test_"))
        (self.dir / "interface-model.yaml").write_text(MODEL, encoding="utf-8")
        (self.dir / "conventions.yaml").write_text(CONVENTIONS, encoding="utf-8")
        self.model = load_pack(self.dir).model

    def tearDown(self):
        shutil.rmtree(self.dir, ignore_errors=True)

    def test_the_dotted_field_answers_with_its_own_value_space(self):
        found = self.model.field_at("plant.obstacle.Offset.Stat")
        self.assertIsNotNone(found)
        self.assertEqual(found.name, "Offset.Stat")
        self.assertEqual(found.values["MAX"], 6)

    def test_the_plain_field_still_answers_with_its_own(self):
        found = self.model.field_at("plant.obstacle.Stat")
        self.assertEqual(found.name, "Stat")
        self.assertEqual(found.values["MAX"], 5)

    def test_a_dotted_field_with_no_plain_namesake_is_found_as_before(self):
        self.assertEqual(self.model.field_at("plant.obstacle.Offset.Value").name,
                         "Offset.Value")
        self.assertEqual(self.model.field_at("plant.sound.Linked.ID").name, "Linked.ID")


if __name__ == "__main__":
    unittest.main()
