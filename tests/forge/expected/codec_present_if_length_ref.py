# SCE-MAP: codec_present_if_length_ref:16 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, LengthOutOfRange, NeedMoreBytes, SceCursor, SceSink
from sce_forge_runtime.bounded import BoundedBytes, BoundedInt

from dataclasses import dataclass, field
from typing import Optional, Protocol, Union


class CodecPresentIfLengthRefFlagsInt(BoundedInt):
    """The ``flags`` element of :class:`CodecPresentIfLengthRef`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


class CodecPresentIfLengthRefPayloadBytes(BoundedBytes):
    """The ``payload`` element of :class:`CodecPresentIfLengthRef`, 0 to 32 bytes,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 0
    MOST = 32


class CodecPresentIfLengthRefPayloadSizeInt(BoundedInt):
    """The ``payload_size`` element of :class:`CodecPresentIfLengthRef`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


@dataclass
class CodecPresentIfLengthRefFlagsHasPayloadHasPayload:
    """The ``flags_has_payload`` group of :class:`CodecPresentIfLengthRef` where ``flags.has_payload`` holds
    (docs/adr/0065 §6 R4): the members present then."""

    payload: CodecPresentIfLengthRefPayloadBytes = CodecPresentIfLengthRefPayloadBytes()

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "payload":
            value = CodecPresentIfLengthRefPayloadBytes(value)
        object.__setattr__(self, name, value)


@dataclass
class CodecPresentIfLengthRefFlagsHasPayloadNotHasPayload:
    """The ``flags_has_payload`` group of :class:`CodecPresentIfLengthRef` where ``flags.has_payload`` does not hold
    (docs/adr/0065 §6 R4): the members present then."""

    payload_size: CodecPresentIfLengthRefPayloadSizeInt = 0

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "payload_size":
            value = CodecPresentIfLengthRefPayloadSizeInt(value)
        object.__setattr__(self, name, value)


CodecPresentIfLengthRefFlagsHasPayload = Union[CodecPresentIfLengthRefFlagsHasPayloadHasPayload, CodecPresentIfLengthRefFlagsHasPayloadNotHasPayload]
"""The ``flags_has_payload`` group of :class:`CodecPresentIfLengthRef`: the arm the value holds."""


class CodecPresentIfLengthRefSource(Protocol):
    """The elements a value of :class:`CodecPresentIfLengthRef` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def flags(self) -> CodecPresentIfLengthRefFlagsInt: ...

    @property
    def flags_has_payload(self) -> CodecPresentIfLengthRefFlagsHasPayload: ...


def encode_source(src: CodecPresentIfLengthRefSource, w: SceSink) -> None:
    """Write any source of :class:`CodecPresentIfLengthRef` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It derives ``payload_size`` where what it measures is there, and reads
    what the source states where none is. It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _g_flags_has_payload = src.flags_has_payload
    _e_payload = _g_flags_has_payload.payload if isinstance(_g_flags_has_payload, CodecPresentIfLengthRefFlagsHasPayloadHasPayload) else None
    _e_payload_size = _g_flags_has_payload.payload_size if isinstance(_g_flags_has_payload, CodecPresentIfLengthRefFlagsHasPayloadNotHasPayload) else None
    _d_payload_size = (len(_e_payload)) if _e_payload is not None else (0 if _e_payload_size is None else _e_payload_size)
    _c_flags = src.flags
    _c_flags = (_c_flags & ~0x1) | (0x1 if _e_payload is not None else 0)
    w.write_u8(_c_flags & 0xFF)
    w.write_u8(_d_payload_size & 0xFF)
    if _e_payload is not None:
        w.write_bytes(_e_payload)


@dataclass
class CodecPresentIfLengthRef:
    flags: CodecPresentIfLengthRefFlagsInt = 0
    flags_has_payload: CodecPresentIfLengthRefFlagsHasPayload = field(default_factory=CodecPresentIfLengthRefFlagsHasPayloadNotHasPayload)

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "flags":
            value = CodecPresentIfLengthRefFlagsInt(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor) -> CodecPresentIfLengthRef:
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
        raw = cursor.peek_slice(1)
        f_payload_size = raw[0]
        cursor.advance(1)
        if (f_flags & 0x01) != 0:
            _len = f_payload_size
            if _len > 32:
                raise LengthOutOfRange()
            _n = _len
            raw = cursor.peek_slice(_n)
            _v = bytes(raw)
            cursor.advance(_n)
            f_payload = _v
        else:
            f_payload = None
        return cls(
            flags=f_flags,
            flags_has_payload=CodecPresentIfLengthRefFlagsHasPayloadHasPayload(payload=f_payload) if (((f_flags >> 0) & 1) != 0) else CodecPresentIfLengthRefFlagsHasPayloadNotHasPayload(payload_size=f_payload_size),
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
