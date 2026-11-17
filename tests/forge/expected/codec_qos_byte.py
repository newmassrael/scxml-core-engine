# SCE-MAP: codec_qos_byte:15 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink

from dataclasses import dataclass
from typing import Optional, Protocol


class CodecQosByteSource(Protocol):
    """The elements a value of :class:`CodecQosByte` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def qos(self) -> int: ...


def encode_source(src: CodecQosByteSource, w: SceSink) -> None:
    """Write any source of :class:`CodecQosByte` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    w.write_u8(src.qos & 0xFF)


@dataclass
class CodecQosByte:
    qos: int = 0

    @classmethod
    def decode(cls, cursor: SceCursor) -> CodecQosByte:
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
        f_qos = raw[0]
        value = cls(
            qos=f_qos,
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
    def priority(self) -> int:
        return (self.qos >> 0) & 0x07

    def set_priority(self, v: int) -> None:
        _shifted_mask = 0x07 << 0
        _val = (v & 0x07) << 0
        self.qos = ((self.qos & (0xFF ^ _shifted_mask)) | _val) & 0xFF

    def reliable(self) -> bool:
        return (self.qos & 0x08) != 0

    def set_reliable(self, v: bool) -> None:
        if v:
            self.qos = (self.qos | 0x08) & 0xFF
        else:
            self.qos = self.qos & (0xFF ^ 0x08)

    def congestion(self) -> int:
        return (self.qos >> 4) & 0x03

    def set_congestion(self, v: int) -> None:
        _shifted_mask = 0x03 << 4
        _val = (v & 0x03) << 4
        self.qos = ((self.qos & (0xFF ^ _shifted_mask)) | _val) & 0xFF

    def express(self) -> bool:
        return (self.qos & 0x40) != 0

    def set_express(self, v: bool) -> None:
        if v:
            self.qos = (self.qos | 0x40) & 0xFF
        else:
            self.qos = self.qos & (0xFF ^ 0x40)

    def reserved(self) -> bool:
        return (self.qos & 0x80) != 0

    def set_reserved(self, v: bool) -> None:
        if v:
            self.qos = (self.qos | 0x80) & 0xFF
        else:
            self.qos = self.qos & (0xFF ^ 0x80)

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
