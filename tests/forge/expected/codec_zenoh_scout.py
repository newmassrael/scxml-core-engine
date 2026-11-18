# SCE-MAP: codec_zenoh_scout:41 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from sce_forge_runtime.bounded import BoundedBytes, BoundedInt

from dataclasses import dataclass
from typing import Optional, Protocol


class CodecZenohScoutVersionInt(BoundedInt):
    """The ``version`` element of :class:`CodecZenohScout`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


class CodecZenohScoutCbyteInt(BoundedInt):
    """The ``cbyte`` element of :class:`CodecZenohScout`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


class CodecZenohScoutZidBytes(BoundedBytes):
    """The ``zid`` element of :class:`CodecZenohScout`, 1 to 16 bytes,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 1
    MOST = 16


class CodecZenohScoutSource(Protocol):
    """The elements a value of :class:`CodecZenohScout` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def version(self) -> CodecZenohScoutVersionInt: ...

    @property
    def cbyte(self) -> CodecZenohScoutCbyteInt: ...

    @property
    def zid(self) -> Optional[CodecZenohScoutZidBytes]: ...


def encode_source(src: CodecZenohScoutSource, w: SceSink) -> None:
    """Write any source of :class:`CodecZenohScout` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _c_cbyte = src.cbyte
    _c_cbyte = (_c_cbyte & ~0x8) | (0x8 if src.zid is not None else 0)
    if src.zid is not None:
        _c_cbyte = (_c_cbyte & ~0xF0) | ((len(src.zid) - 1) << 4)
    w.write_u8(src.version & 0xFF)
    w.write_u8(_c_cbyte & 0xFF)
    if src.zid is not None:
        w.write_bytes(src.zid)


@dataclass
class CodecZenohScout:
    version: CodecZenohScoutVersionInt = 0
    cbyte: CodecZenohScoutCbyteInt = 0
    zid: Optional[CodecZenohScoutZidBytes] = None

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "version":
            value = CodecZenohScoutVersionInt(value)
        elif name == "cbyte":
            value = CodecZenohScoutCbyteInt(value)
        elif name == "zid":
            value = None if value is None else CodecZenohScoutZidBytes(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor) -> CodecZenohScout:
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
        f_version = raw[0]
        cursor.advance(1)
        raw = cursor.peek_slice(1)
        f_cbyte = raw[0]
        cursor.advance(1)
        if (f_cbyte & 0x08) != 0:
            _len = ((f_cbyte >> 4) & 0xF)
            _n = _len + 1
            raw = cursor.peek_slice(_n)
            _v = bytes(raw)
            cursor.advance(_n)
            f_zid = _v
        else:
            f_zid = None
        return cls(
            version=f_version,
            cbyte=f_cbyte,
            zid=f_zid,
        )

    # RFC §synth-5-B flags primitive: per-bit-range accessors over
    # the carrier field. Single-bit (width=1) reads as bool; multi-bit
    # (width>=2) reads as ``int`` (Python ints are unbounded, so a single
    # ``int`` covers every result-type width). Setters mask + shift on
    # the way in so out-of-range callers can't corrupt sibling bits.
    # Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Wire layout is unchanged.
    def what(self) -> int:
        return (self.cbyte >> 0) & 0x07

    def set_what(self, v: int) -> None:
        _shifted_mask = 0x07 << 0
        _val = (v & 0x07) << 0
        self.cbyte = ((self.cbyte & (0xFF ^ _shifted_mask)) | _val) & 0xFF

    def i(self) -> bool:
        return (self.cbyte & 0x08) != 0

    def set_i(self, v: bool) -> None:
        if v:
            self.cbyte = (self.cbyte | 0x08) & 0xFF
        else:
            self.cbyte = self.cbyte & (0xFF ^ 0x08)

    def zid_len_m1(self) -> int:
        return (self.cbyte >> 4) & 0x0F

    def set_zid_len_m1(self, v: int) -> None:
        _shifted_mask = 0x0F << 4
        _val = (v & 0x0F) << 4
        self.cbyte = ((self.cbyte & (0xFF ^ _shifted_mask)) | _val) & 0xFF

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
