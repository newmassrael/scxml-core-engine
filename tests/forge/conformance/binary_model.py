# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""An independent model of the positional codec, and the byte cases it writes.

`SCE_FORGE.md` §4.6 writes the wire rules of a positional codec once, and seven
backends implement them. This file is another reading of the same page, written
from the page and the fixture's document alone (docs/adr/0081 §5): it never
looks at a runtime, a template, the generator or a generated file. It takes the
fields of a codec from the codec's own document, so a change to the document
moves the model with it.

A case states a value as the codec's interface presents it, its `source`: one
member for each element the value presents, named by its field id. An element
the encoding determines is not a member, because no value states it; the model
derives it, as the encoder must. Which element is which is decided by
declaration, as docs/adr/0058 §1 writes it: the field a `sce:length-field` names
is the encoded length of what it bounds, and the flag a one-flag
`sce:present-if` names is set exactly where the field is present. A length whose
content is gated while the length is not is derived where the content is there
and stated by the value where it is not, so the member is `null` in the one
case and the length in the other.

The model writes the frame of each `source`, reads the frame back, and requires
the `source` again, so every case it writes holds against the model before it is
written. A backend that disagrees with a case reads the page differently from
this file, and which of the two is wrong is decided against the page.

What the model reads so far, and refuses by name past it:

* an unsigned field of a fixed width (`sce:bit-size` a number of bits), whole
  bytes in the codec's byte order, or bits of a byte it shares with the fields
  at the same `sce:byte` (`sce:bit-offset`);
* an unsigned field written as a VLE (`sce:bit-size="vle"`): seven bits a byte,
  low group first, the high bit set on every byte but the last, and the ninth
  byte, where a value reaches it, all eight bits
  (docs/spec/synth/rfc-sce-protocol-synthesis.md, Appendix B);
* a flag set (`<sce:flags>`), of a fixed width or a VLE, presented whole: its
  value is the carrier, with each flag the encoding derives set by the encoder;
* bytes or text (`sce:type="bytes"`, `"string"`, the text as UTF-8) whose
  length a field before it says (`sce:bit-size="length-ref"`), a sibling or a
  flag of a carrier (`sce:length-field="carrier.flag"`), less the
  `sce:length-arith` offset on the wire, within `sce:max-size`;
* bytes or text to the end of the frame (`sce:bit-size="tail"`), within
  `sce:max-size`;
* a field gated on one flag of a carrier (`sce:present-if="c.f"` or `"!c.f"`),
  present exactly where the gate holds; one element to a flag, since two or
  more form a group (docs/adr/0065 §6 R4b), which is not read yet. A stated
  length counts on the side where its content is not, so a stated length with
  its gated content is a group too.

