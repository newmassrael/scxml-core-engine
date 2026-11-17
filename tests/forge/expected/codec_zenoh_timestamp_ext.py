# SCE-MAP: codec_zenoh_timestamp_ext:48 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from .codec_zenoh_timestamp import CodecZenohTimestamp
from . import codec_zenoh_timestamp as sce_codec_zenoh_timestamp

from dataclasses import dataclass, field
from typing import Optional, Protocol


class CodecZenohTimestampExtSource(Protocol):
    """The elements a value of :class:`CodecZenohTimestampExt` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def ts(self) -> sce_codec_zenoh_timestamp.CodecZenohTimestampSource: ...


def encode_source(src: CodecZenohTimestampExtSource, w: SceSink) -> None:
    """Write any source of :class:`CodecZenohTimestampExt` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It derives ``ext_size`` from what each measures, and never reads it off
    the source. It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _b_ts = bytearray()
    sce_codec_zenoh_timestamp.encode_source(src.ts, BytearraySink(_b_ts))
    _d_ext_size = len(_b_ts)
    w.write_vle_u64(_d_ext_size)
    w.write_bytes(_b_ts)


@dataclass
class CodecZenohTimestampExt:
    ts: CodecZenohTimestamp = field(default_factory=CodecZenohTimestamp)

    @classmethod
    def decode(cls, cursor: SceCursor) -> CodecZenohTimestampExt:
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
        f_ext_size = cursor.read_vle_u64()
        _len = int(f_ext_size)
        _inner = SceCursor(bytes(cursor.peek_slice(_len)))
        f_ts = CodecZenohTimestamp.decode(_inner)
        cursor.advance(_len)
        return cls(
            ts=f_ts,
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
