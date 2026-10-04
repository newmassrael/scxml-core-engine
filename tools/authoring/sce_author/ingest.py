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


_O = "{urn:schemas-microsoft-com:office:office}"
_DOC_R = "{http://schemas.openxmlformats.org/officeDocument/2006/relationships}"


def _docx_text(node, attached: dict[str, str]) -> str:
    """A paragraph's text, and where an enclosed file was attached in it.

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
    """
    return " ".join(part for part in (_docx_paragraph(node), *enclosed_marks(node, attached)) if part)


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
        root = ET.parse(io.BytesIO(archive.read(declared))).getroot()
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
            root = ET.parse(io.BytesIO(book.read("xl/workbook.xml"))).getroot()
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
            root = ET.parse(io.BytesIO(book.read(part))).getroot()
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
        metadata, values, structures = (ET.parse(io.BytesIO(book.read(p))).getroot() for p in parts)
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
        inner = set(book.namelist())
        shared: list[str] = []
        if "xl/sharedStrings.xml" in inner:
            root = ET.parse(io.BytesIO(book.read("xl/sharedStrings.xml"))).getroot()
            shared = ["".join(t.text or "" for t in si.iter(f"{_XL}t"))
                      for si in root.iter(f"{_XL}si")]
        lines: list[str] = []
        pictures = cells_with_pictures = loose = other = unreadable = rich = 0
        unreadable_on: list[str] = []
        in_cell = _picture_values(book, inner)
        for name, sheet, hidden in _sheets_in_order(book, inner):
            root = ET.parse(io.BytesIO(book.read(sheet))).getroot()
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
                        cells[_column(cell.get("r") or "")] = text.strip()
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
        opened, trouble = (_read_enclosed_sheet(archive.read(part)) if part.lower().endswith(".xlsx")
                           else _read_enclosed_deck(archive.read(part)))
        notes.extend(f"{name}: {t}" for t in trouble)
        if opened:
            rows[name] = opened
    unopened = [names[part] for part in embedded if names[part] not in rows]
    return Enclosed(attached, rows, unopened, notes)


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
            pictures = sorted(n for n in names if n.startswith("word/media/"))
            enclosed = read_enclosed(zf)
    except zipfile.BadZipFile as exc:
        raise IngestError(f"{path}: not a readable document ({exc})") from exc

    lines: list[str] = []
    for node in body.iter():
        if node.tag == f"{_W}p":
            # A paragraph inside a table cell is emitted by the table branch.
            lines.append(_docx_text(node, enclosed.attached))
        elif node.tag == f"{_W}tbl":
            for tr in node.iter(f"{_W}tr"):
                cells = [
                    " ".join(_docx_text(p, enclosed.attached) for p in tc.iter(f"{_W}p")).strip()
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
