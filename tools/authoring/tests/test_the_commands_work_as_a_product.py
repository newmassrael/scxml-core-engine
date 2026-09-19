"""The four commands, run the way a script runs them, for their EXIT CODES.

Everything else in this suite calls the library. A caller does not: it runs a
command and branches on the status, and that status is the product's contract
to a build. ⚠ Measured before this file existed: the failure paths were
covered for three commands, `verify` had no command-line test at all, and NO
command had one for the successful path -- so the codes a script actually
depends on were the untested part.

The contract asserted here, stated rather than read off current behaviour:

    brief      0   it wrote the page
    questions  0   it reports; it does not judge, so it never fails a build
    check      0 with nothing to refuse, 1 when there is something
    verify     0 when every case passed, 1 when any failed or it could not run

`questions` returning 0 on a specification full of holes is deliberate. The
holes are the deliverable, and a command that failed on them could not be run
in the loop that is supposed to produce them.
"""

from __future__ import annotations

import contextlib
import io
import pathlib
import shutil
import tempfile
import unittest

from sce_author.__main__ import main
from sce_author.verify import _default_codegen

HERE = pathlib.Path(__file__).resolve().parent
CROSSING = HERE / "fixtures" / "crossing"
SPEC = CROSSING / "specification.md"


def run(argv):
    """The command, its status, and what it printed."""
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        code = main(argv)
    return code, out.getvalue(), err.getvalue()


class TheCommandsWorkAsAProduct(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = pathlib.Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)

    # ------------------------------------------------------------- reporting

    def test_brief_writes_the_page_and_says_only_its_size(self):
        out = self.tmp / "brief.md"
        code, said, _ = run(["brief", "--pack", str(CROSSING),
                             "--prose", str(SPEC), "--out", str(out)])
        self.assertEqual(0, code)
        self.assertTrue(out.exists() and out.stat().st_size > 0)
        # ⚠ The page carries specification text and a pack may be
        # confidential. The command prints its SIZE, never its content.
        self.assertIn("bytes", said)
        # ⚠ ONE line. Asserting "no line of the page appears in stdout" was
        # vacuous while stdout held a single line, and would stay vacuous the
        # day someone added a summary that happened not to quote. The property
        # worth holding is that this command says one thing and hands the
        # content to a file.
        self.assertEqual(1, len([l for l in said.splitlines() if l.strip()]),
                         f"brief printed more than its size: {said!r}")
        page = out.read_text(encoding="utf-8")
        self.assertGreater(len(page.splitlines()), 10,
                           "an empty page would satisfy the line count above")

    def test_questions_reports_without_failing_the_build(self):
        code, said, _ = run(["questions", "--pack", str(CROSSING),
                             "--prose", str(SPEC)])
        self.assertEqual(0, code, "the holes are the deliverable, not a failure")
        self.assertIn("total", said)
        self.assertIn("example-shows-memory", said)

    def test_questions_can_write_every_finding_as_data(self):
        out = self.tmp / "q.ndjson"
        code, said, _ = run(["questions", "--pack", str(CROSSING),
                             "--prose", str(SPEC), "--out", str(out)])
        self.assertEqual(0, code)
        import json
        rows = [json.loads(line) for line in
                out.read_text(encoding="utf-8").splitlines()]
        self.assertTrue(rows)
        for row in rows:
            self.assertIn("kind", row)
            self.assertIn("severity", row)

    # -------------------------------------------------------------- judging

    def test_check_passes_a_binding_whose_names_are_all_real(self):
        code, said, _ = run(["check", "--pack", str(CROSSING), "--binding",
                             str(CROSSING / "controller.resolved.binding.yaml")])
        self.assertEqual(0, code, f"it refused: {said}")
        self.assertIn("0 refusal", said)

    def test_check_fails_when_a_name_is_not_real(self):
        binding = self.tmp / "b.yaml"
        text = (CROSSING / "controller.resolved.binding.yaml").read_text(
            encoding="utf-8").replace("plant/out/bell", "plant/out/nothing")
        binding.write_text(text, encoding="utf-8")
        shutil.copy(CROSSING / "controller.resolved.scxml",
                    self.tmp / "controller.resolved.scxml")
        code, said, _ = run(["check", "--pack", str(CROSSING),
                             "--binding", str(binding)])
        self.assertEqual(1, code)
        self.assertIn("nothing", said)

    @unittest.skipUnless(_default_codegen().exists(),
                         "the product's code generator is not built")
    def test_verify_fails_the_build_when_a_case_fails(self):
        code, said, _ = run(["verify", "--pack", str(CROSSING), "--binding",
                             str(CROSSING / "controller.resolved.binding.yaml")])
        self.assertEqual(1, code)
        self.assertIn("passed", said)
        self.assertIn("could not be judged", said)
        self.assertIn("plant/out/bell.value", said)

    @unittest.skipUnless(_default_codegen().exists(),
                         "the product's code generator is not built")
    def test_verify_fails_on_a_document_with_an_open_decision(self):
        code, said, _ = run(["verify", "--pack", str(CROSSING), "--binding",
                             str(CROSSING / "controller.binding.yaml")])
        self.assertEqual(1, code)
        self.assertIn("CROSSING_TRAIN_SIGNAL_UNDECIDED", said)

    # ------------------------------------------------------------- the shape

    def test_an_unknown_subcommand_is_refused_by_the_parser(self):
        with self.assertRaises(SystemExit) as caught:
            run(["frobnicate", "--pack", str(CROSSING)])
        self.assertNotEqual(0, caught.exception.code)


if __name__ == "__main__":
    unittest.main()
