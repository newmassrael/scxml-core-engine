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
from collections.abc import Iterable
from dataclasses import dataclass, field


@dataclass
class Ingested:
    """Text, and an honest account of the trip."""

    text: str
    reader: str
    notes: list[str] = field(default_factory=list)
    # The headings of the document when the reader knows them from the file and not from the text: the line
    # of `text` (counted as `splitlines` counts, from 0) mapped to its level. `None` is a reader that does not
    # say, whose text is then read as Markdown is (a line that begins `# ` is a heading); `{}` is a reader that
    # looked and found none, so no line of its text is one, whatever it begins with.
    headings: dict[int, int] | None = None
    # What a note is about, when it is about pictures that are every one marked in `text`: the note's place in
    # `notes` mapped to the NAMEs of their `[picture: NAME]` marks. An import of part of a document that leaves
    # every one of them out has nothing the note says (ADR 0039); a note with no entry here is of the whole
    # document, a picture a header holds among them, and stays with any part.
    concerns: dict[int, tuple[str, ...]] = field(default_factory=dict)

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
_CELL_ATTRS = re.compile(r"<t[dh]\b([^>]*)>(.*?)</t[dh]>", re.S | re.I)
# A table with no table inside it: nested tables are read from the inside out.
_TABLE = re.compile(r"<table\b(?:(?!<table\b).)*?</table>", re.S | re.I)
_SPAN_ATTR = {name: re.compile(name + r"""\s*=\s*["']?(\d+)""", re.I) for name in ("colspan", "rowspan")}
# The most columns or rows one cell is taken to cover. A page that says a cell spans a billion columns
# is not describing a table, and writing it out would fill the memory.
MAX_SPAN = 1000
MAX_NESTING = 50


def _one_line(text: str) -> str:
    """A cell's text on one line.

    ⚠ A table is written as one line per row, and everything downstream reads it that way. A cell that
    holds several lines (a list in a spreadsheet cell, a pretty-printed HTML cell) broke its row into
    pieces, only the first of which looked like a row: in one real document 35% of the rows from
    enclosed workbooks came out as several lines. A line break inside a cell is written as ` / `, so
    that the items of a list stay apart and the row stays whole.
    """
    pieces = [piece.strip() for piece in text.replace("\r\n", "\n").replace("\r", "\n").split("\n")]
    return " / ".join(piece for piece in pieces if piece)


def _span_of(attrs: str, name: str) -> int:
    found = _SPAN_ATTR[name].search(attrs)
    return min(MAX_SPAN, max(1, int(found.group(1)))) if found else 1


def _html_table_rows(table_html: str) -> list[list[str]]:
    """The rows of one HTML table with every cell in the column it sits in.

    A cell with `colspan` is written in each column it covers and a cell with `rowspan` in the rows it
    covers, as the cells of a merged word-processor table are (`_table_lines`), so that every row is as
    wide as the table and the cells under a vertical merge are not empty.
    """
    pending: dict[int, tuple[int, str]] = {}  # column -> (rows it still covers, its text)
    rows: list[list[str]] = []
    for row_html in _ROW.findall(table_html):
        cells: list[str] = []

        def fill_covered() -> None:
            while len(cells) in pending:
                left, text = pending[len(cells)]
                column = len(cells)
                cells.append(text)
                if left <= 1:
                    del pending[column]
                else:
                    pending[column] = (left - 1, text)

        for attrs, inner in _CELL_ATTRS.findall(row_html):
            fill_covered()
            text = _one_line(_TAG.sub("", inner))
            rows_covered = _span_of(attrs, "rowspan")
            for _ in range(_span_of(attrs, "colspan")):
                if rows_covered > 1:
                    pending[len(cells)] = (rows_covered - 1, text)
                cells.append(text)
        fill_covered()
        rows.append(cells)
    return rows


def _html_table(match: "re.Match[str]") -> str:
    rows = _html_table_rows(match.group(0))
    return "\n" + "\n".join("| " + " | ".join(cells) + " |" for cells in rows if cells) + "\n"


_HEADING_TAG = re.compile(r"<h([1-6])\b[^>]*>(.*?)</h\1\s*>", re.S | re.I)
# What stands before a heading's text while the page is being turned into lines. A page cannot hold it: it is
# taken out of the text first.
_HEADING_MARK = "\x00"


def _read_html(path: pathlib.Path, headings: bool = False) -> Ingested:
    """A page as text. With `headings`, it also says which of its lines are `<h1>` to `<h6>` and at what level."""
    raw = path.read_text(encoding="utf-8", errors="replace")
    if headings:
        raw = raw.replace(_HEADING_MARK, " ")
    raw = _SCRIPTISH.sub(" ", raw)
    # Tables with a table inside are read from the inside out: the inner one becomes lines of text
    # that sit in the cell of the outer one.
    for _ in range(MAX_NESTING):
        raw, changed = _TABLE.subn(_html_table, raw)
        if not changed:
            break
    # Tables first, and as rows: a table flattened into a paragraph loses which
    # value went with which condition, which is the entire content.
    def row(match):
        cells = [_one_line(_TAG.sub("", c)) for c in _CELL.findall(match.group(0))]
        return "\n| " + " | ".join(cells) + " |\n"

    raw = _ROW.sub(row, raw)
    if headings:
        def titled(match: "re.Match[str]") -> str:
            words = _one_line(_TAG.sub("", match.group(2)))
            return f"\n{_HEADING_MARK}{match.group(1)}{_HEADING_MARK}{words}\n" if words else "\n"

        raw = _HEADING_TAG.sub(titled, raw)
    raw = re.sub(r"</(p|div|br|h[1-6]|li)>", "\n", raw, flags=re.I)
    text = _TAG.sub("", raw)
    text = re.sub(r"[ \t]+", " ", text)
    text = re.sub(r"\n{3,}", "\n\n", text)
    if not headings:
        return Ingested(text.strip() + "\n", "html")
    # The marks are taken out and the headings said by the line they are on in the text as it is written.
    lines: list[str] = []
    at: dict[int, int] = {}
    for line in text.strip().splitlines():
        marked = re.fullmatch(f"{_HEADING_MARK}([1-6]){_HEADING_MARK}(.*)", line)
        if marked:
            at[len(lines)] = int(marked.group(1))
            line = marked.group(2).strip()
        lines.append(line)
    return Ingested("\n".join(lines) + "\n", "html", [], at)


# --------------------------------------------------------------------- docx

_W = "{http://schemas.openxmlformats.org/wordprocessingml/2006/main}"


def _docx_paragraph(node) -> str:
    return "".join(t.text or "" for t in node.iter(f"{_W}t"))


