# SCE-MAP: codec_repeat_present_if_basic:37 :: _forge_body

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


class CodecRepeatPresentIfBasicElemsList(BoundedTuple):
    """The ``elems`` element of :class:`CodecRepeatPresentIfBasic`, 0 to 32 entries,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 0
    MOST = 32


class CodecRepeatPresentIfBasicSource(Protocol):
    """The elements a value of :class:`CodecRepeatPresentIfBasic` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def carrier(self) -> int: ...

    @property
    def elems(self) -> Optional[CodecRepeatPresentIfBasicElemsList]: ...


def encode_source(src: CodecRepeatPresentIfBasicSource, w: SceSink) -> None:
    """Write any source of :class:`CodecRepeatPresentIfBasic` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It derives ``num_elems`` from what each measures, and never reads it off
    the source. It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _d_num_elems = (len(src.elems)) if src.elems is not None else 0
    _c_carrier = src.carrier
    _c_carrier = (_c_carrier & ~0x1) | (0x1 if src.elems is not None else 0)
    w.write_u8(_c_carrier & 0xFF)
    if ((_c_carrier >> 0) & 1) != 0:
        w.write_u8(_d_num_elems & 0xFF)
    if src.elems is not None:
        for _entry in src.elems:
            sce_codec_repeat_elem.encode_source(_entry, w)


@dataclass
class CodecRepeatPresentIfBasic:
    carrier: int = 0
    elems: Optional[CodecRepeatPresentIfBasicElemsList] = None

    def __setattr__(self, name: str, value: object) -> None:
        # Each byte string and text is held to its bound wherever it is
        # set, where the value is made and after (docs/adr/0081 §2),
        # whatever it was handed.
        if name == "elems":
            value = None if value is None else CodecRepeatPresentIfBasicElemsList(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecRepeatPresentIfBasic]:
        """Decode the next frame from ``cursor``. Returns ``None`` when
        the cursor's tail is shorter than the declared minimum frame
        (RFC §synth-5-B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``, and every codec on a field's refusal: a length
        or a count past its bound (``LengthOutOfRange``, ``TooManyElements``,
        docs/adr/0077), an undeclared enum value."""
        # Streaming cursor decode (SSOT selection: `needs_streaming`).
        # The positional `raw[byte_off]` path is valid only when every
        # field's absolute offset is fixed at codegen time; this branch
        # handles every codec where it is not — present-if-gated fields
        # (runtime presence), VLE / repeat / TLV-chain / embed fields
        # (runtime width), string fields (UTF-8 decode), and a fixed field
        # after a variable-length payload (offset depends on the payload
        # length). Each field reads its own bytes and advances past what it
        # consumed, all inside one outer `try:`. The `except` catches the
        # base `CodecError` so a peek/advance `NeedMoreBytes` and a VLE
        # field's `VleWidthOverflow` both unwind to a single `return None`
        # (non-VLE codecs only ever raise `NeedMoreBytes`, a `CodecError`
        # subclass, so this is behaviour-identical for them).
        try:
            raw = cursor.peek_slice(1)
            f_carrier = raw[0]
            cursor.advance(1)
            if (f_carrier & 0x01) != 0:
                raw = cursor.peek_slice(1)
                _v = raw[0]
                cursor.advance(1)
                f_num_elems = _v
            else:
                f_num_elems = None
            if (f_carrier & 0x01) != 0:
                if f_num_elems > 32:
                    raise TooManyElements()
                f_elems = []
                for _ in range(f_num_elems):
                    _elem = CodecRepeatElem.decode(cursor)
                    if _elem is None:
                        return None
                    f_elems.append(_elem)
            else:
                f_elems = None
        except CodecError:
            return None
        return cls(
            carrier=f_carrier,
            elems=f_elems,
        )

    # RFC §synth-5-B flags primitive: per-bit-range accessors over
    # the carrier field. Single-bit (width=1) reads as bool; multi-bit
    # (width>=2) reads as ``int`` (Python ints are unbounded, so a single
    # ``int`` covers every result-type width). Setters mask + shift on
    # the way in so out-of-range callers can't corrupt sibling bits.
    # Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Wire layout is unchanged.
    def has_list(self) -> bool:
        return (self.carrier & 0x01) != 0

    def set_has_list(self, v: bool) -> None:
        if v:
            self.carrier = (self.carrier | 0x01) & 0xFF
        else:
            self.carrier = self.carrier & (0xFF ^ 0x01)

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
