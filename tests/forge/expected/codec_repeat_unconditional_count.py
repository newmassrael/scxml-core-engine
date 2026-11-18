# SCE-MAP: codec_repeat_unconditional_count:34 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink, TooManyElements
from sce_forge_runtime.bounded import Aligned, BoundedInt, BoundedTuple
from .codec_repeat_elem import CodecRepeatElem
from . import codec_repeat_elem as sce_codec_repeat_elem

from dataclasses import dataclass, field
from typing import Optional, List, Protocol, Union


class CodecRepeatUnconditionalCountOptionsInt(BoundedInt):
    """The ``options`` element of :class:`CodecRepeatUnconditionalCount`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


class CodecRepeatUnconditionalCountLinksList(BoundedTuple):
    """The ``links`` element of :class:`CodecRepeatUnconditionalCount`, 0 to 64 entries,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 0
    MOST = 64


class CodecRepeatUnconditionalCountWeightsList(BoundedTuple):
    """The ``weights`` element of :class:`CodecRepeatUnconditionalCount`, 0 to 64 entries,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 0
    MOST = 64


@dataclass
class CodecRepeatUnconditionalCountOptionsHH:
    """The ``options_h`` group of :class:`CodecRepeatUnconditionalCount` where ``options.H`` holds
    (docs/adr/0065 §6 R4): the members present then."""

    links_weights: Aligned[CodecRepeatUnconditionalCountLinksList, CodecRepeatUnconditionalCountWeightsList] = Aligned(CodecRepeatUnconditionalCountLinksList(), CodecRepeatUnconditionalCountWeightsList())

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "links_weights":
            value = Aligned(CodecRepeatUnconditionalCountLinksList(value.first), CodecRepeatUnconditionalCountWeightsList(value.second))
        object.__setattr__(self, name, value)


@dataclass
class CodecRepeatUnconditionalCountOptionsHNotH:
    """The ``options_h`` group of :class:`CodecRepeatUnconditionalCount` where ``options.H`` does not hold
    (docs/adr/0065 §6 R4): the members present then."""

    links: CodecRepeatUnconditionalCountLinksList = CodecRepeatUnconditionalCountLinksList()

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "links":
            value = CodecRepeatUnconditionalCountLinksList(value)
        object.__setattr__(self, name, value)


CodecRepeatUnconditionalCountOptionsH = Union[CodecRepeatUnconditionalCountOptionsHH, CodecRepeatUnconditionalCountOptionsHNotH]
"""The ``options_h`` group of :class:`CodecRepeatUnconditionalCount`: the arm the value holds."""


class CodecRepeatUnconditionalCountSource(Protocol):
    """The elements a value of :class:`CodecRepeatUnconditionalCount` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def options(self) -> CodecRepeatUnconditionalCountOptionsInt: ...

    @property
    def options_h(self) -> CodecRepeatUnconditionalCountOptionsH: ...


