# SCE-MAP: codec_chain_has_tagged:11 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink, TooManyElements
from sce_forge_runtime.bounded import BoundedInt
from sce_forge_runtime.membership import Chain
from .codec_chain_has_tagged_entry import CodecChainHasTaggedEntry
from . import codec_chain_has_tagged_entry as sce_codec_chain_has_tagged_entry

from dataclasses import dataclass, field
from typing import Optional, List, Protocol, Union


class CodecChainHasTaggedHeaderInt(BoundedInt):
    """The ``header`` element of :class:`CodecChainHasTagged`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


class CodecChainHasTaggedPriorityInt(BoundedInt):
    """The ``priority`` element of :class:`CodecChainHasTagged`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


class CodecChainHasTaggedChecksumInt(BoundedInt):
    """The ``checksum`` element of :class:`CodecChainHasTagged`, an unsigned integer of
    16 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 16
    SIGNED = False


class CodecChainHasTaggedEntriesHas0x7Key:
    """The kind of entry of ``entries`` of :class:`CodecChainHasTagged` a gate or a
    law asks for: an entry whose identifier is 0x7 (docs/adr/0065 §6 R6)."""

    @staticmethod
    def matches(entry: sce_codec_chain_has_tagged_entry.CodecChainHasTaggedEntrySource) -> bool:
        """Whether ``entry`` is of the kind."""
        return entry.entry_type == 0x7


class CodecChainHasTaggedEntriesHas0x9Key:
    """The kind of entry of ``entries`` of :class:`CodecChainHasTagged` a gate or a
    law asks for: an entry whose identifier is 0x9 (docs/adr/0065 §6 R6)."""

    @staticmethod
    def matches(entry: sce_codec_chain_has_tagged_entry.CodecChainHasTaggedEntrySource) -> bool:
        """Whether ``entry`` is of the kind."""
        return entry.entry_type == 0x9


class CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Has0x9WideChain(Chain):
    """The ``entries`` chain :class:`CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Has0x9Wide` holds: an entry of each kind of
    ``HOLDS`` and of none of ``LACKS`` (docs/adr/0065 §6 R6b)."""

    LEAST = 1
    MOST = 3
    HOLDS = (CodecChainHasTaggedEntriesHas0x7Key, CodecChainHasTaggedEntriesHas0x9Key)
    LACKS = ()


@dataclass
class CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Has0x9Wide:
    """The ``entries_has_0x7_has_0x9_header_wide`` group of :class:`CodecChainHasTagged` where ``entries.has(0x7)`` holds and ``entries.has(0x9)`` holds and ``header.wide`` holds
    (docs/adr/0065 §6 R4): the members present then."""

    entries: CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Has0x9WideChain = field(default_factory=CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Has0x9WideChain)
    priority: CodecChainHasTaggedPriorityInt = 0

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "entries":
            value = CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Has0x9WideChain(value)
        elif name == "priority":
            value = CodecChainHasTaggedPriorityInt(value)
        object.__setattr__(self, name, value)


class CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Has0x9Chain(Chain):
    """The ``entries`` chain :class:`CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Has0x9` holds: an entry of each kind of
    ``HOLDS`` and of none of ``LACKS`` (docs/adr/0065 §6 R6b)."""

    LEAST = 1
    MOST = 3
    HOLDS = (CodecChainHasTaggedEntriesHas0x7Key, CodecChainHasTaggedEntriesHas0x9Key)
    LACKS = ()


@dataclass
class CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Has0x9:
    """The ``entries_has_0x7_has_0x9_header_wide`` group of :class:`CodecChainHasTagged` where ``entries.has(0x7)`` holds and ``entries.has(0x9)`` holds and ``header.wide`` does not hold
    (docs/adr/0065 §6 R4): the members present then."""

    entries: CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Has0x9Chain = field(default_factory=CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Has0x9Chain)
    priority: CodecChainHasTaggedPriorityInt = 0

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "entries":
            value = CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Has0x9Chain(value)
        elif name == "priority":
            value = CodecChainHasTaggedPriorityInt(value)
        object.__setattr__(self, name, value)


class CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7WideChain(Chain):
    """The ``entries`` chain :class:`CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Wide` holds: an entry of each kind of
    ``HOLDS`` and of none of ``LACKS`` (docs/adr/0065 §6 R6b)."""

    LEAST = 1
    MOST = 3
    HOLDS = (CodecChainHasTaggedEntriesHas0x7Key,)
    LACKS = (CodecChainHasTaggedEntriesHas0x9Key,)


@dataclass
class CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Wide:
    """The ``entries_has_0x7_has_0x9_header_wide`` group of :class:`CodecChainHasTagged` where ``entries.has(0x7)`` holds and ``entries.has(0x9)`` does not hold and ``header.wide`` holds
    (docs/adr/0065 §6 R4): the members present then."""

    entries: CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7WideChain = field(default_factory=CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7WideChain)
    priority: CodecChainHasTaggedPriorityInt = 0
    checksum: CodecChainHasTaggedChecksumInt = 0

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "entries":
            value = CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7WideChain(value)
        elif name == "priority":
            value = CodecChainHasTaggedPriorityInt(value)
        elif name == "checksum":
            value = CodecChainHasTaggedChecksumInt(value)
        object.__setattr__(self, name, value)


class CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x9WideChain(Chain):
    """The ``entries`` chain :class:`CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x9Wide` holds: an entry of each kind of
    ``HOLDS`` and of none of ``LACKS`` (docs/adr/0065 §6 R6b)."""

    LEAST = 1
    MOST = 3
    HOLDS = (CodecChainHasTaggedEntriesHas0x9Key,)
    LACKS = (CodecChainHasTaggedEntriesHas0x7Key,)


@dataclass
class CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x9Wide:
    """The ``entries_has_0x7_has_0x9_header_wide`` group of :class:`CodecChainHasTagged` where ``entries.has(0x7)`` does not hold and ``entries.has(0x9)`` holds and ``header.wide`` holds
    (docs/adr/0065 §6 R4): the members present then."""

    entries: CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x9WideChain = field(default_factory=CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x9WideChain)
    priority: CodecChainHasTaggedPriorityInt = 0

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "entries":
            value = CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x9WideChain(value)
        elif name == "priority":
            value = CodecChainHasTaggedPriorityInt(value)
        object.__setattr__(self, name, value)


class CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Chain(Chain):
    """The ``entries`` chain :class:`CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7` holds: an entry of each kind of
    ``HOLDS`` and of none of ``LACKS`` (docs/adr/0065 §6 R6b)."""

    LEAST = 1
    MOST = 3
    HOLDS = (CodecChainHasTaggedEntriesHas0x7Key,)
    LACKS = (CodecChainHasTaggedEntriesHas0x9Key,)


@dataclass
class CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7:
    """The ``entries_has_0x7_has_0x9_header_wide`` group of :class:`CodecChainHasTagged` where ``entries.has(0x7)`` holds and ``entries.has(0x9)`` does not hold and ``header.wide`` does not hold
    (docs/adr/0065 §6 R4): the members present then."""

    entries: CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Chain = field(default_factory=CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Chain)
    priority: CodecChainHasTaggedPriorityInt = 0
    checksum: CodecChainHasTaggedChecksumInt = 0

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "entries":
            value = CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Chain(value)
        elif name == "priority":
            value = CodecChainHasTaggedPriorityInt(value)
        elif name == "checksum":
            value = CodecChainHasTaggedChecksumInt(value)
        object.__setattr__(self, name, value)


class CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x9Chain(Chain):
    """The ``entries`` chain :class:`CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x9` holds: an entry of each kind of
    ``HOLDS`` and of none of ``LACKS`` (docs/adr/0065 §6 R6b)."""

    LEAST = 1
    MOST = 3
    HOLDS = (CodecChainHasTaggedEntriesHas0x9Key,)
    LACKS = (CodecChainHasTaggedEntriesHas0x7Key,)


@dataclass
class CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x9:
    """The ``entries_has_0x7_has_0x9_header_wide`` group of :class:`CodecChainHasTagged` where ``entries.has(0x7)`` does not hold and ``entries.has(0x9)`` holds and ``header.wide`` does not hold
    (docs/adr/0065 §6 R4): the members present then."""

    entries: CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x9Chain = field(default_factory=CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x9Chain)

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "entries":
            value = CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x9Chain(value)
        object.__setattr__(self, name, value)


class CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideWideChain(Chain):
    """The ``entries`` chain :class:`CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideWide` holds: an entry of each kind of
    ``HOLDS`` and of none of ``LACKS`` (docs/adr/0065 §6 R6b)."""

    LEAST = 1
    MOST = 3
    HOLDS = ()
    LACKS = (CodecChainHasTaggedEntriesHas0x7Key, CodecChainHasTaggedEntriesHas0x9Key)


@dataclass
class CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideWide:
    """The ``entries_has_0x7_has_0x9_header_wide`` group of :class:`CodecChainHasTagged` where ``entries.has(0x7)`` does not hold and ``entries.has(0x9)`` does not hold and ``header.wide`` holds
    (docs/adr/0065 §6 R4): the members present then."""

    entries: CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideWideChain = field(default_factory=lambda: CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideWideChain(sce_codec_chain_has_tagged_entry.CodecChainHasTaggedEntry() for _ in range(1)))
    priority: CodecChainHasTaggedPriorityInt = 0
    checksum: CodecChainHasTaggedChecksumInt = 0

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "entries":
            value = CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideWideChain(value)
        elif name == "priority":
            value = CodecChainHasTaggedPriorityInt(value)
        elif name == "checksum":
            value = CodecChainHasTaggedChecksumInt(value)
        object.__setattr__(self, name, value)


class CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideNeitherChain(Chain):
    """The ``entries`` chain :class:`CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideNeither` holds: an entry of each kind of
    ``HOLDS`` and of none of ``LACKS`` (docs/adr/0065 §6 R6b)."""

    LEAST = 1
    MOST = 3
    HOLDS = ()
    LACKS = (CodecChainHasTaggedEntriesHas0x7Key, CodecChainHasTaggedEntriesHas0x9Key)


@dataclass
class CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideNeither:
    """The ``entries_has_0x7_has_0x9_header_wide`` group of :class:`CodecChainHasTagged` where ``entries.has(0x7)`` does not hold and ``entries.has(0x9)`` does not hold and ``header.wide`` does not hold
    (docs/adr/0065 §6 R4): the members present then."""

    entries: CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideNeitherChain = field(default_factory=lambda: CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideNeitherChain(sce_codec_chain_has_tagged_entry.CodecChainHasTaggedEntry() for _ in range(1)))
    checksum: CodecChainHasTaggedChecksumInt = 0

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "entries":
            value = CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideNeitherChain(value)
        elif name == "checksum":
            value = CodecChainHasTaggedChecksumInt(value)
        object.__setattr__(self, name, value)


CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide = Union[CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Has0x9Wide, CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Has0x9, CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Wide, CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x9Wide, CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7, CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x9, CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideWide, CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideNeither]
"""The ``entries_has_0x7_has_0x9_header_wide`` group of :class:`CodecChainHasTagged`: the arm the value holds."""


class CodecChainHasTaggedSource(Protocol):
    """The elements a value of :class:`CodecChainHasTagged` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def header(self) -> CodecChainHasTaggedHeaderInt: ...

    @property
    def entries_has_0x7_has_0x9_header_wide(self) -> CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide: ...


