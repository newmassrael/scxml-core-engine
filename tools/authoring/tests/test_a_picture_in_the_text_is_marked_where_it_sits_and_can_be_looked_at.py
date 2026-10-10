"""A picture in a specification is marked where it sits, and can be asked for and looked at.

This core makes no model calls, so it cannot say what a picture shows. That is not a reason to hand a
model the text of a specification and let it write from the text alone. One specification read in
the field had a clause whose conditions were a table in text; the table was followed by "an example of
how the two signals' input timing is processed", and the example -- which decides when the event fires
and when it does not -- was a drawing. The reader reported that pictures had not been read, said that
every clause which shows one also states something in text, and handed the writer text in which the
example was an empty line. Five writers then each guessed the rule, and none of them was ever shown the
picture, because nothing in the tool chain could show it.

What is held here:

  place        a picture is marked `[picture: NAME]` where it sits, in the order it sits, in a paragraph
               and in a table cell, DrawingML or the older VML
  once         the older drawing an alternate-content block keeps beside the new one is not a second picture
  objects      the preview of an embedded object that names an enclosed file is not marked (the object is);
               the preview of one that names none is content and is marked
  outside      a picture linked from outside the document, or held only by a header, is not placed, and the
               note counts it apart
  the note     never says the text is enough: a clause that speaks and shows a picture is counted, not cleared
  handed over  `read_pictures` gives the bytes by the NAME in a mark, from a word document or from the file
               beside a plain-text specification, refuses a name that is a path, a picture too heavy to hand
               over, and says when a format cannot be shown
  the tool     `picture` returns the image itself for the caller to look at, and says what a document holds
               when the name is not there
  the command  `picture` writes it to a file
"""

from __future__ import annotations

import base64
import io
import json
import pathlib
import subprocess
import sys
import tempfile
import unittest
import zipfile

from sce_author import mcp
from sce_author.ingest import MAX_PICTURE_BYTES, IngestError, ingest, picture_names_held, read_pictures

PNG = b"\x89PNG\r\n\x1a\n" + b"the drawing" * 40
NS = ('xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" '
      'xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" '
      'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" '
      'xmlns:v="urn:schemas-microsoft-com:vml" '
      'xmlns:o="urn:schemas-microsoft-com:office:office" '
      'xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"')
REL = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"


def para(text: str = "", *inner: str) -> str:
    run = f"<w:r><w:t>{text}</w:t></w:r>" if text else ""
    return f"<w:p>{run}{''.join(f'<w:r>{i}</w:r>' for i in inner)}</w:p>"


def drawn(rid: str) -> str:
    return f'<w:drawing><wp:inline xmlns:wp="x"><a:graphic><a:blip r:embed="{rid}"/></a:graphic></wp:inline></w:drawing>'


def pict(rid: str) -> str:
    return f'<w:pict><v:shape><v:imagedata r:id="{rid}"/></v:shape></w:pict>'


def rels(**targets: str) -> str:
    """`rId5="image3.png"` -> a picture; a value that starts with `embeddings/` is an embedded object."""
    rows = []
    for rid, target in targets.items():
        kind = "oleObject" if target.startswith("embeddings/") else "image"
        external = ' TargetMode="External"' if target.startswith("http") else ""
        path = target if target.startswith(("embeddings/", "http")) else f"media/{target}"
        rows.append(f'<Relationship Id="{rid}" Type="{REL}/{kind}" Target="{path}"{external}/>')
    return ('<?xml version="1.0" encoding="UTF-8"?><Relationships '
            f'xmlns="http://schemas.openxmlformats.org/package/2006/relationships">{"".join(rows)}</Relationships>')