_O = "{urn:schemas-microsoft-com:office:office}"
_DOC_R = "{http://schemas.openxmlformats.org/officeDocument/2006/relationships}"


def _docx_text(node, attached: dict[str, str], pictured: dict[str, str] | None = None) -> str:
    """A paragraph's text, and where an enclosed file or a picture sits in it.

    ⚠ The text of a paragraph that carries an embedded object says nothing about
    the object: the body keeps a placeholder picture and a reference, so the
    sentence "the limits are in the attached sheet" and the sheet it means were
    separated, and the sheet's rows were carried to the END of the document.
    Whoever then reads a clause has no way to tell which of several enclosed
    files it hands its requirement to. The mark says it where the object sits,
    by the name the file is carried under below.

    `attached` maps a relationship id to the enclosed file it names. An object
    whose reference is not in it -- a picture of an equation, a link that leaves
    the document -- is not marked: there is no file to point at.

    `pictured` maps a relationship id to the picture it names, and the picture's
    place is marked the same way (`picture_marks`), so that a reader of the text
    can tell WHERE a rule may have been drawn and not written.
    """
    return " ".join(part for part in (_docx_paragraph(node), *enclosed_marks(node, attached),
                                      *picture_marks(node, pictured or {}, attached)) if part)


def enclosed_objects(node, attached: dict[str, str]) -> list[tuple[str, str | None]]:
    """`(file name, kind)` of each object under `node` that names an enclosed file, in the order they sit.

    Public so that a caller which slices a document by clause can ask which files a clause
    refers to, and carry them with it, without reading the mark back out of its text.
    """
    found: list[tuple[str, str | None]] = []
    for ole in node.iter(f"{_O}OLEObject"):
        name = attached.get(ole.get(f"{_DOC_R}id") or "")
        if name is not None:
            found.append((name, ole.get("ProgID")))
    return found


def enclosed_marks(node, attached: dict[str, str]) -> list[str]:
    """The `[enclosed object: NAME (KIND)]` marks of one paragraph, in the order its objects sit.

    Public because the mark is what ties a clause that says "the attached sheet" to the file
    it means. A caller that slices a document by clause, rather than reading it whole, places
    the same marks the whole-document reader does and does not write a second spelling of them.
    """
    return [f"[enclosed object: {name}" + (f" ({kind})" if kind else "") + "]"
            for name, kind in enclosed_objects(node, attached)]


_VML = "{urn:schemas-microsoft-com:vml}"
_MC_FALLBACK = "{http://schemas.openxmlformats.org/markup-compatibility/2006}Fallback"
_MEDIA = "word/media/"


def _media_files(names: Iterable[str]) -> list[str]:
    """The pictures a word document carries: its members under `word/media/`, in name order.

    A folder entry is not one. Word writes none, but a document zipped by another writer (`zip -r`) has
    an entry for each folder, and counted as a picture `word/media/` was a second one "not in the body".
    """
    return sorted(n for n in names if n.startswith(_MEDIA) and posixpath.basename(n))

# The formats a client can be SHOWN. A picture in any other (a metafile, a bitmap, a TIFF) is still
# named, counted and placed; `read_pictures` refuses to hand it over and says which format it is.
SHOWABLE = {".png": "image/png", ".jpg": "image/jpeg", ".jpeg": "image/jpeg",
            ".gif": "image/gif", ".webp": "image/webp"}

# The most one picture that is handed over may weigh. A result a client cannot hold is refused by
# name and size rather than truncated: half an image reads as a different image.
MAX_PICTURE_BYTES = 4_000_000

# What a package may expand to before any of it is read. A word document is a zip, and a file of a
# few megabytes can expand to gigabytes (a zip bomb) while every reader below holds a whole part in
# memory. The limits are set against a measured corpus of real specifications, whose largest part
# was 52 MB expanded and whose largest whole was 199 MB in 816 members: a part may be 128 MiB
# (2.5 times the largest), a package 1 GiB (5 times), and a package may hold 10,000 members.
MEMBER_MAX_BYTES = 128 * 1024 * 1024
ARCHIVE_MAX_BYTES = 1024 * 1024 * 1024
ARCHIVE_MAX_MEMBERS = 10_000


def _label_of(archive: zipfile.ZipFile) -> str:
    return str(archive.filename) if archive.filename else "an enclosed file"


def _check_archive(archive: zipfile.ZipFile) -> None:
    """Refuse a package whose members, taken together, are more than a reader should open.

    The sizes are the ones the package declares, and the library reads no more than a member
    declares, so a package that lies about them fails when it is read and does not expand past them.
    """
    infos = archive.infolist()
    label = _label_of(archive)
    if len(infos) > ARCHIVE_MAX_MEMBERS:
        raise IngestError(
            f"{label}: it holds {len(infos):,} parts and the most read is {ARCHIVE_MAX_MEMBERS:,}. "
            "A word document holds a few hundred; open it in a word processor and save it again.")
    total = sum(info.file_size for info in infos)
    if total > ARCHIVE_MAX_BYTES:
        raise IngestError(
            f"{label}: its parts expand to {total:,} bytes and the most read is {ARCHIVE_MAX_BYTES:,}. "
            "A file this small on disk and this large open is how a file that exhausts memory is made; "
            "open it in a word processor and save it again.")


def _member(archive: zipfile.ZipFile, name: str) -> bytes:
    """One part of a package, refused by name and size when it expands past what a reader holds."""
    info = archive.getinfo(name)
    if info.file_size > MEMBER_MAX_BYTES:
        raise IngestError(
            f"{_label_of(archive)}: its part {name} expands to {info.file_size:,} bytes and the most "
            f"read is {MEMBER_MAX_BYTES:,}. Open it in a word processor and save it again.")
    return archive.read(info)


def pictured_in(archive: zipfile.ZipFile) -> dict[str, str]:
    """Which picture each relationship id of the body names: id -> the name it is marked and read under.

    The name is the base name of the media file (`image3.png`), which is unique inside one document.
    A picture the body links to from outside the document names no file here and is not marked.
    """
    return {rid: posixpath.basename(part)
            for rid, (kind, part) in _related(archive, "word/document.xml").items()
            if kind == "image" and part.startswith(_MEDIA)}


def picture_names(node, pictured: dict[str, str], attached: dict[str, str]) -> list[str]:
    """The pictures under `node`, in the order they sit.

    Two things sit in a body that are not a second picture and are skipped, because marking them would
    count one picture twice or mark what another mark already says:

      - the fallback half of an alternate-content block: the same picture, drawn the older way;
      - the preview a word processor keeps for an embedded object that names an enclosed file: the
        object is marked by `enclosed_marks`, and its preview is what the object looks like.

    The preview of an object that names NO enclosed file (a picture of an equation) is content and is kept.
    """
    found: list[str] = []

    def walk(parent) -> None:
        for child in parent:
            if child.tag == _MC_FALLBACK:
                continue
            if child.tag == f"{_W}object" and enclosed_objects(child, attached):
                continue
            rid = None
            if child.tag == f"{_A}blip":
                rid = child.get(f"{_DOC_R}embed")
            elif child.tag == f"{_VML}imagedata":
                rid = child.get(f"{_DOC_R}id")
            name = pictured.get(rid or "")
            if name is not None:
                found.append(name)
            walk(child)

    walk(node)
    return found


