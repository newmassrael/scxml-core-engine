"""The pictures a PDF page holds: named by their page and order, identified by their own bytes, drawn as PNG.

A PDF does not keep a picture under a file name the way a word document does. What it keeps is an image
object on a page, so a picture here is named by where it sits: `page-0003-image-02.png` is the second image
object of page 3, in the order the page draws them. The name ends in `.png` because that is what is handed
over: the object's own bytes may be a JPEG, a run of raw samples or a fax encoding, and a client is shown
the same format whichever it was.

Two hashes are kept apart (ADR 0032). The picture the extraction names is the member the document holds:
its identity is the hash of the image object's bytes as the document stores them, with the dimensions,
colour space and filters that say how to read them, so it is the same on every machine and in every
version of the library. What is handed over is a rendition of it, a PNG drawn from the decoded samples, and
its own hash is what a reading records as the picture the model saw.

The library is PDFium through `pypdfium2`, the pack of `extractors_pdf.py`; it is imported where it is
used, because the pack may be absent and the registry says so before an extractor runs.
"""

from __future__ import annotations

import hashlib
import pathlib
import re
import struct
import zlib
from dataclasses import dataclass

from .errors import READ_ERRORS, IngestError

# `page-0003-image-02.png`: the page and the place among that page's images, both from 1.
_NAME = re.compile(r"^page-(\d{4,})-image-(\d{2,})\.png$")

# The most pixels one image is decoded to be drawn. A scan at 600 dots to the inch is about 35 million; a
# smaller limit would refuse the commonest scan, and none would let a small file ask for gigabytes.
MAX_PIXELS = 40_000_000

# The bitmap layouts PDFium hands over for an image object, by its constant: gray, BGR, BGR with a spare
# byte, and BGR with alpha. Each is written as the PNG colour type that holds it without loss.
_GRAY, _BGR, _BGRX, _BGRA = 1, 2, 3, 4
_COLOUR_TYPE = {_GRAY: 0, _BGR: 2, _BGRX: 2, _BGRA: 6}

_SIGNATURE = b"\x89PNG\r\n\x1a\n"


def picture_name(page: int, ordinal: int) -> str:
    return f"page-{page:04d}-image-{ordinal:02d}.png"


def parse_name(name: str) -> tuple[int, int] | None:
    """The page and the place on it a picture name stands for, or `None` for a name this is not."""
    found = _NAME.match(name)
    if found is None:
        return None
    page, ordinal = int(found.group(1)), int(found.group(2))
    return (page, ordinal) if page >= 1 and ordinal >= 1 else None


def images_of(page, pdfium) -> list:
    """The image objects of a page in the order it draws them, which is the order the names count in."""
    return list(page.get_objects(filter=[pdfium.raw.FPDF_PAGEOBJ_IMAGE]))


def box_of(image, page_height: float) -> tuple[float, float, float, float]:
    """Where an image sits, as `(left, top, right, bottom)` measured down from the top of the page."""
    # The library renamed `get_pos` to `get_bounds` in its 5th major version; the pack is not pinned.
    bounds = getattr(image, "get_bounds", None) or image.get_pos
    left, bottom, right, top = bounds()
    return (round(left, 2), round(page_height - top, 2), round(right, 2), round(page_height - bottom, 2))


def identity_of(image) -> str | None:
    """The hash of what the document stores for this image, or `None` when it cannot be read.

    The stored bytes alone do not say which picture they are: the same samples are another picture at
    another width, so how to read them is part of what is hashed.
    """
    import pypdfium2 as pdfium

    try:
        meta = image.get_metadata()
        filters = "+".join(image.get_filters())
        stored = bytes(image.get_data())
    except (pdfium.PdfiumError, *READ_ERRORS):
        return None
    head = f"{meta.width}x{meta.height}:{meta.bits_per_pixel}:{meta.colorspace}:{filters}\0"
    return hashlib.sha256(head.encode("ascii") + stored).hexdigest()


def _chunk(tag: bytes, body: bytes) -> bytes:
    return struct.pack(">I", len(body)) + tag + body + struct.pack(">I", zlib.crc32(tag + body))


def _row(raw: bytes, form: int, width: int) -> bytes:
    """One scan line from PDFium's layout into the PNG's: blue and red swapped, a spare byte dropped."""
    if form == _GRAY:
        return raw
    if form == _BGR:
        out = bytearray(len(raw))
        out[0::3], out[1::3], out[2::3] = raw[2::3], raw[1::3], raw[0::3]
        return bytes(out)
    out = bytearray(width * (3 if form == _BGRX else 4))
    step = len(out) // width
    out[0::step], out[1::step], out[2::step] = raw[2::4], raw[1::4], raw[0::4]
    if form == _BGRA:
        out[3::4] = raw[3::4]
    return bytes(out)


