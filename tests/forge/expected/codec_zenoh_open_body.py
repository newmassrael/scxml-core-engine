# SCE-MAP: codec_zenoh_open_body:42 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from sce_forge_runtime.bounded import BoundedBytes, BoundedInt

from dataclasses import dataclass
from typing import Optional, Protocol


class CodecZenohOpenBodyLeaseInt(BoundedInt):
    """The ``lease`` element of :class:`CodecZenohOpenBody`, an unsigned integer of
    64 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 64
    SIGNED = False


class CodecZenohOpenBodyInitialSnInt(BoundedInt):
    """The ``initial_sn`` element of :class:`CodecZenohOpenBody`, an unsigned integer of
    64 bits, held to them where a value of it is made (docs/adr/0081 §2)."""

    BITS = 64
    SIGNED = False


class CodecZenohOpenBodyCookieBytes(BoundedBytes):
    """The ``cookie`` element of :class:`CodecZenohOpenBody`, 0 or more bytes, the protocol stating no most,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 0
    MOST = None


class CodecZenohOpenBodySource(Protocol):
    """The elements a value of :class:`CodecZenohOpenBody` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def lease(self) -> CodecZenohOpenBodyLeaseInt: ...

    @property
    def initial_sn(self) -> CodecZenohOpenBodyInitialSnInt: ...

    @property
    def cookie(self) -> Optional[CodecZenohOpenBodyCookieBytes]: ...


def encode_source(src: CodecZenohOpenBodySource, w: SceSink) -> None:
    """Write any source of :class:`CodecZenohOpenBody` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It derives ``cookie_len`` from what each measures, and never reads it off
    the source. It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    a = (0 if src.cookie is not None else 1)
    _d_cookie_len = (len(src.cookie)) if src.cookie is not None else 0
    w.write_vle_u64(src.lease)
    w.write_vle_u64(src.initial_sn)
    if a == 0:
        w.write_vle_u64(_d_cookie_len)
    if src.cookie is not None:
        w.write_bytes(src.cookie)


def input_a(src: CodecZenohOpenBodySource) -> int:
    """The flag input ``A`` a source of :class:`CodecZenohOpenBody` says, which
    its encode is not handed (docs/adr/0065 §6 R4a)."""
    return (0 if src.cookie is not None else 1)


@dataclass
class CodecZenohOpenBody:
    lease: CodecZenohOpenBodyLeaseInt = 0
    initial_sn: CodecZenohOpenBodyInitialSnInt = 0
    cookie: Optional[CodecZenohOpenBodyCookieBytes] = None

    def __setattr__(self, name: str, value: object) -> None:
        # Each integer, byte string and text is held to its bound
        # wherever it is set, where the value is made and after
        # (docs/adr/0081 §2), whatever it was handed.
        if name == "lease":
            value = CodecZenohOpenBodyLeaseInt(value)
        elif name == "initial_sn":
            value = CodecZenohOpenBodyInitialSnInt(value)
        elif name == "cookie":
            value = None if value is None else CodecZenohOpenBodyCookieBytes(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor, a: int) -> CodecZenohOpenBody:
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
        f_lease = cursor.read_vle_u64()
        f_initial_sn = cursor.read_vle_u64()
        if (a & 0x01) == 0:
            _v = cursor.read_vle_u64()
            f_cookie_len = _v
        else:
            f_cookie_len = None
        if (a & 0x01) == 0:
            _len = f_cookie_len
            _n = _len
            raw = cursor.peek_slice(_n)
            _v = bytes(raw)
            cursor.advance(_n)
            f_cookie = _v
        else:
            f_cookie = None
        return cls(
            lease=f_lease,
            initial_sn=f_initial_sn,
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