def picture_marks(node, pictured: dict[str, str], attached: dict[str, str] | None = None) -> list[str]:
    """The `[picture: NAME]` marks of one paragraph, in the order its pictures sit.

    Public for the reason `enclosed_marks` is: a caller that slices a document by clause places the
    same marks the whole-document reader does, and does not write a second spelling of them. The
    NAME is what the `picture` tool is asked for.
    """
    return [f"[picture: {name}]" for name in picture_names(node, pictured, attached or {})]


# A clause heading is a SHORT line that opens with its number. Long numbered
# lines are list items, and counting them as headings only makes the clauses
# finer -- which makes the report below fire more easily, not less, so the
# heuristic errs toward asking rather than toward silence.
_CLAUSE = re.compile(r"^\s*(\d+(?:\.\d+)*)\.?\s+\S")
_CLAUSE_HEADING_CHARS = 80

# A drawing, a legacy picture, and an embedded object. All three are content
# this reader cannot turn into text, and all three are anchored in a paragraph.
_DRAWN = {f"{_W}drawing", f"{_W}pict", f"{_W}object"}


def _clauses_carried_by_a_picture(body) -> tuple[list[str], list[str], int, int, int]:
    """Which numbered clauses show a picture, and which of those state nothing in text.

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

    ⚠⚠⚠ A CLAUSE THAT ALSO STATES SOMETHING IN TEXT IS NOT THEREFORE ONE WHOSE
    PICTURE ADDS NOTHING. That was the belief this report used to rest on --
    "every clause that shows a picture also states something in text" was
    printed as reassurance -- and one specification refuted it: its clause
    carried a table of conditions in text and then said "an example of how the
    two signals' input timing is processed", and the example, which decides
    when the event fires and when it does not, was a drawing. The text did
    speak. It spoke about the picture. So the clauses that show a picture are
    returned whether or not they speak, and the silent ones are returned
    apart because for them reading the text is certainly not reading the clause.

    Returns the silent clause numbers, every clause number that shows a picture,
    how many drawn things were attributed to a clause, how many there were in
    total, and how many clauses were found -- the last three so the caller can
    tell "none" from "could not tell".
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
    showing = [number for number, _lines, shown in clauses if shown]
    return silent, showing, attributed, drawn, len(clauses)


def _listed(numbers: list[str], most: int = 12) -> str:
    """Clause numbers for a sentence: all of them, or the first few and how many were left."""
    if len(numbers) <= most:
        return ", ".join(numbers)
    return f"{', '.join(numbers[:most])} and {len(numbers) - most} more"


# ------------------------------------------------- what a document encloses

_XL ="{http://schemas.openxmlformats.org/spreadsheetml/2006/main}"
_A = "{http://schemas.openxmlformats.org/drawingml/2006/main}"
_R = "{http://schemas.openxmlformats.org/officeDocument/2006/relationships}"
_PKG = "{http://schemas.openxmlformats.org/package/2006/relationships}"


class _Related(dict):
    """The relationships of one part: id -> (type, path), and whether they could be read.

    `readable` is False when the part's relationship file is THERE and cannot be parsed. That
    is not the same as a part that declares none: an empty answer for "this part has no
    neighbours" and for "this part's neighbours are unknown" read alike, and the second is the
    one that loses a picture without a word.
    """

    readable: bool = True


def _related(archive: zipfile.ZipFile, part: str) -> _Related:
    """What a part of an office package points at: relationship id -> (type, path).

    The path is the part's name inside the zip, resolved from where the
    relationship was declared. A link that leaves the package, or a package
    that declares none, answers nothing rather than raising. A relationship
    file that is present and unreadable also answers nothing, but says so in
    `readable`, and the callers that would lose something by it report it.
    """
    folder, name = posixpath.split(part)
    declared = posixpath.join(folder, "_rels", name + ".rels")
    out = _Related()
    try:
        root = ET.parse(io.BytesIO(_member(archive, declared))).getroot()
    except KeyError:
        return out
    except ET.ParseError:
        out.readable = False
        return out
    for rel in root.iter(f"{_PKG}Relationship"):
        if rel.get("TargetMode") == "External" or not rel.get("Id"):
            continue
        target = rel.get("Target") or ""
        # A target written from the package root names its part from the root; any
        # other is relative to the folder of the part that declared it.
        resolved = (target[1:] if posixpath.isabs(target)
                    else posixpath.normpath(posixpath.join(folder, target)))
        # The kind is the last part of the relationship's type name, which is a URL.
        out[rel.get("Id")] = (posixpath.basename(rel.get("Type") or ""), resolved)
    return out


def _sheets_in_order(book: zipfile.ZipFile, inner: set[str]) -> list[tuple[str, str, bool]]:
    """The sheets of a workbook as (name, part, hidden), in the order the workbook lists them.

    ⚠ A workbook's sheets are told apart by their NAMES and ordered by the
    workbook, not by the file names they happen to be stored under. Sorting the
    files puts `sheet10` before `sheet2` and drops every name, so a reader of
    nine concatenated grids could not say which table it was reading -- and
    "which sheet applies to this product" is a question for a person, who can
    only be asked about a sheet by its name.

    A sheet the workbook does not list, or a workbook whose list cannot be
    read, falls back to the stored file names in file-name order, which is
    what was always read.
    """
    stored = sorted(n for n in inner if n.startswith("xl/worksheets/sheet"))
    listed: list[tuple[str, str, bool]] = []
    if "xl/workbook.xml" in inner:
        try:
            root = ET.parse(io.BytesIO(_member(book, "xl/workbook.xml"))).getroot()
        except ET.ParseError:
            root = None
        if root is not None:
            related = _related(book, "xl/workbook.xml")
            for sheet in root.iter(f"{_XL}sheet"):
                part = related.get(sheet.get(f"{_R}id") or "", ("", ""))[1]
                if part in inner:
                    hidden = sheet.get("state") in ("hidden", "veryHidden")
                    listed.append((sheet.get("name") or posixpath.basename(part), part, hidden))
    named = {part for _, part, _ in listed}
    listed += [(posixpath.basename(part), part, False) for part in stored if part not in named]
    return listed


def _column(reference: str) -> int:
    """`C7` -> 3. The column letters of a cell reference, as a number."""
    index = 0
    for ch in reference:
        if not ch.isalpha():
            break
        index = index * 26 + (ord(ch.upper()) - 64)
    return index


_XDR = "{http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing}"

# What else a sheet's drawing can hold besides a picture: a shape, a chart, a
# group of them, a connector. None of them is read here.
_DRAWN_ELSE = (f"{_XDR}sp", f"{_XDR}graphicFrame", f"{_XDR}grpSp", f"{_XDR}cxnSp")


@dataclass
class _Drawn:
    """What one sheet's drawings hold."""

    # (row, column), both from 1, -> how many pictures are anchored on that cell.
    placed: dict[tuple[int, int], int] = field(default_factory=dict)
    # Pictures not anchored on a cell: floating at a page position, or inside a group.
    loose: int = 0
    # Shapes and charts, which are not read at all.
    other: int = 0
    # Drawings the sheet refers to and this reader could not use: the part is not in the file,
    # it cannot be parsed, or the sheet's own relationships could not be read so that it is not
    # known whether it has any. Each is a place a picture could have been and was not seen.
    unreadable: int = 0


