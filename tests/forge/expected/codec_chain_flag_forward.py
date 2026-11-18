# SCE-MAP: codec_chain_flag_forward:12 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink, TooManyElements
from sce_forge_runtime.bounded import BoundedInt, BoundedTuple
from sce_forge_runtime.law import Lawful, Uniform
from .codec_chain_flag_entry import CodecChainFlagEntry
from . import codec_chain_flag_entry as sce_codec_chain_flag_entry

from dataclasses import dataclass, field
from typing import Optional, List, Protocol


class CodecChainFlagForwardHeadInt(BoundedInt):
    """The ``head`` element of :class:`CodecChainFlagForward`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


class CodecChainFlagForwardEntriesList(BoundedTuple):
    """The ``entries`` element of :class:`CodecChainFlagForward`, 1 to 4 entries,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 1
    MOST = 4


CodecChainFlagForwardEntriesLaw = (Uniform(sce_codec_chain_flag_entry.input_wide),)
"""The law the entries of ``entries`` of :class:`CodecChainFlagForward` follow in the inputs
they say, an automaton for each, which the list holds them to (docs/adr/0065 §6 R8)."""


class CodecChainFlagForwardEntriesLawful(Lawful):
    """The ``entries`` element of :class:`CodecChainFlagForward`, its entries held to their
    law from the start the value gives (docs/adr/0065 §6 R8)."""

    ENTRIES = CodecChainFlagForwardEntriesList
    LAW = CodecChainFlagForwardEntriesLaw


class CodecChainFlagForwardSource(Protocol):
    """The elements a value of :class:`CodecChainFlagForward` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def head(self) -> CodecChainFlagForwardHeadInt: ...

    @property
    def entries(self) -> Optional[CodecChainFlagForwardEntriesLawful]: ...


def encode_source(src: CodecChainFlagForwardSource, w: SceSink) -> None:
    """Write any source of :class:`CodecChainFlagForward` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _c_head = src.head
    _c_head = (_c_head & ~0x1) | (0x1 if src.entries is not None else 0)
    w.write_u8(_c_head & 0xFF)
    if src.entries is not None:
        _left_entries = len(src.entries)
        for _entry in src.entries:
            _left_entries -= 1
            sce_codec_chain_flag_entry.encode_source(_entry, w, override_header_z=1 if _left_entries != 0 else 0)


def input_wide(src: CodecChainFlagForwardSource) -> Optional[int]:
    """The flag input ``wide`` a source of :class:`CodecChainFlagForward` says, which
    its encode is not handed (docs/adr/0065 §6 R4a),
    or ``None`` where the value says nothing of it."""
    return (None if src.entries is None else int(src.entries.start))


@dataclass
class CodecChainFlagForward:
    head: CodecChainFlagForwardHeadInt = 0
    entries: Optional[CodecChainFlagForwardEntriesLawful] = None

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "head":
            value = CodecChainFlagForwardHeadInt(value)
        elif name == "entries":
            value = None if value is None else CodecChainFlagForwardEntriesLawful(value.start, value.entries)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor, wide: int) -> CodecChainFlagForward:
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
        if (f_head & 0x01) != 0:
            f_entries = []
            _more = False
            for _ in range(4):
                _elem = CodecChainFlagEntry.decode(cursor, wide)
                _more = _elem.z()
                f_entries.append(_elem)
                if not _more:
                    break
            if _more and cursor.remaining() == 0:
                raise NeedMoreBytes()
            if _more:
                raise TooManyElements()
        else:
            f_entries = None
        return cls(
            head=f_head,
            entries=None if f_entries is None else CodecChainFlagForwardEntriesLawful((wide != 0), f_entries),
        )

    # RFC §synth-5-B flags primitive: per-bit-range accessors over
    # the carrier field. Single-bit (width=1) reads as bool; multi-bit
    # (width>=2) reads as ``int`` (Python ints are unbounded, so a single
    # ``int`` covers every result-type width). Setters mask + shift on
    # the way in so out-of-range callers can't corrupt sibling bits.
    # Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Wire layout is unchanged.
    def e(self) -> bool:
        return (self.head & 0x01) != 0

    def set_e(self, v: bool) -> None:
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
