"""What an extractor could not carry is a code and the facts that go with it, and a sentence is written from them.

An extraction is named by its hash, so it cannot hold the sentences of whichever language its reader used.
`notes.py` writes each sentence once per language beside the code it says, an extractor gives a code and facts,
and a host asks `render-notes` for the language it needs and writes none of its own (the rule
`sce-build/src/open_matters.rs` holds for what a design leaves open). What is held:

  the lexicon   every code has a sentence in every language, a code that has one and not the other does not
                load, and facts that are not the ones a code takes are refused where they are made
  the entry     keeps its code and facts through the wire form and the schema; one an older tool wrote (no code)
                and one a newer tool wrote (a code this version does not know) are read and said as stored
  the sentence  English is what an extraction stores beside the code and is the sentence every test of an
                extractor reads; another language is written from the code and facts and never from the stored
                sentence
  the extractors each entry an extractor makes carries a code this version knows, and its stored sentence is
                the one the code says in English
  the command   `render-notes` says a list in a language, and leaves what has no sentence there as it was
"""

from __future__ import annotations

import contextlib
import io
import json
import pathlib
import re
import tempfile
import unittest
import zipfile

import jsonschema

from sce_author import notes
from sce_author.__main__ import main
from sce_author.errors import IngestError
from sce_author.extraction import Block, Extraction, NotCarried, Place
from sce_author.extractors import builtin
from sce_author.notes import LANGUAGES, NoteError, Note
from tests.test_a_pdf_with_a_text_layer_is_read_by_page_and_box import build_pdf, needs_pack
from tests.test_a_picture_in_the_text_is_marked_where_it_sits_and_can_be_looked_at import NS, PNG, drawn, para, rels

HANGUL = re.compile("[가-힣]")
SCHEMA = json.loads(
    (pathlib.Path(__file__).resolve().parent.parent / "schema" / "extraction.v1.schema.json").read_text(
        encoding="utf-8"
    )
)

# A fact of each kind, in plain ASCII so that the language a sentence is in is the lexicon's and not the fact's.
SAMPLE = {"int": 4, "str": "Sheet1", "bool": True, "ints": [2, 5], "strs": ["A.1", "B.2"]}


def sample_facts(code: str) -> dict:
    return {name: SAMPLE[kind] for name, kind in notes.FACTS[code].items()}