def _drawn_on(book: zipfile.ZipFile, inner: set[str], sheet: str, root) -> _Drawn:
    """Where the pictures of a sheet sit, as the cells they are anchored on.

    `root` is the sheet's own XML. What the SHEET says it refers to is checked against what its
    relationships list: the links are walked below, and a walk of the links alone cannot see a
    drawing the sheet names and the links do not -- the relationship file is missing, or it is
    there and has no entry for that id. Those read as a sheet with no pictures, and the cells
    that held them as empty, with no word said (found by a review, 2026-10-05). A sheet that
    names no drawing and has no links file is a plain sheet and says nothing.

    ⚠ A GRID CELL CAN BE A PICTURE. In a table of marks, whether a cell holds
    one is the datum: the text of the cell says '-' or 'O' and the picture
    beside it says which row has an image. A reader that opens the cells and
    skips the drawing reports the table as complete -- on one specification the
    workbook it opened held 2,188 pictures on its cells, and the reader said
    "4 enclosed files WERE opened and their contents carried".

    Only WHERE is read. What a picture shows is for whoever can read it, and so
    is the meaning of the column it sits in; this says which cell holds one and
    leaves the rest alone.
    """
    drawn = _Drawn()
    related = _related(book, sheet)
    if not related.readable:
        # Whether the sheet has a drawing is not known, which is not the same as having none.
        drawn.unreadable += 1
    else:
        for reference in root.iter(f"{_XL}drawing"):
            listed = related.get(reference.get(f"{_R}id") or "")
            if listed is None or listed[0] != "drawing":
                # The sheet names a drawing its links do not list (or list as something else):
                # the pictures it held cannot be found.
                drawn.unreadable += 1
    for kind, part in related.values():
        if kind != "drawing":
            continue
        if part not in inner:
            # The sheet says it has a drawing and the file has none to give: whatever pictures
            # it held are gone, and the cells they sat on read as empty.
            drawn.unreadable += 1
            continue
        try:
            root = ET.parse(io.BytesIO(_member(book, part))).getroot()
        except ET.ParseError:
            drawn.unreadable += 1
            continue
        for anchor in root:
            cell = anchor.find(f"{_XDR}from") if anchor.tag in (
                f"{_XDR}oneCellAnchor", f"{_XDR}twoCellAnchor") else None
            at = None
            if cell is not None:
                try:
                    at = (int(cell.findtext(f"{_XDR}row")) + 1,
                          int(cell.findtext(f"{_XDR}col")) + 1)
                except (TypeError, ValueError):
                    at = None
            for item in anchor:
                if item.tag == f"{_XDR}pic":
                    if at is None:
                        drawn.loose += 1
                    else:
                        drawn.placed[at] = drawn.placed.get(at, 0) + 1
                elif item.tag in _DRAWN_ELSE:
                    drawn.other += 1
                    if item.tag == f"{_XDR}grpSp":
                        drawn.loose += sum(1 for _ in item.iter(f"{_XDR}pic"))
    return drawn


_RD = "{http://schemas.microsoft.com/office/spreadsheetml/2017/richdata}"


