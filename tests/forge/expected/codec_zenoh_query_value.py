# SCE-MAP: codec_zenoh_query_value:24 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink, TooManyElements
from sce_forge_runtime.bounded import BoundedTuple
from sce_forge_runtime.law import AfterKey, Lawful
from .codec_zenoh_value_entry import CodecZenohValueEntry
from . import codec_zenoh_value_entry as sce_codec_zenoh_value_entry

from dataclasses import dataclass, field
from typing import Optional, List, Protocol


class CodecZenohQueryValueExtsList(BoundedTuple):
    """The ``exts`` element of :class:`CodecZenohQueryValue`, 1 or more entries, the protocol stating no most,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 1
    MOST = None


class CodecZenohQueryValueExtsHas0x4Key:
    """The kind of entry of ``exts`` of :class:`CodecZenohQueryValue` a gate or a
    law asks for: an entry whose identifier is 0x4 (docs/adr/0065 §6 R6)."""

    @staticmethod
    def matches(entry: sce_codec_zenoh_value_entry.CodecZenohValueEntrySource) -> bool:
        """Whether ``entry`` is of the kind."""
        return ((entry.header >> 0) & 0xF) == 0x4


CodecZenohQueryValueExtsLaw = (AfterKey(sce_codec_zenoh_value_entry.input_after_shm, CodecZenohQueryValueExtsHas0x4Key.matches),)
"""The law the entries of ``exts`` of :class:`CodecZenohQueryValue` follow in the inputs
they say, an automaton for each, which the list holds them to (docs/adr/0065 §6 R8)."""


class CodecZenohQueryValueExtsLawful(Lawful):
    """The ``exts`` element of :class:`CodecZenohQueryValue`, its entries held to their
    law from the start the value gives (docs/adr/0065 §6 R8)."""

    ENTRIES = CodecZenohQueryValueExtsList
    LAW = CodecZenohQueryValueExtsLaw


class CodecZenohQueryValueSource(Protocol):
    """The elements a value of :class:`CodecZenohQueryValue` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def exts(self) -> CodecZenohQueryValueExtsLawful: ...


def encode_source(src: CodecZenohQueryValueSource, w: SceSink) -> None:
    """Write any source of :class:`CodecZenohQueryValue` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _left_exts = len(src.exts)
    for _entry in src.exts:
        _left_exts -= 1
        sce_codec_zenoh_value_entry.encode_source(_entry, w, override_header_z=1 if _left_exts != 0 else 0)


@dataclass
class CodecZenohQueryValue:
    exts: CodecZenohQueryValueExtsLawful = field(default_factory=lambda: CodecZenohQueryValueExtsLawful.of(sce_codec_zenoh_value_entry.CodecZenohValueEntry() for _ in range(1)))

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "exts":
            value = CodecZenohQueryValueExtsLawful(value.start, value.entries)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor) -> CodecZenohQueryValue:
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
        f_exts = []
        _prev_exts_after_shm = 0
        _more = False
        while True:
            _elem = CodecZenohValueEntry.decode(cursor, _prev_exts_after_shm)
            _more = _elem.z()
            _prev_exts_after_shm = 1 if _elem.ext_id() == 4 else 0
            f_exts.append(_elem)
            if not _more:
                break
        if _more and cursor.remaining() == 0:
            raise NeedMoreBytes()
        return cls(
            exts=CodecZenohQueryValueExtsLawful(None, f_exts),
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
