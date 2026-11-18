# SCE-MAP: codec_zenoh_ext_envelope:35 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink, TooManyElements
from sce_forge_runtime.bounded import BoundedInt, BoundedTuple
from .codec_zenoh_ext_entry import CodecZenohExtEntry
from . import codec_zenoh_ext_entry as sce_codec_zenoh_ext_entry

from dataclasses import dataclass, field
from typing import Optional, List, Protocol


class CodecZenohExtEnvelopeHeaderFlagsInt(BoundedInt):
    """The ``header_flags`` element of :class:`CodecZenohExtEnvelope`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


class CodecZenohExtEnvelopeExtensionsList(BoundedTuple):
    """The ``extensions`` element of :class:`CodecZenohExtEnvelope`, 1 or more entries, the protocol stating no most,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 1
    MOST = None


class CodecZenohExtEnvelopeSource(Protocol):
    """The elements a value of :class:`CodecZenohExtEnvelope` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def header_flags(self) -> CodecZenohExtEnvelopeHeaderFlagsInt: ...

    @property
    def extensions(self) -> CodecZenohExtEnvelopeExtensionsList: ...


def encode_source(src: CodecZenohExtEnvelopeSource, w: SceSink) -> None:
    """Write any source of :class:`CodecZenohExtEnvelope` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    w.write_u8(src.header_flags & 0xFF)
    _left_extensions = len(src.extensions)
    for _entry in src.extensions:
        _left_extensions -= 1
        sce_codec_zenoh_ext_entry.encode_source(_entry, w, override_header_z=1 if _left_extensions != 0 else 0)


@dataclass
class CodecZenohExtEnvelope:
    header_flags: CodecZenohExtEnvelopeHeaderFlagsInt = 0
    extensions: CodecZenohExtEnvelopeExtensionsList = field(default_factory=lambda: CodecZenohExtEnvelopeExtensionsList(sce_codec_zenoh_ext_entry.CodecZenohExtEntry() for _ in range(1)))

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "header_flags":
            value = CodecZenohExtEnvelopeHeaderFlagsInt(value)
        elif name == "extensions":
            value = CodecZenohExtEnvelopeExtensionsList(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor) -> CodecZenohExtEnvelope:
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
        f_header_flags = raw[0]
        cursor.advance(1)
        f_extensions = []
        _more = False
        while True:
            _elem = CodecZenohExtEntry.decode(cursor)
            _more = _elem.z()
            f_extensions.append(_elem)
            if not _more:
                break
        if _more and cursor.remaining() == 0:
            raise NeedMoreBytes()
        return cls(
            header_flags=f_header_flags,
            extensions=f_extensions,
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
