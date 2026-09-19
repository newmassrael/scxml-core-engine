"""The one route a picture is allowed to take, walked end to end.

This core makes no model calls, so it cannot say what a diagram SHOWS. That is
a real limit and it is deliberate: the answers have to be the same answers
twice, and a model's reading of a picture is a plausible reading, which is the
one thing this whole tool exists to refuse taking on trust.

⚠ REFUSING TO READ IT IS NOT THE SAME AS REFUSING TO USE IT. A reading can
come from anywhere -- a person, a model, a phone call with whoever drew it --
and enter the document as long as it enters MARKED. `sce:assumed` is that
mark, and `verify` is what eventually judges it by running the thing.

So the route has four hops, and every one of them existed before this file
while the route had never once been walked:

    ingest     says a clause hands its whole content to a picture
    questions  puts that in front of the author as a question
    the author reads the picture and writes the rule, marked `sce:assumed`
    verify     runs it and says THE GUESS YOU RECORDED is what failed

⚠⚠ The assertion that makes this a route rather than four features is that
the author's own sentence -- the one naming the clause the picture sits in --
comes back out of `verify`. If it does not survive the trip, the last report
says "your document is wrong" about a value its author had already written
down as a guess, and the marking bought nothing.
"""

from __future__ import annotations

import io
import pathlib
import shutil
import tempfile
import unittest
import zipfile

import yaml

from sce_author.pack import load_pack
from sce_author.prose import load_prose
from sce_author.questions import ask
from sce_author.verify import _default_codegen, verify

HERE = pathlib.Path(__file__).resolve().parent
CROSSING = HERE / "fixtures" / "crossing"
CLOSED_BINDING = CROSSING / "controller_resolved.binding.yaml"

W = 'xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"'

# The clause the picture is the whole content of, and the sentence the author
# writes about it. Both travel: the number reaches the author through
# `questions`, and the sentence comes back through `verify`.
CLAUSE = "3.2"
THE_AUTHORS_REASON = (
    f"read off the sequence diagram in clause {CLAUSE}; "
    "the drawing shows the order and states no timeout"
)


def paragraph(text: str = "", drawn: bool = False) -> str:
    run = f"<w:r><w:t>{text}</w:t></w:r>" if text else ""
    art = "<w:r><w:drawing><w:inline/></w:drawing></w:r>" if drawn else ""
    return f"<w:p>{run}{art}</w:p>"