def encode_source(src: CodecRepeatUnconditionalCountSource, w: SceSink) -> None:
    """Write any source of :class:`CodecRepeatUnconditionalCount` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It derives ``links_len`` from what each measures, and never reads it off
    the source. It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _g_options_h = src.options_h
    _e_links = _g_options_h.links_weights.first if isinstance(_g_options_h, CodecRepeatUnconditionalCountOptionsHH) else _g_options_h.links if isinstance(_g_options_h, CodecRepeatUnconditionalCountOptionsHNotH) else None
    _e_weights = _g_options_h.links_weights.second if isinstance(_g_options_h, CodecRepeatUnconditionalCountOptionsHH) else None
    _d_links_len = len(_e_links)
    _c_options = src.options
    _c_options = (_c_options & ~0x1) | (0x1 if _e_weights is not None else 0)
    w.write_u8(_c_options & 0xFF)
    w.write_u8(_d_links_len & 0xFF)
    if _e_links is not None:
        for _entry in _e_links:
            sce_codec_repeat_elem.encode_source(_entry, w)
    if _e_weights is not None:
        for _entry in _e_weights:
            sce_codec_repeat_elem.encode_source(_entry, w)


@dataclass
class CodecRepeatUnconditionalCount:
    options: CodecRepeatUnconditionalCountOptionsInt = 0
    options_h: CodecRepeatUnconditionalCountOptionsH = field(default_factory=CodecRepeatUnconditionalCountOptionsHNotH)

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "options":
            value = CodecRepeatUnconditionalCountOptionsInt(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor) -> CodecRepeatUnconditionalCount:
        """Decode the next frame from ``cursor``; on success the cursor
        advances past the consumed bytes. A frame the codec refuses raises
        the ``CodecError`` subclass that names the refusal (docs/adr/0099,
        amendment of 2026-11-16): ``NeedMoreBytes`` when the cursor's tail
        is shorter than the frame (RFC §synth-5-B L494-519), which the
        caller resumes after appending bytes, and for a frame no more bytes
        make whole ``VleWidthOverflow``, ``InvalidUtf8``,
        ``UndeclaredEnumValue``, or a length or a count past its bound
        (``LengthOutOfRange``, ``TooManyElements``, docs/adr/0077)."""
        # Streaming cursor decode (SSOT selection: `needs_streaming`).
        # The positional `raw[byte_off]` path is valid only when every
        # field's absolute offset is fixed at codegen time; this branch
        # handles every codec where it is not — present-if-gated fields
        # (runtime presence), VLE / repeat / TLV-chain / embed fields
        # (runtime width), string fields (UTF-8 decode), and a fixed field
        # after a variable-length payload (offset depends on the payload
        # length). Each field reads its own bytes and advances past what it
        # consumed; a refusal raises the `CodecError` that names it.
        raw = cursor.peek_slice(1)
        f_options = raw[0]
        cursor.advance(1)
        raw = cursor.peek_slice(1)
        f_links_len = raw[0]
        cursor.advance(1)
        if f_links_len > 64:
            raise TooManyElements()
        f_links = []
        for _ in range(f_links_len):
            f_links.append(CodecRepeatElem.decode(cursor))
        if (f_options & 0x01) != 0:
            if f_links_len > 64:
                raise TooManyElements()
            f_weights = []
            for _ in range(f_links_len):
                f_weights.append(CodecRepeatElem.decode(cursor))
        else:
            f_weights = None
        return cls(
            options=f_options,
            options_h=CodecRepeatUnconditionalCountOptionsHH(links_weights=Aligned(f_links, f_weights)) if (((f_options >> 0) & 1) != 0) else CodecRepeatUnconditionalCountOptionsHNotH(links=f_links),
        )

    # RFC §synth-5-B flags primitive: per-bit-range accessors over
    # the carrier field. Single-bit (width=1) reads as bool; multi-bit
    # (width>=2) reads as ``int`` (Python ints are unbounded, so a single
    # ``int`` covers every result-type width). Setters mask + shift on
    # the way in so out-of-range callers can't corrupt sibling bits.
    # Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Wire layout is unchanged.
    def h(self) -> bool:
        return (self.options & 0x01) != 0

    def set_h(self, v: bool) -> None:
        if v:
            self.options = (self.options | 0x01) & 0xFF
        else:
            self.options = self.options & (0xFF ^ 0x01)

    def encode(self, w: SceSink) -> None:
        """RFC §synth-5-B encode-side primary: write ``self`` into the
        caller-owned ``w`` sink. Returns ``None`` on success; raises
        :class:`BufferOverflow` from a bounded sink when the destination
        has insufficient remaining capacity; growable sinks (e.g.
        :class:`BytearraySink`) are effectively infallible."""
        encode_source(self, w)

    def encode_to_bytes(self) -> bytes:
        """Heap-backed convenience facade. Runs :meth:`encode` over a
        :class:`BytearraySink` and returns the freshly-encoded bytes.
        Callers targeting zero-alloc hot paths should call :meth:`encode`
        directly against a caller-owned sink."""
        _dst = bytearray()
        self.encode(BytearraySink(_dst))
        return bytes(_dst)
