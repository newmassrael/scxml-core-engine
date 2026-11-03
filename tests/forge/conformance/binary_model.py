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
derives it, as the encoder must. The model writes the frame of each `source`,
reads the frame back, and requires the `source` again, so every case it writes
holds against the model before it is written. A backend that disagrees with a
case reads the page differently from this file, and which of the two is wrong is
decided against the page.

What the model reads so far, and refuses by name past it:

* a field of a fixed width (`sce:bit-size` a number of bits), whole bytes in
  the codec's byte order, or bits of a byte it shares with the fields at the
  same `sce:byte` (`sce:bit-offset`);
* a flag set (`<sce:flags>`), presented whole: its value is the carrier;
* a field gated on one flag of a carrier (`sce:present-if="c.f"` or `"!c.f"`),
  present exactly where the gate holds. The flag is derived: the encoder sets
  it from the presence of the field, whatever bit the carrier presents there.

Each fixture this module writes is listed in FIXTURES with its hand-chosen
sources; `regenerate` writes them into `numerical_reference.json` under the
fixture's `sources`.
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
    byte: int
    bits: int
    bit_offset: int | None
    endian: str
    flags: list[Flag] = field(default_factory=list)
    gate: Gate | None = None


@dataclass
class Codec:
    name: str
    fields: list[Field]

    def field(self, id: str) -> Field:
        return next(f for f in self.fields if f.id == id)

    def derived_flags(self) -> dict[tuple[str, str], list[Field]]:
        """Each flag a gate of one flag names, and the fields it gates: the
        encoder sets it from their presence."""
        out: dict[tuple[str, str], list[Field]] = {}
        for f in self.fields:
            if f.gate is not None:
                out.setdefault((f.gate.carrier, f.gate.flag), []).append(f)
        return out


def read_codec(stem: str) -> Codec:
    """The codec `stem` of the resources, as its document declares it."""
    root = ET.parse(RESOURCES / f"{stem}.scxml").getroot()
    default_endian = root.get(f"{SCE}default-endian", "big")
    datamodel = root.find(f"{SCXML}datamodel")
    if datamodel is None:
        raise Unread(f"{stem}: no datamodel")
    fields = []
    for node in datamodel:
        tag = node.tag
        if tag not in (f"{SCE}flags", f"{SCE}field", f"{SCXML}data"):
            raise Unread(f"{stem}: <{tag}> is not read yet")
        sce_type = node.get(f"{SCE}type")
        if sce_type not in WIDTH:
            raise Unread(f"{stem}: type {sce_type!r} of `{node.get('id')}` is not read yet")
        size = node.get(f"{SCE}bit-size")
        if size is None or not size.isdigit():
            raise Unread(f"{stem}: bit-size {size!r} of `{node.get('id')}` is not read yet")
        offset = node.get(f"{SCE}bit-offset")
        gate = None
        if (present_if := node.get(f"{SCE}present-if")) is not None:
            text = present_if.strip()
            when_clear = text.startswith("!")
            carrier, _, flag = text.lstrip("!").partition(".")
            if not flag or "|" in text or "(" in text:
                raise Unread(f"{stem}: gate {present_if!r} is not one flag of a carrier")
            gate = Gate(carrier.strip(), flag.strip(), when_clear)
        fields.append(
            Field(
                id=node.get("id"),
                sce_type=sce_type,
                byte=int(node.get(f"{SCE}byte")),
                bits=int(size),
                bit_offset=int(offset) if offset is not None else None,
                endian=node.get(f"{SCE}endian", default_endian),
                flags=[
                    Flag(f.get("name"), int(f.get("bit")), int(f.get("width", "1")))
                    for f in node.findall(f"{SCE}flag")
                ],
                gate=gate,
            )
        )
    return Codec(root.get("name"), fields)


def gate_holds(codec: Codec, gate: Gate, carriers: dict[str, int]) -> bool:
    flag = next(f for f in codec.field(gate.carrier).flags if f.name == gate.flag)
    set_ = (carriers[gate.carrier] >> flag.bit) & 1 == 1
    return set_ != gate.when_clear