class TheLexiconSaysEveryCodeInEveryLanguage(unittest.TestCase):
    def test_there_are_codes_and_two_languages(self):
        self.assertEqual(LANGUAGES, ("en", "ko"))
        self.assertIn("docx-pictures-unread", notes.codes())
        self.assertEqual(len(notes.codes()), len(set(notes.codes())))

    def test_every_code_has_a_sentence_in_every_language(self):
        for code in notes.codes():
            for language in LANGUAGES:
                with self.subTest(code=code, language=language):
                    said = notes.sentence(code, sample_facts(code), language)
                    self.assertTrue(said.what.strip())

    def test_the_two_languages_are_two_sentences_and_not_one_written_twice(self):
        for code in notes.codes():
            with self.subTest(code=code):
                english = notes.sentence(code, sample_facts(code), "en")
                korean = notes.sentence(code, sample_facts(code), "ko")
                self.assertIsNone(HANGUL.search(english.what + english.why), english)
                self.assertIsNotNone(HANGUL.search(korean.what), korean)
                self.assertNotEqual(english.what, korean.what)

    def test_a_code_that_has_a_sentence_in_one_language_and_not_the_other_does_not_load(self):
        saved = notes._WRITERS["ko"].pop("pdf-layout")
        try:
            with self.assertRaisesRegex(NoteError, "no sentence in 'ko'.*pdf-layout"):
                notes._held()
        finally:
            notes._WRITERS["ko"]["pdf-layout"] = saved
        notes._WRITERS["en"]["orphan"] = lambda f: notes.Sentence("x")
        try:
            with self.assertRaisesRegex(NoteError, "undeclared.*orphan"):
                notes._held()
        finally:
            del notes._WRITERS["en"]["orphan"]

    def test_a_code_declared_twice_or_with_a_fact_of_no_kind_is_refused(self):
        with self.assertRaisesRegex(NoteError, "declared twice"):
            notes._code("pdf-layout", {})
        with self.assertRaisesRegex(NoteError, "unknown kind"):
            notes._code("brand-new", {"n": "float"})
        self.assertNotIn("brand-new", notes.FACTS)

    def test_what_a_code_does_not_take_or_needs_is_refused_where_it_is_made(self):
        cases = {
            "an unknown code": ("no-such-code", {}),
            "a fact it does not take": ("pdf-layout", {"pages": 3}),
            "a fact it needs and was not given": ("xlsx-rich-values", {}),
            "a fact of another kind": ("xlsx-rich-values", {"count": "3"}),
            "a boolean for a number": ("xlsx-rich-values", {"count": True}),
            "a list for a word": ("xlsx-merged-ranges", {"count": 1, "sheet": ["a"]}),
            "a name that is not one": ("xlsx-rich-values", {"count": 1, "in_file": 3}),
        }
        for label, (code, facts) in cases.items():
            with self.subTest(label):
                with self.assertRaises(NoteError):
                    notes.sentence(code, facts)

    def test_an_empty_list_is_a_list_of_either_kind(self):
        self.assertTrue(notes.sentence("pdf-pictures-unplaced", {"pictures": 0, "limit": 1, "pages": []}).what)
        self.assertTrue(notes.sentence("enclosed-objects-unopened", {"names": []}).what)

    def test_a_language_the_lexicon_does_not_write_is_refused(self):
        with self.assertRaisesRegex(NoteError, "not a language"):
            notes.sentence("pdf-layout", {}, "fr")

    def test_a_long_list_is_cut_to_what_a_sentence_holds_in_each_language(self):
        pages = list(range(1, 31))
        english = notes.sentence("pdf-pictures-unplaced", {"pictures": 9, "limit": 5, "pages": pages}, "en")
        korean = notes.sentence("pdf-pictures-unplaced", {"pictures": 9, "limit": 5, "pages": pages}, "ko")
        self.assertTrue(english.what.endswith("1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, ..."))
        self.assertTrue(korean.what.endswith("1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, ..."))
        self.assertEqual(notes.listed(["a"] * 15), ", ".join(["a"] * 12) + " and 3 more")
        self.assertEqual(notes.listed_ko(["a"] * 15), ", ".join(["a"] * 12) + " 외 3개")

    def test_an_enclosed_files_note_begins_with_its_name_in_every_language(self):
        for language in LANGUAGES:
            said = notes.sentence("enclosed-file-unopened", {"reason": "bad zip", "in_file": "Limits.xlsx"}, language)
            self.assertTrue(said.what.startswith("Limits.xlsx: "), said)


class ANoteIsItsEnglishSentenceThatSaysWhatItIs(unittest.TestCase):
    def test_it_is_a_string_that_reads_as_the_english_sentence_and_keeps_its_code_and_facts(self):
        note = Note("xlsx-rich-values", count=3)

        self.assertIsInstance(note, str)
        self.assertIn("3 cell(s) in this workbook hold a rich value", note)
        self.assertEqual((note.code, note.facts), ("xlsx-rich-values", {"count": 3}))
        self.assertEqual(str(note), notes.sentence("xlsx-rich-values", {"count": 3}).what)

    def test_a_note_that_gives_a_reason_reads_as_one_sentence_and_keeps_what_it_says_apart_from_why(self):
        note = Note("docx-pictures-unread", pictures=1, in_body=1, numbered=False, silent=[], speaking=0, drawn=1, attributed=0)

        said = notes.sentence("docx-pictures-unread", dict(note.facts))
        self.assertEqual((note.headline, note.why), (said.what, said.why))
        self.assertEqual(str(note), f"{said.what} -- {said.why}")
        self.assertTrue(note.startswith("1 picture(s) were not read -- "))

    def test_an_entry_made_of_a_note_that_gives_a_reason_shows_them_apart_and_a_note_without_one_shows_none(self):
        with_reason = NotCarried.from_note(
            Note("docx-pictures-unread", pictures=2, in_body=1, numbered=True, silent=["5.1"], speaking=0, drawn=1, attributed=1)
        )
        self.assertEqual("2 picture(s) were not read", with_reason.what)
        self.assertIn("5.1", with_reason.why)
        without = NotCarried.from_note(Note("xlsx-rich-values", count=3))
        self.assertEqual("", without.why)


