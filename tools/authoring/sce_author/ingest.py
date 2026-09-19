"""Getting a document to text, and saying what did not survive the trip.

**A format is not a subject matter.** A word-processor file is the same file
whatever it is about, so a reader for one belongs here, in the general core,
and not in a pack. Leaving extraction to each pack would make every pack
re-implement it, and the one that did it worst would be the one nobody
noticed.

⚠ Extraction is where fidelity is lost, and a loss here is invisible
downstream: a table that flattens wrong does not raise an error, it produces a
document that reads as though the specification never said anything. In the
corpus this was built against, the decision logic lives almost entirely in
tables -- so a reader that drops tables silently turns a whole specification
into "nothing here", which is indistinguishable from a specification that
really says nothing.

So every reader answers TWO things: the text, and what it could not carry.
The second is reported, never discarded, and a reader that produces
suspiciously little says so rather than letting the caller find out from an
empty question list.
"""

from __future__ import annotations

import io
import pathlib
import posixpath
import re
import xml.etree.ElementTree as ET
import zipfile
from dataclasses import dataclass, field


@dataclass
class Ingested:
    """Text, and an honest account of the trip."""

    text: str
    reader: str
    notes: list[str] = field(default_factory=list)

    @property
    def clean(self) -> bool:
        return not self.notes


from .errors import READ_ERRORS, IngestError, describe_path  # noqa: E402,F401


# ---------------------------------------------------------------- plain text


def _read_plain(path: pathlib.Path) -> Ingested:
    # ⚠ A document that is not text in the encoding it claims reached the
    # caller as a UnicodeDecodeError naming a byte offset. The offset is true
    # and useless; what the caller needs is the file and the fact that it is
    # not text.
    try:
        return Ingested(path.read_text(encoding="utf-8"), "plain")
    except READ_ERRORS as exc:
        raise IngestError(
            f"{path}: not UTF-8 text ({exc}). A binary document needs a reader "
            f"for its format, not a different encoding."
        ) from exc


# --------------------------------------------------------------------- html

_TAG = re.compile(r"<[^>]+>")
_SCRIPTISH = re.compile(r"<(script|style)\b.*?</\1>", re.S | re.I)
_ROW = re.compile(r"<tr\b.*?</tr>", re.S | re.I)
_CELL = re.compile(r"<t[dh]\b[^>]*>(.*?)</t[dh]>", re.S | re.I)


def _read_html(path: pathlib.Path) -> Ingested:
    raw = path.read_text(encoding="utf-8", errors="replace")
    raw = _SCRIPTISH.sub(" ", raw)
    # Tables first, and as rows: a table flattened into a paragraph loses which
    # value went with which condition, which is the entire content.
    def row(match):
        cells = [_TAG.sub("", c).strip() for c in _CELL.findall(match.group(0))]
        return "\n| " + " | ".join(cells) + " |\n"

    raw = _ROW.sub(row, raw)
    raw = re.sub(r"</(p|div|br|h[1-6]|li)>", "\n", raw, flags=re.I)
    text = _TAG.sub("", raw)
    text = re.sub(r"[ \t]+", " ", text)
    text = re.sub(r"\n{3,}", "\n\n", text)
    return Ingested(text.strip() + "\n", "html")


# --------------------------------------------------------------------- docx

_W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


def _docx_paragraph(node) -> str:
    return "".join(t.text or "" for t in node.iter(f"{_W}t"))


# A clause heading is a SHORT line that opens with its number. Long numbered
# lines are list items, and counting them as headings only makes the clauses
# finer -- which makes the report below fire more easily, not less, so the
# heuristic errs toward asking rather than toward silence.
_CLAUSE = re.compile(r"^\s*(\d+(?:\.\d+)*)\.?\s+\S")
_CLAUSE_HEADING_CHARS = 80

# A drawing, a legacy picture, and an embedded object. All three are content
# this reader cannot turn into text, and all three are anchored in a paragraph.
_DRAWN = {f"{_W}drawing", f"{_W}pict", f"{_W}object"}


