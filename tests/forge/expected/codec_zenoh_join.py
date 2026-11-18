# SCE-MAP: codec_zenoh_join:41 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from sce_forge_runtime.bounded import BoundedBytes, BoundedInt

from dataclasses import dataclass
from typing import Optional, Protocol


class CodecZenohJoinVersionInt(BoundedInt):
    """The ``version`` element of :class:`CodecZenohJoin`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


class CodecZenohJoinCbyteInt(BoundedInt):
    """The ``cbyte`` element of :class:`CodecZenohJoin`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


class CodecZenohJoinZidBytes(BoundedBytes):
    """The ``zid`` element of :class:`CodecZenohJoin`, 1 to 16 bytes,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 1
    MOST = 16


class CodecZenohJoinLeaseInt(BoundedInt):
    """The ``lease`` element of :class:`CodecZenohJoin`, an unsigned integer of
    64 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 64
    SIGNED = False


class CodecZenohJoinNextSnReliableInt(BoundedInt):
    """The ``next_sn_reliable`` element of :class:`CodecZenohJoin`, an unsigned integer of
    64 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 64
    SIGNED = False


class CodecZenohJoinNextSnBestEffortInt(BoundedInt):
    """The ``next_sn_best_effort`` element of :class:`CodecZenohJoin`, an unsigned integer of
    64 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 64
    SIGNED = False


class CodecZenohJoinSnResInt(BoundedInt):
    """The ``sn_res`` element of :class:`CodecZenohJoin`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


class CodecZenohJoinBatchSizeInt(BoundedInt):
    """The ``batch_size`` element of :class:`CodecZenohJoin`, an unsigned integer of
    16 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 16
    SIGNED = False


@dataclass
class CodecZenohJoinS:
    """The ``s`` group of :class:`CodecZenohJoin` where ``S`` holds
    (docs/adr/0065 §6 R4): the members present then."""

    sn_res: CodecZenohJoinSnResInt = 0
    batch_size: CodecZenohJoinBatchSizeInt = 0

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "sn_res":
            value = CodecZenohJoinSnResInt(value)
        elif name == "batch_size":
            value = CodecZenohJoinBatchSizeInt(value)
        object.__setattr__(self, name, value)


class CodecZenohJoinSource(Protocol):
    """The elements a value of :class:`CodecZenohJoin` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def version(self) -> CodecZenohJoinVersionInt: ...

    @property
    def cbyte(self) -> CodecZenohJoinCbyteInt: ...

    @property
    def zid(self) -> CodecZenohJoinZidBytes: ...

    @property
    def lease(self) -> CodecZenohJoinLeaseInt: ...

    @property
    def next_sn_reliable(self) -> CodecZenohJoinNextSnReliableInt: ...

    @property
    def next_sn_best_effort(self) -> CodecZenohJoinNextSnBestEffortInt: ...

    @property
    def s(self) -> Optional[CodecZenohJoinS]: ...


def encode_source(src: CodecZenohJoinSource, w: SceSink) -> None:
    """Write any source of :class:`CodecZenohJoin` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _g_s = src.s
    _e_sn_res = None if _g_s is None else _g_s.sn_res
    _e_batch_size = None if _g_s is None else _g_s.batch_size
    _c_cbyte = src.cbyte
    _c_cbyte = (_c_cbyte & ~0xF0) | ((len(src.zid) - 1) << 4)
    w.write_u8(src.version & 0xFF)
    w.write_u8(_c_cbyte & 0xFF)
    w.write_bytes(src.zid)
    if _e_sn_res is not None:
        w.write_u8(_e_sn_res & 0xFF)
    if _e_batch_size is not None:
        w.write_u8(_e_batch_size & 0xFF)
        w.write_u8((_e_batch_size >> 8) & 0xFF)
    w.write_vle_u64(src.lease)
    w.write_vle_u64(src.next_sn_reliable)
    w.write_vle_u64(src.next_sn_best_effort)


def input_s(src: CodecZenohJoinSource) -> int:
    """The flag input ``S`` a source of :class:`CodecZenohJoin` says, which
    its encode is not handed (docs/adr/0065 §6 R4a)."""
    return (1 if isinstance(src.s, CodecZenohJoinS) else 0)


@dataclass
class CodecZenohJoin:
    version: CodecZenohJoinVersionInt = 0
    cbyte: CodecZenohJoinCbyteInt = 0
    zid: CodecZenohJoinZidBytes = CodecZenohJoinZidBytes(b"\x00" * 1)
    lease: CodecZenohJoinLeaseInt = 0
    next_sn_reliable: CodecZenohJoinNextSnReliableInt = 0
    next_sn_best_effort: CodecZenohJoinNextSnBestEffortInt = 0
    s: Optional[CodecZenohJoinS] = None

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "version":
            value = CodecZenohJoinVersionInt(value)
        elif name == "cbyte":
            value = CodecZenohJoinCbyteInt(value)
        elif name == "zid":
            value = CodecZenohJoinZidBytes(value)
        elif name == "lease":
            value = CodecZenohJoinLeaseInt(value)
        elif name == "next_sn_reliable":
            value = CodecZenohJoinNextSnReliableInt(value)
        elif name == "next_sn_best_effort":
            value = CodecZenohJoinNextSnBestEffortInt(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor, s: int) -> CodecZenohJoin:
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
        if (s & 0x01) != 0:
            raw = cursor.peek_slice(1)
            _v = raw[0]
            cursor.advance(1)
            f_sn_res = _v
        else:
            f_sn_res = None
        if (s & 0x01) != 0:
            raw = cursor.peek_slice(2)
            _v = raw[0] | (raw[1] << 8)
            cursor.advance(2)
            f_batch_size = _v
        else:
            f_batch_size = None
        f_lease = cursor.read_vle_u64()
        f_next_sn_reliable = cursor.read_vle_u64()
        f_next_sn_best_effort = cursor.read_vle_u64()
        return cls(
            version=f_version,
            cbyte=f_cbyte,
            zid=f_zid,
            lease=f_lease,
            next_sn_reliable=f_next_sn_reliable,
            next_sn_best_effort=f_next_sn_best_effort,
            s=CodecZenohJoinS(sn_res=f_sn_res, batch_size=f_batch_size) if (s != 0) else None,
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
