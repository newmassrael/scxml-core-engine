# SCE-MAP: codec_zenoh_interest_body:56 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from sce_forge_runtime.bounded import BoundedInt
from .codec_zenoh_wireexpr import CodecZenohWireexpr
from . import codec_zenoh_wireexpr as sce_codec_zenoh_wireexpr

from dataclasses import dataclass
from typing import Optional, Protocol


class CodecZenohInterestBodyHeaderInt(BoundedInt):
    """The ``header`` element of :class:`CodecZenohInterestBody`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


class CodecZenohInterestBodySource(Protocol):
    """The elements a value of :class:`CodecZenohInterestBody` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def header(self) -> CodecZenohInterestBodyHeaderInt: ...

    @property
    def keyexpr(self) -> Optional[sce_codec_zenoh_wireexpr.CodecZenohWireexprSource]: ...


def encode_source(src: CodecZenohInterestBodySource, w: SceSink, *, override_header_keyexprs: Optional[int] = None, override_header_subscribers: Optional[int] = None, override_header_queryables: Optional[int] = None, override_header_tokens: Optional[int] = None, override_header_m: Optional[int] = None, override_header_aggregate: Optional[int] = None) -> None:
    """Write any source of :class:`CodecZenohInterestBody` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). A chain that holds it
    sets the flag its entries end on by its keyword, in place of the value's. It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _c_header = src.header
    _c_header = (_c_header & ~0x10) | (0x10 if src.keyexpr is not None else 0)
    if src.keyexpr is not None:
        _c_header = (_c_header & ~0x20) | ((sce_codec_zenoh_wireexpr.input_n(src.keyexpr)) << 5)
    if override_header_keyexprs is not None:
        _c_header = (_c_header & ~0x1) | ((override_header_keyexprs & 0x1) << 0)
    if override_header_subscribers is not None:
        _c_header = (_c_header & ~0x2) | ((override_header_subscribers & 0x1) << 1)
    if override_header_queryables is not None:
        _c_header = (_c_header & ~0x4) | ((override_header_queryables & 0x1) << 2)
    if override_header_tokens is not None:
        _c_header = (_c_header & ~0x8) | ((override_header_tokens & 0x1) << 3)
    if override_header_m is not None:
        _c_header = (_c_header & ~0x40) | ((override_header_m & 0x1) << 6)
    if override_header_aggregate is not None:
        _c_header = (_c_header & ~0x80) | ((override_header_aggregate & 0x1) << 7)
    w.write_u8(_c_header & 0xFF)
    if src.keyexpr is not None:
        sce_codec_zenoh_wireexpr.encode_source(src.keyexpr, w)


@dataclass
class CodecZenohInterestBody:
    header: CodecZenohInterestBodyHeaderInt = 0
    keyexpr: Optional[CodecZenohWireexpr] = None

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "header":
            value = CodecZenohInterestBodyHeaderInt(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor) -> CodecZenohInterestBody:
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
        f_header = raw[0]
        cursor.advance(1)
        if (f_header & 0x10) != 0:
            f_keyexpr = CodecZenohWireexpr.decode(cursor, ((f_header >> 5) & 0x1))
        else:
            f_keyexpr = None
        return cls(
            header=f_header,
            keyexpr=f_keyexpr,
        )

    # RFC §synth-5-B flags primitive: per-bit-range accessors over
    # the carrier field. Single-bit (width=1) reads as bool; multi-bit
    # (width>=2) reads as ``int`` (Python ints are unbounded, so a single
    # ``int`` covers every result-type width). Setters mask + shift on
    # the way in so out-of-range callers can't corrupt sibling bits.
    # Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Wire layout is unchanged.
    def keyexprs(self) -> bool:
        return (self.header & 0x01) != 0

    def set_keyexprs(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x01) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x01)

    def subscribers(self) -> bool:
        return (self.header & 0x02) != 0

    def set_subscribers(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x02) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x02)

    def queryables(self) -> bool:
        return (self.header & 0x04) != 0

    def set_queryables(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x04) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x04)

    def tokens(self) -> bool:
        return (self.header & 0x08) != 0

    def set_tokens(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x08) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x08)

    def restricted(self) -> bool:
        return (self.header & 0x10) != 0

    def set_restricted(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x10) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x10)

    def n(self) -> bool:
        return (self.header & 0x20) != 0

    def set_n(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x20) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x20)

    def m(self) -> bool:
        return (self.header & 0x40) != 0

    def set_m(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x40) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x40)

    def aggregate(self) -> bool:
        return (self.header & 0x80) != 0

    def set_aggregate(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x80) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x80)

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
