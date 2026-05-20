# SCE-MAP: codec_repeat_present_if_basic:37

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.

from __future__ import annotations

from sce_forge_runtime.codec import BytearraySink, CodecError, NeedMoreBytes, SceCursor, SceSink
from .codec_repeat_elem import CodecRepeatElem

from dataclasses import dataclass, field
from typing import Optional, List


@dataclass
class CodecRepeatPresentIfBasic:
    carrier: int = 0
    num_elems: Optional[int] = None
    elems: Optional[List[CodecRepeatElem]] = None

    @classmethod
    def decode(cls, cursor: SceCursor) -> Optional[CodecRepeatPresentIfBasic]:
        """Decode the next frame from ``cursor``. Returns ``None`` when
        the cursor's tail is shorter than the declared minimum frame
        (RFC §5.B L494-519); on success the cursor advances past the
        consumed bytes. VLE codecs also return ``None`` on
        ``VleWidthOverflow``."""
        # RFC §5.B B1-δ + B2-β present-if primitive: streaming decode
        # advances the cursor per field. Per-field statements live
        # inside one outer `try:` block so the first peek/advance
        # failure unwinds to a single `except NeedMoreBytes`. Per-
        # field `is_repeat` routes Repeat fields to the dedicated
        # helper. Branch fires before has_vle_fields so a codec mixing
        # VLE + present-if uses the unified streaming path.
        try:
            raw = cursor.peek_slice(1)
            carrier = raw[0]
            cursor.advance(1)
            if (carrier & 0x01) != 0:
                raw = cursor.peek_slice(1)
                _v = raw[0]
                cursor.advance(1)
                num_elems = _v
            else:
                num_elems = None
            if (carrier & 0x01) != 0:
                elems = []
                for _ in range(num_elems):
                    _elem = CodecRepeatElem.decode(cursor)
                    if _elem is None:
                        return None
                    elems.append(_elem)
            else:
                elems = None
        except NeedMoreBytes:
            return None
        return cls(
            carrier=carrier,
            num_elems=num_elems,
            elems=elems,
        )

    # RFC §5.B B1-γ + B5-α flags primitive: per-bit-range accessors over
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
        """RFC §5.B B1-α encode-side primary: write ``self`` into the
        caller-owned ``w`` sink. Returns ``None`` on success; raises
        :class:`BufferOverflow` from a bounded sink when the destination
        has insufficient remaining capacity; growable sinks (e.g.
        :class:`BytearraySink`) are effectively infallible."""
        # RFC §5.B B1-δ + B2-β present-if encode.
        w.write_u8(self.carrier & 0xFF)
        if self.num_elems is not None:
            w.write_u8(self.num_elems & 0xFF)
        if self.elems is not None:
            for _e in self.elems:
                _e.encode(w)

    def encode_to_bytes(self) -> bytes:
        """Heap-backed convenience facade. Runs :meth:`encode` over a
        :class:`BytearraySink` and returns the freshly-encoded bytes.
        Callers targeting zero-alloc hot paths should call :meth:`encode`
        directly against a caller-owned sink."""
        _dst = bytearray()
        self.encode(BytearraySink(_dst))
        return bytes(_dst)