def _clauses_carried_by_a_picture(body) -> tuple[list[str], int, int, int]:
    """Which numbered clauses state nothing in text and only show a picture.

    ⚠ THIS EXISTS BECAUSE COUNTING PICTURES DOES NOT ANSWER THE QUESTION IT
    RAISES. "71 pictures were not read" leaves a reader with no way to tell a
    screenshot beside a paragraph from a diagram that IS the requirement, and
    the only honest thing a text reader can do about the second is to say
    where it is.

    ⚠⚠ A cheaper discriminator was built first and MEASURED WRONG, which is
    why this one is clause-shaped: "a stretch with pictures and no text
    between them" was true of 156 drawings out of 156, because a word
    processor anchors a picture in a paragraph of its own. A test every
    instance passes says nothing.

    Returns the clause numbers, how many drawn things were attributed to a
    clause, how many there were in total, and how many clauses were found --
    the last three so the caller can tell "none" from "could not tell".
    """
    clauses: list[list] = []
    current: list | None = None
    drawn = attributed = 0
    for para in body.iter(f"{_W}p"):
        said = _docx_paragraph(para).strip()
        here = sum(1 for node in para.iter() if node.tag in _DRAWN)
        drawn += here
        heading = _CLAUSE.match(said) if said else None
        if heading and len(said) <= _CLAUSE_HEADING_CHARS:
            current = [heading.group(1), 0, here]
            clauses.append(current)
            attributed += here
            continue
        if current is None:
            continue
        attributed += here
        current[2] += here
        if said:
            current[1] += 1
    silent = [number for number, lines, shown in clauses if shown and not lines]
    return silent, attributed, drawn, len(clauses)


# ------------------------------------------------- what a document encloses

_XL = "{http://schemas.openxmlformats.org/spreadsheetml/2006/main}"
_A = "{http://schemas.openxmlformats.org/drawingml/2006/main}"


def _column(reference: str) -> int:
    """`C7` -> 3. The column letters of a cell reference, as a number."""
    index = 0
    for ch in reference:
        if not ch.isalpha():
            break
        index = index * 26 + (ord(ch.upper()) - 64)
    return index


def _read_enclosed_sheet(blob: bytes) -> tuple[list[str], list[str]]:
    """An embedded workbook as rows, keeping the grid.

    ⚠ THE GRID IS THE LOGIC. These documents state decisions as tables, so a
    reader that flattens cells into a stream turns "condition A gives X" into
    "condition A gives Y" without any sign that it happened. Every cell is
    placed by the COLUMN ITS REFERENCE NAMES rather than by the order the file
    lists it in -- a sparse row omits its empty cells, and reading positionally
    shifts every value after the first gap one column left.
    """
    notes: list[str] = []
    try:
        book = zipfile.ZipFile(io.BytesIO(blob))
        inner = set(book.namelist())
        shared: list[str] = []
        if "xl/sharedStrings.xml" in inner:
            root = ET.parse(io.BytesIO(book.read("xl/sharedStrings.xml"))).getroot()
            shared = ["".join(t.text or "" for t in si.iter(f"{_XL}t"))
                      for si in root.iter(f"{_XL}si")]
        lines: list[str] = []
        sheets = sorted(n for n in inner if n.startswith("xl/worksheets/sheet"))
        for sheet in sheets:
            root = ET.parse(io.BytesIO(book.read(sheet))).getroot()
            # ⚠ A merged cell holds its value in the top-left of the range and
            # leaves the rest empty, so the columns a reader sees are not the
            # columns a person saw. Counted rather than un-merged: guessing
            # which rows a merge was meant to cover is the flattening this
            # whole function exists to refuse.
            merges = len(list(root.iter(f"{_XL}mergeCell")))
            if merges:
                notes.append(f"{merges} merged cell range(s) in "
                             f"{posixpath.basename(sheet)}"
                             " were left as the file stores them -- the value"
                             " sits in the first cell and the rest are empty")
            for row in root.iter(f"{_XL}row"):
                cells: dict[int, str] = {}
                for cell in row.iter(f"{_XL}c"):
                    value = cell.find(f"{_XL}v")
                    text = ""
                    if cell.get("t") == "s" and value is not None:
                        try:
                            text = shared[int(value.text)]
                        except (ValueError, IndexError, TypeError):
                            text = ""
                    elif cell.get("t") == "inlineStr":
                        text = "".join(t.text or "" for t in cell.iter(f"{_XL}t"))
                    elif value is not None:
                        text = value.text or ""
                    if text.strip():
                        cells[_column(cell.get("r") or "")] = text.strip()
                if cells:
                    width = max(cells)
                    lines.append("| " + " | ".join(cells.get(i + 1, "")
                                                   for i in range(width)) + " |")
        return lines, notes
    except (zipfile.BadZipFile, ET.ParseError, KeyError) as exc:
        return [], [f"an enclosed workbook could not be opened ({exc})"]


