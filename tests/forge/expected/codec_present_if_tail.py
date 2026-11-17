# SCE-MAP: codec_present_if_tail:8 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, LengthOutOfRange, NeedMoreBytes, SceCursor, SceSink
from sce_forge_runtime.bounded import BoundedBytes

from dataclasses import dataclass
from typing import Optional, Protocol


class CodecPresentIfTailPayloadBytes(BoundedBytes):
    """The ``payload`` element of :class:`CodecPresentIfTail`, 0 to 64 bytes,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 0
    MOST = 64


class CodecPresentIfTailSource(Protocol):
    """The elements a value of :class:`CodecPresentIfTail` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def flags(self) -> int: ...

    @property
    def payload(self) -> Optional[CodecPresentIfTailPayloadBytes]: ...


def encode_source(src: CodecPresentIfTailSource, w: SceSink) -> None:
    """Write any source of :class:`CodecPresentIfTail` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _c_flags = src.flags
    _c_flags = (_c_flags & ~0x1) | (0x1 if src.payload is not None else 0)
    w.write_u8(_c_flags & 0xFF)
    if src.payload is not None:
        w.write_bytes(src.payload)


@dataclass
class CodecPresentIfTail:
    flags: int = 0
    payload: Optional[CodecPresentIfTailPayloadBytes] = None

    def __setattr__(self, name: str, value: object) -> None:
        # Each byte string and text is held to its bound wherever it is
        # set, where the value is made and after (docs/adr/0081 §2),
        # whatever it was handed.
        if name == "payload":
            value = None if value is None else CodecPresentIfTailPayloadBytes(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor) -> CodecPresentIfTail:
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
        if (f_flags & 0x01) != 0:
            _n = cursor.remaining()
            if _n > 64:
                raise LengthOutOfRange()
            raw = cursor.peek_slice(_n)
            _v = bytes(raw)
            cursor.advance(_n)
            f_payload = _v
        else:
            f_payload = None
        return cls(
            flags=f_flags,
            payload=f_payload,
        )

    # RFC §synth-5-B flags primitive: per-bit-range accessors over
    # the carrier field. Single-bit (width=1) reads as bool; multi-bit
    # (width>=2) reads as ``int`` (Python ints are unbounded, so a single
    # ``int`` covers every result-type width). Setters mask + shift on
    # the way in so out-of-range callers can't corrupt sibling bits.
    # Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Wire layout is unchanged.
    def has_payload(self) -> bool:
        return (self.flags & 0x01) != 0

    def set_has_payload(self, v: bool) -> None:
        if v:
            self.flags = (self.flags | 0x01) & 0xFF
        else:
            self.flags = self.flags & (0xFF ^ 0x01)

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