def carrier_value(codec: Codec, f: Field, source: dict) -> int:
    """The carrier `f` as the encoder writes it: what the value presents, with
    each derived flag set from the presence of the fields it gates."""
    value = source[f.id]
    for (carrier, name), gated in codec.derived_flags().items():
        if carrier != f.id:
            continue
        flag = next(d for d in f.flags if d.name == name)
        presences = {(source[g.id] is not None) != g.gate.when_clear for g in gated}
        if len(presences) != 1:
            raise ValueError(f"{codec.name}: the fields `{name}` gates disagree about it")
        mask = 1 << flag.bit
        value = (value & ~mask) | (mask if presences.pop() else 0)
    return value


def encode(codec: Codec, source: dict) -> bytes:
    """The frame of `source`: the fields in declaration order, a field gated off
    left out, and the fields that share a byte packed into it."""
    values: dict[str, int] = {}
    for f in codec.fields:
        values[f.id] = carrier_value(codec, f, source) if f.flags else source[f.id]
    out = bytearray()
    packed: dict[int, int] = {}
    for f in codec.fields:
        if f.gate is not None and source[f.id] is None:
            continue
        value = values[f.id]
        if value >= 1 << f.bits or value < 0:
            raise ValueError(f"{codec.name}: `{f.id}` = {value} has more than {f.bits} bits")
        if f.bit_offset is not None or f.bits < 8:
            packed[f.byte] = packed.get(f.byte, 0) | (value << (f.bit_offset or 0))
            if not any(
                g.byte == f.byte and codec.fields.index(g) > codec.fields.index(f)
                for g in codec.fields
            ):
                out.append(packed[f.byte])
            continue
        out += value.to_bytes(f.bits // 8, "big" if f.endian == "big" else "little")
    return bytes(out)


def decode(codec: Codec, frame: bytes) -> dict:
    """The `source` a frame says, the inverse of [`encode`]: a derived flag is
    not a member, and the carrier is presented as read."""
    source: dict = {}
    carriers: dict[str, int] = {}
    at = 0
    for f in codec.fields:
        if f.gate is not None and not gate_holds(codec, f.gate, carriers):
            source[f.id] = None
            continue
        if f.bit_offset is not None or f.bits < 8:
            byte = frame[at]
            value = (byte >> (f.bit_offset or 0)) & ((1 << f.bits) - 1)
            if not any(
                g.byte == f.byte and codec.fields.index(g) > codec.fields.index(f)
                for g in codec.fields
            ):
                at += 1
        else:
            n = f.bits // 8
            value = int.from_bytes(frame[at : at + n], "big" if f.endian == "big" else "little")
            at += n
        source[f.id] = value
        if f.flags:
            carriers[f.id] = value
    if at != len(frame):
        raise ValueError(f"{codec.name}: {len(frame) - at} bytes left over")
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


#: Each fixture this model writes, with its hand-chosen sources: every edge of
#: each rule it touches.
FIXTURES = {
    "codec_present_if_basic": [
        ({"flags": 0x00, "seq": None}, "has_seq clear: the frame is the carrier alone"),
        ({"flags": 0x01, "seq": 0x1234}, "has_seq set: seq follows, big-endian"),
        ({"flags": 0xFE, "seq": None}, "the bits the value presents are kept beside the derived one"),
        ({"flags": 0xFF, "seq": 0x0000}, "a present seq of zero is still present"),
        ({"flags": 0x01, "seq": 0xFFFF}, "seq at its largest"),
    ],
    "codec_flags_basic": [
        ({"header": 0x00}, "no flag set"),
        ({"header": 0xF0}, "every named flag set"),
        ({"header": 0x0F}, "only bits no flag names: the carrier is presented whole"),
        ({"header": 0xA5}, "reliable and drop set, with unnamed bits"),
    ],
}


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
    for fixture, sources in FIXTURES.items():
        written = write(fixture, sources)
        start, end = span(text, fixture, "sources")
        body = "\n" + ",\n".join(case_text(item) for item in written.sources)
        text = text[:start] + body + text[end:]
        reports.append(f"{fixture}: {len(written.sources)} sources; the model holds each")
    return text, "\n".join(reports)
