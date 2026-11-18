# SCE-MAP: codec_init_syn_body:30 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from sce_forge_runtime.bounded import BoundedInt

from dataclasses import dataclass
from typing import Optional, Protocol


class CodecInitSynBodyVersionInt(BoundedInt):
    """The ``version`` element of :class:`CodecInitSynBody`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


class CodecInitSynBodySnResInt(BoundedInt):
    """The ``sn_res`` element of :class:`CodecInitSynBody`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


class CodecInitSynBodyBatchSizeInt(BoundedInt):
    """The ``batch_size`` element of :class:`CodecInitSynBody`, an unsigned integer of
    16 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 16
    SIGNED = False


@dataclass
class CodecInitSynBodyS:
    """The ``s`` group of :class:`CodecInitSynBody` where ``S`` holds
    (docs/adr/0065 §6 R4): the members present then."""

    sn_res: CodecInitSynBodySnResInt = 0
    batch_size: CodecInitSynBodyBatchSizeInt = 0

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "sn_res":
            value = CodecInitSynBodySnResInt(value)
        elif name == "batch_size":
            value = CodecInitSynBodyBatchSizeInt(value)
        object.__setattr__(self, name, value)


class CodecInitSynBodySource(Protocol):
    """The elements a value of :class:`CodecInitSynBody` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def version(self) -> CodecInitSynBodyVersionInt: ...

    @property
    def s(self) -> Optional[CodecInitSynBodyS]: ...


def encode_source(src: CodecInitSynBodySource, w: SceSink) -> None:
    """Write any source of :class:`CodecInitSynBody` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _g_s = src.s
    _e_sn_res = None if _g_s is None else _g_s.sn_res
    _e_batch_size = None if _g_s is None else _g_s.batch_size
    w.write_u8(src.version & 0xFF)
    if _e_sn_res is not None:
        w.write_u8(_e_sn_res & 0xFF)
    if _e_batch_size is not None:
        w.write_u8((_e_batch_size >> 8) & 0xFF)
        w.write_u8(_e_batch_size & 0xFF)


def input_s(src: CodecInitSynBodySource) -> int:
    """The flag input ``S`` a source of :class:`CodecInitSynBody` says, which
    its encode is not handed (docs/adr/0065 §6 R4a)."""
    return (1 if isinstance(src.s, CodecInitSynBodyS) else 0)


@dataclass
class CodecInitSynBody:
    version: CodecInitSynBodyVersionInt = 0
    s: Optional[CodecInitSynBodyS] = None

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "version":
            value = CodecInitSynBodyVersionInt(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor, s: int) -> CodecInitSynBody:
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
        if (s & 0x01) != 0:
            raw = cursor.peek_slice(1)
            _v = raw[0]
            cursor.advance(1)
            f_sn_res = _v
        else:
            f_sn_res = None
        if (s & 0x01) != 0:
            raw = cursor.peek_slice(2)
            _v = (raw[0] << 8) | raw[1]
            cursor.advance(2)
            f_batch_size = _v
        else:
            f_batch_size = None
        return cls(
            version=f_version,
            s=CodecInitSynBodyS(sn_res=f_sn_res, batch_size=f_batch_size) if (s != 0) else None,
        )

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
