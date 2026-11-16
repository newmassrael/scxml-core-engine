# SCE-MAP: codec_length_ref:3 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, LengthOutOfRange, NeedMoreBytes, SceCursor, SceSink
from sce_forge_runtime.bounded import BoundedBytes

from dataclasses import dataclass
from typing import Optional, Protocol


class CodecLengthRefPayloadBytes(BoundedBytes):
    """The ``payload`` element of :class:`CodecLengthRef`, 0 to 32 bytes,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 0
    MOST = 32


class CodecLengthRefSource(Protocol):
    """The elements a value of :class:`CodecLengthRef` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def msg_id(self) -> int: ...

    @property
    def payload(self) -> CodecLengthRefPayloadBytes: ...


def encode_source(src: CodecLengthRefSource, w: SceSink) -> None:
    """Write any source of :class:`CodecLengthRef` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It derives ``len`` from what each measures, and never reads it off
    the source. It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _d_len = len(src.payload)
    w.write_u8(src.msg_id & 0xFF)
    w.write_u8(_d_len & 0xFF)
    w.write_bytes(src.payload)


@dataclass
class CodecLengthRef:
    msg_id: int = 0
    payload: CodecLengthRefPayloadBytes = CodecLengthRefPayloadBytes()

    def __setattr__(self, name: str, value: object) -> None:
        # Each byte string and text is held to its bound wherever it is
        # set, where the value is made and after (docs/adr/0081 §2),
        # whatever it was handed.
        if name == "payload":
            value = CodecLengthRefPayloadBytes(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecLengthRef]:
        """Decode the next frame from ``cursor``. Returns ``None`` when
        the cursor's tail is shorter than the declared minimum frame
        (RFC §synth-5-B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``, and every codec on a field's refusal: a length
        or a count past its bound (``LengthOutOfRange``, ``TooManyElements``,
        docs/adr/0077), an undeclared enum value."""
        try:
            _frame_len = cursor.remaining()
            if _frame_len < 2:
                return None
            raw = cursor.peek_slice(_frame_len)
            f_msg_id = raw[0]
            f_len = raw[1]
            if f_len > 32:
                raise LengthOutOfRange()
            if _frame_len < 2 + f_len:
                return None
            f_payload = raw[2:2 + f_len]
        except CodecError:
            return None
        value = cls(
            msg_id=f_msg_id,
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