class AnEntryKeepsItsCodeAndFactsAndIsSaidInALanguage(unittest.TestCase):
    def test_an_entry_made_of_a_code_says_the_english_sentence_the_lexicon_writes(self):
        entry = NotCarried.of("pdf-pages-without-text", count=2, blank=[2, 3], pages=3)

        english = notes.sentence("pdf-pages-without-text", {"blank": [2, 3], "pages": 3})
        self.assertEqual((entry.what, entry.why), (english.what, english.why))
        self.assertEqual((entry.code, entry.facts, entry.count), ("pdf-pages-without-text", {"blank": [2, 3], "pages": 3}, 2))
        self.assertIn("2 of 3 page(s) carry no text: 2, 3", entry.what)

    def test_it_comes_through_the_wire_form_with_its_code_and_facts_and_the_schema_allows_it(self):
        entry = NotCarried.of("pdf-pictures-unplaced", count=7, blocks=("b0001",), pictures=7, limit=5, pages=[1])
        figure = Block("b0001", "figure", "[picture: p.png]", Place("a.pdf", page=1, box=(0, 0, 1, 1), precision="exact"))
        extraction = Extraction("x", "1", "a.pdf", (figure,), (entry,))

        written = json.loads(extraction.to_json())
        read = Extraction.from_json(extraction.to_json())

        jsonschema.validate(written, SCHEMA)
        self.assertEqual(read.not_carried, (entry,))
        self.assertEqual(written["not_carried"][0]["code"], "pdf-pictures-unplaced")
        self.assertEqual(written["not_carried"][0]["facts"], {"pictures": 7, "limit": 5, "pages": [1]})

    def test_the_stored_extraction_is_the_same_whatever_language_a_reader_will_read_it_in(self):
        entry = NotCarried.of("pdf-layout")
        extraction = Extraction("x", "1", "a.pdf", (), (entry,))

        said = tuple(e.in_language("ko") for e in extraction.not_carried)

        self.assertEqual(Extraction.from_json(extraction.to_json()).revision(), extraction.revision())
        self.assertNotEqual(said, extraction.not_carried)
        self.assertEqual(extraction.not_carried[0].what, notes.sentence("pdf-layout", {}).what)

    def test_an_entry_is_said_in_a_language_from_its_code_and_facts_and_keeps_all_else(self):
        entry = NotCarried.of("pdf-pictures-unplaced", count=7, blocks=("b0001",), pictures=7, limit=5, pages=[1, 2])

        korean = entry.in_language("ko")

        self.assertEqual(korean.what, notes.sentence("pdf-pictures-unplaced", entry.facts, "ko").what)
        self.assertEqual((korean.count, korean.blocks, korean.code, korean.facts), (7, ("b0001",), entry.code, entry.facts))
        self.assertEqual(entry.in_language("en"), entry)

    def test_an_entry_an_older_tool_wrote_has_no_code_and_is_said_as_it_was_stored(self):
        old = NotCarried.from_dict({"what": "3 picture(s) were not read -- an old sentence", "count": 3})

        self.assertIsNone(old.code)
        self.assertIs(old.in_language("ko"), old)
        jsonschema.validate(json.loads(Extraction("x", "1", "a", (), (old,)).to_json()), SCHEMA)

    def test_an_entry_a_newer_tool_wrote_with_a_code_this_version_does_not_know_is_read_and_said_as_stored(self):
        newer = NotCarried.from_dict(
            {"what": "an English sentence", "code": "a-code-from-the-future", "facts": {"n": 1, "names": ["a"]}}
        )

        self.assertIs(newer.in_language("ko"), newer)
        self.assertEqual(newer.to_dict()["code"], "a-code-from-the-future")

    def test_an_entry_whose_facts_are_not_the_ones_its_code_takes_is_said_as_stored_and_not_refused(self):
        odd = NotCarried.from_dict({"what": "stored", "code": "xlsx-rich-values", "facts": {"count": "three"}})

        self.assertIs(odd.in_language("ko"), odd)

    def test_a_language_there_is_no_sentence_in_leaves_the_entry_as_stored(self):
        entry = NotCarried.of("pdf-layout")

        self.assertIs(entry.in_language("fr"), entry)

    def test_what_a_wire_form_cannot_hold_is_refused(self):
        bad = {
            "facts and no code": {"what": "w", "facts": {"n": 1}},
            "a fact that is a mapping": {"what": "w", "code": "c", "facts": {"n": {"a": 1}}},
            "a fact that is a float": {"what": "w", "code": "c", "facts": {"n": 1.5}},
            "a list of both": {"what": "w", "code": "c", "facts": {"n": [1, "a"]}},
            "an empty code": {"what": "w", "code": ""},
        }
        for label, raw in bad.items():
            with self.subTest(label), self.assertRaises(IngestError):
                NotCarried.from_dict(raw)

    def test_the_schema_refuses_what_the_code_refuses(self):
        for raw in (
            {"what": "w", "facts": {"n": 1}},
            {"what": "w", "code": "c", "facts": {"n": 1.5}},
            {"what": "w", "code": "c", "facts": {"n": [1, "a"]}},
            {"what": "w", "unknown": 1},
        ):
            with self.subTest(raw=raw), self.assertRaises(jsonschema.ValidationError):
                jsonschema.validate(raw, SCHEMA["$defs"]["notCarried"] | {"$defs": SCHEMA["$defs"]})


