# SCE-MAP: codec_simple_frame:3 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from sce_forge_runtime.bounded import BoundedInt

from dataclasses import dataclass
from typing import Optional, Protocol


class CodecSimpleFrameMsgIdInt(BoundedInt):
    """The ``msgId`` element of :class:`CodecSimpleFrame`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


class CodecSimpleFrameLengthInt(BoundedInt):
    """The ``length`` element of :class:`CodecSimpleFrame`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


class CodecSimpleFramePayloadInt(BoundedInt):
    """The ``payload`` element of :class:`CodecSimpleFrame`, an unsigned integer of
    16 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 16
    SIGNED = False


class CodecSimpleFrameSource(Protocol):
    """The elements a value of :class:`CodecSimpleFrame` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def msg_id(self) -> CodecSimpleFrameMsgIdInt: ...

    @property
    def length(self) -> CodecSimpleFrameLengthInt: ...

    @property
    def payload(self) -> CodecSimpleFramePayloadInt: ...


def encode_source(src: CodecSimpleFrameSource, w: SceSink) -> None:
    """Write any source of :class:`CodecSimpleFrame` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    w.write_u8(src.msg_id & 0xFF)
    w.write_u8(src.length & 0xFF)
    w.write_u8((src.payload >> 8) & 0xFF)
    w.write_u8(src.payload & 0xFF)


@dataclass
class CodecSimpleFrame:
    msg_id: CodecSimpleFrameMsgIdInt = 0
    length: CodecSimpleFrameLengthInt = 0
    payload: CodecSimpleFramePayloadInt = 0

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "msg_id":
            value = CodecSimpleFrameMsgIdInt(value)
        elif name == "length":
            value = CodecSimpleFrameLengthInt(value)
        elif name == "payload":
            value = CodecSimpleFramePayloadInt(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor) -> CodecSimpleFrame:
        """Decode the next frame from ``cursor``; on success the cursor
        advances past the consumed bytes. A frame the codec refuses raises
        the ``CodecError`` subclass that names the refusal (docs/adr/0099,
        amendment of 2026-11-16): ``NeedMoreBytes`` when the cursor's tail
        is shorter than the frame (RFC §synth-5-B L494-519), which the
        caller resumes after appending bytes, and for a frame no more bytes
        make whole ``VleWidthOverflow``, ``InvalidUtf8``,
        ``UndeclaredEnumValue``, or a length or a count past its bound
        (``LengthOutOfRange``, ``TooManyElements``, docs/adr/0077)."""
        raw = cursor.peek_slice(4)
        f_msg_id = raw[0]
        f_length = raw[1]
        f_payload = (raw[2] << 8) | raw[3]
        value = cls(
            msg_id=f_msg_id,
            length=f_length,
            payload=f_payload,
        )
        cursor.advance(4)
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