def draw(image) -> bytes | None:
    """The image as a PNG, or `None` when it cannot be drawn: no decoder for it, a layout this does not know,
    or more pixels than a reader holds.

    The samples are the image's own, before the page's matrix scales or turns them and before a mask is laid
    over them, so what is handed over is the picture and not where it was set.
    """
    import pypdfium2 as pdfium

    try:
        meta = image.get_metadata()
        if not 0 < meta.width * meta.height <= MAX_PIXELS:
            return None
        bitmap = image.get_bitmap(render=False)
    except (pdfium.PdfiumError, *READ_ERRORS):
        return None
    try:
        form = bitmap.format
        if form not in _COLOUR_TYPE:
            return None
        width, height, stride, channels = bitmap.width, bitmap.height, bitmap.stride, bitmap.n_channels
        samples = memoryview(bitmap.buffer).cast("B")
        deflate = zlib.compressobj(6)
        parts = [
            deflate.compress(b"\x00" + _row(bytes(samples[y * stride : y * stride + width * channels]), form, width))
            for y in range(height)
        ]
        parts.append(deflate.flush())
    finally:
        bitmap.close()
    header = struct.pack(">IIBBBBB", width, height, 8, _COLOUR_TYPE[form], 0, 0, 0)
    return _SIGNATURE + _chunk(b"IHDR", header) + _chunk(b"IDAT", b"".join(parts)) + _chunk(b"IEND", b"")


@dataclass(frozen=True)
class Drawn:
    """A picture of a PDF as it is handed over: its PNG (`None` when it cannot be drawn) and its identity."""

    png: bytes | None
    identity: str | None


def _open(path: pathlib.Path):
    try:
        import pypdfium2 as pdfium
    except ImportError as exc:
        raise IngestError(
            f"{path}: a picture in a PDF is drawn by the pack pdf-text (pip install pypdfium2), which is not installed"
        ) from exc
    try:
        return pdfium, pdfium.PdfDocument(str(path))
    except (pdfium.PdfiumError, *READ_ERRORS) as exc:
        raise IngestError(f"{path}: not a readable PDF ({exc})") from exc


def draw_named(path: pathlib.Path, name: str) -> Drawn | None:
    """The picture a PDF holds under `name`, or `None` when the document has no such page or image."""
    where = parse_name(name)
    if where is None:
        return None
    number, ordinal = where
    pdfium, document = _open(path)
    try:
        if number > len(document):
            return None
        page = document[number - 1]
        try:
            images = images_of(page, pdfium)
            if ordinal > len(images):
                return None
            return Drawn(draw(images[ordinal - 1]), identity_of(images[ordinal - 1]))
        finally:
            page.close()
    finally:
        document.close()


# The folder, beside a PDF in its bundle, that holds the renditions made of its pictures (ADR 0032): a
# picture named `NAME` that is a file there is the rendition and is handed over as it is, so that what a
# reading saw is kept and a machine without the pack can still be shown it.
RENDITIONS = "renditions"

# The most pictures one document is rendered for, and the most they weigh together. A document with more is
# not refused: the rest are not rendered, are said, and are drawn from the document when they are read.
MAX_RENDERED = 1000
MAX_RENDERED_BYTES = 128 * 1024 * 1024


def renderer() -> str:
    """What draws a rendition, with its version, in the words a bundle records it by."""
    import pypdfium2 as pdfium

    return f"pdfium {pdfium.version.PDFIUM_INFO}; pypdfium2 {pdfium.version.PYPDFIUM_INFO}"


@dataclass(frozen=True)
class Rendered:
    """A picture of a PDF drawn to be kept: its name, its PNG, and the hash of the PNG."""

    name: str
    png: bytes
    sha256: str


def render_all(
    path: pathlib.Path, limit: int, most: int | None = None, heaviest: int | None = None
) -> tuple[list[Rendered], list[tuple[str, str]]]:
    """Every picture a PDF holds that can be drawn and handed over, and the ones that were not, with why.

    A picture is not rendered when it cannot be drawn, when its PNG is heavier than `limit` (a picture is
    refused and not cut, as it is when it is read), or when the pictures already rendered are `most` or weigh
    `heaviest`. None of those is an error: the picture is drawn from the document if it is ever read.
    """
    most = MAX_RENDERED if most is None else most
    heaviest = MAX_RENDERED_BYTES if heaviest is None else heaviest
    pdfium, document = _open(path)
    rendered: list[Rendered] = []
    skipped: list[tuple[str, str]] = []
    weighed = 0
    try:
        for number in range(1, len(document) + 1):
            page = document[number - 1]
            try:
                for ordinal, image in enumerate(images_of(page, pdfium), 1):
                    name = picture_name(number, ordinal)
                    if len(rendered) >= most:
                        skipped.append((name, "past the most pictures one document is rendered for"))
                        continue
                    png = draw(image)
                    if png is None:
                        skipped.append((name, "it cannot be drawn"))
                    elif len(png) > limit:
                        skipped.append((name, f"it is {len(png)} bytes drawn and the most handed over is {limit}"))
                    elif weighed + len(png) > heaviest:
                        skipped.append((name, "past the most the renditions of one document weigh"))
                    else:
                        weighed += len(png)
                        rendered.append(Rendered(name, png, hashlib.sha256(png).hexdigest()))
            finally:
                page.close()
    finally:
        document.close()
    return rendered, skipped


def names_in(path: pathlib.Path) -> list[str]:
    """Every picture name a PDF holds, in page order, for saying what could have been asked for."""
    pdfium, document = _open(path)
    try:
        names = []
        for number in range(1, len(document) + 1):
            page = document[number - 1]
            try:
                names.extend(picture_name(number, i) for i in range(1, len(images_of(page, pdfium)) + 1))
            finally:
                page.close()
        return names
    finally:
        document.close()
