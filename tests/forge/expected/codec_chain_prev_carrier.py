# SCE-MAP: codec_chain_prev_carrier:18 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink, TooManyElements
from sce_forge_runtime.bounded import BoundedInt, BoundedTuple
from sce_forge_runtime.law import AfterKey, Lawful, Uniform
from .codec_chain_prev_entry import CodecChainPrevEntry
from . import codec_chain_prev_entry as sce_codec_chain_prev_entry

from dataclasses import dataclass, field
from typing import Optional, List, Protocol


class CodecChainPrevCarrierHeadInt(BoundedInt):
    """The ``head`` element of :class:`CodecChainPrevCarrier`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


class CodecChainPrevCarrierEntriesList(BoundedTuple):
    """The ``entries`` element of :class:`CodecChainPrevCarrier`, 1 to 4 entries,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 1
    MOST = 4


class CodecChainPrevCarrierEntriesHas0x4Key:
    """The kind of entry of ``entries`` of :class:`CodecChainPrevCarrier` a gate or a
    law asks for: an entry whose identifier is 0x4 (docs/adr/0065 §6 R6)."""

    @staticmethod
    def matches(entry: sce_codec_chain_prev_entry.CodecChainPrevEntrySource) -> bool:
        """Whether ``entry`` is of the kind."""
        return ((entry.header >> 0) & 0xF) == 0x4


CodecChainPrevCarrierEntriesLaw = (Uniform(sce_codec_chain_prev_entry.input_tagged), AfterKey(sce_codec_chain_prev_entry.input_after_marker, CodecChainPrevCarrierEntriesHas0x4Key.matches))
"""The law the entries of ``entries`` of :class:`CodecChainPrevCarrier` follow in the inputs
they say, an automaton for each, which the list holds them to (docs/adr/0065 §6 R8)."""


class CodecChainPrevCarrierEntriesLawful(Lawful):
    """The ``entries`` element of :class:`CodecChainPrevCarrier`, its entries held to their
    law from the start the value gives (docs/adr/0065 §6 R8)."""

    ENTRIES = CodecChainPrevCarrierEntriesList
    LAW = CodecChainPrevCarrierEntriesLaw


class CodecChainPrevCarrierSource(Protocol):
    """The elements a value of :class:`CodecChainPrevCarrier` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def head(self) -> CodecChainPrevCarrierHeadInt: ...

    @property
    def entries(self) -> CodecChainPrevCarrierEntriesLawful: ...


def encode_source(src: CodecChainPrevCarrierSource, w: SceSink) -> None:
    """Write any source of :class:`CodecChainPrevCarrier` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _c_head = src.head
    _c_head = (_c_head & ~0x1) | ((int(src.entries.start[0])) << 0)
    w.write_u8(_c_head & 0xFF)
    _left_entries = len(src.entries)
    for _entry in src.entries:
        _left_entries -= 1
        sce_codec_chain_prev_entry.encode_source(_entry, w, override_header_z=1 if _left_entries != 0 else 0)


@dataclass
class CodecChainPrevCarrier:
    head: CodecChainPrevCarrierHeadInt = 0
    entries: CodecChainPrevCarrierEntriesLawful = field(default_factory=lambda: CodecChainPrevCarrierEntriesLawful.of(sce_codec_chain_prev_entry.CodecChainPrevEntry() for _ in range(1)))

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "head":
            value = CodecChainPrevCarrierHeadInt(value)
        elif name == "entries":
            value = CodecChainPrevCarrierEntriesLawful(value.start, value.entries)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor) -> CodecChainPrevCarrier:
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
        f_head = raw[0]
        cursor.advance(1)
        f_entries = []
        _prev_entries_after_marker = 0
        _more = False
        for _ in range(4):
            _elem = CodecChainPrevEntry.decode(cursor, _prev_entries_after_marker, ((f_head >> 0) & 0x1))
            _more = _elem.z()
            _prev_entries_after_marker = 1 if _elem.ext_id() == 4 else 0
            f_entries.append(_elem)
            if not _more:
                break
        if _more and cursor.remaining() == 0:
            raise NeedMoreBytes()
        if _more:
            raise TooManyElements()
        return cls(
            head=f_head,
            entries=CodecChainPrevCarrierEntriesLawful(((((f_head >> 0) & 1) != 0), None), f_entries),
        )

    # RFC §synth-5-B flags primitive: per-bit-range accessors over
    # the carrier field. Single-bit (width=1) reads as bool; multi-bit
    # (width>=2) reads as ``int`` (Python ints are unbounded, so a single
    # ``int`` covers every result-type width). Setters mask + shift on
    # the way in so out-of-range callers can't corrupt sibling bits.
    # Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Wire layout is unchanged.
    def t(self) -> bool:
        return (self.head & 0x01) != 0

    def set_t(self, v: bool) -> None:
        if v:
            self.head = (self.head | 0x01) & 0xFF
        else:
            self.head = self.head & (0xFF ^ 0x01)

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