Each fixture this module writes is listed in FIXTURES with its hand-chosen
sources; `regenerate` writes them into `numerical_reference.json` under the
fixture's `sources`, adding the key, and the fixture's entry, where the file
has none.
"""

from __future__ import annotations

import json
import xml.etree.ElementTree as ET
from dataclasses import dataclass, field
from pathlib import Path

RESOURCES = Path("tests/forge/resources")

SCE = "{http://sce.dev/ext}"
SCXML = "{http://www.w3.org/2005/07/scxml}"

WIDTH = {"uint8": 8, "uint16": 16, "uint32": 32, "uint64": 64}
SEQUENCE = ("bytes", "string")

#: The attributes of a field the model reads; any other is refused by name, so a
#: rule the document states is never silently left out of a case.
READ_ATTRIBUTES = {
    "id",
    f"{SCE}type",
    f"{SCE}byte",
    f"{SCE}bit-size",
    f"{SCE}bit-offset",
    f"{SCE}endian",
    f"{SCE}present-if",
    f"{SCE}length-field",
    f"{SCE}length-arith",
    f"{SCE}max-size",
}

#: The attributes of the document root the model reads.
READ_ROOT_ATTRIBUTES = {"name", f"{SCE}kind", f"{SCE}default-endian"}

#: The children of the document root besides its datamodel that say nothing of
#: the wire: why the document is a codec, and its own decoded examples.
SILENT_CHILDREN = {f"{SCE}kind-basis", f"{SCE}test-vector"}


class Unread(Exception):
    """A shape of the document this model does not read yet."""


@dataclass
class Flag:
    name: str
    bit: int
    width: int


@dataclass
class Gate:
    """`sce:present-if="carrier.flag"`, or its negation."""

    carrier: str
    flag: str
    when_clear: bool


@dataclass
class Field:
    id: str
    sce_type: str
    #: `fixed`, `vle`, `length-ref` or `tail`.
    size: str
    #: The bits of a fixed field, or the bits a VLE holds.
    bits: int
    byte: int
    bit_offset: int | None
    endian: str
    carrier: bool = False
    flags: list[Flag] = field(default_factory=list)
    gate: Gate | None = None
    #: The sibling id or `carrier.flag` a `length-ref` field's length is in.
    length_field: str | None = None
    #: What the wire's length is short of the field's (`sce:length-arith`).
    arith: int = 0
    max_size: int | None = None

    def shares_a_byte(self) -> bool:
        return self.size == "fixed" and (self.bit_offset is not None or self.bits < 8)


@dataclass
class Codec:
    name: str
    fields: list[Field]

    def field(self, id: str) -> Field:
        return next(f for f in self.fields if f.id == id)

    def flag(self, carrier: str, name: str) -> Flag:
        return next(f for f in self.field(carrier).flags if f.name == name)

    def measured(self) -> dict[str, list[Field]]:
        """Each `sce:length-field` named, and the fields whose length it says."""
        out: dict[str, list[Field]] = {}
        for f in self.fields:
            if f.length_field is not None:
                out.setdefault(f.length_field, []).append(f)
        return out

    def is_length(self, f: Field) -> bool:
        return f.id in self.measured()

    def stated(self, f: Field) -> bool:
        """A length not gated itself whose every content is gated: the value
        states it where the content is absent (docs/adr/0058 §1)."""
        contents = self.measured().get(f.id, [])
        return f.gate is None and bool(contents) and all(c.gate is not None for c in contents)

    def presented(self, f: Field) -> bool:
        return not self.is_length(f) or self.stated(f)

    def gate_flags(self) -> dict[tuple[str, str], list[Field]]:
        """Each flag a gate of one flag names, and the presented fields it gates:
        the encoder sets it from their presence."""
        out: dict[tuple[str, str], list[Field]] = {}
        for f in self.fields:
            if f.gate is not None and self.presented(f):
                out.setdefault((f.gate.carrier, f.gate.flag), []).append(f)
        return out

    def decided(self) -> dict[tuple[str, str], list[Field]]:
        """Each flag a gate of one flag names, and every element of the value
        its gate decides: the presented fields it gates, and each stated length
        whose content it gates, which the value holds on the side where the
        content is not (`SCE_FORGE.md` §4.6, docs/adr/0065 §6 R4b)."""
        out = {key: list(gated) for key, gated in self.gate_flags().items()}
        for f in self.fields:
            if self.stated(f):
                for c in self.measured()[f.id]:
                    out.setdefault((c.gate.carrier, c.gate.flag), []).append(f)
        return out

    def check(self) -> None:
        """Refuse what the model does not read among the relations of fields."""
        for (carrier, name), decided in self.decided().items():
            if self.flag(carrier, name).width != 1:
                raise Unread(f"{self.name}: a gate on the {name} flag, wider than one bit")
            if len(decided) > 1:
                raise Unread(
                    f"{self.name}: the gate {carrier}.{name} decides "
                    f"{', '.join(g.id for g in decided)}, a group"
                )
        for (carrier, name) in self.gate_flags():
            if f"{carrier}.{name}" in self.measured():
                raise Unread(f"{self.name}: {carrier}.{name} is derived by two kinds")
        for name in self.measured():
            carrier, _, flag = name.partition(".")
            if flag:
                self.flag(carrier, flag)
            elif self.field(name).sce_type not in WIDTH:
                raise Unread(f"{self.name}: the length {name} is not an unsigned number")


def attribute_int(node, name: str) -> int | None:
    value = node.get(f"{SCE}{name}")
    return int(value, 0) if value is not None else None


def read_codec(stem: str) -> Codec:
    """The codec `stem` of the resources, as its document declares it."""
    root = ET.parse(RESOURCES / f"{stem}.scxml").getroot()
    default_endian = root.get(f"{SCE}default-endian", "big")
    if extra := sorted(set(root.attrib) - READ_ROOT_ATTRIBUTES):
        raise Unread(f"{stem}: the root declares {', '.join(extra)}, which is not read yet")
    if root.get(f"{SCE}kind") != "codec":
        raise Unread(f"{stem}: not a codec")
    for child in root:
        if child.tag != f"{SCXML}datamodel" and child.tag not in SILENT_CHILDREN:
            raise Unread(f"{stem}: <{child.tag}> is not read yet")
    datamodel = root.find(f"{SCXML}datamodel")
    if datamodel is None:
        raise Unread(f"{stem}: no datamodel")
    fields = []
    for node in datamodel:
        tag = node.tag
        if tag not in (f"{SCE}flags", f"{SCE}field", f"{SCXML}data"):
            raise Unread(f"{stem}: <{tag}> is not read yet")
        id = node.get("id")
        if extra := sorted(set(node.attrib) - READ_ATTRIBUTES):
            raise Unread(f"{stem}: `{id}` declares {', '.join(extra)}, which is not read yet")
        sce_type = node.get(f"{SCE}type")
        size_text = node.get(f"{SCE}bit-size")
        if sce_type in SEQUENCE:
            if size_text not in ("length-ref", "tail"):
                raise Unread(f"{stem}: `{id}` is {sce_type} of bit-size {size_text!r}")
            size, bits = size_text, 0
        elif sce_type in WIDTH:
            if size_text == "vle":
                size, bits = "vle", WIDTH[sce_type]
            elif size_text is not None and size_text.isdigit():
                size, bits = "fixed", int(size_text)
                if bits > WIDTH[sce_type] or (bits > 8 and bits % 8):
                    raise Unread(f"{stem}: `{id}` has {bits} bits")
            else:
                raise Unread(f"{stem}: bit-size {size_text!r} of `{id}` is not read yet")
        else:
            raise Unread(f"{stem}: type {sce_type!r} of `{id}` is not read yet")
        gate = None
        if (present_if := node.get(f"{SCE}present-if")) is not None:
            text = present_if.strip()
            when_clear = text.startswith("!")
            carrier, _, flag = text.lstrip("!").partition(".")
            if not flag or "|" in text or "&" in text or "(" in text:
                raise Unread(f"{stem}: gate {present_if!r} is not one flag of a carrier")
            gate = Gate(carrier.strip(), flag.strip(), when_clear)
        flags = []
        for f in node:
            if f.tag != f"{SCE}flag" or set(f.attrib) - {"name", "bit", "width"}:
                raise Unread(f"{stem}: a flag of `{id}` declares what is not read yet")
            flags.append(Flag(f.get("name"), int(f.get("bit")), int(f.get("width", "1"))))
        length_field = node.get(f"{SCE}length-field")
        if (size == "length-ref") != (length_field is not None):
            raise Unread(f"{stem}: `{id}` pairs length-ref and length-field otherwise")
        fields.append(
            Field(
                id=id,
                sce_type=sce_type,
                size=size,
                bits=bits,
                byte=int(node.get(f"{SCE}byte")),
                bit_offset=attribute_int(node, "bit-offset"),
                endian=node.get(f"{SCE}endian", default_endian),
                carrier=tag == f"{SCE}flags",
                flags=flags,
                gate=gate,
                length_field=length_field,
                arith=attribute_int(node, "length-arith") or 0,
                max_size=attribute_int(node, "max-size"),
            )
        )
    codec = Codec(root.get("name"), fields)
    codec.check()
    return codec


def vle(value: int) -> bytes:
    """Appendix B: at most eight bytes of seven bits each with the high bit set,
    then one byte of all eight."""
    out = bytearray()
    while value > 0x7F and len(out) < 8:
        out.append((value & 0x7F) | 0x80)
        value >>= 7
    out.append(value & 0xFF)
    return bytes(out)


def read_vle(frame: bytes, at: int) -> tuple[int, int]:
    value = 0
    for i in range(9):
        byte = frame[at + i]
        if i == 8:
            return value | byte << 56, at + 9
        value |= (byte & 0x7F) << (7 * i)
        if byte & 0x80 == 0:
            return value, at + i + 1
    raise AssertionError("unreachable")


def gate_holds(codec: Codec, gate: Gate, carriers: dict[str, int]) -> bool:
    flag = codec.flag(gate.carrier, gate.flag)
    set_ = (carriers[gate.carrier] >> flag.bit) & 1 == 1
    return set_ != gate.when_clear


def content(f: Field, value) -> bytes:
    """The bytes a sequence field's member states."""
    if f.sce_type == "string":
        if not isinstance(value, str):
            raise ValueError(f"`{f.id}` is text, a JSON string")
        return value.encode("utf-8")
    if not isinstance(value, list) or any(not 0 <= b <= 0xFF for b in value):
        raise ValueError(f"`{f.id}` is bytes, an array of numbers in 0..=255")
    return bytes(value)


