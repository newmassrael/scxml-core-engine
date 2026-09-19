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
    if embedded:
        notes.append(
            "embedded objects were not read: " + ", ".join(embedded)
            + " -- when a document hands a requirement to an attached "
              "spreadsheet, the decision logic is in there and not in the "
              "text above. This is the loss that looks most like success."
        )
    if pictures:
        notes.append(
            f"{len(pictures)} picture(s) were not read"
            + " -- a picture is usually what a screen LOOKS like rather than"
              " what decides it, so this is reported and not refused. It"
              " becomes a real gap only where a section's content is the"
              " picture, which no reader of text can tell you."
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
