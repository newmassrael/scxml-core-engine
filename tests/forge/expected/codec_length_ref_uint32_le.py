# SCE-MAP: codec_length_ref_uint32_le:13 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, LengthOutOfRange, NeedMoreBytes, SceCursor, SceSink
from sce_forge_runtime.bounded import BoundedBytes

from dataclasses import dataclass
from typing import Optional, Protocol


class CodecLengthRefUint32LePayloadBytes(BoundedBytes):
    """The ``payload`` element of :class:`CodecLengthRefUint32Le`, 0 to 1024 bytes,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 0
    MOST = 1024


class CodecLengthRefUint32LeSource(Protocol):
    """The elements a value of :class:`CodecLengthRefUint32Le` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def payload(self) -> CodecLengthRefUint32LePayloadBytes: ...


def encode_source(src: CodecLengthRefUint32LeSource, w: SceSink) -> None:
    """Write any source of :class:`CodecLengthRefUint32Le` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It derives ``payload_len`` from what each measures, and never reads it off
    the source. It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _d_payload_len = len(src.payload)
    w.write_u8(_d_payload_len & 0xFF)
    w.write_u8((_d_payload_len >> 8) & 0xFF)
    w.write_u8((_d_payload_len >> 16) & 0xFF)
    w.write_u8((_d_payload_len >> 24) & 0xFF)
    w.write_bytes(src.payload)


@dataclass
class CodecLengthRefUint32Le:
    payload: CodecLengthRefUint32LePayloadBytes = CodecLengthRefUint32LePayloadBytes()

    def __setattr__(self, name: str, value: object) -> None:
        # Each byte string and text is held to its bound wherever it is
        # set, where the value is made and after (docs/adr/0081 §2),
        # whatever it was handed.
        if name == "payload":
            value = CodecLengthRefUint32LePayloadBytes(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecLengthRefUint32Le]:
        """Decode the next frame from ``cursor``. Returns ``None`` when
        the cursor's tail is shorter than the declared minimum frame
        (RFC §synth-5-B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``, and every codec on a field's refusal: a length
        or a count past its bound (``LengthOutOfRange``, ``TooManyElements``,
        docs/adr/0077), an undeclared enum value."""
        try:
            _frame_len = cursor.remaining()
            if _frame_len < 4:
                return None
            raw = cursor.peek_slice(_frame_len)
            f_payload_len = raw[0] | (raw[1] << 8) | (raw[2] << 16) | (raw[3] << 24)
            if f_payload_len > 1024:
                raise LengthOutOfRange()
            if _frame_len < 4 + f_payload_len:
                return None
            f_payload = raw[4:4 + f_payload_len]
        except CodecError:
            return None
        value = cls(
            payload=f_payload,
        )
        try:
            cursor.advance(_frame_len)
        except NeedMoreBytes:
            return None
        return value

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
