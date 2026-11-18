# SCE-MAP: codec_init_cookie_body:36 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, LengthOutOfRange, NeedMoreBytes, SceCursor, SceSink
from sce_forge_runtime.bounded import BoundedBytes, BoundedInt

from dataclasses import dataclass
from typing import Optional, Protocol


class CodecInitCookieBodyVersionInt(BoundedInt):
    """The ``version`` element of :class:`CodecInitCookieBody`, an unsigned integer of
    8 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 8
    SIGNED = False


class CodecInitCookieBodyCookieBytes(BoundedBytes):
    """The ``cookie`` element of :class:`CodecInitCookieBody`, 0 to 64 bytes,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 0
    MOST = 64


class CodecInitCookieBodySource(Protocol):
    """The elements a value of :class:`CodecInitCookieBody` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def version(self) -> CodecInitCookieBodyVersionInt: ...

    @property
    def cookie(self) -> Optional[CodecInitCookieBodyCookieBytes]: ...


def encode_source(src: CodecInitCookieBodySource, w: SceSink) -> None:
    """Write any source of :class:`CodecInitCookieBody` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It derives ``cookie_size`` from what each measures, and never reads it off
    the source. It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    a = (1 if src.cookie is not None else 0)
    _d_cookie_size = (len(src.cookie)) if src.cookie is not None else 0
    w.write_u8(src.version & 0xFF)
    if a != 0:
        w.write_vle_u16(_d_cookie_size)
    if src.cookie is not None:
        w.write_bytes(src.cookie)


def input_a(src: CodecInitCookieBodySource) -> int:
    """The flag input ``A`` a source of :class:`CodecInitCookieBody` says, which
    its encode is not handed (docs/adr/0065 §6 R4a)."""
    return (1 if src.cookie is not None else 0)


@dataclass
class CodecInitCookieBody:
    version: CodecInitCookieBodyVersionInt = 0
    cookie: Optional[CodecInitCookieBodyCookieBytes] = None

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "version":
            value = CodecInitCookieBodyVersionInt(value)
        elif name == "cookie":
            value = None if value is None else CodecInitCookieBodyCookieBytes(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor, a: int) -> CodecInitCookieBody:
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
        if (a & 0x01) != 0:
            _v = cursor.read_vle_u16()
            f_cookie_size = _v
        else:
            f_cookie_size = None
        if (a & 0x01) != 0:
            _len = f_cookie_size
            if _len > 64:
                raise LengthOutOfRange()
            _n = _len
            raw = cursor.peek_slice(_n)
            _v = bytes(raw)
            cursor.advance(_n)
            f_cookie = _v
        else:
            f_cookie = None
        return cls(
            version=f_version,
            cookie=f_cookie,
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