def _picture_values(book: zipfile.ZipFile, inner: set[str]) -> set[int]:
    """The `vm` values of a workbook that stand for a picture held IN a cell.

    A spreadsheet stores a picture two ways. Laid over the grid it is a drawing
    anchored on a cell, which `_drawn_on` reads. Held in the cell -- the "place
    in cell" picture -- it is a rich value: the cell names it by `vm`, counted
    from 1, an index into the workbook's value metadata, which names a rich
    value, whose structure names its type. The cell's own text is only a
    placeholder, so a reader that takes the cell at its word reads an error.

    Anything that cannot be followed answers nothing, and the cell is then
    counted as holding a rich value that was not read -- never as plain text.
    """
    parts = ("xl/metadata.xml", "xl/richData/rdrichvalue.xml", "xl/richData/rdrichvaluestructure.xml")
    if not all(part in inner for part in parts):
        return set()
    try:
        metadata, values, structures = (ET.parse(io.BytesIO(_member(book, p))).getroot() for p in parts)
        types = [(s.get("t") or "").lower() for s in structures.iter(f"{_RD}s")]
        value_types = [int(v.get("s")) for v in values.iter(f"{_RD}rv")]
        kinds = [m.get("name") for m in metadata.iter(f"{_XL}metadataType")]
        futures: list[int] = []
        for future in metadata.iter(f"{_XL}futureMetadata"):
            if future.get("name") != "XLRICHVALUE":
                continue
            for bk in future.iter(f"{_XL}bk"):
                rvb = next(bk.iter(f"{_RD}rvb"), None)
                futures.append(int(rvb.get("i")) if rvb is not None else -1)
        pictures: set[int] = set()
        for held in metadata.iter(f"{_XL}valueMetadata"):
            for vm, bk in enumerate(held.iter(f"{_XL}bk"), start=1):
                # ⚠ One entry whose chain breaks is that cell's problem, not the
                # workbook's: the cells that can be followed stay pictures.
                try:
                    rc = next(bk.iter(f"{_XL}rc"), None)
                    if rc is None or kinds[int(rc.get("t")) - 1] != "XLRICHVALUE":
                        continue
                    value = futures[int(rc.get("v"))]
                    if value >= 0 and "image" in types[value_types[value]]:
                        pictures.add(vm)
                except (ValueError, TypeError, IndexError):
                    continue
        return pictures
    except (ET.ParseError, ValueError, TypeError, IndexError, KeyError):
        return set()


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
        _check_archive(book)
        inner = set(book.namelist())
        shared: list[str] = []
        if "xl/sharedStrings.xml" in inner:
            root = ET.parse(io.BytesIO(_member(book, "xl/sharedStrings.xml"))).getroot()
            shared = ["".join(t.text or "" for t in si.iter(f"{_XL}t"))
                      for si in root.iter(f"{_XL}si")]
        lines: list[str] = []
        pictures = cells_with_pictures = loose = other = unreadable = rich = 0
        unreadable_on: list[str] = []
        in_cell = _picture_values(book, inner)
        for name, sheet, hidden in _sheets_in_order(book, inner):
            root = ET.parse(io.BytesIO(_member(book, sheet))).getroot()
            rows: list[str] = []
            grid: dict[int, dict[int, str]] = {}
            # ⚠ A merged cell holds its value in the top-left of the range and
            # leaves the rest empty, so the columns a reader sees are not the
            # columns a person saw. Counted rather than un-merged: guessing
            # which rows a merge was meant to cover is the flattening this
            # whole function exists to refuse.
            merges = len(list(root.iter(f"{_XL}mergeCell")))
            if merges:
                notes.append(f"{merges} merged cell range(s) in "
                             f"sheet \"{name}\""
                             " were left as the file stores them -- the value"
                             " sits in the first cell and the rest are empty")
            held: dict[tuple[int, int], int] = {}
            number = 0
            for row in root.iter(f"{_XL}row"):
                # A row names its own number; one that does not follows the row
                # before it. The number is what a picture is placed by.
                try:
                    number = int(row.get("r"))
                except (TypeError, ValueError):
                    number += 1
                cells = grid.setdefault(number, {})
                for cell in row.iter(f"{_XL}c"):
                    value = cell.find(f"{_XL}v")
                    text = ""
                    if cell.get("vm"):
                        # A rich value. A picture held in the cell is the cell's content
                        # and its stored text is a placeholder; any other rich value
                        # keeps the text the file stores, and is counted.
                        try:
                            is_picture = int(cell.get("vm")) in in_cell
                        except ValueError:
                            is_picture = False
                        if is_picture:
                            at = (number, _column(cell.get("r") or ""))
                            held[at] = held.get(at, 0) + 1
                            continue
                        rich += 1
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
                        cells[_column(cell.get("r") or "")] = _one_line(text)
            drawn = _drawn_on(book, inner, sheet, root)
            loose += drawn.loose
            other += drawn.other
            unreadable += drawn.unreadable
            if drawn.unreadable:
                unreadable_on.append(f"\"{name}\"")
            placed = dict(drawn.placed)
            for at, count in held.items():
                placed[at] = placed.get(at, 0) + count
            for (at_row, at_column), count in placed.items():
                pictures += count
                cells_with_pictures += 1
                mark = "[picture]" if count == 1 else f"[{count} pictures]"
                there = grid.setdefault(at_row, {})
                there[at_column] = f"{there[at_column]} {mark}" if at_column in there else mark
            for at_row in sorted(grid):
                cells = grid[at_row]
                if cells:
                    width = max(cells)
                    rows.append("| " + " | ".join(cells.get(i + 1, "")
                                                  for i in range(width)) + " |")
            if rows:
                # A hidden sheet is still the author's sheet, and carried; it is
                # said to be hidden because "the sheet nobody sees on screen"
                # is the one a person is likelier to have meant to leave out.
                lines.append(f"--- sheet: {name}" + (" (hidden)" if hidden else ""))
                lines.extend(rows)
        # ⚠ Said once per workbook, and said even when nothing else is: this is
        # the part of an opened workbook a reader would take for read.
        if pictures:
            notes.append(
                f"{pictures} picture(s) sit on {cells_with_pictures} cell(s) of this "
                f"workbook (anchored on the cell, or held in it) and are marked "
                f"[picture] there. Which cell holds one is carried; what a picture "
                f"shows is not read, and in a grid a picture can be the whole content "
                f"of its cell")
        if rich:
            notes.append(f"{rich} cell(s) in this workbook hold a rich value that is not "
                         f"a picture (a data type or an object) and keep only the text the "
                         f"file stores for them")
        if loose:
            notes.append(
                f"{loose} picture(s) in this workbook are not anchored on a cell "
                f"(floating, or inside a group) and were not placed")
        if other:
            notes.append(f"{other} drawn object(s) in this workbook other than pictures "
                         f"(shapes, charts) were not read")
        if unreadable:
            # ⚠ The loudest of these notes, because this is the loss that looks like an empty
            # cell: a sheet that refers to a drawing the file does not hold, or whose
            # relationships cannot be read, may have held pictures that were never seen, and
            # the cells they sat on read as blank. A reader cannot tell a cell that is empty
            # from one whose picture was lost; this is how it is told.
            notes.append(
                f"{unreadable} drawing(s) referred to by sheet(s) {', '.join(unreadable_on)} of "
                f"this workbook could not be read (the drawing is missing from the file, cannot "
                f"be parsed, or the sheet's links to it cannot be read). Pictures on those "
                f"sheets were not seen, and a cell that reads as empty there may have held one")
        return lines, notes
    except (zipfile.BadZipFile, ET.ParseError, KeyError, IngestError) as exc:
        return [], [f"an enclosed workbook could not be opened ({exc})"]


def _read_enclosed_deck(blob: bytes) -> tuple[list[str], list[str]]:
    """An embedded presentation as its slides' text, in slide order."""
    try:
        deck = zipfile.ZipFile(io.BytesIO(blob))
        _check_archive(deck)
        slides = sorted((n for n in deck.namelist()
                         if n.startswith("ppt/slides/slide")),
                        key=lambda n: int("".join(c for c in n if c.isdigit()) or 0))
        lines: list[str] = []
        for slide in slides:
            root = ET.parse(io.BytesIO(_member(deck, slide))).getroot()
            for para in root.iter(f"{_A}p"):
                said = "".join(t.text or "" for t in para.iter(f"{_A}t")).strip()
                if said:
                    lines.append(said)
        return lines, []
    except (zipfile.BadZipFile, ET.ParseError, KeyError, IngestError) as exc:
        return [], [f"an enclosed presentation could not be opened ({exc})"]


