# SCE-MAP: codec_zenoh_wireexpr:55 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, InvalidUtf8, LengthOutOfRange, NeedMoreBytes, SceCursor, SceSink
from sce_forge_runtime.bounded import BoundedInt, BoundedText

from dataclasses import dataclass
from typing import Optional, Protocol


class CodecZenohWireexprIdInt(BoundedInt):
    """The ``id`` element of :class:`CodecZenohWireexpr`, an unsigned integer of
    64 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 64
    SIGNED = False


class CodecZenohWireexprSuffixText(BoundedText):
    """The ``suffix`` element of :class:`CodecZenohWireexpr`, 0 to 65535 UTF-8 bytes,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 0
    MOST = 65535


class CodecZenohWireexprSource(Protocol):
    """The elements a value of :class:`CodecZenohWireexpr` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def id(self) -> CodecZenohWireexprIdInt: ...

    @property
    def suffix(self) -> Optional[CodecZenohWireexprSuffixText]: ...


def encode_source(src: CodecZenohWireexprSource, w: SceSink) -> None:
    """Write any source of :class:`CodecZenohWireexpr` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It derives ``suffix_len`` from what each measures, and never reads it off
    the source. It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    n = (1 if src.suffix is not None else 0)
    _d_suffix_len = (len(src.suffix.encode("utf-8"))) if src.suffix is not None else 0
    w.write_vle_u64(src.id)
    if n != 0:
        w.write_vle_u64(_d_suffix_len)
    if src.suffix is not None:
        w.write_bytes(src.suffix.encode("utf-8"))


def input_n(src: CodecZenohWireexprSource) -> int:
    """The flag input ``N`` a source of :class:`CodecZenohWireexpr` says, which
    its encode is not handed (docs/adr/0065 §6 R4a)."""
    return (1 if src.suffix is not None else 0)


@dataclass
class CodecZenohWireexpr:
    id: CodecZenohWireexprIdInt = 0
    suffix: Optional[CodecZenohWireexprSuffixText] = None

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "id":
            value = CodecZenohWireexprIdInt(value)
        elif name == "suffix":
            value = None if value is None else CodecZenohWireexprSuffixText(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor, n: int) -> CodecZenohWireexpr:
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
        f_id = cursor.read_vle_u64()
        if (n & 0x01) != 0:
            _v = cursor.read_vle_u64()
            f_suffix_len = _v
        else:
            f_suffix_len = None
        if (n & 0x01) != 0:
            _len = f_suffix_len
            if _len > 65535:
                raise LengthOutOfRange()
            _n = _len
            raw = cursor.peek_slice(_n)
            try:
                _v = bytes(raw).decode('utf-8')
            except UnicodeDecodeError as exc:
                raise InvalidUtf8() from exc
            cursor.advance(_n)
            f_suffix = _v
        else:
            f_suffix = None
        return cls(
            id=f_id,
            suffix=f_suffix,
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