class APictureIsMarkedWhereItSits(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = pathlib.Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)

    def docx(self, body: str, relationships: str, media: dict[str, bytes] | None = None,
             embedded: dict[str, bytes] | None = None, name: str = "spec.docx") -> pathlib.Path:
        path = self.tmp / name
        with zipfile.ZipFile(path, "w") as z:
            z.writestr("word/document.xml",
                       f'<?xml version="1.0" encoding="UTF-8"?><w:document {NS}><w:body>{body}</w:body></w:document>')
            z.writestr("word/_rels/document.xml.rels", relationships)
            for name_, data in (media or {}).items():
                z.writestr(f"word/media/{name_}", data)
            for name_, data in (embedded or {}).items():
                z.writestr(f"word/embeddings/{name_}", data)
        return path

    def lines(self, path: pathlib.Path) -> list[str]:
        return ingest(path).text.splitlines()

    def note(self, path: pathlib.Path) -> str:
        return " ".join(n for n in ingest(path).notes if "picture(s) were not read" in n)

    # ------------------------------------------------------------- place

    def test_a_picture_is_marked_right_after_the_sentence_that_refers_to_it(self):
        path = self.docx(para("5.1 The event") + para("- an example of the timing of the two inputs")
                         + para("", drawn("rId5")) + para("5.2 The fault"),
                         rels(rId5="image3.png"), {"image3.png": PNG})
        lines = self.lines(path)
        at = lines.index("- an example of the timing of the two inputs")
        self.assertEqual("[picture: image3.png]", lines[at + 1])

    def test_a_picture_in_a_table_cell_is_marked_in_its_row(self):
        cell = f"<w:tc>{para('timing', drawn('rId5'))}</w:tc>"
        path = self.docx(f"<w:tbl><w:tr>{cell}<w:tc>{para('ok')}</w:tc></w:tr></w:tbl>",
                         rels(rId5="image3.png"), {"image3.png": PNG})
        self.assertIn("| timing [picture: image3.png] | ok |", self.lines(path))

    def test_the_older_drawing_is_marked_too(self):
        path = self.docx(para("5.1 The event") + para("", pict("rId5")),
                         rels(rId5="image4.png"), {"image4.png": PNG})
        self.assertIn("[picture: image4.png]", self.lines(path))

    def test_pictures_are_marked_in_the_order_they_sit(self):
        path = self.docx(para("", drawn("rId5"), drawn("rId6")),
                         rels(rId5="b.png", rId6="a.png"), {"a.png": PNG, "b.png": PNG})
        self.assertIn("[picture: b.png] [picture: a.png]", self.lines(path))

    # -------------------------------------------------------------- once

    def test_the_older_copy_in_an_alternate_content_block_is_not_a_second_picture(self):
        both = (f"<mc:AlternateContent><mc:Choice>{drawn('rId5')}</mc:Choice>"
                f"<mc:Fallback>{pict('rId6')}</mc:Fallback></mc:AlternateContent>")
        path = self.docx(para("5.1 The event") + para("", both),
                         rels(rId5="image3.png", rId6="image4.png"),
                         {"image3.png": PNG, "image4.png": PNG})
        text = ingest(path).text
        self.assertEqual(1, text.count("[picture:"), text)
        self.assertIn("[picture: image3.png]", text)

    # ----------------------------------------------------------- objects

    def test_the_preview_of_an_embedded_object_is_not_marked_but_the_object_is(self):
        obj = ('<w:object><v:shape><v:imagedata r:id="rId6"/></v:shape>'
               '<o:OLEObject ProgID="Excel.Sheet.12" r:id="rId7"/></w:object>')
        path = self.docx(para("see the attached sheet", obj),
                         rels(rId6="preview.png", rId7="embeddings/oleObject1.bin"),
                         {"preview.png": PNG}, {"oleObject1.bin": b"not a workbook"})
        text = ingest(path).text
        self.assertIn("[enclosed object: oleObject1.bin", text)
        self.assertNotIn("[picture: preview.png]", text)

    def test_the_preview_of_an_object_that_names_no_enclosed_file_is_content_and_is_marked(self):
        """A picture of an equation: there is no file to point at, so the picture is all there is."""
        obj = ('<w:object><v:shape><v:imagedata r:id="rId6"/></v:shape>'
               '<o:OLEObject ProgID="Equation.3" r:id="rId9"/></w:object>')
        path = self.docx(para("the limit is", obj), rels(rId6="equation.png"), {"equation.png": PNG})
        self.assertIn("[picture: equation.png]", ingest(path).text)

    # ----------------------------------------------------------- outside

    def test_a_picture_linked_from_outside_the_document_is_not_placed(self):
        path = self.docx(para("5.1 The event", drawn("rId5")),
                         rels(rId5="http://example.invalid/x.png"))
        self.assertNotIn("[picture:", ingest(path).text)

    def test_a_picture_only_a_header_holds_is_counted_apart_from_the_ones_in_the_body(self):
        path = self.docx(para("5.1 The event") + para("", drawn("rId5")),
                         rels(rId5="image3.png"), {"image3.png": PNG, "logo.png": PNG})
        said = self.note(path)
        self.assertIn("2 picture(s) were not read", said)
        self.assertIn("1 of them sit in the body and are named where they sit", said)
        self.assertIn("1 more are not in the body", said)

    # ---------------------------------------------------------- the note

    def test_the_note_never_says_the_text_is_enough(self):
        """The sentence this replaces told the reader that every clause showing a picture also states
        something in text, as if that cleared it. Here the clause states conditions in text and
        refers to the picture for the rule."""
        path = self.docx(para("5.1 The event") + para("The event fires when both signals are LOCK")
                         + para("- an example of the timing of the two inputs") + para("", drawn("rId5")),
                         rels(rId5="image3.png"), {"image3.png": PNG})
        said = self.note(path)
        self.assertIn("state something in text and also show a picture", said)
        self.assertIn("does not settle that the picture adds nothing", said)
        self.assertIn("with the `picture` tool", said)
        for old in ("LOOKS like", "No numbered clause hands", "also states something"):
            self.assertNotIn(old, said)

    def test_a_document_with_no_picture_says_nothing_about_one(self):
        path = self.docx(para("5.1 The event"), rels())
        self.assertEqual([], [n for n in ingest(path).notes if "picture" in n])


