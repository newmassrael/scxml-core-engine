# SCE-MAP: codec_flags_basic:7 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from sce_forge_runtime.bounded import BoundedInt

from dataclasses import dataclass
from typing import Optional, Protocol


class CodecFlagsBasicHeaderInt(BoundedInt):
    """The ``header`` element of :class:`CodecFlagsBasic`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


class CodecFlagsBasicSource(Protocol):
    """The elements a value of :class:`CodecFlagsBasic` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def header(self) -> CodecFlagsBasicHeaderInt: ...


def encode_source(src: CodecFlagsBasicSource, w: SceSink, *, override_header_reliable: Optional[int] = None, override_header_more: Optional[int] = None, override_header_drop: Optional[int] = None, override_header_first: Optional[int] = None) -> None:
    """Write any source of :class:`CodecFlagsBasic` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). A chain that holds it
    sets the flag its entries end on by its keyword, in place of the value's. It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _c_header = src.header
    if override_header_reliable is not None:
        _c_header = (_c_header & ~0x80) | ((override_header_reliable & 0x1) << 7)
    if override_header_more is not None:
        _c_header = (_c_header & ~0x40) | ((override_header_more & 0x1) << 6)
    if override_header_drop is not None:
        _c_header = (_c_header & ~0x20) | ((override_header_drop & 0x1) << 5)
    if override_header_first is not None:
        _c_header = (_c_header & ~0x10) | ((override_header_first & 0x1) << 4)
    w.write_u8(_c_header & 0xFF)


@dataclass
class CodecFlagsBasic:
    header: CodecFlagsBasicHeaderInt = 0

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "header":
            value = CodecFlagsBasicHeaderInt(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor) -> CodecFlagsBasic:
        """Decode the next frame from ``cursor``; on success the cursor
        advances past the consumed bytes. A frame the codec refuses raises
        the ``CodecError`` subclass that names the refusal (docs/adr/0099,
        amendment of 2026-11-16): ``NeedMoreBytes`` when the cursor's tail
        is shorter than the frame (RFC §synth-5-B L494-519), which the
        caller resumes after appending bytes, and for a frame no more bytes
        make whole ``VleWidthOverflow``, ``InvalidUtf8``,
        ``UndeclaredEnumValue``, or a length or a count past its bound
        (``LengthOutOfRange``, ``TooManyElements``, docs/adr/0077)."""
        raw = cursor.peek_slice(1)
        f_header = raw[0]
        value = cls(
            header=f_header,
        )
        cursor.advance(1)
        return value

    # RFC §synth-5-B flags primitive: per-bit-range accessors over
    # the carrier field. Single-bit (width=1) reads as bool; multi-bit
    # (width>=2) reads as ``int`` (Python ints are unbounded, so a single
    # ``int`` covers every result-type width). Setters mask + shift on
    # the way in so out-of-range callers can't corrupt sibling bits.
    # Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Wire layout is unchanged.
    def reliable(self) -> bool:
        return (self.header & 0x80) != 0

    def set_reliable(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x80) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x80)

    def more(self) -> bool:
        return (self.header & 0x40) != 0

    def set_more(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x40) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x40)

    def drop(self) -> bool:
        return (self.header & 0x20) != 0

    def set_drop(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x20) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x20)

    def first(self) -> bool:
        return (self.header & 0x10) != 0

    def set_first(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x10) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x10)

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
