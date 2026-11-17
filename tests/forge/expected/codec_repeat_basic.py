# SCE-MAP: codec_repeat_basic:11 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink, TooManyElements
from sce_forge_runtime.bounded import BoundedTuple
from .codec_repeat_elem import CodecRepeatElem
from . import codec_repeat_elem as sce_codec_repeat_elem

from dataclasses import dataclass, field
from typing import Optional, List, Protocol


class CodecRepeatBasicFragsList(BoundedTuple):
    """The ``frags`` element of :class:`CodecRepeatBasic`, 0 to 32 entries,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 0
    MOST = 32


class CodecRepeatBasicSource(Protocol):
    """The elements a value of :class:`CodecRepeatBasic` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def frags(self) -> CodecRepeatBasicFragsList: ...


def encode_source(src: CodecRepeatBasicSource, w: SceSink) -> None:
    """Write any source of :class:`CodecRepeatBasic` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It derives ``num_frags`` from what each measures, and never reads it off
    the source. It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _d_num_frags = len(src.frags)
    w.write_u8(_d_num_frags & 0xFF)
    for _entry in src.frags:
        sce_codec_repeat_elem.encode_source(_entry, w)


@dataclass
class CodecRepeatBasic:
    frags: CodecRepeatBasicFragsList = CodecRepeatBasicFragsList()

    def __setattr__(self, name: str, value: object) -> None:
        # Each byte string and text is held to its bound wherever it is
        # set, where the value is made and after (docs/adr/0081 §2),
        # whatever it was handed.
        if name == "frags":
            value = CodecRepeatBasicFragsList(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor) -> CodecRepeatBasic:
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
        f_num_frags = raw[0]
        cursor.advance(1)
        if f_num_frags > 32:
            raise TooManyElements()
        f_frags = []
        for _ in range(f_num_frags):
            f_frags.append(CodecRepeatElem.decode(cursor))
        return cls(
            frags=f_frags,
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
