"""The targets know no subject matter and no place on anyone's disk.

The same two guards the authoring core is held to, for the same reason: the way a general tool
stops being general is that somebody adds one helpful special case for the subject in front of
them. The vocabulary is the core's own list, read from its test rather than copied, so the two
cannot drift; and no source names a location of its own.
"""

from __future__ import annotations

import importlib.util
import pathlib
import re
import unittest

HERE = pathlib.Path(__file__).resolve().parent
SOURCES = sorted((HERE.parent / "sce_targets").glob("*.py"))
CORE_TEST = HERE.parent.parent / "authoring" / "tests" / "test_core_is_domain_free.py"

# A string that begins like an absolute location on a person's machine.
LOCATION = re.compile(r"""["'](?:/home/|/Users/|/tmp/|/mnt/|[A-Za-z]:\\)""")


def subject_words() -> set:
    spec = importlib.util.spec_from_file_location("core_domain_free", CORE_TEST)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return set(module.SUBJECT_WORDS)


class TheTargetsAreDomainFree(unittest.TestCase):
    def test_there_are_sources_to_hold(self):
        """The discriminator: a guard over an empty list holds nothing."""
        self.assertGreaterEqual(len(SOURCES), 3)
        self.assertGreater(len(subject_words()), 10)

    def test_no_source_spells_a_subject_matter(self):
        words = subject_words()
        for source in SOURCES:
            seen = {w.lower() for w in re.findall(r"[A-Za-z]+", source.read_text(encoding="utf-8"))}
            self.assertEqual(set(), seen & words, f"{source.name} spells a subject matter")

    def test_no_source_names_a_location_of_its_own(self):
        for source in SOURCES:
            for number, line in enumerate(source.read_text(encoding="utf-8").splitlines(), 1):
                self.assertIsNone(LOCATION.search(line), f"{source.name}:{number}: {line.strip()}")

    def test_the_fixture_is_made_up(self):
        """The test pack is read by the same guard: a platform's names do not get in through it."""
        words = subject_words()
        for path in sorted((HERE / "fixtures").rglob("*")):
            if path.is_file():
                seen = {w.lower() for w in re.findall(r"[A-Za-z]+", path.read_text(encoding="utf-8"))}
                self.assertEqual(set(), seen & words, f"{path.name} spells a subject matter")


if __name__ == "__main__":
    unittest.main()