class APictureIsHandedOver(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = pathlib.Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)
        self.word = self.tmp / "spec.docx"
        with zipfile.ZipFile(self.word, "w") as z:
            z.writestr("word/document.xml", f'<w:document {NS}><w:body>{para("x")}</w:body></w:document>')
            z.writestr("word/media/image3.png", PNG)
            z.writestr("word/media/vector.emf", b"EMF bytes")
        self.plain = self.tmp / "slice" / "spec.md"
        self.plain.parent.mkdir()
        self.plain.write_text("- an example\n[picture: image3.png]\n", encoding="utf-8")
        (self.plain.parent / "image3.png").write_bytes(PNG)

    def test_a_word_document_gives_its_picture_by_name(self):
        got = read_pictures([self.word], "image3.png")
        self.assertEqual([(self.word, "image3.png", "image/png", PNG)],
                         [(g.source, g.name, g.mime, g.data) for g in got])

    def test_a_plain_text_specification_names_a_picture_that_is_the_file_beside_it(self):
        got = read_pictures([self.plain], "image3.png")
        self.assertEqual(PNG, got[0].data)

    def test_several_files_that_hold_the_name_each_answer_and_say_which_they_are(self):
        got = read_pictures([self.word, self.plain], "image3.png")
        self.assertEqual({self.word, self.plain}, {g.source for g in got})

    def test_a_name_that_is_not_held_answers_nothing_and_the_document_says_what_it_does_hold(self):
        self.assertEqual([], read_pictures([self.word], "absent.png"))
        self.assertEqual({self.word: ["image3.png", "vector.emf"]}, picture_names_held([self.word, self.plain]))

    def test_a_name_that_is_a_path_is_refused(self):
        """A path here would turn a request for a picture into a request for any file readable."""
        for bad in ("../spec.md", "slice/image3.png", "/etc/passwd", "..", "", "a\\b"):
            with self.subTest(bad=bad), self.assertRaises(IngestError):
                read_pictures([self.plain], bad)

    def test_a_link_beside_the_specification_that_leaves_its_folder_is_not_followed(self):
        outside = self.tmp / "secret.png"
        outside.write_bytes(PNG)
        (self.plain.parent / "linked.png").symlink_to(outside)
        self.assertEqual([], read_pictures([self.plain], "linked.png"))

    def test_a_picture_too_heavy_to_hand_over_is_refused_by_name_and_size(self):
        with self.assertRaises(IngestError) as caught:
            read_pictures([self.word], "image3.png", limit=10)
        self.assertIn("image3.png", str(caught.exception))
        self.assertIn(str(len(PNG)), str(caught.exception))

    def test_the_most_that_is_handed_over_is_a_number_a_client_can_hold(self):
        self.assertLessEqual(MAX_PICTURE_BYTES, 5_000_000)

    def test_a_format_that_cannot_be_shown_is_handed_over_as_such(self):
        got = read_pictures([self.word], "vector.emf")
        self.assertIsNone(got[0].mime)