class EveryExtractorGivesACodeThisVersionKnows(unittest.TestCase):
    def setUp(self) -> None:
        self._temp = tempfile.TemporaryDirectory()
        self.addCleanup(self._temp.cleanup)
        self.root = pathlib.Path(self._temp.name)

    def entries(self, extraction: Extraction) -> None:
        self.assertTrue(extraction.not_carried)
        for entry in extraction.not_carried:
            with self.subTest(entry=entry.what[:40]):
                self.assertIn(entry.code, notes.codes())
                # The stored English is the sentence the code says, and said again in English it is the same.
                self.assertEqual(entry.in_language("en"), entry)
                self.assertEqual(entry.what, notes.sentence(entry.code, entry.facts).what)
                self.assertIsNotNone(HANGUL.search(entry.in_language("ko").what))

    def docx(self, body: str, relationships: str, name: str = "spec.docx") -> pathlib.Path:
        path = self.root / name
        with zipfile.ZipFile(path, "w") as z:
            z.writestr(
                "word/document.xml",
                f'<?xml version="1.0" encoding="UTF-8"?><w:document {NS}><w:body>{body}</w:body></w:document>',
            )
            z.writestr("word/_rels/document.xml.rels", relationships)
            z.writestr("word/media/image3.png", PNG)
        return path

    def test_a_word_document_with_a_picture_in_a_numbered_clause(self):
        body = para("5.1 The event") + para("", drawn("rId5")) + para("5.2 The fault") + para("the lamp is lit")
        done = builtin().extract(self.docx(body, rels(rId5="image3.png")))

        self.entries(done)
        entry = next(e for e in done.not_carried if e.code == "docx-pictures-unread")
        self.assertEqual(entry.facts["pictures"], 1)
        self.assertEqual(entry.facts["in_body"], 1)
        self.assertEqual(entry.facts["silent"], ["5.1"])
        self.assertTrue(entry.blocks, "a note about pictures the blocks mark names the blocks")
        korean = entry.in_language("ko")
        self.assertEqual("그림 1개를 읽지 않았어요", korean.what)
        self.assertIn("5.1", korean.why)

    def test_a_word_document_whose_picture_no_numbered_clause_holds(self):
        done = builtin().extract(self.docx(para("A plain sentence.") + para("", drawn("rId5")), rels(rId5="image3.png")))

        self.entries(done)
        entry = done.not_carried[0]
        self.assertFalse(entry.facts["numbered"])
        self.assertIn("번호가 붙은 조항이 없는 문서", entry.in_language("ko").why)

    def test_a_markdown_table_with_a_short_row(self):
        path = self.root / "t.md"
        path.write_text("| a | b |\n| --- | --- |\n| 1 |\n", encoding="utf-8")

        done = builtin().extract(path)

        self.entries(done)
        self.assertEqual(done.not_carried[0].code, "short-rows")

    @needs_pack
    def test_a_pdf_with_blank_pages_pictures_and_text(self):
        path = self.root / "s.pdf"
        path.write_bytes(build_pdf([{"lines": [(72, 700, 11, "Text.")]}, {}, {}]))

        done = builtin().extract(path)

        self.entries(done)
        codes = [e.code for e in done.not_carried]
        self.assertEqual(codes, ["pdf-pages-without-text", "pdf-layout", "pdf-drawings"])
        self.assertEqual(done.not_carried[0].facts, {"blank": [2, 3], "pages": 3})
        self.assertEqual(done.not_carried[0].count, 2)
        self.assertIn("3쪽 중 2쪽에 글이 없어요: 2, 3", done.not_carried[0].in_language("ko").what)