def encode_source(src: CodecChainHasTaggedSource, w: SceSink) -> None:
    """Write any source of :class:`CodecChainHasTagged` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _g_entries_has_0x7_has_0x9_header_wide = src.entries_has_0x7_has_0x9_header_wide
    _e_entries = _g_entries_has_0x7_has_0x9_header_wide.entries
    _e_priority = _g_entries_has_0x7_has_0x9_header_wide.priority if isinstance(_g_entries_has_0x7_has_0x9_header_wide, (CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Has0x9Wide, CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Has0x9, CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Wide, CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x9Wide, CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7, CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideWide)) else None
    _e_checksum = _g_entries_has_0x7_has_0x9_header_wide.checksum if isinstance(_g_entries_has_0x7_has_0x9_header_wide, (CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Wide, CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7, CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideWide, CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideNeither)) else None
    _c_header = src.header
    _c_header = (_c_header & ~0x1) | (0x1 if isinstance(_g_entries_has_0x7_has_0x9_header_wide, (CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Has0x9Wide, CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Wide, CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x9Wide, CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideWide)) else 0)
    w.write_u8(_c_header & 0xFF)
    if _e_entries is not None:
        _left_entries = len(_e_entries)
        for _entry in _e_entries:
            _left_entries -= 1
            sce_codec_chain_has_tagged_entry.encode_source(_entry, w, override_ctl_more=1 if _left_entries != 0 else 0)
    if _e_priority is not None:
        w.write_u8(_e_priority & 0xFF)
    if _e_checksum is not None:
        w.write_u8((_e_checksum >> 8) & 0xFF)
        w.write_u8(_e_checksum & 0xFF)


@dataclass
class CodecChainHasTagged:
    header: CodecChainHasTaggedHeaderInt = 0
    entries_has_0x7_has_0x9_header_wide: CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWide = field(default_factory=CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideNeither)

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "header":
            value = CodecChainHasTaggedHeaderInt(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor) -> CodecChainHasTagged:
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
        f_entries = []
        _more = False
        for _ in range(3):
            _elem = CodecChainHasTaggedEntry.decode(cursor)
            _more = _elem.more()
            f_entries.append(_elem)
            if not _more:
                break
        if _more and cursor.remaining() == 0:
            raise NeedMoreBytes()
        if _more:
            raise TooManyElements()
        _has_entries_7 = any(_e.entry_type == 7 for _e in f_entries)
        _has_entries_9 = any(_e.entry_type == 9 for _e in f_entries)
        if _has_entries_7 or (f_header & 0x01) != 0:
            raw = cursor.peek_slice(1)
            _v = raw[0]
            cursor.advance(1)
            f_priority = _v
        else:
            f_priority = None
        if not _has_entries_9:
            raw = cursor.peek_slice(2)
            _v = (raw[0] << 8) | raw[1]
            cursor.advance(2)
            f_checksum = _v
        else:
            f_checksum = None
        return cls(
            header=f_header,
            entries_has_0x7_has_0x9_header_wide=CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Has0x9Wide(entries=CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Has0x9WideChain(f_entries), priority=f_priority) if ((f_entries is not None and any(_x.entry_type == 0x7 for _x in f_entries))) and ((f_entries is not None and any(_x.entry_type == 0x9 for _x in f_entries))) and (((f_header >> 0) & 1) != 0) else CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Has0x9(entries=CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Has0x9Chain(f_entries), priority=f_priority) if ((f_entries is not None and any(_x.entry_type == 0x7 for _x in f_entries))) and ((f_entries is not None and any(_x.entry_type == 0x9 for _x in f_entries))) and not (((f_header >> 0) & 1) != 0) else CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Wide(entries=CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7WideChain(f_entries), priority=f_priority, checksum=f_checksum) if ((f_entries is not None and any(_x.entry_type == 0x7 for _x in f_entries))) and not ((f_entries is not None and any(_x.entry_type == 0x9 for _x in f_entries))) and (((f_header >> 0) & 1) != 0) else CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x9Wide(entries=CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x9WideChain(f_entries), priority=f_priority) if not ((f_entries is not None and any(_x.entry_type == 0x7 for _x in f_entries))) and ((f_entries is not None and any(_x.entry_type == 0x9 for _x in f_entries))) and (((f_header >> 0) & 1) != 0) else CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7(entries=CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x7Chain(f_entries), priority=f_priority, checksum=f_checksum) if ((f_entries is not None and any(_x.entry_type == 0x7 for _x in f_entries))) and not ((f_entries is not None and any(_x.entry_type == 0x9 for _x in f_entries))) and not (((f_header >> 0) & 1) != 0) else CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x9(entries=CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideHas0x9Chain(f_entries)) if not ((f_entries is not None and any(_x.entry_type == 0x7 for _x in f_entries))) and ((f_entries is not None and any(_x.entry_type == 0x9 for _x in f_entries))) and not (((f_header >> 0) & 1) != 0) else CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideWide(entries=CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideWideChain(f_entries), priority=f_priority, checksum=f_checksum) if not ((f_entries is not None and any(_x.entry_type == 0x7 for _x in f_entries))) and not ((f_entries is not None and any(_x.entry_type == 0x9 for _x in f_entries))) and (((f_header >> 0) & 1) != 0) else CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideNeither(entries=CodecChainHasTaggedEntriesHas0x7Has0x9HeaderWideNeitherChain(f_entries), checksum=f_checksum),
        )

    # RFC §synth-5-B flags primitive: per-bit-range accessors over
    # the carrier field. Single-bit (width=1) reads as bool; multi-bit
    # (width>=2) reads as ``int`` (Python ints are unbounded, so a single
    # ``int`` covers every result-type width). Setters mask + shift on
    # the way in so out-of-range callers can't corrupt sibling bits.
    # Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Wire layout is unchanged.
    def wide(self) -> bool:
        return (self.header & 0x01) != 0

    def set_wide(self, v: bool) -> None:
        if v:
            self.header = (self.header | 0x01) & 0xFF
        else:
            self.header = self.header & (0xFF ^ 0x01)

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