def said_length(codec: Codec, name: str, contents: dict[str, bytes]) -> int | None:
    """What the length `name` says on the wire, from the contents it measures
    that are present; `None` where none is. Every present one must agree."""
    lengths = set()
    for f in codec.measured()[name]:
        if f.id in contents:
            n = len(contents[f.id])
            if f.max_size is not None and n > f.max_size:
                raise ValueError(f"{codec.name}: `{f.id}` holds {n} bytes, past {f.max_size}")
            lengths.add(n - f.arith)
    if len(lengths) > 1:
        raise ValueError(f"{codec.name}: the contents `{name}` measures disagree")
    if not lengths:
        return None
    wire = lengths.pop()
    if wire < 0:
        raise ValueError(f"{codec.name}: a length below what `{name}` can say")
    return wire


def encode(codec: Codec, source: dict) -> bytes:
    """The frame of `source`: the fields in declaration order, a field gated off
    left out, each derived element computed from what the value presents, and
    the fields that share a byte packed into it."""
    presented = [f.id for f in codec.fields if codec.presented(f)]
    if sorted(source) != sorted(presented):
        raise ValueError(f"{codec.name}: a source states {presented}, not {sorted(source)}")
    contents = {
        f.id: content(f, source[f.id])
        for f in codec.fields
        if f.sce_type in SEQUENCE and source[f.id] is not None
    }
    carriers: dict[str, int] = {}
    for f in codec.fields:
        if not f.carrier:
            continue
        value = source[f.id]
        for flag in f.flags:
            mask = ((1 << flag.width) - 1) << flag.bit
            gated = codec.gate_flags().get((f.id, flag.name))
            if gated is not None:
                (g,) = gated
                set_ = (source[g.id] is not None) != g.gate.when_clear
                value = (value & ~mask) | (mask if set_ else 0)
            elif f"{f.id}.{flag.name}" in codec.measured():
                wire = said_length(codec, f"{f.id}.{flag.name}", contents)
                if wire is None:
                    continue
                if wire >= 1 << flag.width:
                    raise ValueError(f"{codec.name}: {wire} past the {flag.name} flag")
                value = (value & ~mask) | (wire << flag.bit)
        carriers[f.id] = value
    out = bytearray()
    packed: dict[int, int] = {}
    for f in codec.fields:
        if f.gate is not None:
            holds = gate_holds(codec, f.gate, carriers)
            if codec.presented(f) and holds != (source[f.id] is not None):
                raise ValueError(f"{codec.name}: `{f.id}` disagrees with its gate")
            if not holds:
                continue
        if f.carrier:
            value = carriers[f.id]
        elif codec.is_length(f):
            value = said_length(codec, f.id, contents)
            if value is None:
                value = source[f.id] if codec.stated(f) else None
            elif codec.stated(f) and source[f.id] is not None:
                raise ValueError(f"{codec.name}: `{f.id}` is stated where it is derived")
            if value is None:
                raise ValueError(f"{codec.name}: nothing says `{f.id}`")
        elif f.sce_type in SEQUENCE:
            data = contents[f.id]
            if f.max_size is not None and len(data) > f.max_size:
                raise ValueError(f"{codec.name}: `{f.id}` holds more than {f.max_size} bytes")
            out += data
            continue
        else:
            value = source[f.id]
        if not isinstance(value, int) or value < 0 or value >= 1 << f.bits:
            raise ValueError(f"{codec.name}: `{f.id}` = {value!r} has more than {f.bits} bits")
        if f.size == "vle":
            out += vle(value)
        elif f.shares_a_byte():
            packed[f.byte] = packed.get(f.byte, 0) | (value << (f.bit_offset or 0))
            if last_of_its_byte(codec, f):
                out.append(packed[f.byte])
        else:
            out += value.to_bytes(f.bits // 8, "big" if f.endian == "big" else "little")
    return bytes(out)


def last_of_its_byte(codec: Codec, f: Field) -> bool:
    return not any(
        g.shares_a_byte() and g.byte == f.byte and codec.fields.index(g) > codec.fields.index(f)
        for g in codec.fields
    )


def decode(codec: Codec, frame: bytes) -> dict:
    """The `source` a frame says, the inverse of [`encode`]: a derived element
    is not a member, a stated length is a member only where its content is
    absent, and a carrier is presented as read."""
    raw: dict = {}
    carriers: dict[str, int] = {}
    at = 0
    for f in codec.fields:
        if f.gate is not None and not gate_holds(codec, f.gate, carriers):
            raw[f.id] = None
            continue
        if f.sce_type in SEQUENCE:
            if f.size == "tail":
                n = len(frame) - at
            else:
                carrier, _, flag = f.length_field.partition(".")
                if flag:
                    bits = codec.flag(carrier, flag)
                    said = (carriers[carrier] >> bits.bit) & ((1 << bits.width) - 1)
                else:
                    said = raw[carrier]
                n = said + f.arith
            if n < 0 or (f.max_size is not None and n > f.max_size):
                raise ValueError(f"{codec.name}: `{f.id}` of {n} bytes is past its bound")
            if at + n > len(frame):
                raise ValueError(f"{codec.name}: the frame ends inside `{f.id}`")
            data = frame[at : at + n]
            at += n
            raw[f.id] = data.decode("utf-8") if f.sce_type == "string" else list(data)
            continue
        if f.size == "vle":
            value, at = read_vle(frame, at)
        elif f.shares_a_byte():
            value = (frame[at] >> (f.bit_offset or 0)) & ((1 << f.bits) - 1)
            if last_of_its_byte(codec, f):
                at += 1
        else:
            n = f.bits // 8
            value = int.from_bytes(frame[at : at + n], "big" if f.endian == "big" else "little")
            at += n
        raw[f.id] = value
        if f.carrier:
            carriers[f.id] = value
    if at != len(frame):
        raise ValueError(f"{codec.name}: {len(frame) - at} bytes left over")
    source: dict = {}
    for f in codec.fields:
        if not codec.presented(f):
            continue
        if codec.stated(f) and any(raw[c.id] is not None for c in codec.measured()[f.id]):
            source[f.id] = None
        else:
            source[f.id] = raw[f.id]
    return source


@dataclass
class Written:
    """The sources of a fixture with their frames, each held by the model."""

    sources: list[dict]


def write(stem: str, sources: list[tuple[dict, str]]) -> Written:
    codec = read_codec(stem)
    out = []
    for source, note in sources:
        frame = encode(codec, source)
        again = decode(codec, frame)
        if again != source:
            raise AssertionError(f"{stem}: {source} reads back as {again}")
        out.append({"source": source, "encoded": list(frame), "note": note})
    return Written(out)


@dataclass
class Fixture:
    """A fixture's hand-chosen sources, and what its entry says where the model
    writes the entry itself."""

    sources: list[tuple[dict, str]]
    description: str = ""


ABOUT = (
    "docs/adr/0081 §5: each source states the value as the codec's interface presents it, "
    "and its frame is written by tests/forge/conformance/binary_model.py, an independent "
    "reading of the page, which holds each to itself before writing it. "
)

ASCII = list(range(0x61, 0x61 + 26))


#: Each fixture this model writes, with its hand-chosen sources: every edge of
#: each rule it touches.
FIXTURES = {
    "codec_present_if_basic": Fixture(
        [
            ({"flags": 0x00, "seq": None}, "has_seq clear: the frame is the carrier alone"),
            ({"flags": 0x01, "seq": 0x1234}, "has_seq set: seq follows, big-endian"),
            ({"flags": 0xFE, "seq": None}, "the bits the value presents are kept beside the derived one"),
            ({"flags": 0xFF, "seq": 0x0000}, "a present seq of zero is still present"),
            ({"flags": 0x01, "seq": 0xFFFF}, "seq at its largest"),
        ]
    ),
    "codec_flags_basic": Fixture(
        [
            ({"header": 0x00}, "no flag set"),
            ({"header": 0xF0}, "every named flag set"),
            ({"header": 0x0F}, "only bits no flag names: the carrier is presented whole"),
            ({"header": 0xA5}, "reliable and drop set, with unnamed bits"),
        ]
    ),
    "codec_little_endian": Fixture(
        [
            ({"sensorId": 0, "value": 0, "status": 0}, "every field zero"),
            ({"sensorId": 7, "value": 0x1234, "status": 1}, "value low byte first"),
            ({"sensorId": 0xFF, "value": 0xFFFF, "status": 0xFF}, "every field at its largest"),
        ]
    ),
    "codec_simple_frame": Fixture(
        [
            ({"msgId": 0, "length": 0, "payload": 0}, "every field zero"),
            ({"msgId": 1, "length": 2, "payload": 0x1234}, "payload high byte first"),
            ({"msgId": 0xFF, "length": 0xFF, "payload": 0xFFFF}, "every field at its largest"),
        ]
    ),
    "codec_subbyte": Fixture(
        [
            ({"priority": 0, "channel": 0, "direction": 0}, "every field zero: one zero byte"),
            ({"priority": 7, "channel": 0, "direction": 0}, "priority alone, the top three bits"),
            ({"priority": 0, "channel": 7, "direction": 0}, "channel alone, bits two to four"),
            ({"priority": 0, "channel": 0, "direction": 3}, "direction alone, the low two bits"),
            ({"priority": 5, "channel": 2, "direction": 1}, "three fields in one byte"),
        ]
    ),
    "codec_tail": Fixture(
        [
            ({"msgId": 1, "status": 0, "payload": []}, "an empty tail: the header alone"),
            ({"msgId": 2, "status": 3, "payload": [10, 20, 30]}, "a tail of three bytes"),
            ({"msgId": 0xFF, "status": 0xFF, "payload": list(range(32))}, "a tail at its bound"),
        ]
    ),
    "codec_length_ref": Fixture(
        [
            ({"msgId": 1, "payload": []}, "an empty payload: its length is zero"),
            ({"msgId": 2, "payload": [0xAA]}, "the length is derived from the payload"),
            ({"msgId": 3, "payload": list(range(32))}, "a payload at its bound"),
        ]
    ),
    "codec_vle_zint_u64": Fixture(
        [
            ({"value": 0}, "zero: one byte"),
            ({"value": 0x7F}, "the largest value of one byte"),
            ({"value": 0x80}, "the smallest value of two bytes"),
            ({"value": 0x3FFF}, "the largest value of two bytes"),
            ({"value": 0x4000}, "the smallest value of three bytes"),
            ({"value": (1 << 56) - 1}, "the largest value of eight bytes"),
            ({"value": 1 << 56}, "the smallest value of nine bytes"),
            ({"value": 1 << 63}, "bit 63 rides in the ninth byte's high bit"),
            ({"value": (1 << 64) - 1}, "the largest value: the ninth byte is all eight bits"),
        ]
    ),
    "codec_zenoh_close": Fixture(
        [
            ({"reason": 0}, "reason zero"),
            ({"reason": 0xFF}, "reason at its largest"),
        ]
    ),
    "codec_zenoh_keep_alive": Fixture([({}, "a codec of no field writes nothing")]),
    "codec_zenoh_put": Fixture([({"payload": 0x5A}, "one byte")]),
    "codec_zenoh_del": Fixture([({}, "a codec of no field writes nothing")]),
    "codec_zenoh_decl_final": Fixture([({}, "a codec of no field writes nothing")]),
    "codec_zenoh_ext_unit": Fixture(
        [({}, "a codec of no field writes nothing")],
        "a codec of no field: its value states nothing and its frame is empty.",
    ),
    "codec_zenoh_frame": Fixture(
        [
            ({"sn": 0, "payload": []}, "sn zero and no payload"),
            ({"sn": 0x80, "payload": [0xDE, 0xAD]}, "a two-byte sn, then the tail"),
            ({"sn": (1 << 64) - 1, "payload": [1]}, "a nine-byte sn"),
        ]
    ),
    "codec_zenoh_fragment": Fixture(
        [
            ({"sn": 0x7F, "payload": [0xCA, 0xFE]}, "a one-byte sn, then the tail"),
            ({"sn": 0x4000, "payload": []}, "a three-byte sn and no payload"),
        ]
    ),
    "codec_qos_byte": Fixture(
        [
            ({"qos": 0x00}, "no flag set"),
            ({"qos": 0x5D}, "priority 5, reliable, congestion 1, express"),
            ({"qos": 0xFF}, "every bit set"),
        ]
    ),
    "codec_zenoh_locator": Fixture(
        [
            ({"locator": ""}, "empty text: its length is zero"),
            ({"locator": "tcp/127.0.0.1:7447"}, "the length is derived from the text"),
            ({"locator": "héllo"}, "the length counts the bytes of the UTF-8, not the characters"),
            ({"locator": "".join(chr(c) for c in ASCII) * 4 + "abcdefghijklmnopqrstuvwx"},
             "text at its bound of 128 bytes: the length takes two VLE bytes"),
        ]
    ),
    "codec_length_ref_dotted_basic": Fixture(
        [
            ({"carrier": 0x00, "payload": []}, "an empty payload: the length bits are zero"),
            ({"carrier": 0x21, "payload": [0xAA, 0xBB]}, "the length bits are derived, hdr kept"),
            ({"carrier": 0xF5, "payload": list(range(15))}, "a payload at its bound of fifteen"),
        ]
    ),
    "codec_declared_bounds_gated": Fixture(
        [
            ({"flags": 0x00, "name": None, "rest": []}, "G clear: no name and no length"),
            ({"flags": 0x01, "name": "abc", "rest": [1, 2]}, "a name and a tail at their bounds"),
            ({"flags": 0x01, "name": "", "rest": []}, "an empty name is still present"),
            ({"flags": 0xFE, "name": None, "rest": [9]}, "the bits the value presents are kept"),
        ]
    ),
    "codec_origin_leaf": Fixture(
        [
            ({"data": []}, "empty: its length is zero"),
            ({"data": list(range(16))}, "at its bound"),
        ],
        "one length-ref field: its length is derived from the bytes.",
    ),
    "codec_origin_scalar": Fixture(
        [({"value": 0}, "zero"), ({"value": 0xFF}, "at its largest")],
        "one byte.",
    ),
    "codec_chain_has_slice": Fixture(
        [
            ({"kind": 0, "bytes": []}, "a one-byte kind and no bytes"),
            ({"kind": 0xFFFF, "bytes": list(range(128))},
             "a three-byte kind; 128 bytes take a two-byte VLE length"),
        ],
        "a VLE kind and a VLE length of the bytes, derived from them.",
    ),
    "codec_chain_has_marker_slice": Fixture(
        [
            ({"kind": 4, "data": [1, 2, 3]}, "the size is derived from the data"),
            ({"kind": 0x80, "data": []}, "a two-byte kind and no data"),
        ],
        "a VLE kind and a VLE size of the data, derived from them.",
    ),
    "codec_chain_has_tagged_entry": Fixture(
        [
            ({"entry_type": 1, "ctl": 0x00, "body": 7}, "more clear"),
            ({"entry_type": 2, "ctl": 0x01, "body": 0xFF}, "more set: presented with the carrier"),
        ],
        "a flag set no gate reads is presented whole.",
    ),
    "codec_zenoh_value_slice": Fixture(
        [
            ({"kind": 0, "data": []}, "no data"),
            ({"kind": 0xFF, "data": list(range(16))}, "data at its bound"),
        ],
        "a kind, then bytes whose VLE length is derived from them.",
    ),
    "codec_ext_attachment": Fixture(
        [
            ({"body": []}, "an empty body"),
            ({"body": list(range(64))}, "a body at its bound"),
        ],
        "bytes whose one-byte length is derived from them.",
    ),
    "codec_ext_timestamp": Fixture(
        [
            ({"time": 0, "zid": []}, "time zero and no zid"),
            ({"time": 0x3FFF, "zid": list(range(16))}, "a two-byte time and a zid at its bound"),
        ],
        "a VLE time, then a zid whose length is derived from it.",
    ),
    "codec_present_if_negation": Fixture(
        [
            ({"flags": 0x00, "seq": 5}, "absent_seq clear: seq follows"),
            ({"flags": 0x01, "seq": None}, "absent_seq set: no seq"),
            ({"flags": 0xFE, "seq": 0xFFFF}, "the bits the value presents are kept beside the derived one"),
            ({"flags": 0xFF, "seq": None}, "every bit set: no seq"),
        ],
        "the flag of a negated gate is set exactly where the field is absent.",
    ),
    "codec_present_if_string": Fixture(
        [
            ({"carrier": 0x00, "text": None}, "no text: the frame is the carrier alone"),
            ({"carrier": 0x01, "text": ""}, "empty text is present: its length is zero"),
            ({"carrier": 0x01, "text": "abc"}, "has_text and the length are derived"),
            ({"carrier": 0x01, "text": "héllo"}, "the length counts UTF-8 bytes"),
        ],
        "a length gated with its content follows the content: neither is a member but the text.",
    ),
    "codec_present_if_tail": Fixture(
        [
            ({"flags": 0x00, "payload": None}, "no payload"),
            ({"flags": 0x01, "payload": []}, "an empty payload is present"),
            ({"flags": 0x01, "payload": list(range(64))}, "a payload at its bound"),
        ],
        "a gated tail: its flag is derived from its presence.",
    ),
    "codec_present_if_vle": Fixture(
        [
            ({"flags": 0x00, "optional_id": None}, "no id"),
            ({"flags": 0x01, "optional_id": 0}, "an id of zero is present"),
            ({"flags": 0x01, "optional_id": (1 << 64) - 1}, "a nine-byte id"),
        ],
        "a gated VLE: its flag is derived from its presence.",
    ),
    "codec_scout_zid_body": Fixture(
        [
            ({"zid": [0x42]}, "the shortest zid, one byte: the wire says zero"),
            ({"zid": list(range(16))}, "a zid at its bound: the wire says fifteen"),
        ],
        "the wire holds the length less one (sce:length-arith=\"+1\"), so the zid is never empty.",
    ),
    "codec_zenoh_ext_zbuf": Fixture(
        [
            ({"value": []}, "no bytes"),
            ({"value": list(range(32))}, "bytes at their bound"),
        ],
        "bytes whose VLE length is derived from them.",
    ),
    "codec_zenoh_ext_zint": Fixture(
        [({"value": 0}, "zero"), ({"value": 0x80}, "two VLE bytes")],
        "one VLE.",
    ),
    "codec_zenoh_encoding": Fixture(
        [
            ({"packed_id": 0x00, "schema": None}, "no schema"),
            ({"packed_id": 0x01, "schema": "json"}, "a schema: has_schema and its length are derived"),
            ({"packed_id": 0x80, "schema": None}, "an id of two VLE bytes, no schema"),
        ],
        "a VLE flag set whose has_schema is derived from the schema, and a length gated with it.",
    ),
    "codec_zenoh_scout": Fixture(
        [
            ({"version": 2, "cbyte": 0x05, "zid": None}, "I clear: no zid"),
            ({"version": 2, "cbyte": 0x2D, "zid": [1, 2, 3]}, "I and zid_len_m1 are derived from the zid"),
            ({"version": 2, "cbyte": 0xFD, "zid": list(range(0x10, 0x20))}, "a zid at its bound"),
            ({"version": 2, "cbyte": 0xF0, "zid": None}, "no zid: the length bits are what the value presents"),
        ]
    ),
    "codec_zenoh_source_info": Fixture(
        [
            ({"header": 0x00, "zid": [9], "eid": 0, "sn": 0}, "a zid of one byte: the bits say zero"),
            ({"header": 0xF3, "zid": list(range(16)), "eid": 0xFFFFFFFF, "sn": 0x80},
             "a zid at its bound; a five-byte eid"),
        ],
        "a zid whose length less one is in four bits of the header, derived from it.",
    ),
    "codec_zenoh_undecl_kexpr": Fixture(
        [({"id": 0}, "id zero"), ({"id": 0xFFFF}, "the largest id: three VLE bytes")],
        "one VLE id.",
    ),
    "codec_repeat_elem": Fixture(
        [({"seq": 0}, "zero"), ({"seq": 0xABCD}, "two bytes, high first")],
        "the entry of a list, written alone: two bytes.",
    ),
    "codec_tlv_entry": Fixture(
        [
            ({"entry_type": 1, "entry_body": []}, "an empty body: its length is zero"),
            ({"entry_type": 0xFF, "entry_body": list(range(32))}, "a body at its bound"),
        ],
        "a type, then a body whose one-byte length is derived from it.",
    ),
    "codec_zenoh_timestamp": Fixture(
        [
            ({"time": 0, "zid": []}, "time zero and no zid"),
            ({"time": 0x80, "zid": list(range(16))}, "a two-byte time and a zid at its bound"),
        ],
        "a VLE time, then a zid whose VLE length is derived from it.",
    ),
    "codec_zenoh_decl_ext_keyexpr_inner": Fixture(
        [
            ({"inner_header": 0x00, "id": 0, "suffix": None}, "N clear: no suffix"),
            ({"inner_header": 0x03, "id": 5, "suffix": [0x61, 0x2F, 0x62]},
             "N derived from the suffix; M is the value's"),
            ({"inner_header": 0x02, "id": 0x80, "suffix": None},
             "M set, no suffix: the frame ends after the id"),
            ({"inner_header": 0x01, "id": 1, "suffix": []}, "an empty suffix is present"),
        ],
        "a gated tail whose flag is derived; the other flag is presented with the carrier.",
    ),
    "codec_variant_session_close": Fixture(
        [({"reason": 3}, "one byte")],
        "one byte.",
    ),
    "codec_variant_session_open": Fixture(
        [({"version": 0x0102}, "two bytes, high first")],
        "two bytes.",
    ),
}


def pascal(name: str) -> str:
    return "".join(part[:1].upper() + part[1:] for part in name.split("_"))


def codecs_span(text: str) -> tuple[int, int]:
    """(start, end) of the members of the `codecs` section."""
    opening = text.index('\n  "codecs": {') + len('\n  "codecs": {')
    return opening, text.index("\n  }", opening)


def entry_span(text: str, fixture: str) -> tuple[int, int] | None:
    """(start, end) of `fixture`'s entry in the `codecs` section, the end at the
    line that closes it; `None` where there is none."""
    start, end = codecs_span(text)
    anchor = text.find(f'\n    "{fixture}": {{', start, end)
    if anchor < 0:
        return None
    return anchor, text.index("\n    }", anchor)


def ensure_sources(text: str, fixture: str, description: str) -> str:
    """`text` with a `sources` key in `fixture`'s entry, and the entry itself
    where the file has none, to be filled by [`regenerate`]."""
    found = entry_span(text, fixture)
    if found is None:
        _, end = codecs_span(text)
        entry = (
            f',\n    "{fixture}": {{\n'
            f'      "fixture": "{pascal(fixture)}",\n'
            f"      \"description\": {json.dumps(ABOUT + description, ensure_ascii=True)},\n"
            '      "sources": [\n      ]\n    }'
        )
        return text[:end] + entry + text[end:]
    start, end = found
    if '"sources": [' in text[start:end]:
        return text
    return text[:end] + ',\n      "sources": [\n      ]' + text[end:]


def span(text: str, fixture: str, key: str):
    """(start, end) of the contents of `fixture`'s `key` array."""
    anchor = text.index(f'"{fixture}": {{')
    opening = text.index(f'"{key}": [', anchor) + len(f'"{key}": [')
    closing = text.index("\n      ]", opening)
    return opening, closing


def case_text(case: dict) -> str:
    return "        " + json.dumps(case, ensure_ascii=True)


def regenerate(text: str) -> tuple[str, str]:
    """`text` with each fixture's sources written, and a report."""
    reports = []
    for fixture, spec in FIXTURES.items():
        written = write(fixture, spec.sources)
        text = ensure_sources(text, fixture, spec.description)
        start, end = span(text, fixture, "sources")
        body = "\n" + ",\n".join(case_text(item) for item in written.sources)
        text = text[:start] + body + text[end:]
        reports.append(f"{fixture}: {len(written.sources)} sources; the model holds each")
    return text, "\n".join(reports)
