# SCE-MAP: codec_peek_arm_b:13 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from sce_forge_runtime.bounded import BoundedInt

from dataclasses import dataclass
from typing import Optional, Protocol


class CodecPeekArmBHeaderInt(BoundedInt):
    """The ``header`` element of :class:`CodecPeekArmB`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


class CodecPeekArmBPayloadInt(BoundedInt):
    """The ``payload`` element of :class:`CodecPeekArmB`, an unsigned integer of
    16 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 16
    SIGNED = False


class CodecPeekArmBSource(Protocol):
    """The elements a value of :class:`CodecPeekArmB` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def header(self) -> CodecPeekArmBHeaderInt: ...

    @property
    def payload(self) -> CodecPeekArmBPayloadInt: ...


def encode_source(src: CodecPeekArmBSource, w: SceSink, *, override_header_kind: Optional[int] = None) -> None:
    """Write any source of :class:`CodecPeekArmB` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). A chain that holds it
    sets the flag its entries end on by its keyword, in place of the value's. It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _c_header = src.header
    if override_header_kind is not None:
        _c_header = (_c_header & ~0x1) | ((override_header_kind & 0x1) << 0)
    w.write_u8(_c_header & 0xFF)
    w.write_u8((src.payload >> 8) & 0xFF)
    w.write_u8(src.payload & 0xFF)


@dataclass
class CodecPeekArmB:
    header: CodecPeekArmBHeaderInt = 0x01
    payload: CodecPeekArmBPayloadInt = 0

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "header":
            value = CodecPeekArmBHeaderInt(value)
        elif name == "payload":
            value = CodecPeekArmBPayloadInt(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor) -> CodecPeekArmB:
        """Decode the next frame from ``cursor``; on success the cursor
        advances past the consumed bytes. A frame the codec refuses raises
        the ``CodecError`` subclass that names the refusal (docs/adr/0099,
        amendment of 2026-11-16): ``NeedMoreBytes`` when the cursor's tail
        is shorter than the frame (RFC §synth-5-B L494-519), which the
        caller resumes after appending bytes, and for a frame no more bytes
        make whole ``VleWidthOverflow``, ``InvalidUtf8``,
        ``UndeclaredEnumValue``, or a length or a count past its bound
        (``LengthOutOfRange``, ``TooManyElements``, docs/adr/0077)."""
        raw = cursor.peek_slice(3)
        f_header = raw[0]
        f_payload = (raw[1] << 8) | raw[2]
        value = cls(
            header=f_header,
            payload=f_payload,
        )
        cursor.advance(3)
        return value

    # RFC §synth-5-B flags primitive: per-bit-range accessors over
    # the carrier field. Single-bit (width=1) reads as bool; multi-bit
    # (width>=2) reads as ``int`` (Python ints are unbounded, so a single
    # ``int`` covers every result-type width). Setters mask + shift on
    # the way in so out-of-range callers can't corrupt sibling bits.
    # Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Wire layout is unchanged.
    def kind(self) -> bool:
        return (self.header & 0x01) != 0

    def set_kind(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x01) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x01)

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