@dataclass(frozen=True)
class Enclosed:
    """What a word document encloses, read.

    ⚠ Reading an attachment is not one caller's feature. A reader that takes the whole document
    and a reader that slices it by clause meet the same objects, and a reader that opens only the
    document part SUCCEEDS QUIETLY: the sentence "the limits are in the attached sheet" survives
    and the sheet does not. The opening therefore lives here once, and both callers ask it.

    `attached`  a relationship id -> the name the enclosed file is carried under, for placing a mark
    `rows`      that name -> the file's rows as the reader carries them; only files that opened,
                in name order
    `unopened`  names of enclosed objects that could not be read as rows -- named, never dropped
    `notes`     what went wrong opening them, one entry per trouble
    """

    attached: dict[str, str]
    rows: dict[str, list[str]]
    unopened: list[str]
    notes: list[str]

    def carried(self, names: Iterable[str] | None = None) -> list[str]:
        """The lines that carry enclosed files under their names: a heading, then the rows.

        `None` carries every file that opened. A name that did not open is refused rather than
        skipped: a slice that asked for a file and was handed nothing would read as one that
        never referred to it.
        """
        wanted = list(self.rows) if names is None else list(names)
        lines: list[str] = []
        for name in wanted:
            if name not in self.rows:
                raise KeyError(f"{name!r} is not an enclosed file that opened; "
                               f"those that did: {', '.join(self.rows) or 'none'}")
            lines.append(f"--- enclosed: {name}")
            lines.extend(self.rows[name])
        return lines


_EMBEDDINGS = "word/embeddings/"


def _names_of(parts: list[str]) -> dict[str, str]:
    """The name each enclosed file is carried and marked under: part path -> name.

    ⚠ The base name, and where two files share one, the path under the embeddings folder. Keying by
    base name alone let the second `Book.xlsx` replace the first -- both objects in the body then read
    `Book.xlsx`, one table was gone, and nothing was said (a review, 2026-10-05). EVERY file that shares
    a name is renamed, not only the later ones: which of two is "the" `Book.xlsx` is an accident of
    the order the zip lists them, and a name that depends on it would move when the zip is rewritten.
    A name that is unique stays the plain base name, so nothing else changes.
    """
    bases = [posixpath.basename(p) for p in parts]
    return {part: (part[len(_EMBEDDINGS):] if bases.count(posixpath.basename(part)) > 1
                   else posixpath.basename(part))
            for part in parts}


def read_enclosed(archive: zipfile.ZipFile) -> Enclosed:
    """Open every workbook and presentation a word document encloses, and say which object is which."""
    embedded = sorted(n for n in archive.namelist()
                      # A folder entry has no base name, and is not a file to read.
                      if n.startswith(_EMBEDDINGS) and posixpath.basename(n))
    names = _names_of(embedded)
    # Which enclosed file each object in the body refers to, by the name the file is carried
    # under below.
    attached = {rid: names[part]
                for rid, (_kind, part) in _related(archive, "word/document.xml").items()
                if part in names}
    rows: dict[str, list[str]] = {}
    notes: list[str] = []
    for part in embedded:
        if not part.lower().endswith((".xlsx", ".pptx")):
            continue
        name = names[part]
        try:
            blob = _member(archive, part)
        except IngestError as exc:
            # An enclosed file too large to hold is said to be unopened, as one that cannot be parsed is.
            notes.append(f"{name}: could not be opened ({exc})")
            continue
        opened, trouble = (_read_enclosed_sheet(blob) if part.lower().endswith(".xlsx")
                           else _read_enclosed_deck(blob))
        notes.extend(f"{name}: {t}" for t in trouble)
        if opened:
            rows[name] = opened
    unopened = [names[part] for part in embedded if names[part] not in rows]
    return Enclosed(attached, rows, unopened, notes)


def _int_val(parent, tag: str, default: int) -> int:
    """The `w:val` of the child `tag` of `parent` as a whole number, or `default` when it is not there."""
    if parent is None:
        return default
    child = parent.find(f"{_W}{tag}")
    if child is None:
        return default
    try:
        return int(child.get(f"{_W}val", str(default)))
    except ValueError:
        return default


def _table_lines(table, attached: dict[str, str], pictured: dict[str, str]) -> list[str]:
    """A table as one line per row, every cell in the column it sits in.

    ⚠ A merged cell is written once in the file and covers several columns or rows, so a reader that
    writes the cells it finds shifts every later value of the row one column left (a cell that spans
    two columns) and leaves the cells under a vertical merge empty. In a requirements table that is
    where the header bands are and where a condition continues down several rows, and the shift does
    not announce itself: the row still looks like a row. So a cell that spans columns is written in
    each column it covers, and a cell that continues the one above is written with that cell's text,
    which makes every row as wide as the table.
    """
    rows: list[list[str]] = []
    carried: dict[int, str] = {}  # column -> the text of the cell a vertical merge goes on from
    for tr in table.findall(f"{_W}tr"):
        row_props = tr.find(f"{_W}trPr")
        cells: list[str] = [""] * _int_val(row_props, "gridBefore", 0)
        for tc in tr.findall(f"{_W}tc"):
            props = tc.find(f"{_W}tcPr")
            span = max(1, _int_val(props, "gridSpan", 1))
            merge = props.find(f"{_W}vMerge") if props is not None else None
            text = _one_line(" ".join(_docx_text(p, attached, pictured) for p in tc.iter(f"{_W}p")))
            column = len(cells)
            if merge is not None and merge.get(f"{_W}val") != "restart":
                text = carried.get(column, text)
            elif merge is not None:
                for covered in range(column, column + span):
                    carried[covered] = text
            else:
                for covered in range(column, column + span):
                    carried.pop(covered, None)
            cells.extend([text] * span)
        cells.extend([""] * _int_val(row_props, "gridAfter", 0))
        if cells:
            rows.append(cells)
    return ["| " + " | ".join(cells) + " |" for cells in rows]


_HEADING_NAME = re.compile(r"^heading\s*([1-9])$", re.IGNORECASE)
# The deepest level a heading is given. Word has nine; a document that goes deeper than six is read as six.
_MOST_HEADING_LEVEL = 6


def _outline_level(props) -> int | None:
    """The level a `pPr` or a style's properties give by their outline level (0 is the top), or none."""
    if props is None:
        return None
    node = props.find(f"{_W}outlineLvl")
    value = None if node is None else node.get(f"{_W}val")
    # 9 is "body text": the level a paragraph has when it is in no outline.
    return int(value) + 1 if value is not None and value.isdigit() and int(value) < 9 else None


