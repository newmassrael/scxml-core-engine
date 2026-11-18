# SCE-MAP: codec_zenoh_hello:41 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from sce_forge_runtime.bounded import BoundedBytes, BoundedInt, BoundedTuple
from .codec_zenoh_locator import CodecZenohLocator
from . import codec_zenoh_locator as sce_codec_zenoh_locator

from dataclasses import dataclass, field
from typing import Optional, List, Protocol


class CodecZenohHelloVersionInt(BoundedInt):
    """The ``version`` element of :class:`CodecZenohHello`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


class CodecZenohHelloCbyteInt(BoundedInt):
    """The ``cbyte`` element of :class:`CodecZenohHello`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


class CodecZenohHelloZidBytes(BoundedBytes):
    """The ``zid`` element of :class:`CodecZenohHello`, 1 to 16 bytes,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 1
    MOST = 16


class CodecZenohHelloLocatorsList(BoundedTuple):
    """The ``locators`` element of :class:`CodecZenohHello`, 0 or more entries, the protocol stating no most,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 0
    MOST = None


class CodecZenohHelloSource(Protocol):
    """The elements a value of :class:`CodecZenohHello` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def version(self) -> CodecZenohHelloVersionInt: ...

    @property
    def cbyte(self) -> CodecZenohHelloCbyteInt: ...

    @property
    def zid(self) -> CodecZenohHelloZidBytes: ...

    @property
    def locators(self) -> Optional[CodecZenohHelloLocatorsList]: ...


def encode_source(src: CodecZenohHelloSource, w: SceSink) -> None:
    """Write any source of :class:`CodecZenohHello` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It derives ``num_locators`` from what each measures, and never reads it off
    the source. It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    l = (1 if src.locators is not None else 0)
    _d_num_locators = (len(src.locators)) if src.locators is not None else 0
    _c_cbyte = src.cbyte
    _c_cbyte = (_c_cbyte & ~0xF0) | ((len(src.zid) - 1) << 4)
    w.write_u8(src.version & 0xFF)
    w.write_u8(_c_cbyte & 0xFF)
    w.write_bytes(src.zid)
    if l != 0:
        w.write_vle_u64(_d_num_locators)
    if src.locators is not None:
        for _entry in src.locators:
            sce_codec_zenoh_locator.encode_source(_entry, w)


def input_l(src: CodecZenohHelloSource) -> int:
    """The flag input ``L`` a source of :class:`CodecZenohHello` says, which
    its encode is not handed (docs/adr/0065 §6 R4a)."""
    return (1 if src.locators is not None else 0)


@dataclass
class CodecZenohHello:
    version: CodecZenohHelloVersionInt = 0
    cbyte: CodecZenohHelloCbyteInt = 0
    zid: CodecZenohHelloZidBytes = CodecZenohHelloZidBytes(b"\x00" * 1)
    locators: Optional[CodecZenohHelloLocatorsList] = None

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "version":
            value = CodecZenohHelloVersionInt(value)
        elif name == "cbyte":
            value = CodecZenohHelloCbyteInt(value)
        elif name == "zid":
            value = CodecZenohHelloZidBytes(value)
        elif name == "locators":
            value = None if value is None else CodecZenohHelloLocatorsList(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor, l: int) -> CodecZenohHello:
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
        _len = ((f_cbyte >> 4) & 0xF)
        _n = _len + 1
        raw = cursor.peek_slice(_n)
        f_zid = bytes(raw)
        cursor.advance(_n)
        if (l & 0x01) != 0:
            _v = cursor.read_vle_u64()
            f_num_locators = _v
        else:
            f_num_locators = None
        if (l & 0x01) != 0:
            f_locators = []
            for _ in range(f_num_locators):
                f_locators.append(CodecZenohLocator.decode(cursor))
        else:
            f_locators = None
        return cls(
            version=f_version,
            cbyte=f_cbyte,
            zid=f_zid,
            locators=f_locators,
        )

    # RFC §synth-5-B flags primitive: per-bit-range accessors over
    # the carrier field. Single-bit (width=1) reads as bool; multi-bit
    # (width>=2) reads as ``int`` (Python ints are unbounded, so a single
    # ``int`` covers every result-type width). Setters mask + shift on
    # the way in so out-of-range callers can't corrupt sibling bits.
    # Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Wire layout is unchanged.
    def whatami(self) -> int:
        return (self.cbyte >> 0) & 0x03

    def set_whatami(self, v: int) -> None:
        _shifted_mask = 0x03 << 0
        _val = (v & 0x03) << 0
        self.cbyte = ((self.cbyte & (0xFF ^ _shifted_mask)) | _val) & 0xFF

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
