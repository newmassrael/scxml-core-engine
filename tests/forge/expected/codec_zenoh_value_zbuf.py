# SCE-MAP: codec_zenoh_value_zbuf:27 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, LengthOutOfRange, NeedMoreBytes, SceCursor, SceSink
from sce_forge_runtime.bounded import BoundedBytes, BoundedInt, BoundedTuple
from .codec_zenoh_value_slice import CodecZenohValueSlice
from . import codec_zenoh_value_slice as sce_codec_zenoh_value_slice

from dataclasses import dataclass, field
from typing import Optional, List, Protocol, Union


class CodecZenohValueZbufEncodingInt(BoundedInt):
    """The ``encoding`` element of :class:`CodecZenohValueZbuf`, an unsigned integer of
    64 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 64
    SIGNED = False


class CodecZenohValueZbufSlicesList(BoundedTuple):
    """The ``slices`` element of :class:`CodecZenohValueZbuf`, 0 or more entries, the protocol stating no most,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 0
    MOST = None


class CodecZenohValueZbufValueLenInt(BoundedInt):
    """The ``value_len`` element of :class:`CodecZenohValueZbuf`, an unsigned integer of
    64 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 64
    SIGNED = False


class CodecZenohValueZbufValueBytes(BoundedBytes):
    """The ``value`` element of :class:`CodecZenohValueZbuf`, 0 to 4294967295 bytes,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 0
    MOST = 4294967295


@dataclass
class CodecZenohValueZbufAfterShmAfterShm:
    """The ``after_shm`` group of :class:`CodecZenohValueZbuf` where ``after_shm`` holds
    (docs/adr/0065 §6 R4): the members present then."""

    encoding: CodecZenohValueZbufEncodingInt = 0
    slices: CodecZenohValueZbufSlicesList = CodecZenohValueZbufSlicesList()
    value_len: CodecZenohValueZbufValueLenInt = 0

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "encoding":
            value = CodecZenohValueZbufEncodingInt(value)
        elif name == "slices":
            value = CodecZenohValueZbufSlicesList(value)
        elif name == "value_len":
            value = CodecZenohValueZbufValueLenInt(value)
        object.__setattr__(self, name, value)


@dataclass
class CodecZenohValueZbufAfterShmNotAfterShm:
    """The ``after_shm`` group of :class:`CodecZenohValueZbuf` where ``after_shm`` does not hold
    (docs/adr/0065 §6 R4): the members present then."""

    value: CodecZenohValueZbufValueBytes = CodecZenohValueZbufValueBytes()

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "value":
            value = CodecZenohValueZbufValueBytes(value)
        object.__setattr__(self, name, value)


CodecZenohValueZbufAfterShm = Union[CodecZenohValueZbufAfterShmAfterShm, CodecZenohValueZbufAfterShmNotAfterShm]
"""The ``after_shm`` group of :class:`CodecZenohValueZbuf`: the arm the value holds."""


class CodecZenohValueZbufSource(Protocol):
    """The elements a value of :class:`CodecZenohValueZbuf` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def after_shm(self) -> CodecZenohValueZbufAfterShm: ...


def encode_source(src: CodecZenohValueZbufSource, w: SceSink) -> None:
    """Write any source of :class:`CodecZenohValueZbuf` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It derives ``slice_count`` from what each measures, and never reads it off
    the source. It derives ``value_len`` where what it measures is there, and reads
    what the source states where none is. It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _g_after_shm = src.after_shm
    _e_encoding = _g_after_shm.encoding if isinstance(_g_after_shm, CodecZenohValueZbufAfterShmAfterShm) else None
    _e_slices = _g_after_shm.slices if isinstance(_g_after_shm, CodecZenohValueZbufAfterShmAfterShm) else None
    _e_value_len = _g_after_shm.value_len if isinstance(_g_after_shm, CodecZenohValueZbufAfterShmAfterShm) else None
    _e_value = _g_after_shm.value if isinstance(_g_after_shm, CodecZenohValueZbufAfterShmNotAfterShm) else None
    after_shm = (1 if isinstance(src.after_shm, CodecZenohValueZbufAfterShmAfterShm) else 0)
    _d_value_len = (len(_e_value)) if _e_value is not None else (0 if _e_value_len is None else _e_value_len)
    _d_slice_count = (len(_e_slices)) if _e_slices is not None else 0
    w.write_vle_u64(_d_value_len)
    if _e_value is not None:
        w.write_bytes(_e_value)
    if _e_encoding is not None:
        w.write_vle_u64(_e_encoding)
    if after_shm != 0:
        w.write_vle_u64(_d_slice_count)
    if _e_slices is not None:
        for _entry in _e_slices:
            sce_codec_zenoh_value_slice.encode_source(_entry, w)


def input_after_shm(src: CodecZenohValueZbufSource) -> int:
    """The flag input ``after_shm`` a source of :class:`CodecZenohValueZbuf` says, which
    its encode is not handed (docs/adr/0065 §6 R4a)."""
    return (1 if isinstance(src.after_shm, CodecZenohValueZbufAfterShmAfterShm) else 0)


@dataclass
class CodecZenohValueZbuf:
    after_shm: CodecZenohValueZbufAfterShm = field(default_factory=CodecZenohValueZbufAfterShmNotAfterShm)

    @classmethod
    def decode(cls, cursor: SceCursor, after_shm: int) -> CodecZenohValueZbuf:
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
        f_value_len = cursor.read_vle_u64()
        if (after_shm & 0x01) == 0:
            _len = f_value_len
            if _len > 4294967295:
                raise LengthOutOfRange()
            _n = _len
            raw = cursor.peek_slice(_n)
            _v = bytes(raw)
            cursor.advance(_n)
            f_value = _v
        else:
            f_value = None
        if (after_shm & 0x01) != 0:
            _v = cursor.read_vle_u64()
            f_encoding = _v
        else:
            f_encoding = None
        if (after_shm & 0x01) != 0:
            _v = cursor.read_vle_u64()
            f_slice_count = _v
        else:
            f_slice_count = None
        if (after_shm & 0x01) != 0:
            f_slices = []
            for _ in range(f_slice_count):
                f_slices.append(CodecZenohValueSlice.decode(cursor))
        else:
            f_slices = None
        return cls(
            after_shm=CodecZenohValueZbufAfterShmAfterShm(encoding=f_encoding, slices=f_slices, value_len=f_value_len) if (after_shm != 0) else CodecZenohValueZbufAfterShmNotAfterShm(value=f_value),
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
