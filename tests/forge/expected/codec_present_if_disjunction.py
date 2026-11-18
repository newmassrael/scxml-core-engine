# SCE-MAP: codec_present_if_disjunction:15 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from sce_forge_runtime.bounded import BoundedInt

from dataclasses import dataclass
from typing import Optional, Protocol, Union


class CodecPresentIfDisjunctionFlagsInt(BoundedInt):
    """The ``flags`` element of :class:`CodecPresentIfDisjunction`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


class CodecPresentIfDisjunctionSeqInt(BoundedInt):
    """The ``seq`` element of :class:`CodecPresentIfDisjunction`, an unsigned integer of
    16 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 16
    SIGNED = False


@dataclass
class CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsA:
    """The ``flags_wants_a_or_wants_b`` group of :class:`CodecPresentIfDisjunction` where ``flags.wants_a`` holds and ``flags.wants_b`` does not hold
    (docs/adr/0065 §6 R4): the members present then."""

    seq: CodecPresentIfDisjunctionSeqInt = 0

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "seq":
            value = CodecPresentIfDisjunctionSeqInt(value)
        object.__setattr__(self, name, value)


@dataclass
class CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsB:
    """The ``flags_wants_a_or_wants_b`` group of :class:`CodecPresentIfDisjunction` where ``flags.wants_a`` does not hold and ``flags.wants_b`` holds
    (docs/adr/0065 §6 R4): the members present then."""

    seq: CodecPresentIfDisjunctionSeqInt = 0

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "seq":
            value = CodecPresentIfDisjunctionSeqInt(value)
        object.__setattr__(self, name, value)


@dataclass
class CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsAWantsB:
    """The ``flags_wants_a_or_wants_b`` group of :class:`CodecPresentIfDisjunction` where ``flags.wants_a`` holds and ``flags.wants_b`` holds
    (docs/adr/0065 §6 R4): the members present then."""

    seq: CodecPresentIfDisjunctionSeqInt = 0

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "seq":
            value = CodecPresentIfDisjunctionSeqInt(value)
        object.__setattr__(self, name, value)


CodecPresentIfDisjunctionFlagsWantsAOrWantsB = Union[CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsA, CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsB, CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsAWantsB]
"""The ``flags_wants_a_or_wants_b`` group of :class:`CodecPresentIfDisjunction`: the arm the value holds."""


class CodecPresentIfDisjunctionSource(Protocol):
    """The elements a value of :class:`CodecPresentIfDisjunction` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def flags(self) -> CodecPresentIfDisjunctionFlagsInt: ...

    @property
    def flags_wants_a_or_wants_b(self) -> Optional[CodecPresentIfDisjunctionFlagsWantsAOrWantsB]: ...


def encode_source(src: CodecPresentIfDisjunctionSource, w: SceSink) -> None:
    """Write any source of :class:`CodecPresentIfDisjunction` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _g_flags_wants_a_or_wants_b = src.flags_wants_a_or_wants_b
    _e_seq = None if _g_flags_wants_a_or_wants_b is None else _g_flags_wants_a_or_wants_b.seq
    _c_flags = src.flags
    _c_flags = (_c_flags & ~0x1) | (0x1 if isinstance(_g_flags_wants_a_or_wants_b, (CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsA, CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsAWantsB)) else 0)
    _c_flags = (_c_flags & ~0x2) | (0x2 if isinstance(_g_flags_wants_a_or_wants_b, (CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsB, CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsAWantsB)) else 0)
    w.write_u8(_c_flags & 0xFF)
    if _e_seq is not None:
        w.write_u8((_e_seq >> 8) & 0xFF)
        w.write_u8(_e_seq & 0xFF)


@dataclass
class CodecPresentIfDisjunction:
    flags: CodecPresentIfDisjunctionFlagsInt = 0
    flags_wants_a_or_wants_b: Optional[CodecPresentIfDisjunctionFlagsWantsAOrWantsB] = None

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "flags":
            value = CodecPresentIfDisjunctionFlagsInt(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor) -> CodecPresentIfDisjunction:
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
        f_flags = raw[0]
        cursor.advance(1)
        if (f_flags & 0x01) != 0 or (f_flags & 0x02) != 0:
            raw = cursor.peek_slice(2)
            _v = (raw[0] << 8) | raw[1]
            cursor.advance(2)
            f_seq = _v
        else:
            f_seq = None
        return cls(
            flags=f_flags,
            flags_wants_a_or_wants_b=CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsA(seq=f_seq) if (((f_flags >> 0) & 1) != 0) and not (((f_flags >> 1) & 1) != 0) else CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsB(seq=f_seq) if not (((f_flags >> 0) & 1) != 0) and (((f_flags >> 1) & 1) != 0) else CodecPresentIfDisjunctionFlagsWantsAOrWantsBWantsAWantsB(seq=f_seq) if (((f_flags >> 0) & 1) != 0) and (((f_flags >> 1) & 1) != 0) else None,
        )

    # RFC §synth-5-B flags primitive: per-bit-range accessors over
    # the carrier field. Single-bit (width=1) reads as bool; multi-bit
    # (width>=2) reads as ``int`` (Python ints are unbounded, so a single
    # ``int`` covers every result-type width). Setters mask + shift on
    # the way in so out-of-range callers can't corrupt sibling bits.
    # Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Wire layout is unchanged.
    def wants_a(self) -> bool:
        return (self.flags & 0x01) != 0

    def set_wants_a(self, v: bool) -> None:
        if v:
            self.flags = (self.flags | 0x01) & 0xFF
        else:
            self.flags = self.flags & (0xFF ^ 0x01)

    def wants_b(self) -> bool:
        return (self.flags & 0x02) != 0

    def set_wants_b(self, v: bool) -> None:
        if v:
            self.flags = (self.flags | 0x02) & 0xFF
        else:
            self.flags = self.flags & (0xFF ^ 0x02)

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