def _read_enclosed_deck(blob: bytes) -> tuple[list[str], list[str]]:
    """An embedded presentation as its slides' text, in slide order."""
    try:
        deck = zipfile.ZipFile(io.BytesIO(blob))
        slides = sorted((n for n in deck.namelist()
                         if n.startswith("ppt/slides/slide")),
                        key=lambda n: int("".join(c for c in n if c.isdigit()) or 0))
        lines: list[str] = []
        for slide in slides:
            root = ET.parse(io.BytesIO(deck.read(slide))).getroot()
            for para in root.iter(f"{_A}p"):
                said = "".join(t.text or "" for t in para.iter(f"{_A}t")).strip()
                if said:
                    lines.append(said)
        return lines, []
    except (zipfile.BadZipFile, ET.ParseError, KeyError) as exc:
        return [], [f"an enclosed presentation could not be opened ({exc})"]


def _read_docx(path: pathlib.Path) -> Ingested:
    notes: list[str] = []
    try:
        with zipfile.ZipFile(path) as zf:
            names = set(zf.namelist())
            if "word/document.xml" not in names:
                raise IngestError(f"{path}: a zip, but not a word document")
            body = ET.parse(io.BytesIO(zf.read("word/document.xml"))).getroot()
            # Named rather than counted: "3 embedded objects" tells a reader
            # nothing they can act on, and the name is what they search for.
            #
            # ⚠ The two are kept apart because they are not equally dangerous,
            # and saying so is the difference between a warning that gets read
            # and one that gets skipped. Measured over one corpus of 4,056
            # sections: pictures were layout -- what a screen looks like, not
            # what decides -- and the sections whose content was only a
            # picture carried ZERO condition cells. Embedded objects were the
            # opposite: the body text hands requirements to "the spreadsheet
            # attached", and a reader that opens only the document part sees
            # none of it AND SUCCEEDS QUIETLY.
            embedded = sorted(n for n in names if n.startswith("word/embeddings/"))
            pictures = sorted(n for n in names if n.startswith("word/media/"))
            enclosed = {n: zf.read(n) for n in embedded
                        if n.lower().endswith((".xlsx", ".pptx"))}
    except zipfile.BadZipFile as exc:
        raise IngestError(f"{path}: not a readable document ({exc})") from exc

    lines: list[str] = []
    for node in body.iter():
        if node.tag == f"{_W}p":
            # A paragraph inside a table cell is emitted by the table branch.
            lines.append(_docx_paragraph(node))
        elif node.tag == f"{_W}tbl":
            for tr in node.iter(f"{_W}tr"):
                cells = [
                    " ".join(_docx_paragraph(p) for p in tc.iter(f"{_W}p")).strip()
                    for tc in tr.iter(f"{_W}tc")
                ]
                lines.append("| " + " | ".join(cells) + " |")

    # ⚠ The loop above walks every node, so a paragraph inside a table is
    # emitted twice -- once bare, once as part of its row. Dropping the bare
    # one would lose a paragraph that merely looks like a cell, so the rows
    # are kept and the duplicates are left: a reader of the brief sees the
    # table, and a name search is unaffected by a repeated line.
    # ⚠ What a document ENCLOSES is read, not merely named. The note this
    # replaces was right about the danger and wrong about what to do: an
    # embedded workbook is a zip of XML, so its rows come out mechanically --
    # no guessing, no model, nothing to get wrong except the grid, which the
    # reader above keeps. Measured on one 22,669-line specification: eleven
    # enclosed files held 732 spreadsheet rows and 14 slides that the body
    # text handed its requirements to and no reader had ever opened.
    carried, opened = [], []
    for name, blob in sorted(enclosed.items()):
        rows, trouble = (_read_enclosed_sheet(blob)
                         if name.lower().endswith(".xlsx")
                         else _read_enclosed_deck(blob))
        notes.extend(f"{posixpath.basename(name)}: {t}" for t in trouble)
        if rows:
            opened.append(name)
            carried.append(f"--- enclosed: {posixpath.basename(name)}")
            carried.extend(rows)
    lines.extend(carried)

    unopened = [n for n in embedded if n not in opened]
    if unopened:
        notes.append(
            "enclosed objects that could not be opened: "
            + ", ".join(posixpath.basename(n) for n in unopened)
            + " -- when a document hands a requirement to an attachment, the "
              "decision logic is in there and not in the text. This is the "
              "loss that looks most like success."
        )
    if opened:
        notes.append(
            f"{len(opened)} enclosed file(s) WERE opened and their contents "
            f"carried; a merged cell keeps the file's own layout, so a table "
            f"that looked merged on screen reads wider here"
        )
    if pictures:
        # ⚠ The count alone used to end with "which no reader of text can tell
        # you". That was false, and being false it left the most dangerous
        # picture -- the one a clause hands its whole content to -- looking
        # exactly like a screenshot. A text reader cannot say what a picture
        # SHOWS; it can say whether anything else in that clause says
        # anything, and that is the part a person can act on.
        silent, attributed, drawn, numbered = _clauses_carried_by_a_picture(body)
        if not numbered:
            where = (" This document has no numbered clauses, so where they"
                     " sit could not be answered.")
        elif silent:
            where = (f" {len(silent)} numbered clause(s) state nothing in text"
                     f" and show only a picture: {', '.join(silent)}."
                     " There, reading the text is not reading the clause.")
        else:
            where = (" No numbered clause hands its whole content to one:"
                     " every clause that shows a picture also states"
                     " something in text.")
        if drawn and attributed < drawn:
            where += (f" {drawn - attributed} of {drawn} sit outside the"
                      " numbering and were not placed.")
        notes.append(
            f"{len(pictures)} picture(s) were not read"
            " -- a picture is usually what a screen LOOKS like rather than"
            " what decides it, so this is reported and not refused." + where
        )

    text = "\n".join(line for line in lines if line.strip())
    if not text.strip():
        raise IngestError(
            f"{path}: read as a document and produced no text. An empty "
            f"document answers every question cleanly, which is "
            f"indistinguishable from one that says everything."
        )
    return Ingested(text + "\n", "docx", notes)