def drive(*messages):
    stdin = io.StringIO("".join(json.dumps(m) + "\n" for m in messages))
    stdout = io.StringIO()
    mcp.serve(stdin, stdout)
    return [json.loads(line) for line in stdout.getvalue().splitlines() if line.strip()]


class ThePictureToolShowsTheImage(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = pathlib.Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)
        self.word = self.tmp / "spec.docx"
        with zipfile.ZipFile(self.word, "w") as z:
            z.writestr("word/document.xml", f'<w:document {NS}><w:body>{para("x")}</w:body></w:document>')
            z.writestr("word/media/image3.png", PNG)
            z.writestr("word/media/vector.emf", b"EMF bytes")

    def call(self, **arguments) -> dict:
        replies = drive({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
                         "params": {"name": "picture", "arguments": arguments}})
        self.assertEqual(1, len(replies))
        return replies[0]["result"]

    def test_the_picture_comes_back_as_an_image_the_caller_can_look_at(self):
        result = self.call(prose=[str(self.word)], name="image3.png")
        self.assertFalse(result.get("isError"), result)
        kinds = [block["type"] for block in result["content"]]
        self.assertEqual(["text", "image"], kinds)
        image = result["content"][1]
        self.assertEqual("image/png", image["mimeType"])
        self.assertEqual(PNG, base64.b64decode(image["data"]))
        self.assertIn("[picture: image3.png]", result["content"][0]["text"])

    def test_a_name_the_document_does_not_hold_is_an_error_that_says_what_it_does_hold(self):
        result = self.call(prose=[str(self.word)], name="image9.png")
        self.assertTrue(result["isError"])
        self.assertIn("image3.png", result["content"][0]["text"])

    def test_a_format_that_cannot_be_shown_is_said_and_not_sent(self):
        result = self.call(prose=[str(self.word)], name="vector.emf")
        self.assertTrue(result["isError"])
        self.assertIn("cannot be shown", result["content"][0]["text"])
        self.assertEqual(["text"], [b["type"] for b in result["content"]])

    def test_a_name_that_is_a_path_is_an_error(self):
        result = self.call(prose=[str(self.word)], name="../spec.docx")
        self.assertTrue(result["isError"])

    def test_the_name_is_required(self):
        result = self.call(prose=[str(self.word)])
        self.assertTrue(result["isError"])

    def test_a_remote_caller_is_not_offered_the_servers_files(self):
        """Like `brief`, it reads files on the machine the server runs on."""
        result = mcp.call_tool("picture", {"prose": [str(self.word)], "name": "image3.png"}, remote=True)
        self.assertTrue(result["isError"])


class ThePictureCommandWritesTheFile(unittest.TestCase):
    def test_it_writes_the_picture_and_says_only_its_size(self):
        with tempfile.TemporaryDirectory() as tmp:
            word = pathlib.Path(tmp) / "spec.docx"
            with zipfile.ZipFile(word, "w") as z:
                z.writestr("word/document.xml", f'<w:document {NS}><w:body>{para("x")}</w:body></w:document>')
                z.writestr("word/media/image3.png", PNG)
            out = pathlib.Path(tmp) / "out.png"
            done = subprocess.run([sys.executable, "-m", "sce_author", "picture", "--prose", str(word),
                                   "--name", "image3.png", "--out", str(out)],
                                  capture_output=True, text=True, check=False)
            self.assertEqual(0, done.returncode, done.stderr)
            self.assertEqual(PNG, out.read_bytes())
            self.assertIn(f"{len(PNG)} bytes", done.stdout)

    def test_a_name_the_document_does_not_hold_fails_and_says_what_it_does(self):
        with tempfile.TemporaryDirectory() as tmp:
            word = pathlib.Path(tmp) / "spec.docx"
            with zipfile.ZipFile(word, "w") as z:
                z.writestr("word/document.xml", f'<w:document {NS}><w:body>{para("x")}</w:body></w:document>')
                z.writestr("word/media/image3.png", PNG)
            done = subprocess.run([sys.executable, "-m", "sce_author", "picture", "--prose", str(word),
                                   "--name", "nope.png", "--out", str(pathlib.Path(tmp) / "o.png")],
                                  capture_output=True, text=True, check=False)
            self.assertNotEqual(0, done.returncode)
            self.assertIn("image3.png", done.stderr)


if __name__ == "__main__":
    unittest.main()