class TheCommandSaysAListInALanguage(unittest.TestCase):
    def setUp(self) -> None:
        self._temp = tempfile.TemporaryDirectory()
        self.addCleanup(self._temp.cleanup)
        self.root = pathlib.Path(self._temp.name)

    def run_main(self, *argv: str) -> tuple[int, str, str]:
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            try:
                code = main(list(argv))
            except SystemExit as exit_:
                code = int(exit_.code)
        return code, out.getvalue(), err.getvalue()

    def listed(self, entries: list) -> str:
        path = self.root / "n.json"
        path.write_text(json.dumps(entries), encoding="utf-8")
        return str(path)

    def test_a_typed_entry_is_said_in_korean_and_what_has_no_sentence_comes_out_as_it_went_in(self):
        typed = NotCarried.of("pdf-pictures-unidentified", count=2, pictures=2).to_dict()
        old = {"what": "an old sentence", "count": 1}
        future = {"what": "from a newer tool", "code": "not-yet-known", "facts": {"n": 1}}

        code, out, _ = self.run_main("render-notes", "--notes", self.listed([typed, old, future]), "--lexicon", "ko")

        said = json.loads(out)
        self.assertEqual(code, 0)
        self.assertEqual(said[0]["what"], "파일에서 데이터를 읽지 못한 그림 2개")
        self.assertEqual(said[0]["count"], 2)
        self.assertEqual(said[0]["code"], "pdf-pictures-unidentified")
        self.assertEqual(said[1:], [old, future])

    def test_in_english_a_typed_entry_comes_out_as_it_was_stored(self):
        typed = NotCarried.of("pdf-layout").to_dict()

        code, out, _ = self.run_main("render-notes", "--notes", self.listed([typed]), "--lexicon", "en")

        self.assertEqual((code, json.loads(out)), (0, [typed]))

    def test_the_korean_is_written_without_escapes_so_a_host_reads_it_as_it_is(self):
        typed = NotCarried.of("pdf-layout").to_dict()

        _, out, _ = self.run_main("render-notes", "--notes", self.listed([typed]), "--lexicon", "ko")

        self.assertIn("쪽의 배치", out)
        self.assertNotIn("\\u", out)

    def test_every_language_at_once_keeps_the_entry_and_adds_the_sentences_of_each_other_language(self):
        typed = NotCarried.of("pdf-pages-without-text", count=2, blank=[2, 3], pages=3)
        old = {"what": "an old sentence"}
        future = {"what": "from a newer tool", "code": "not-yet-known", "facts": {"n": 1}}

        code, out, _ = self.run_main(
            "render-notes", "--notes", self.listed([typed.to_dict(), old, future]), "--all"
        )

        said = json.loads(out)
        korean = typed.in_language("ko")
        self.assertEqual(code, 0)
        # The entry is as it was stored, in English, and the other language is beside it with its reason.
        self.assertEqual({k: v for k, v in said[0].items() if k != "said"}, typed.to_dict())
        self.assertEqual(said[0]["said"], {"ko": {"what": korean.what, "why": korean.why}})
        # What has no sentence in another language has nothing under `said`.
        self.assertEqual(said[1:], [old, future])

    def test_a_reason_a_code_does_not_give_is_not_said(self):
        typed = NotCarried.of("pdf-layout").to_dict()
        short = NotCarried.of("pdf-pictures-unidentified", count=1, pictures=1).to_dict()

        _, out, _ = self.run_main("render-notes", "--notes", self.listed([typed, short]), "--all")

        said = json.loads(out)
        self.assertIn("why", said[1]["said"]["ko"])
        self.assertNotIn("en", said[0]["said"])

    def test_one_language_or_all_of_them_and_not_neither_or_both(self):
        for label, extra in {"neither": [], "both": ["--lexicon", "ko", "--all"]}.items():
            with self.subTest(label):
                code, _, err = self.run_main("render-notes", "--notes", self.listed([]), *extra)

                self.assertNotEqual(code, 0)
                self.assertIn("one language (--lexicon) or in all of them (--all)", err)

    def test_a_language_nobody_wrote_is_refused_by_the_command_line(self):
        code, _, err = self.run_main("render-notes", "--notes", self.listed([]), "--lexicon", "fr")

        self.assertEqual(code, 2)
        self.assertIn("invalid choice", err)

    def test_what_is_not_a_list_of_entries_is_refused_and_says_which_file(self):
        for label, text in {"not json": "{", "not a list": "{}", "an entry that is not one": '[{"what": 1}]'}.items():
            with self.subTest(label):
                path = self.root / "bad.json"
                path.write_text(text, encoding="utf-8")

                code, _, err = self.run_main("render-notes", "--notes", str(path), "--lexicon", "ko")

                self.assertNotEqual(code, 0)

    def test_a_file_that_is_not_there_is_refused(self):
        code, _, _ = self.run_main("render-notes", "--notes", str(self.root / "nowhere.json"), "--lexicon", "ko")

        self.assertNotEqual(code, 0)


if __name__ == "__main__":
    unittest.main()