class APictureBecomesAMarkedGuess(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = pathlib.Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)

    # ------------------------------------------------- hop 1 and 2: told

    def specification(self) -> pathlib.Path:
        """A specification that states most of its rules and draws one."""
        body = "".join([
            paragraph("3.1 The road signal"),
            paragraph("roadSignal shows FLASHING while approaching is true."),
            paragraph("bell rings whenever roadSignal is not DARK."),
            paragraph("barrier lowers while approaching and obstacleNone."),
            paragraph(f"{CLAUSE} The train signal sequence"),
            paragraph(drawn=True),
        ])
        path = self.tmp / "spec.docx"
        with zipfile.ZipFile(path, "w") as z:
            z.writestr("word/document.xml",
                       f'<?xml version="1.0" encoding="UTF-8"?>'
                       f"<w:document {W}><w:body>{body}</w:body></w:document>")
            z.writestr("word/media/image1.png", b"\x89PNG not really")
        return path

    def test_the_author_is_told_which_clause_hands_its_rule_to_a_picture(self):
        """Hops one and two. Without this the author never learns there is
        anything to read: every command downstream runs clean on the text
        that remains, which is the loss that looks most like success.
        """
        pack = load_pack(CROSSING)
        prose = load_prose([self.specification()])
        said = "\n".join(q.detail for q in ask(
            prose, pack.model, pack.conventions, pack.examples))
        self.assertIn("picture(s) were not read", said)
        self.assertIn(CLAUSE, said,
                      "the report does not say WHERE, so the author is told a "
                      "picture exists and not that a rule is inside it")
        self.assertIn("state nothing in text", said)

    def test_a_clause_that_also_speaks_is_not_reported_as_a_hole(self):
        """The discriminator. A report that named every picture would pass the
        case above and mean nothing -- most pictures are what a screen looks
        like, and this corpus measured zero clauses of the dangerous kind in a
        22,669-line specification.
        """
        body = "".join([
            paragraph("3.1 The road signal"),
            paragraph("roadSignal shows FLASHING while approaching is true."),
            paragraph(drawn=True),
        ])
        path = self.tmp / "illustrated.docx"
        with zipfile.ZipFile(path, "w") as z:
            z.writestr("word/document.xml",
                       f'<?xml version="1.0" encoding="UTF-8"?>'
                       f"<w:document {W}><w:body>{body}</w:body></w:document>")
            z.writestr("word/media/image1.png", b"\x89PNG not really")
        said = "\n".join(n for src in load_prose([path]).sources
                         for n in src.notes)
        self.assertIn("picture(s) were not read", said)
        self.assertIn("No numbered clause", said)

    # --------------------------------------------- hops 3 and 4: judged

    def staged(self, marked: bool):
        """The crossing pack, with the picture's rule written into the
        document -- wrongly, which is what a guess read off a drawing does
        when the drawing does not say everything.
        """
        tmp = pathlib.Path(tempfile.mkdtemp(prefix="picture_guess_"))
        self.addCleanup(shutil.rmtree, tmp, ignore_errors=True)
        for name in ("interface-model.yaml", "conventions.yaml",
                     "examples.yaml"):
            shutil.copy(CROSSING / name, tmp / name)

        document = (CROSSING / "controller_resolved.scxml").read_text(
            encoding="utf-8")
        mark = (f'sce:assumed="CROSSING_TRAIN_SIGNAL_SEQUENCE"\n'
                f'          sce:assumed-reason="{THE_AUTHORS_REASON}"\n'
                f'          ') if marked else ""
        document = document.replace(
            '<data id="trainSignal" sce:type="int32" sce:direction="out"\n'
            '          expr=',
            f'<data id="trainSignal" sce:type="int32" sce:direction="out"\n'
            f'          {mark}expr=')
        # The drawing showed an order; it did not show that the mains matter.
        document = document.replace(
            'expr="!mainsOk ? 0 : (barrierDown ? 2 : (approaching ? 1 : 2))"',
            'expr="barrierDown ? 2 : (approaching ? 1 : 2)"')
        (tmp / "drawn.scxml").write_text(document, encoding="utf-8")

        binding = yaml.safe_load(CLOSED_BINDING.read_text(encoding="utf-8"))
        binding["document"] = "drawn.scxml"
        (tmp / "b.yaml").write_text(yaml.safe_dump(binding), encoding="utf-8")
        return load_pack(tmp), tmp / "b.yaml"

    @unittest.skipUnless(_default_codegen().exists(),
                         "the product's code generator is not built")
    def test_the_authors_own_sentence_comes_back_out_of_the_run(self):
        """⚠ THE HOP THAT MAKES IT A ROUTE. The reason travels from the
        clause the reader could not read, through the mark, to the report --
        so the last thing anybody reads is "the guess you recorded failed
        here", with the clause number still attached.
        """
        pack, binding = self.staged(marked=True)
        result = verify(pack, binding)
        self.assertTrue(result.ran, f"it would not run: {result.refusal}")
        # ⚠ The precondition has to name the address. This pack fails one case
        # on its own -- a planted memory gap at the bell -- so "something
        # failed" and "something was refuted" are both true of a fixture in
        # which the drawn rule was never reached at all.
        self.assertIn("plant/out/train-signal.value", result.refuted,
                      f"the drawn rule was not what failed: {result.refuted}")
        said = result.refuted["plant/out/train-signal.value"]
        self.assertIn(f"clause {CLAUSE}", said)
        self.assertIn("sequence diagram", said)

    @unittest.skipUnless(_default_codegen().exists(),
                         "the product's code generator is not built")
    def test_the_same_wrong_reading_unmarked_is_just_a_wrong_document(self):
        """The discriminator for the hop above, and the reason to mark at all.

        Identical document, identical failure, one attribute removed: the
        report can no longer tell the author that the thing that failed is the
        thing they already knew they were guessing at.
        """
        pack, binding = self.staged(marked=False)
        result = verify(pack, binding)
        self.assertTrue(result.ran, f"it would not run: {result.refusal}")
        self.assertTrue(result.failed)
        self.assertEqual({}, result.refuted)


if __name__ == "__main__":
    unittest.main()
