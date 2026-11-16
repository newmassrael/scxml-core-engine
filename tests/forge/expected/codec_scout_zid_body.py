# SCE-MAP: codec_scout_zid_body:35 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, LengthOutOfRange, NeedMoreBytes, SceCursor, SceSink
from sce_forge_runtime.bounded import BoundedBytes

from dataclasses import dataclass
from typing import Optional, Protocol


class CodecScoutZidBodyZidBytes(BoundedBytes):
    """The ``zid`` element of :class:`CodecScoutZidBody`, 1 to 16 bytes,
    held to that bound where a value of it is made (docs/adr/0077 §3)."""

    LEAST = 1
    MOST = 16


class CodecScoutZidBodySource(Protocol):
    """The elements a value of :class:`CodecScoutZidBody` presents, which its encode
    reads (docs/adr/0081 §2): any object that presents them is a source of it.
    The encode derives every other element of the frame."""

    @property
    def zid(self) -> CodecScoutZidBodyZidBytes: ...


def encode_source(src: CodecScoutZidBodySource, w: SceSink) -> None:
    """Write any source of :class:`CodecScoutZidBody` into ``w``, each element read
    off ``src`` and written where the codec's encode plan lays it out
    (docs/adr/0081). It derives ``zid_len_m1`` from what each measures, and never reads it off
    the source. It refuses no value; a bounded sink raises
    :class:`BufferOverflow` where it has no room left."""
    _d_zid_len_m1 = len(src.zid) - 1
    w.write_u8(_d_zid_len_m1 & 0xFF)
    w.write_bytes(src.zid)


@dataclass
class CodecScoutZidBody:
    zid: CodecScoutZidBodyZidBytes = CodecScoutZidBodyZidBytes(b"\x00" * 1)

    def __setattr__(self, name: str, value: object) -> None:
        # Each byte string and text is held to its bound wherever it is
        # set, where the value is made and after (docs/adr/0081 §2),
        # whatever it was handed.
        if name == "zid":
            value = CodecScoutZidBodyZidBytes(value)
        object.__setattr__(self, name, value)

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecScoutZidBody]:
        """Decode the next frame from ``cursor``. Returns ``None`` when
        the cursor's tail is shorter than the declared minimum frame
        (RFC §synth-5-B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``, and every codec on a field's refusal: a length
        or a count past its bound (``LengthOutOfRange``, ``TooManyElements``,
        docs/adr/0077), an undeclared enum value."""
        try:
            _frame_len = cursor.remaining()
            if _frame_len < 1:
                return None
            raw = cursor.peek_slice(_frame_len)
            f_zid_len_m1 = raw[0]
            if f_zid_len_m1 > 15:
                raise LengthOutOfRange()
            if _frame_len < 1 + f_zid_len_m1 + 1:
                return None
            f_zid = raw[1:1 + f_zid_len_m1 + 1]
        except CodecError:
            return None
        value = cls(
            zid=f_zid,
        )
        try:
            cursor.advance(_frame_len)
        except NeedMoreBytes:
            return None
        return value

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
