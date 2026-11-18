# SCE-MAP: codec_length_ref_dotted_basic:27 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from sce_forge_runtime.bounded import BoundedBytes, BoundedInt

from dataclasses import dataclass
from typing import Optional, Protocol


class CodecLengthRefDottedBasicCarrierInt(BoundedInt):
    """The ``carrier`` element of :class:`CodecLengthRefDottedBasic`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


class CodecLengthRefDottedBasicPayloadBytes(BoundedBytes):
    """The ``payload`` element of :class:`CodecLengthRefDottedBasic`, 0 to 15 bytes,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 0
    MOST = 15


class CodecLengthRefDottedBasicSource(Protocol):
    """The elements a value of :class:`CodecLengthRefDottedBasic` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def carrier(self) -> CodecLengthRefDottedBasicCarrierInt: ...

    @property
    def payload(self) -> CodecLengthRefDottedBasicPayloadBytes: ...


def encode_source(src: CodecLengthRefDottedBasicSource, w: SceSink) -> None:
    """Write any source of :class:`CodecLengthRefDottedBasic` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _c_carrier = src.carrier
    _c_carrier = (_c_carrier & ~0xF0) | ((len(src.payload)) << 4)
    w.write_u8(_c_carrier & 0xFF)
    w.write_bytes(src.payload)


@dataclass
class CodecLengthRefDottedBasic:
    carrier: CodecLengthRefDottedBasicCarrierInt = 0
    payload: CodecLengthRefDottedBasicPayloadBytes = CodecLengthRefDottedBasicPayloadBytes()

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "carrier":
            value = CodecLengthRefDottedBasicCarrierInt(value)
        elif name == "payload":
            value = CodecLengthRefDottedBasicPayloadBytes(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor) -> CodecLengthRefDottedBasic:
        """Decode the next frame from ``cursor``; on success the cursor
        advances past the consumed bytes. A frame the codec refuses raises
        the ``CodecError`` subclass that names the refusal (docs/adr/0099,
        amendment of 2026-11-16): ``NeedMoreBytes`` when the cursor's tail
        is shorter than the frame (RFC §synth-5-B L494-519), which the
        caller resumes after appending bytes, and for a frame no more bytes
        make whole ``VleWidthOverflow``, ``InvalidUtf8``,
        ``UndeclaredEnumValue``, or a length or a count past its bound
        (``LengthOutOfRange``, ``TooManyElements``, docs/adr/0077)."""
        _frame_len = cursor.remaining()
        if _frame_len < 1:
            raise NeedMoreBytes()
        raw = cursor.peek_slice(_frame_len)
        f_carrier = raw[0]
        if _frame_len < 1 + ((f_carrier >> 4) & 0xF):
            raise NeedMoreBytes()
        f_payload = raw[1:1 + ((f_carrier >> 4) & 0xF)]
        value = cls(
            carrier=f_carrier,
            payload=f_payload,
        )
        cursor.advance(_frame_len)
        return value

    # RFC §synth-5-B flags primitive: per-bit-range accessors over
    # the carrier field. Single-bit (width=1) reads as bool; multi-bit
    # (width>=2) reads as ``int`` (Python ints are unbounded, so a single
    # ``int`` covers every result-type width). Setters mask + shift on
    # the way in so out-of-range callers can't corrupt sibling bits.
    # Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Wire layout is unchanged.
    def hdr(self) -> int:
        return (self.carrier >> 0) & 0x0F

    def set_hdr(self, v: int) -> None:
        _shifted_mask = 0x0F << 0
        _val = (v & 0x0F) << 0
        self.carrier = ((self.carrier & (0xFF ^ _shifted_mask)) | _val) & 0xFF

    def payload_len(self) -> int:
        return (self.carrier >> 4) & 0x0F

    def set_payload_len(self, v: int) -> None:
        _shifted_mask = 0x0F << 4
        _val = (v & 0x0F) << 4
        self.carrier = ((self.carrier & (0xFF ^ _shifted_mask)) | _val) & 0xFF

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