def _docx_heading_levels(zf: zipfile.ZipFile, names: set[str]) -> dict[str, int]:
    """The level of each paragraph style that is a heading, by the style's id.

    A style is a heading by what the file says of it: an outline level of its own, or the name Word gives
    its built-in headings (`heading 1`, and `Title`, which is the top), and a style based on a heading is
    one at the same level unless it says another. The names are the file's and not the language Word ran in:
    Word writes the built-in names in English whatever its interface says.
    """
    if "word/styles.xml" not in names:
        return {}
    try:
        root = ET.parse(io.BytesIO(_member(zf, "word/styles.xml"))).getroot()
    except ET.ParseError:
        # A styles part that is not XML is a document whose headings are not known, and not one with none:
        # its paragraphs are read as they were, and the extraction does not claim a structure it did not see.
        return {}
    own: dict[str, int] = {}
    parents: dict[str, str] = {}
    for style in root.iter(f"{_W}style"):
        if style.get(f"{_W}type") != "paragraph":
            continue
        style_id = style.get(f"{_W}styleId")
        if not style_id:
            continue
        based = style.find(f"{_W}basedOn")
        if based is not None and based.get(f"{_W}val"):
            parents[style_id] = based.get(f"{_W}val")
        level = _outline_level(style.find(f"{_W}pPr"))
        if level is None:
            name = style.find(f"{_W}name")
            said = (name.get(f"{_W}val") if name is not None else "") or ""
            named = _HEADING_NAME.match(said.strip())
            level = int(named.group(1)) if named else 1 if said.strip().lower() == "title" else None
        if level is not None:
            own[style_id] = level
    levels: dict[str, int] = {}
    for style_id in set(own) | set(parents):
        seen, current = set(), style_id
        # A chain of styles is followed to the first that says a level; one that loops, or is long, says none.
        while current and current not in seen and len(seen) < 20:
            if current in own:
                levels[style_id] = min(_MOST_HEADING_LEVEL, own[current])
                break
            seen.add(current)
            current = parents.get(current, "")
    return levels


def _heading_level_of(paragraph, levels: dict[str, int]) -> int | None:
    """The heading level of a paragraph: its own outline level, else its style's."""
    props = paragraph.find(f"{_W}pPr")
    if props is None:
        return None
    own = _outline_level(props)
    if own is not None:
        return min(_MOST_HEADING_LEVEL, own)
    style = props.find(f"{_W}pStyle")
    if style is None:
        return None
    style_id = style.get(f"{_W}val") or ""
    if style_id in levels:
        return levels[style_id]
    # A style the styles part does not describe (a document without one) is a heading by the id Word gives
    # its built-in ones, which is the same word as the name.
    named = _HEADING_NAME.match(style_id.strip())
    if named:
        return int(named.group(1))
    return 1 if style_id.strip().lower() == "title" else None


def _docx_lines(
    body, attached: dict[str, str], pictured: dict[str, str], levels: dict[str, int] | None = None
) -> tuple[list[str], dict[int, int]]:
    """The lines of a document body in reading order: a paragraph is a line, a table is a line per row.

    A paragraph inside a table is part of its row and is written there and nowhere else. (The first
    version of this reader walked every node, which wrote each such paragraph twice, once bare and once
    in its row; in a corpus of real specifications 93% of the paragraphs are in tables, so the text came
    out nearly doubled and a name search found every name twice.)

    With `levels` (the heading styles of the document), the index of each line that is a heading paragraph
    is given beside the lines, with its level: the text stays what it was, and the structure is said apart
    from it, where a paragraph that happens to begin with `#` cannot be mistaken for it.
    """
    lines: list[str] = []
    heading_at: dict[int, int] = {}

    def walk(parent) -> None:
        for child in parent:
            if child.tag == f"{_W}p":
                said = _docx_text(child, attached, pictured)
                level = _heading_level_of(child, levels) if levels is not None and said.strip() else None
                if level is not None:
                    heading_at[len(lines)] = level
                lines.append(said)
            elif child.tag == f"{_W}tbl":
                lines.extend(_table_lines(child, attached, pictured))
            else:
                # A wrapper (a content control, a section) holds blocks of its own.
                walk(child)

    walk(body)
    return lines, heading_at