# ------------------------------------------------------------------ dispatch

READERS = {
    ".md": _read_plain,
    ".txt": _read_plain,
    ".text": _read_plain,
    ".html": _read_html,
    ".htm": _read_html,
    ".docx": _read_docx,
}

# Formats this core deliberately does not read, and what to do instead. Named
# rather than silently refused, because "unsupported" sends a reader looking
# for a bug in their own file.
ELSEWHERE = {
    ".pdf": "convert it first (a page-layout format needs a layout-aware "
            "extractor, and a wrong one silently reorders table cells)",
    ".doc": "a pre-2007 word file; convert it to .docx",
    ".xls": "a pre-2007 spreadsheet; convert it to .csv or .xlsx",
    ".xlsx": "export the sheets that carry decision logic to .csv",
    ".vsd": "a diagram; its text needs a diagram-aware extractor",
    ".vsdx": "a diagram; its text needs a diagram-aware extractor",
}


def ingest(path: pathlib.Path) -> Ingested:
    """One document to text, with an account of what did not come along."""
    path = pathlib.Path(path)
    if not path.is_file():
        # ⚠ Not "no such file". A directory, a broken symlink and a missing
        # file all answer false here, and calling all three absent sent a
        # reader looking for something that was sitting in front of them.
        raise IngestError(describe_path(path))
    reader = READERS.get(path.suffix.lower())
    if reader is None:
        hint = ELSEWHERE.get(path.suffix.lower())
        raise IngestError(
            f"{path}: no reader for {path.suffix or 'a file with no suffix'}"
            + (f" -- {hint}" if hint else "")
            + ". Readers live here because a format is not a subject matter; "
              "add one rather than converting in a pack."
        )
    return reader(path)
