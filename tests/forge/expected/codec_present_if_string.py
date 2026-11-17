# SCE-MAP: codec_present_if_string:47 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, InvalidUtf8, LengthOutOfRange, NeedMoreBytes, SceCursor, SceSink
from sce_forge_runtime.bounded import BoundedText

from dataclasses import dataclass
from typing import Optional, Protocol


class CodecPresentIfStringTextText(BoundedText):
    """The ``text`` element of :class:`CodecPresentIfString`, 0 to 32 UTF-8 bytes,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 0
    MOST = 32


class CodecPresentIfStringSource(Protocol):
    """The elements a value of :class:`CodecPresentIfString` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def carrier(self) -> int: ...

    @property
    def text(self) -> Optional[CodecPresentIfStringTextText]: ...


def encode_source(src: CodecPresentIfStringSource, w: SceSink) -> None:
    """Write any source of :class:`CodecPresentIfString` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It derives ``text_len`` from what each measures, and never reads it off
    the source. It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _d_text_len = (len(src.text.encode("utf-8"))) if src.text is not None else 0
    _c_carrier = src.carrier
    _c_carrier = (_c_carrier & ~0x1) | (0x1 if src.text is not None else 0)
    w.write_u8(_c_carrier & 0xFF)
    if ((_c_carrier >> 0) & 1) != 0:
        w.write_u8(_d_text_len & 0xFF)
    if src.text is not None:
        w.write_bytes(src.text.encode("utf-8"))


@dataclass
class CodecPresentIfString:
    carrier: int = 0
    text: Optional[CodecPresentIfStringTextText] = None

    def __setattr__(self, name: str, value: object) -> None:
        # Each byte string and text is held to its bound wherever it is
        # set, where the value is made and after (docs/adr/0081 §2),
        # whatever it was handed.
        if name == "text":
            value = None if value is None else CodecPresentIfStringTextText(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor) -> CodecPresentIfString:
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
        f_carrier = raw[0]
        cursor.advance(1)
        if (f_carrier & 0x01) != 0:
            raw = cursor.peek_slice(1)
            _v = raw[0]
            cursor.advance(1)
            f_text_len = _v
        else:
            f_text_len = None
        if (f_carrier & 0x01) != 0:
            _len = f_text_len
            if _len > 32:
                raise LengthOutOfRange()
            _n = _len
            raw = cursor.peek_slice(_n)
            try:
                _v = bytes(raw).decode('utf-8')
            except UnicodeDecodeError as exc:
                raise InvalidUtf8() from exc
            cursor.advance(_n)
            f_text = _v
        else:
            f_text = None
        return cls(
            carrier=f_carrier,
            text=f_text,
        )

    # RFC §synth-5-B flags primitive: per-bit-range accessors over
    # the carrier field. Single-bit (width=1) reads as bool; multi-bit
    # (width>=2) reads as ``int`` (Python ints are unbounded, so a single
    # ``int`` covers every result-type width). Setters mask + shift on
    # the way in so out-of-range callers can't corrupt sibling bits.
    # Plain methods (rather than @property) for API symmetry with
    # Rust / Cpp / Kotlin / Go / C11. Wire layout is unchanged.
    def has_text(self) -> bool:
        return (self.carrier & 0x01) != 0

    def set_has_text(self, v: bool) -> None:
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
