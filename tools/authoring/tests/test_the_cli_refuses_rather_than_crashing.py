"""Ordinary mistakes get a sentence, not a traceback.

⚠ This file exists because it was measured missing. Eleven things a caller
will eventually do were put to the command line -- a path that is not there, a
directory where a file belongs, a symlink pointing at itself, bytes that are
not text, a YAML file that is binary, a binding that is a markdown document,
an output path that cannot be written -- and SEVEN produced a traceback. Four
of those did not name the offending file anywhere in the output.

A traceback tells the caller where this program is, which is never what they
asked. The contract these cases hold is narrow and total:

    a bad input returns 2 and prints one line that names the path

Returning 0 would be worse than crashing, so each case asserts the code as
well. And the cases are run IN PROCESS rather than through a subprocess, so an
escaping exception fails the test as an error rather than being flattened into
an exit status.
"""

from __future__ import annotations

import contextlib
import io
import os
import pathlib
import tempfile
import unittest

from sce_author.__main__ import main

HERE = pathlib.Path(__file__).resolve().parent
CROSSING = HERE / "fixtures" / "crossing"
SPEC = CROSSING / "specification.md"


class TheCommandLineRefusesRatherThanCrashing(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = pathlib.Path(self._tmp.name)

    def tearDown(self):
        self._tmp.cleanup()

    def refuse(self, argv, naming):
        """Run the command line and require a refusal that names `naming`."""
        err = io.StringIO()
        with contextlib.redirect_stderr(err), contextlib.redirect_stdout(io.StringIO()):
            code = main(argv)
        said = err.getvalue()
        self.assertEqual(
            2, code,
            f"{argv[0]} should refuse with 2, returned {code}; said: {said!r}")
        self.assertIn(
            str(naming), said,
            f"the refusal has to name what was wrong with; it said {said!r}")
        return said

    # ------------------------------------------------------------ the pack

    def test_a_pack_directory_that_is_not_there(self):
        missing = self.tmp / "nope"
        self.refuse(["questions", "--pack", str(missing), "--prose", str(SPEC)],
                    missing)

    def test_a_pack_directory_with_nothing_in_it(self):
        empty = self.tmp / "empty"
        empty.mkdir()
        self.refuse(["questions", "--pack", str(empty), "--prose", str(SPEC)],
                    empty)

    def test_an_interface_model_that_is_not_text(self):
        junk = self.tmp / "junk"
        junk.mkdir()
        (junk / "interface-model.yaml").write_bytes(b"\x00\x01\x02 not yaml {[")
        (junk / "conventions.yaml").write_text("version: 1\n", encoding="utf-8")
        self.refuse(["questions", "--pack", str(junk), "--prose", str(SPEC)],
                    junk / "interface-model.yaml")

    def test_an_interface_model_that_is_valid_yaml_of_the_wrong_shape(self):
        shape = self.tmp / "shape"
        shape.mkdir()
        (shape / "interface-model.yaml").write_text("just a string\n",
                                                    encoding="utf-8")
        (shape / "conventions.yaml").write_text("version: 1\n", encoding="utf-8")
        self.refuse(["questions", "--pack", str(shape), "--prose", str(SPEC)],
                    shape / "interface-model.yaml")

    # ----------------------------------------------------------- the prose

    def test_a_prose_file_that_is_not_there(self):
        missing = self.tmp / "nope.md"
        self.refuse(["questions", "--pack", str(CROSSING), "--prose", str(missing)],
                    missing)

    def test_a_prose_path_that_is_a_directory(self):
        said = self.refuse(
            ["questions", "--pack", str(CROSSING), "--prose", str(self.tmp)],
            self.tmp)
        # ⚠ Not "no such file". It is right there, and saying it is absent sent
        # a reader looking for something already in front of them.
        self.assertIn("is a directory", said)

    def test_a_prose_file_that_is_a_symlink_to_itself(self):
        loop = self.tmp / "loop.md"
        try:
            os.symlink(loop, loop)
        except OSError as exc:  # pragma: no cover - platform without symlinks
            self.skipTest(f"symlinks unavailable: {exc}")
        self.refuse(["questions", "--pack", str(CROSSING), "--prose", str(loop)],
                    loop)

    def test_a_prose_file_that_is_not_text(self):
        binary = self.tmp / "binary.md"
        binary.write_bytes(bytes(range(256)) * 8)
        said = self.refuse(
            ["questions", "--pack", str(CROSSING), "--prose", str(binary)],
            binary)
        self.assertIn("UTF-8", said)

    # --------------------------------------------------------- the binding

    def test_a_binding_that_is_a_prose_document(self):
        self.refuse(["check", "--pack", str(CROSSING), "--binding", str(SPEC)],
                    SPEC)

    def test_a_binding_that_is_not_there(self):
        missing = self.tmp / "nope.yaml"
        self.refuse(["check", "--pack", str(CROSSING), "--binding", str(missing)],
                    missing)

    # ---------------------------------------------------------- the output

    def test_a_brief_whose_output_cannot_be_written(self):
        """Writing is the one refusal no error type of this package owns."""
        unwritable = self.tmp / "no-such-dir" / "brief.md"
        self.refuse(["brief", "--pack", str(CROSSING), "--prose", str(SPEC),
                     "--out", str(unwritable)], unwritable)


if __name__ == "__main__":
    unittest.main()