def _read_docx(path: pathlib.Path, headings: bool = False) -> Ingested:
    """A word document as text. With `headings`, it also says which of its lines are headings, and at what level.

    The prose reading of a specification (a brief for a model) takes the text as it was; an extraction
    (ADR 0026) is of blocks and needs to know which of them are headings, or a Word document cannot be
    divided into the parts an owner chooses between (ADR 0039).
    """
    notes: list[str] = []
    concerns: dict[int, tuple[str, ...]] = {}
    levels: dict[str, int] | None = None
    try:
        with zipfile.ZipFile(path) as zf:
            _check_archive(zf)
            names = set(zf.namelist())
            if "word/document.xml" not in names:
                raise IngestError(f"{path}: a zip, but not a word document")
            try:
                body = ET.parse(io.BytesIO(_member(zf, "word/document.xml"))).getroot()
            except ET.ParseError as exc:
                # An entity that names a file or expands without end is refused by the parser; what
                # reaches the caller is a refusal that names the document, not a traceback.
                raise IngestError(f"{path}: its text part is not well-formed XML ({exc})") from exc
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
            #
            # ⚠ "Pictures were layout" is a measurement of one corpus, not a property of
            # pictures: a later specification drew the rule that decides when an event
            # fires, in a clause whose conditions were a text table. A picture is
            # therefore marked where it sits and can be asked for (`read_pictures`), and
            # the note below does not say the text is enough.
            pictures = _media_files(names)
            enclosed = read_enclosed(zf)
            pictured = pictured_in(zf)
            if headings:
                levels = _docx_heading_levels(zf, names)
    except zipfile.BadZipFile as exc:
        raise IngestError(f"{path}: not a readable document ({exc})") from exc

    lines, heading_at = _docx_lines(body, enclosed.attached, pictured, levels)

    # ⚠ What a document ENCLOSES is read, not merely named. The note this
    # replaces was right about the danger and wrong about what to do: an
    # embedded workbook is a zip of XML, so its rows come out mechanically --
    # no guessing, no model, nothing to get wrong except the grid, which the
    # reader above keeps. Measured on one 22,669-line specification: eleven
    # enclosed files held 732 spreadsheet rows and 14 slides that the body
    # text handed its requirements to and no reader had ever opened.
    notes.extend(enclosed.notes)
    lines.extend(enclosed.carried())

    if enclosed.unopened:
        notes.append(
            "enclosed objects that could not be opened: "
            + ", ".join(enclosed.unopened)
            + " -- when a document hands a requirement to an attachment, the "
              "decision logic is in there and not in the text. This is the "
              "loss that looks most like success."
        )
    if enclosed.rows:
        notes.append(
            f"{len(enclosed.rows)} enclosed file(s) WERE opened and their contents "
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
        silent, showing, attributed, drawn, numbered = _clauses_carried_by_a_picture(body)
        # ⚠ The reassurance this note used to end on -- "a picture is usually what a screen
        # LOOKS like rather than what decides it", and "every clause that shows a picture also
        # states something in text" -- was measured true of one corpus and was false of the next
        # specification read: a timing example that decides when an event fires was a drawing,
        # in a clause whose table was text. A reader that says the text is enough, when it has
        # not seen the picture, is the failure this module exists to prevent. What it can say
        # is where each picture is, by a name that can be asked for.
        marked = sorted({name for para in body.iter(f"{_W}p")
                         for name in picture_names(para, pictured, enclosed.attached)})
        if marked:
            placed = (f" {len(marked)} of them sit in the body and are named where they sit, as"
                      " [picture: NAME]; look at one with the `picture` tool.")
            if len(pictures) > len(marked):
                placed += (f" {len(pictures) - len(marked)} more are not in the body"
                           " (a header, a footer) and are not placed.")
        else:
            placed = " None could be named where it sits."
        speaking = sorted({number for number in showing if number not in silent})
        if not numbered:
            where = (" This document has no numbered clauses, so where they"
                     " sit could not be answered.")
        else:
            where = ""
            if silent:
                where += (f" {len(silent)} numbered clause(s) state nothing in text"
                          f" and show only a picture: {_listed(silent)}."
                          " There, reading the text is not reading the clause.")
            if speaking:
                # Counted, not listed: a word document numbers its headings by a field whose
                # number is not in the text, so a number read from the text can be a table row's.
                where += (f" {len(speaking)} numbered clause(s) state something in text"
                          " and also show a picture."
                          " The text does not settle that the picture adds nothing: a clause"
                          " can state a condition in text and leave the rule that decides it"
                          " to the picture beside it (an example it refers to, a timing, a"
                          " range). Look at each picture before relying on the text around it.")
        if drawn and attributed < drawn:
            where += (f" {drawn - attributed} of {drawn} sit outside the"
                      " numbering and were not placed.")
        if marked and len(marked) == len(pictures):
            concerns[len(notes)] = tuple(marked)
        notes.append(
            f"{len(pictures)} picture(s) were not read"
            " -- this reader makes no model calls, so it cannot say what a picture"
            " shows; it says where each one is." + placed + where
        )

    kept = [(index, line) for index, line in enumerate(lines) if line.strip()]
    text = "\n".join(line for _, line in kept)
    if not text.strip():
        raise IngestError(
            f"{path}: read as a document and produced no text. An empty "
            f"document answers every question cleanly, which is "
            f"indistinguishable from one that says everything."
        )
    said: dict[int, int] | None = None
    if headings:
        # The headings are said by the line they are on in the text as it is written: empty lines are not in
        # it, and a paragraph whose own text breaks across lines takes as many as it breaks into.
        said, at = {}, 0
        for index, line in kept:
            if index in heading_at:
                said[at] = heading_at[index]
            at += len(line.splitlines()) or 1
    return Ingested(text + "\n", "docx", notes, said, concerns)


# ------------------------------------------------------------------ dispatch

def read_docx_with_headings(path: pathlib.Path) -> Ingested:
    """A word document as text that also says which of its lines are headings (what an extraction reads)."""
    return _read_docx(path, headings=True)


def read_html_with_headings(path: pathlib.Path) -> Ingested:
    """An HTML document as text that also says which of its lines are headings (what an extraction reads)."""
    return _read_html(path, headings=True)


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


# ------------------------------------------------------- a picture, handed over


@dataclass(frozen=True)
class PictureFile:
    """One picture a specification names, as bytes, and whether a client can be shown it."""

    source: pathlib.Path  # the specification file it was found with
    name: str  # the NAME of its `[picture: NAME]` mark
    mime: str | None  # None: a format that cannot be shown (a metafile, a bitmap)
    data: bytes


def _plain_name(name: str) -> str:
    """A picture is asked for by the name in its mark, and a name has no folder in it.

    A path here would turn a request for a picture into a request for any file the server can read.
    """
    # A name is its own base name under both separators, whichever system the file was written on.
    if (not isinstance(name, str) or not name or name in (".", "..") or "\x00" in name
            or posixpath.basename(name) != name or pathlib.PureWindowsPath(name).name != name):
        raise IngestError(
            f"{name!r} is not the name of a picture. Ask for the NAME in a `[picture: NAME]` mark, "
            "which is a file name and has no folder in it.")
    return name


def read_pictures(paths: Iterable[pathlib.Path], name: str,
                  limit: int = MAX_PICTURE_BYTES) -> list[PictureFile]:
    """The picture marked `[picture: NAME]`, from every specification file that holds one by that name.

    A word document holds its pictures under their names. A plain-text specification (a slice of a
    longer one, say) names a picture it cannot hold, and the picture is the file of that name in the
    SAME folder. Several files may hold a name: one name is not one picture across documents, so every
    holder answers and each says which file it came from.

    An empty answer is not an error here; the caller knows which files it asked and says what they hold.
    A picture heavier than `limit` is refused by name and size, not cut: half an image is a different one.
    """
    name = _plain_name(name)
    suffix = pathlib.PurePosixPath(name).suffix.lower()
    found: list[PictureFile] = []
    for given in paths:
        path = pathlib.Path(given)
        if not path.is_file():
            raise IngestError(describe_path(path))
        if path.suffix.lower() == ".docx":
            try:
                with zipfile.ZipFile(path) as zf:
                    part = _MEDIA + name
                    if part not in zf.namelist():
                        continue
                    size = zf.getinfo(part).file_size
                    if size > limit:
                        raise IngestError(
                            f"{path}: the picture {name} is {size} bytes and the most handed over is "
                            f"{limit}. Open it from the document itself.")
                    data = zf.read(part)
            except zipfile.BadZipFile as exc:
                raise IngestError(f"{path}: not a readable document ({exc})") from exc
        else:
            beside = path.parent / name
            # A link that leaves the folder would make "the picture beside the specification" any
            # file the server can read.
            if not beside.is_file() or beside.resolve().parent != path.parent.resolve():
                continue
            size = beside.stat().st_size
            if size > limit:
                raise IngestError(
                    f"{beside}: the picture {name} is {size} bytes and the most handed over is {limit}.")
            data = beside.read_bytes()
        found.append(PictureFile(path, name, SHOWABLE.get(suffix), data))
    return found


def picture_names_held(paths: Iterable[pathlib.Path]) -> dict[pathlib.Path, list[str]]:
    """The pictures each WORD document holds, by name, for saying what could have been asked for.

    A plain-text file holds none by itself (its pictures are files beside it), so it has no entry.
    """
    held: dict[pathlib.Path, list[str]] = {}
    for given in paths:
        path = pathlib.Path(given)
        if path.suffix.lower() != ".docx" or not path.is_file():
            continue
        try:
            with zipfile.ZipFile(path) as zf:
                held[path] = sorted(posixpath.basename(n) for n in _media_files(zf.namelist()))
        except zipfile.BadZipFile:
            continue
    return held
