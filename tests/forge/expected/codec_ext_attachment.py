# SCE-MAP: codec_ext_attachment:27 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, LengthOutOfRange, NeedMoreBytes, SceCursor, SceSink
from sce_forge_runtime.bounded import BoundedBytes

from dataclasses import dataclass
from typing import Optional, Protocol


class CodecExtAttachmentBodyBytes(BoundedBytes):
    """The ``body`` element of :class:`CodecExtAttachment`, 0 to 64 bytes,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 0
    MOST = 64


class CodecExtAttachmentSource(Protocol):
    """The elements a value of :class:`CodecExtAttachment` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def body(self) -> CodecExtAttachmentBodyBytes: ...


def encode_source(src: CodecExtAttachmentSource, w: SceSink) -> None:
    """Write any source of :class:`CodecExtAttachment` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It derives ``length`` from what each measures, and never reads it off
    the source. It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _d_length = len(src.body)
    w.write_u8(_d_length & 0xFF)
    w.write_bytes(src.body)


@dataclass
class CodecExtAttachment:
    body: CodecExtAttachmentBodyBytes = CodecExtAttachmentBodyBytes()

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "body":
            value = CodecExtAttachmentBodyBytes(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor) -> CodecExtAttachment:
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
        f_length = raw[0]
        if f_length > 64:
            raise LengthOutOfRange()
        if _frame_len < 1 + f_length:
            raise NeedMoreBytes()
        f_body = raw[1:1 + f_length]
        value = cls(
            body=f_body,
        )
        # The bytes it read and no more (RFC §synth-5-B B3, as the Rust decode
        # does): the end of the last length-ref field, at least the fixed
        # fields' bytes, so that a codec holding this one as a body, an entry
        # or an arm decodes on after it.
        _consumed = 1
        if 1 + f_length > _consumed:
            _consumed = 1 + f_length
        cursor.advance(_consumed)
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
