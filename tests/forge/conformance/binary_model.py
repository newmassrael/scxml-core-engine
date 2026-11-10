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

* an unsigned field of a fixed width (`sce:bit-size` a number of bits `N`) at
  `sce:byte` `B` and `sce:bit-offset` `O` (0 where absent), as docs/adr/0095
  writes it: the `W = ceil((O + N) / 8)` bytes from `B` on, read as one
  unsigned integer in the field's byte order (its `sce:endian`, else the
  codec's `sce:default-endian`, else big; `native` reads as big), are its
  word, and its value is bits `O` to `O + N` of the word counted from the
  least significant. A layout of fixed fields and then byte strings, none of
  them gated, is positional ([`Codec.positional`]): its fixed bytes run from 0
  to the end of the last fixed field, each the or of the bits each field holds
  in it, a byte no field holds written zero and not read. Any other layout
  writes each element in turn, so its fixed fields are whole bytes. Two fields
  over one bit, a partial field in a layout written in turn, a word past 64
  bits and more bits than the type holds are refused by name ([`Refused`]);
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
* a field aligned to a DMA burst (`sce:dma-burst-align`): it starts at its
  declared byte, a multiple of the burst; the encode writes zeros up to it
  and the decode skips to it unread;
* a codec it imports (`<sce:import>`), read from its own document: embedded
  in place (`<sce:embed>`), its encoded length said by a field before it where
  `sce:length-from` names one; or the entries of a list (`<sce:repeat>`),
  counted by a field before it (`count=`) or read to the end of the scope
  (`until-eof`), or of a chain (`<sce:tlv-chain>`) read to the end of the
  scope or until an entry whose flag (`terminate-on="entry-flag"`) is clear,
  within `max-count` or `max-depth`; any of them gated as a field is. A body
  is stated as its codec's value, a list as an array of them, and a count or
  a body's length is derived as a byte length is, as is the flag a chain ends
  on: set on every entry but the last, so such a chain holds one or more. Two
  lists of one count, one always there and one gated on a flag, are a group
  of that flag (R5) whose arm where it holds holds both, each stated by its
  id and of one length; other lists of one count are not read yet;
* a field of an enum the codec imports (`sce:type="enum:<alias>"`,
  `<sce:import kind="enum">`), written as the enum's carrier (its
  `sce:underlying-type`) and stated by the name of the variant it holds; an
  open enum (`sce:strict-variants="false"`) holds every number of its
  carrier, one no variant names stated as the number. A closed enum holds no
  number no variant declares, so the decode of a frame that carries one fails;
* a field gated on one flag of a carrier (`sce:present-if="c.f"` or `"!c.f"`),
  present exactly where the gate holds, its flag derived from its presence;
* a gate group ([`Group`], docs/adr/0065 §6 R4b, R4c, R6): two or more
  elements one flag decides, a stated length whose content is all on one side
  of it counting on the other, or the fields a disjunction `c.a || c.b` of the
  codec's own flags decides where no other gate reads them, or the fields the
  kinds of a chain's entries decide with the flags joined to them; a body or a
  list it holds is stated in its arm as an element is. A stated length whose
  content is under more than one gate, or one side of none, is no member of a
  group: it is presented alone, the value's own where all of its content is
  absent and `null` where some is there;
* the flag inputs a codec declares (`<sce:flag-inputs>`, one bit each): a gate
  on one is decided by the value the parent hands, which a case states in its
  `inputs`, and two elements one input decides are a group keyed by its name.
  An embedded body or an arm of a variant whose codec takes inputs is handed
  each as its import binds it (`<sce:flag-bind>`): the parent's own input, or
  a flag of a carrier, which is then derived from what the held codec says
  (docs/adr/0065 §6 R4a) and is the value's own where nothing held says it.
  The entries of a list are handed each input it binds, and, for one a chain
  supplies (`<sce:entry-flag-bind previous-entry-id>`), whether the entry
  before is of that kind, the first clear. Where they say an input they take,
  they follow a law over it (R8): the list is `{"start": {input: 0|1},
  "entries": [..]}`, each entry handed the start for an input the list binds,
  and a flag bound to the start is derived from it. An input they do not say
  is no law's: the list is its entries alone, each handed the parent's input
  or the value's own flag, or what the entry before is, as the walk writes it.
  A codec handed an input its value does not say takes it at its encode as
  its decode does, and a case states it in its `inputs`;
* a variant (`<sce:variant>`), stated as its member `body`: the arm's codec
  and its value, its tag derived from the arm where a field or a carrier's
  flag holds it; where a peeked byte holds it (`<sce:peek-byte>`), the tag is
  the arm's own first byte, which the arm's value states and the decode reads
  without moving past. A variant whose caller supplies its tag is not read
  yet;
* a gate the encode cannot derive a flag from: no group (R4c, R6) but a
  check (`codec_roles::source_checks`). It is a disjunction of the codec's own
  flags one of whose flags another gate reads, a disjunction that reads a flag
  input, or a gate on the kinds of a chain's entries no group holds, because
  it joins them with an input or another chain's kinds or reads more of them
  than a group holds. Each flag it reads is derived where something else
  derives it and is the value's own where nothing does, an input is the one
  the parent hands, and a kind is tested on the entries the value presents.
  The field it gates is presented alone, present or not as the value says; a
  value whose field contradicts the gate is one the encode refuses
  ([`EncodeRefused`]).

The encode refuses a value of the codec's interface only where the page says
it does (docs/adr/0065 §2): a gate it cannot derive that the value
contradicts, two things the value says of one number that disagree (two
byte strings of one length, or a length the value states beside content
that says another), a codec it holds refusing, or a
derived number past the field that carries it where no type of the value
bounds it. It raises [`EncodeRefused`] there, naming the refusal. A value no
type of the interface holds (a byte string past its bound, two lists of one
count that differ, an entry of a list that says other than its law) is no
value of the codec, and a case that states one is written wrong: the model
raises `ValueError` for it, as for any other.

Each fixture this module writes is listed in FIXTURES with its hand-chosen
sources and, where its encode can refuse, its hand-chosen refused cases;
`regenerate` writes them into `numerical_reference.json` under the fixture's
`sources` and `refused`, adding the keys, and the fixture's entry, where the
file has none. A refused case states the value and the refusal (`error`) its
encode answers it with; the model holds it to refusing exactly that.
"""

from __future__ import annotations

import json
import re
import xml.etree.ElementTree as ET
from dataclasses import dataclass, field
from pathlib import Path

RESOURCES = Path("tests/forge/resources")

SCE = "{http://sce.dev/ext}"
SCXML = "{http://www.w3.org/2005/07/scxml}"

WIDTH = {"uint8": 8, "uint16": 16, "uint32": 32, "uint64": 64}
SEQUENCE = ("bytes", "string")

#: The attributes of each element of a datamodel the model reads; any other is
#: refused by name, so a rule the document states is never silently left out of
#: a case.
READ_ATTRIBUTES = {
    "field": {
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
        f"{SCE}dma-burst-align",
    },
    "embed": {"id", "type", f"{SCE}byte", f"{SCE}present-if", f"{SCE}length-from"},
    "repeat": {"id", "type", f"{SCE}byte", "count", "until-eof", "max-count", f"{SCE}present-if"},
    "tlv-chain": {
        "id",
        "type",
        f"{SCE}byte",
        "max-depth",
        "terminate-on",
        "entry-flag-name",
        "entry-id",
        "entry-id-except",
        f"{SCE}present-if",
    },
    "variant": {"tag"},
}

#: The attributes of the document root the model reads. `sce:encode-may-fail`
#: says nothing of the wire: it states that the encode refuses some value
#: (docs/adr/0065 §1), which a refused case is held to. Nor does SCXML's own
#: `version`.
READ_ROOT_ATTRIBUTES = {
    "name",
    "version",
    f"{SCE}kind",
    f"{SCE}default-endian",
    f"{SCE}encode-may-fail",
}

#: The attributes of an enum document's root the model reads: its carrier
#: (`sce:underlying-type`) and whether a number no variant declares is a value
#: of it (`sce:strict-variants`); and of each of its variants. A variant's
#: `sce:text` is how a content line writes it, which says nothing of a
#: positional wire.
READ_ENUM_ATTRIBUTES = {
    "name",
    "version",
    f"{SCE}kind",
    f"{SCE}underlying-type",
    f"{SCE}strict-variants",
}
READ_VARIANT_ATTRIBUTES = {"name", "value", f"{SCE}text"}

#: The children of the document root besides its datamodel that say nothing of
#: the wire: why the document is a codec, and its own decoded examples.
SILENT_CHILDREN = {f"{SCE}kind-basis", f"{SCE}test-vector"}

#: How a measured element is measured: the bytes of a `length-ref` field, the
#: entries of a list (`count=`), the encoded bytes of a body (`sce:length-from`).
LENGTH, COUNT, BODY_LENGTH = "length", "count", "body length"

#: The most atoms a group of a chain's kinds is made of (docs/adr/0065 §6 R6):
#: a value holds an arm for each assignment of them.
MOST_GROUP_ATOMS = 8


class Unread(Exception):
    """A shape of the document this model does not read yet."""


class Refused(Exception):
    """A document the page refuses (`validation/attribute-rule-violated`,
    docs/adr/0095 §3): no frame is written for it."""


#: The refusals an encode answers a value of its interface with
#: (docs/adr/0065 §2), as a refused case names them.
LENGTH_OUT_OF_RANGE = "length-out-of-range"
PRESENT_IF_MISMATCH = "present-if-mismatch"
INCONSISTENT_SOURCE = "inconsistent-source"
REFUSALS = (LENGTH_OUT_OF_RANGE, PRESENT_IF_MISMATCH, INCONSISTENT_SOURCE)


class EncodeRefused(Exception):
    """A value of the codec's interface that its encode refuses
    (docs/adr/0065 §2): the value says what no frame can carry, and the
    encode answers it with the refusal `kind`, before its first byte.

    A value outside the type its interface presents (a byte string past its
    bound, two lists of one count that differ, an entry of a list that says
    other than its law) is no value of the codec at all and is not this: the
    type refuses it before an encode is called, and the model raises
    `ValueError` for it, as for any case written wrong."""

    def __init__(self, kind: str, why: str):
        assert kind in REFUSALS, kind
        super().__init__(f"{kind}: {why}")
        self.kind = kind


@dataclass
class Flag:
    name: str
    bit: int
    width: int


@dataclass
class Enum:
    """An enum a codec imports (`<sce:import kind="enum">`): the carrier its
    values are written as, the number each variant declares, and whether a
    number no variant declares is a value of it (`sce:strict-variants="false"`,
    an open enum)."""

    name: str
    carrier: str
    variants: dict[str, int]
    open: bool

    def number(self, value) -> int:
        """The number a source's value of this enum is written as: the one the
        variant it names declares, or, of an open enum, a number no variant
        declares, stated as itself."""
        if isinstance(value, str) and value in self.variants:
            return self.variants[value]
        if (self.open and isinstance(value, int) and not isinstance(value, bool)
                and value not in self.variants.values()):
            return value
        raise ValueError(
            f"a value of {self.name} is the name of a variant it declares"
            + (", or a number none declares" if self.open else "")
            + f", not {value!r}"
        )

    def value_of(self, number: int):
        """What a decode presents for the carrier `number`: the variant that
        declares it, or, of an open enum, the number itself where none does. A
        closed enum holds no other number, so a frame that carries one is no
        frame of the codec and its decode fails."""
        for name, declared in self.variants.items():
            if declared == number:
                return name
        if self.open:
            return number
        raise ValueError(f"{self.name} declares no variant of {number}: the decode refuses it")


@dataclass
class Clause:
    """One atom a gate reads, or its negation: a flag of a carrier
    (`carrier.flag`), a flag input the parent hands the codec (`flag`,
    `carrier` empty), or whether a chain holds an entry of the kind `key`
    (`carrier.has(key)`, `carrier` the chain, `SCE_FORGE.md` §4.6.2)."""

    carrier: str
    flag: str
    when_clear: bool
    key: int | None = None

    def label(self) -> str:
        if self.is_member():
            return f"{self.carrier}.has(0x{self.key:X})"
        return f"{self.carrier}.{self.flag}" if self.carrier else self.flag

    def is_input(self) -> bool:
        return not self.carrier

    def is_member(self) -> bool:
        return self.key is not None

    def atom(self) -> Clause:
        """The clause read as it is, not negated: the atom it reads."""
        return Clause(self.carrier, self.flag, False, self.key)


@dataclass
class Gate:
    """`sce:present-if`: one clause, or a disjunction `a.X || b.Y` of them,
    and its text as the document writes it."""

    clauses: list[Clause]
    text: str

    def one(self) -> Clause | None:
        """The clause of a gate of one flag; `None` for a disjunction."""
        return self.clauses[0] if len(self.clauses) == 1 else None

    @property
    def carrier(self) -> str:
        return self.only().carrier

    @property
    def flag(self) -> str:
        return self.only().flag

    @property
    def when_clear(self) -> bool:
        return self.only().when_clear

    def only(self) -> Clause:
        clause = self.one()
        if clause is None:
            raise AssertionError(f"`{self.text}` is a disjunction, not one flag")
        return clause


@dataclass
class Group:
    """The fields one gate decides, held together (`SCE_FORGE.md` §4.6,
    docs/adr/0065 §6 R4b, R4c): a member of the source keyed by the gate as
    the document writes it, whose value is the arm the value holds, `{"arm": i,
    ...its members}`, or `null` where it may hold none.

    The page names the arms and leaves their order to a case's notation
    (docs/adr/0081 §5): for one flag, the side where it holds and then the
    side where it does not, each where it holds an element; for a
    disjunction, each assignment of its flags that satisfies it, the ones
    holding fewer flags set first, and among those the one holding the
    earlier flags the gate names. A disjunction's flags are derived from the
    arm, and so is one flag."""

    label: str
    clauses: list[Clause]
    arms: list[tuple[tuple[bool, ...], list[Field]]]
    optional: bool
    #: Of a group the kinds a chain holds decide (docs/adr/0065 §6 R6): the
    #: chain, which every arm holds refined by the kinds it sets and clears.
    chain: Field | None = None
    #: Where that group has one atom, a kind whose entry is its kind alone (a
    #: bare entry): the kind. The arm that holds it states the chain as the
    #: other entries and the place of the first of the kind among them,
    #: `{"rest": [..], "place": n}`, and the encode derives the entry.
    placed: int | None = None

    def members(self) -> list[Field]:
        """Each element any arm holds, once, in the order the arms hold them."""
        out: list[Field] = []
        for _, fields in self.arms:
            out += [f for f in fields if f not in out]
        return out


@dataclass
class Arm:
    """An arm of a variant: the codec it holds, imported as `alias`, the tag
    that names it (`None` for the default arm, which keeps a tag no arm names),
    and what the parent hands each flag input of the codec."""

    tag: int | None
    alias: str
    codec: Codec
    binds: dict[str, str]


@dataclass
class Field:
    id: str
    #: The `sce:type` of a field, or the kind of element: `embed`, `repeat`,
    #: `tlv-chain`.
    sce_type: str
    #: `fixed`, `vle`, `length-ref` or `tail` for a field; `body`, `list` for
    #: the codec it holds.
    size: str
    #: The bits of a fixed field, or the bits a VLE holds.
    bits: int
    byte: int
    bit_offset: int | None
    endian: str
    carrier: bool = False
    flags: list[Flag] = field(default_factory=list)
    gate: Gate | None = None
    #: The element that measures it, `sce:length-field` (a sibling id or
    #: `carrier.flag`), `count=` or `sce:length-from`; `None` where nothing
    #: does, as for a `tail`, a list to the end of its scope or a body that
    #: ends where its own fields do.
    measured_by: str | None = None
    #: What the wire's length is short of the field's (`sce:length-arith`).
    arith: int = 0
    #: The most bytes (`sce:max-size`) or entries (`max-count`, `max-depth`).
    most: int | None = None
    #: The codec of a body or of each entry of a list.
    child: Codec | None = None
    #: Of a variant (`<sce:variant>`): the field or `carrier.flag` its tag is
    #: in, each known arm, and the default arm.
    tag_ref: str | None = None
    arms: list[Arm] = field(default_factory=list)
    default_arm: Arm | None = None
    #: Of a variant whose tag a peeked byte holds (`<sce:peek-byte>`): that
    #: byte's flags, `tag_ref` naming one as `peek.flag`. The byte is the arm's
    #: own first, which the arm's value states: the variant writes no tag, and
    #: the decode reads it without moving past it.
    peek: list[Flag] | None = None
    #: Of a chain that ends on a flag of its entries (`terminate-on=
    #: "entry-flag"`): the entry's carrier and the flag, set on every entry
    #: but the last, which the chain derives (docs/adr/0058 §1).
    end_flag: tuple[str, str] | None = None
    #: Of an embedded body: what the parent hands each flag input of the body's
    #: codec (`<sce:flag-bind input source>` on its import), a flag input of
    #: the parent or a flag of its carrier (`carrier.flag`).
    binds: dict[str, str] = field(default_factory=dict)
    #: Of a chain whose entries carry a kind (`entry-id`): the element of the
    #: entry the kind is, a field, a carrier or `carrier.flag`, and the flags of
    #: a whole carrier it leaves out (`entry-id-except`).
    entry_id: str | None = None
    entry_except: list[str] = field(default_factory=list)
    #: Of a chain: each input its entries take as whether the entry before is
    #: of a kind (`<sce:entry-flag-bind previous-entry-id>`), the first clear.
    after_key: dict[str, int] = field(default_factory=dict)
    #: Of a field aligned to a DMA burst: the byte it starts at, up to which
    #: the encode writes zeros and which the decode skips to unread.
    aligned_at: int | None = None
    #: Of a field of an imported enum (`sce:type="enum:<alias>"`): the enum.
    enum: Enum | None = None

    def law(self) -> list[str]:
        """The inputs of this list's entries its law runs over (docs/adr/0065
        §6 R8): each one the list hands them, as its parent's or from the entry
        before, that they say. An input they do not say is handed to each as
        the walk writes it, and no law holds them to it."""
        if self.size != "list":
            return []
        return [name for name in [*self.binds, *self.after_key] if codec_says(self.child, name)]

    def lawful(self) -> bool:
        """Whether this is a list held to a law (R8): its entries say an input
        it hands them. Its value is then `{"start": {..}, "entries": [..]}`,
        the start giving each input of the law the list hands all its entries
        from its parent."""
        return bool(self.law())

    def started(self) -> list[str]:
        """The inputs of this list's law its start gives: those it hands every
        entry from its parent; one the entry before decides starts clear."""
        return [name for name in self.law() if name in self.binds]

    def handed_entries(
        self, given: dict[str, int], entries: list[dict]
    ) -> list[dict[str, int]]:
        """What each entry of this list is handed: `given` for an input the
        list hands every entry, its law's start or what its parent hands, and
        for one the chain supplies whether the entry before is of its kind."""
        out = []
        for i, _ in enumerate(entries):
            handed = dict(given)
            for name, key in self.after_key.items():
                handed[name] = int(i > 0 and self.kind_of(entries[i - 1]) == key)
            out.append(handed)
        return out

    def kind_of(self, entry: dict) -> int:
        """The kind of an entry of this chain, as the entry's value states the
        element it is in."""
        carrier, _, flag = self.entry_id.partition(".")
        value = entry[carrier]
        if flag:
            bits = self.child.flag(carrier, flag)
            return (value >> bits.bit) & ((1 << bits.width) - 1)
        for name in self.entry_except:
            bits = self.child.flag(carrier, name)
            value &= ~(((1 << bits.width) - 1) << bits.bit)
        return value

    def whole(self) -> bool:
        """Whether this fixed field is whole bytes: no bit offset, and a
        width of whole bytes (docs/adr/0095 §2)."""
        return self.size == "fixed" and not self.bit_offset and self.bits % 8 == 0

    def word_bytes(self) -> int:
        """`W`, the bytes the word of this fixed field spans from its
        `sce:byte` on: ceil((O + N) / 8) (docs/adr/0095 §1)."""
        return ((self.bit_offset or 0) + self.bits + 7) // 8

    def order(self) -> str:
        """The byte order of the word: `native` reads as big."""
        return "little" if self.endian == "little" else "big"

    def word(self, value: int) -> bytes:
        """The `W` bytes of the word that holds `value` in this field's bits
        and nothing in any other bit."""
        return (value << (self.bit_offset or 0)).to_bytes(self.word_bytes(), self.order())

    def of_word(self, data: bytes) -> int:
        """The value of this field in the `W` bytes `data` of its word: bits
        `O` to `O + N`, counted from the least significant."""
        word = int.from_bytes(data, self.order())
        return (word >> (self.bit_offset or 0)) & ((1 << self.bits) - 1)

    def measure(self) -> str:
        """How the element that measures this one measures it."""
        if self.size == "list":
            return COUNT
        return BODY_LENGTH if self.size == "body" else LENGTH


@dataclass
class Codec:
    name: str
    fields: list[Field]
    #: The flag inputs the codec declares (`<sce:flag-inputs>`), in order: the
    #: values a parent hands its decode, each one bit.
    inputs: list[str] = field(default_factory=list)
    #: Whether the document states that its encode refuses some value
    #: (`sce:encode-may-fail`, docs/adr/0065 §1).
    may_fail: bool = False

    def field(self, id: str) -> Field:
        return next(f for f in self.fields if f.id == id)

    def flag(self, carrier: str, name: str) -> Flag:
        return next(f for f in self.field(carrier).flags if f.name == name)

    def positional(self) -> bool:
        """Whether each fixed field is at its declared byte: the layout is
        fixed fields, none gated or aligned to a burst, and after them only
        byte strings (`tail`, `length-ref`), none gated, written in turn. Any
        other layout writes every element in turn (docs/adr/0095 §2, §3)."""
        strings = False
        for f in self.fields:
            if f.size == "fixed" and f.gate is None and f.aligned_at is None:
                if strings:
                    return False
            elif f.sce_type in SEQUENCE and f.gate is None:
                strings = True
            else:
                return False
        return True

    def fixed(self) -> list[Field]:
        """The fixed fields, in the order the document declares them."""
        return [f for f in self.fields if f.size == "fixed"]

    def fixed_end(self) -> int:
        """Of a positional layout, the end of its last fixed field: its fixed
        bytes run from 0 to here, a byte no field holds among them."""
        return max((f.byte + f.word_bytes() for f in self.fixed()), default=0)

    def fixed_frame(self, values: dict[str, int]) -> bytes:
        """The fixed bytes of a positional layout holding `values`, by field
        id: each byte the or of the bits each field holds in it, and zero where
        no field holds a bit (docs/adr/0095 §2)."""
        out = bytearray(self.fixed_end())
        for f in self.fixed():
            for i, bits in enumerate(f.word(values[f.id])):
                out[f.byte + i] |= bits
        return bytes(out)

    def check_bits(self) -> None:
        """Refuse what docs/adr/0095 §3 refuses of where the fixed fields'
        bits lie: in a positional layout two fields over one bit, and in a
        layout written in turn a field that is not whole bytes."""
        if not self.positional():
            for f in self.fixed():
                if not f.whole():
                    raise Refused(
                        f"{self.name}: `{f.id}` is not whole bytes in a layout written in turn"
                    )
            return
        held = bytearray(self.fixed_end())
        for f in self.fixed():
            for i, bits in enumerate(f.word((1 << f.bits) - 1)):
                if held[f.byte + i] & bits:
                    raise Refused(
                        f"{self.name}: `{f.id}` holds a bit of byte {f.byte + i} "
                        "another field holds"
                    )
                held[f.byte + i] |= bits

    def measured(self) -> dict[str, list[Field]]:
        """Each measuring element named, and the elements it measures."""
        out: dict[str, list[Field]] = {}
        for f in self.fields:
            if f.measured_by is not None:
                out.setdefault(f.measured_by, []).append(f)
        return out

    def is_length(self, f: Field) -> bool:
        return f.id in self.measured()

    def stated(self, f: Field) -> bool:
        """A measure not gated itself whose every content is gated: the value
        states it where the content is absent (docs/adr/0058 §1)."""
        contents = self.measured().get(f.id, [])
        return f.gate is None and bool(contents) and all(c.gate is not None for c in contents)

    def variant(self) -> Field | None:
        return next((f for f in self.fields if f.size == "variant"), None)

    def is_tag(self, f: Field) -> bool:
        """Whether `f` is the field a variant's tag is in: a known arm's tag is
        that arm's, and a default arm's the one it holds (docs/adr/0058 §1)."""
        variant = self.variant()
        return variant is not None and variant.tag_ref == f.id

    def presented(self, f: Field) -> bool:
        return (not self.is_length(f) or self.stated(f)) and not self.is_tag(f)

    def one_flag_gated(self) -> dict[tuple[str, str], list[Field]]:
        """Each flag a gate of one flag names, a carrier's or a flag input
        (carrier empty), and the presented fields it gates; a gate a chain's
        kinds decide belongs to their group, and one no group holds is a check
        ([`Codec.checked`])."""
        held = self.by_kind()
        out: dict[tuple[str, str], list[Field]] = {}
        for f in self.fields:
            if (f.gate is not None and f.gate.one() is not None and self.presented(f)
                    and f.id not in held and not f.gate.one().is_member()):
                out.setdefault((f.gate.carrier, f.gate.flag), []).append(f)
        return out

    def aligned(self, count: str) -> tuple[Field, Field] | None:
        """The two lists `count` counts where they are aligned (docs/adr/0065
        §6 R5): one always there and one gated on one flag of a carrier, as
        (always there, gated). The gate decides the second whole, so the first
        is on both sides of its group, and both are held to the one count."""
        lists = [m for m in self.measured().get(count, []) if m.size == "list"]
        if len(lists) != 2:
            return None
        always = [m for m in lists if m.gate is None]
        gated = [m for m in lists if m.gate is not None and m.gate.one() is not None
                 and not m.gate.one().is_input() and not m.gate.one().is_member()]
        if len(always) != 1 or len(gated) != 1:
            return None
        return always[0], gated[0]

    def by_kind(self) -> set[str]:
        """Each element a group of a chain's kinds decides (R6): the chain's
        own gate, if it has one, stays its own."""
        return {m.id for g in self.membership_groups() for m in g.members() if m is not g.chain}

    def membership_groups(self) -> list[Group]:
        """The groups the kinds a chain holds decide (docs/adr/0065 §6 R6,
        `codec_roles::membership_groups`): for each chain a gate reads the kinds
        of, the least set of atoms closed under the gates that read one, the
        chain's kinds first and then the flags joined with them, each in the
        order the gates first name them, and an arm for every assignment of
        them, the ones setting more first and among those the one setting the
        earlier atoms. Each arm holds the chain, the fields whose gates hold
        and a stated length where its content is absent; a flag among the atoms
        is derived from the arm."""
        gated = [(f, [c.atom() for c in f.gate.clauses]) for f in self.fields
                 if f.gate is not None and self.presented(f)]
        out = []
        for chain in self.fields:
            atoms: list[Clause] = []
            for _, read in gated:
                atoms += [a for a in read
                          if a.is_member() and a.carrier == chain.id and a not in atoms]
            if not atoms:
                continue
            if chain.entry_id is None:
                raise Unread(f"{self.name}: a gate reads the kinds of `{chain.id}`, which has none")
            while True:
                more = [a for _, read in gated if any(x in atoms for x in read)
                        for a in read if a not in atoms]
                if not more:
                    break
                atoms += [a for i, a in enumerate(more) if a not in more[:i]]
            for a in atoms:
                if a.is_member() and a.carrier == chain.id and not kind_fits(chain, a.key):
                    raise Unread(f"{self.name}: `{a.label()}` names a kind no entry can be")
                if not a.is_member() and not a.is_input() and self.flag(a.carrier, a.flag).width != 1:
                    raise Unread(f"{self.name}: `{a.label()}` is wider than one bit")
            # An arm is an assignment of the atoms the value holds, so the value
            # decides each: a flag input is the parent's, which no value holds,
            # and another chain's kinds are that chain's. A gate that joins one
            # with this chain's kinds, or reads more of them than a group holds,
            # leaves every field the atoms reach to the checks (R6).
            if len(atoms) > MOST_GROUP_ATOMS or any(
                a.is_input() or (a.is_member() and a.carrier != chain.id) for a in atoms
            ):
                continue
            atoms = [a for a in atoms if a.is_member()] + [a for a in atoms if not a.is_member()]
            reached = [f for f, read in gated if any(x in atoms for x in read)]

            def holds(gate: Gate, values: tuple[bool, ...]) -> bool:
                return any(values[atoms.index(c.atom())] != c.when_clear for c in gate.clauses)

            stated = {}
            for f in self.fields:
                contents = self.measured().get(f.id, [])
                if (self.stated(f) and all(c in reached for c in contents)
                        and len({c.gate.text for c in contents}) == 1):
                    stated[f.id] = contents[0].gate
            k = len(atoms)
            assignments = [tuple(bool(mask >> (k - 1 - i) & 1) for i in range(k))
                           for mask in range(1 << k)]
            assignments.sort(key=lambda values: (-sum(values), tuple(not v for v in values)))
            arms = [
                (values, [f for f in self.fields
                          if f is chain or (f in reached and holds(f.gate, values))
                          or (f.id in stated and not holds(stated[f.id], values))])
                for values in assignments
            ]
            group = Group(
                label=" || ".join(a.label() for a in atoms),
                clauses=atoms,
                arms=arms,
                optional=False,
                chain=chain,
            )
            if k == 1 and bare(chain, atoms[0].key):
                group.placed = atoms[0].key
            out.append(group)
        return out

    def checked(self, f: Field) -> bool:
        """Whether the gate of `f` is one the encode cannot derive a flag from,
        and so holds the value to, refusing one that contradicts it
        (docs/adr/0065 §2, `codec_roles::source_checks`): a gate on the kinds
        of a chain's entries no group holds (R6); a disjunction that reads a
        flag input, which the parent hands and no value says (R4c); or a
        disjunction of the codec's own flags that is no group, because another
        gate reads one of its flags, so that flag is not the disjunction's to
        derive. Its field is then presented alone, present or not as the value
        says."""
        if f.gate is None or f.id in self.by_kind():
            return False
        if any(c.is_member() for c in f.gate.clauses):
            return True
        if f.gate.one() is not None:
            return False
        if any(c.is_input() for c in f.gate.clauses):
            return True
        one_flag = {(c.carrier, c.flag) for g in self.fields
                    if g.gate is not None and g.gate.one() is not None for c in g.gate.clauses}
        return any(
            (c.carrier, c.flag) in one_flag or any(
                g.gate is not None and g.gate.one() is None and g.gate.text != f.gate.text
                and any((d.carrier, d.flag) == (c.carrier, c.flag) for d in g.gate.clauses)
                for g in self.fields
            )
            for c in f.gate.clauses
        )

    def gate_flags(self) -> dict[tuple[str, str], list[Field]]:
        """Each flag of a carrier a gate of one flag names, and the presented
        fields it gates: the encoder sets it from their presence. A flag input
        is the parent's to set, and no field of this codec holds it."""
        return {key: gated for key, gated in self.one_flag_gated().items() if key[0]}

    def decided(self) -> dict[tuple[str, str], list[tuple[Field, bool]]]:
        """Each flag a gate of one flag names, and every element of the value
        its gate decides with the side it is on, the value of the flag while it
        is present: the presented fields it gates, and each stated length whose
        content is all on one side of it, which the value holds on the other
        side (`SCE_FORGE.md` §4.6, docs/adr/0065 §6 R4b). A stated length whose
        content is under two gates, or on both sides of one, or under a gate of
        no one flag, joins no side: it is presented alone."""
        one_flag = self.one_flag_gated()
        out: dict[tuple[str, str], list[tuple[Field, bool]]] = {}
        for key, gated in one_flag.items():
            out[key] = [(f, not f.gate.when_clear) for f in gated]
        held = self.by_kind()
        for f in self.fields:
            if not self.stated(f) or f.id in held:
                continue
            sides = set()
            for c in self.measured()[f.id]:
                clause = c.gate.one()
                key = (clause.carrier, clause.flag) if clause is not None else None
                if key is None or c not in one_flag.get(key, []):
                    sides = None
                    break
                sides.add((key, clause.when_clear))
            if sides is not None and len(sides) == 1:
                ((key, when_clear),) = sides
                out[key].append((f, when_clear))
        # Two lists of one count, one always there and one the flag gates (R5):
        # the first is on both sides, so the arm where the flag holds holds the
        # two together, of one count, and the other the first alone.
        for name in self.measured():
            pair = self.aligned(name)
            if pair is not None:
                always, gated = pair
                clause = gated.gate.one()
                out.setdefault((clause.carrier, clause.flag), []).extend(
                    [(always, True), (always, False)]
                )
        return out

    def groups(self) -> list[Group]:
        """The gate groups of the codec (`SCE_FORGE.md` §4.6, R4b, R4c, R6)."""
        out = self.membership_groups()
        held = {m.id for g in out for m in g.members()}
        for (carrier, name), decided in self.decided().items():
            if len(decided) < 2:
                continue
            sides = [
                (side, [f for f, s in decided if s == side])
                for side in (True, False)
                if any(s == side for _, s in decided)
            ]
            out.append(
                Group(
                    label=Clause(carrier, name, False).label(),
                    clauses=[Clause(carrier, name, False)],
                    arms=[((side,), fields) for side, fields in sides],
                    optional=len(sides) == 1,
                )
            )
        disjunctions: dict[str, list[Field]] = {}
        for f in self.fields:
            if (f.gate is not None and f.gate.one() is None and f.id not in held
                    and not self.checked(f)):
                disjunctions.setdefault(f.gate.text, []).append(f)
        for label, fields in disjunctions.items():
            clauses = fields[0].gate.clauses
            n = len(clauses)
            assignments = [tuple(bool(v >> i & 1) for i in range(n)) for v in range(1, 1 << n)]
            assignments.sort(key=lambda values: (sum(values), tuple(not v for v in values)))
            arms = [(values, list(fields)) for values in assignments]
            out.append(Group(label=label, clauses=list(clauses), arms=arms, optional=True))
        return out

    def check(self) -> None:
        """Refuse what the model does not read among the relations of fields,
        and what the page refuses of where their bits lie."""
        self.check_bits()
        for (carrier, name), decided in self.decided().items():
            if carrier and self.flag(carrier, name).width != 1:
                raise Unread(f"{self.name}: a gate on the {name} flag, wider than one bit")
        held = self.by_kind()
        for f in self.fields:
            if f.gate is None or f.id in held:
                continue
            # A gate the value is held to reads each atom as it is: a flag of
            # one bit, an input the parent hands, a kind an entry can be. A
            # disjunction that is a group reads set flags, an arm for each
            # assignment of them that satisfies it.
            checked = self.checked(f)
            for c in f.gate.clauses:
                if c.is_member():
                    chain = self.field(c.carrier)
                    if chain.entry_id is None or not kind_fits(chain, c.key):
                        raise Unread(f"{self.name}: `{c.label()}` names a kind no entry can be")
                elif c.is_input():
                    continue
                elif f.gate.one() is not None:
                    continue
                elif (c.when_clear and not checked) or self.flag(c.carrier, c.flag).width != 1:
                    raise Unread(f"{self.name}: the disjunction `{f.gate.text}` is not of set flags")
        # A disjunction one of whose flags another gate reads is no group but a
        # check ([`Codec.checked`]): each of its flags is derived by what derives
        # it elsewhere or is the value's own, and the field is held to it.
        for (carrier, name) in self.gate_flags():
            if f"{carrier}.{name}" in self.measured():
                raise Unread(f"{self.name}: {carrier}.{name} is derived by two kinds")
        for name, measured in self.measured().items():
            carrier, _, flag = name.partition(".")
            if flag:
                self.flag(carrier, flag)
            elif self.field(name).sce_type not in WIDTH:
                raise Unread(f"{self.name}: the measure {name} is not an unsigned number")
            if len({m.measure() for m in measured}) > 1:
                raise Unread(f"{self.name}: {name} is measured by declarations of two kinds")
            if sum(m.size == "list" for m in measured) > 1 and self.aligned(name) is None:
                raise Unread(
                    f"{self.name}: {name} counts lists other than one always there and one "
                    "gated on a flag, which is not read yet"
                )


def kind_fits(chain: Field, key: int) -> bool:
    """Whether an entry's kind can be `key` (`codec_roles::named_key_fits`):
    no wider than the element it is, and no bit of a flag it leaves out."""
    carrier, _, flag = chain.entry_id.partition(".")
    width = chain.child.flag(carrier, flag).width if flag else chain.child.field(carrier).bits
    left_out = 0
    for name in chain.entry_except:
        bits = chain.child.flag(carrier, name)
        left_out |= ((1 << bits.width) - 1) << bits.bit
    return key < 1 << width and not key & left_out


def bare(chain: Field, key: int) -> bool:
    """Whether the entry of the kind `key` in `chain` is its kind alone
    (docs/adr/0065 §6 R6a, `codec_roles::named_entry_is_bare`): the kind is a
    whole carrier, the entry's one field besides its variant, the flags it
    leaves out are the chain's own, and the known arm the kind's tag selects
    presents nothing."""
    carrier, _, flag = chain.entry_id.partition(".")
    entry = chain.child
    held_in = entry.field(carrier)
    if flag or not held_in.carrier:
        return False
    if [f.id for f in entry.fields if f.size != "variant"] != [carrier]:
        return False
    if any(chain.end_flag is None or name != chain.end_flag[1] for name in chain.entry_except):
        return False
    variant = entry.variant()
    if variant is None:
        return True
    tag_carrier, _, tag_flag = variant.tag_ref.partition(".")
    if tag_carrier != carrier or not tag_flag or tag_flag in chain.entry_except:
        return False
    bits = entry.flag(carrier, tag_flag)
    tag = (key >> bits.bit) & ((1 << bits.width) - 1)
    arm = next((a for a in variant.arms if a.tag == tag), None)
    return arm is not None and not any(arm.codec.presented(f) for f in arm.codec.fields)


def named_entry(chain: Field, key: int) -> dict:
    """The value of the bare entry of the kind `key` in `chain`: its carrier
    the kind, the chain's own flag clear (the chain sets it where the entry
    stands), and the empty arm its tag selects."""
    carrier, _, _ = chain.entry_id.partition(".")
    entry = {carrier: key}
    variant = chain.child.variant()
    if variant is not None:
        _, _, tag_flag = variant.tag_ref.partition(".")
        bits = chain.child.flag(carrier, tag_flag)
        tag = (key >> bits.bit) & ((1 << bits.width) - 1)
        arm = next(a for a in variant.arms if a.tag == tag)
        entry[variant.id] = {"arm": arm.alias, "body": {}}
    return entry


def aligned_at(stem: str, node) -> int | None:
    """Where a field aligned to a DMA burst (`sce:dma-burst-align`) starts:
    its declared byte, a multiple of the burst, which the encode pads up to
    with zeros and the decode moves to."""
    burst = attribute_int(node, "dma-burst-align")
    if burst is None:
        return None
    at = int(node.get(f"{SCE}byte"))
    if burst <= 0 or at % burst:
        raise Unread(f"{stem}: `{node.get('id')}` at byte {at} is not aligned to {burst}")
    return at


def attribute_int(node, name: str, namespace: str = SCE) -> int | None:
    value = node.get(f"{namespace}{name}")
    return int(value, 0) if value is not None else None


def read_enum(path: Path) -> Enum:
    """The enum at `path` (`sce:kind="enum"`), as its document declares it: its
    carrier, an unsigned number, and each variant's name and number, within
    the carrier, two variants never declaring one number."""
    stem = path.stem
    root = ET.parse(path).getroot()
    if root.get(f"{SCE}kind") != "enum":
        raise Unread(f"{stem}: not an enum")
    if extra := sorted(set(root.attrib) - READ_ENUM_ATTRIBUTES):
        raise Unread(f"{stem}: the root declares {', '.join(extra)}, which is not read yet")
    carrier = root.get(f"{SCE}underlying-type")
    if carrier not in WIDTH:
        raise Unread(f"{stem}: the carrier {carrier!r} is not an unsigned number")
    strict = root.get(f"{SCE}strict-variants", "true").strip()
    if strict not in ("true", "false"):
        raise Unread(f"{stem}: sce:strict-variants={strict!r} is not read yet")
    variants: dict[str, int] = {}
    for node in root.iter(f"{SCE}variant"):
        if extra := sorted(set(node.attrib) - READ_VARIANT_ATTRIBUTES):
            raise Unread(f"{stem}: a variant declares {', '.join(extra)}, which is not read yet")
        name, value = node.get("name"), int(node.get("value"), 0)
        if name in variants or value in variants.values() or not 0 <= value < 1 << WIDTH[carrier]:
            raise Unread(f"{stem}: the variant `{name}` = {value} is not one more of its carrier")
        variants[name] = value
    if not variants:
        raise Unread(f"{stem}: declares no variant")
    return Enum(root.get("name"), carrier, variants, strict == "false")


def read_codec(stem: str) -> Codec:
    """The codec `stem` of the resources, as its document declares it."""
    return read_document(RESOURCES / f"{stem}.scxml")


def read_gate(stem: str, node, inputs: list[str]) -> Gate | None:
    present_if = node.get(f"{SCE}present-if")
    if present_if is None:
        return None
    text = present_if.strip()
    if "&" in text or "(" in re.sub(r"\.has\([^()]*\)", "", text):
        raise Unread(f"{stem}: gate {present_if!r} is not atoms joined by ||")
    clauses = []
    for part in text.split("||"):
        part = part.strip()
        when_clear = part.startswith("!")
        member = re.fullmatch(r"(\w+)\.has\((0[xX][0-9a-fA-F]+|\d+)\)", part.lstrip("!"))
        if member is not None:
            clauses.append(Clause(member[1], "", when_clear, int(member[2], 0)))
            continue
        carrier, _, flag = part.lstrip("!").partition(".")
        if not flag:
            name = carrier.strip()
            if name not in inputs:
                raise Unread(f"{stem}: gate {present_if!r} names `{name}`, no flag input")
            clauses.append(Clause("", name, when_clear))
        else:
            clauses.append(Clause(carrier.strip(), flag.strip(), when_clear))
    return Gate(clauses, " || ".join(c.label() for c in clauses))


def read_flags(stem: str, what: str, node) -> list[Flag]:
    """The `<sce:flag>` children of `node`, a carrier or a peeked byte. A flag's
    `value` is the carrier's value in the builder's default instance; it says
    nothing of the wire."""
    flags = []
    for f in node:
        if f.tag != f"{SCE}flag" or set(f.attrib) - {"name", "bit", "width", "value"}:
            raise Unread(f"{stem}: a flag of {what} declares what is not read yet")
        flags.append(Flag(f.get("name"), int(f.get("bit")), int(f.get("width", "1"))))
    return flags


def bound_inputs(
    stem: str,
    what: str,
    alias: str,
    child: Codec,
    binds: dict[str, dict[str, str]],
    inputs: list[str],
    supplied: set[str] = frozenset(),
) -> dict[str, str]:
    """What the parent hands each flag input of the codec imported as `alias`,
    which `what` holds: every input bound, each to an input of the parent or a
    flag of one of its carriers (`carrier.flag`), but those the holder
    `supplied` otherwise (a chain's entry from the entry before)."""
    bound = binds.get(alias, {})
    if set(bound) & supplied or sorted([*bound, *supplied]) != sorted(child.inputs):
        raise Unread(f"{stem}: {what} binds {sorted(bound)}, not every input of `{alias}`")
    for source in bound.values():
        _, _, flag = source.partition(".")
        if not flag and source not in inputs:
            raise Unread(f"{stem}: {what} binds `{source}`, no flag input of the codec")
    return dict(bound)


def read_held(
    stem: str,
    node,
    kind: str,
    imports: dict[str, Codec],
    inputs: list[str],
    binds: dict[str, dict[str, str]],
) -> Field:
    """An embedded body, a `<sce:repeat>` or a `<sce:tlv-chain>`: a field
    holding the codec imported as its `type`, and what the parent hands each
    flag input of it: as its import binds it, or, for an entry of a chain, as
    the entry before it is or is not of a kind (`<sce:entry-flag-bind>`)."""
    id = node.get("id")
    after_key: dict[str, int] = {}
    for child_node in node:
        if (kind == "tlv-chain" and child_node.tag == f"{SCE}entry-flag-bind"
                and set(child_node.attrib) == {"input", "previous-entry-id"}):
            after_key[child_node.get("input")] = int(child_node.get("previous-entry-id"), 0)
        else:
            raise Unread(f"{stem}: `{id}` holds <{child_node.tag}>, which is not read yet")
    alias = node.get("type")
    child = imports.get(alias)
    if child is None:
        raise Unread(f"{stem}: `{id}` holds `{alias}`, which is not imported")
    bound = bound_inputs(stem, f"`{id}`", alias, child, binds, inputs, supplied=set(after_key))
    if kind == "embed":
        size, measured_by, most = "body", node.get(f"{SCE}length-from"), None
    elif kind == "repeat":
        count, until_eof = node.get("count"), node.get("until-eof") == "true"
        if (count is None) == (not until_eof):
            raise Unread(f"{stem}: `{id}` is counted and read to the end, or neither")
        size, measured_by, most = "list", count, attribute_int(node, "max-count", "")
    else:
        size, measured_by, most = "list", None, attribute_int(node, "max-depth", "")
    end_flag = None
    if kind == "tlv-chain" and node.get("terminate-on", "exhaust-or-depth") != "exhaust-or-depth":
        if node.get("terminate-on") != "entry-flag":
            raise Unread(f"{stem}: `{id}` ends by {node.get('terminate-on')!r}")
        name = node.get("entry-flag-name")
        carrier = next((f for f in child.fields if f.carrier and any(g.name == name for g in f.flags)),
                       None)
        if carrier is None or child.flag(carrier.id, name).width != 1:
            raise Unread(f"{stem}: the entries of `{id}` hold no one-bit flag `{name}`")
        end_flag = (carrier.id, name)
    elif node.get("entry-flag-name") is not None:
        raise Unread(f"{stem}: `{id}` names an entry flag and does not end on it")
    entry_id = node.get("entry-id")
    entry_except: list[str] = []
    if entry_id is not None:
        carrier, _, flag = entry_id.partition(".")
        held_in = next((f for f in child.fields if f.id == carrier), None)
        # The kind is read from what the entry's value states, so it is an
        # element the value presents: a number, or a carrier stated whole.
        if held_in is None or held_in.sce_type not in WIDTH or not child.presented(held_in):
            raise Unread(f"{stem}: the kind `{entry_id}` of `{id}` is no number an entry states")
        if flag:
            child.flag(carrier, flag)
        for name in filter(None, (n.strip() for n in node.get("entry-id-except", "").split(","))):
            except_carrier, _, except_flag = name.partition(".")
            if flag or except_carrier != carrier:
                raise Unread(f"{stem}: `{id}` leaves `{name}` out of a kind it is not in")
            child.flag(carrier, except_flag)
            entry_except.append(except_flag)
    elif node.get("entry-id-except") is not None:
        raise Unread(f"{stem}: `{id}` leaves a flag out of a kind it does not declare")
    if after_key and entry_id is None:
        raise Unread(f"{stem}: `{id}` hands an input by the entry before's kind, and has none")
    return Field(
        id=id,
        sce_type=kind,
        size=size,
        bits=0,
        byte=int(node.get(f"{SCE}byte")),
        bit_offset=None,
        endian="big",
        gate=read_gate(stem, node, inputs),
        measured_by=measured_by,
        most=most,
        child=child,
        end_flag=end_flag,
        binds=bound,
        entry_id=entry_id,
        entry_except=entry_except,
        after_key=after_key,
    )


def read_variant(
    stem: str,
    node,
    imports: dict[str, Codec],
    inputs: list[str],
    binds: dict[str, dict[str, str]],
) -> Field:
    """A `<sce:variant>` whose tag is a field or a flag before it: the value
    holds one arm, `body`, and the tag is the arm's; the parent hands the arm's
    codec each flag input as its import binds it."""
    tag_ref = node.get("tag")
    if tag_ref is None:
        raise Unread(f"{stem}: a variant whose caller supplies its tag is not read yet")
    arms: list[Arm] = []
    default_arm = None
    peek: list[Flag] | None = None

    def arm(tag: int | None, alias: str) -> Arm:
        codec = imports.get(alias)
        if codec is None:
            raise Unread(f"{stem}: an arm holds `{alias}`, which is not imported")
        bound = bound_inputs(stem, f"the arm of `{alias}`", alias, codec, binds, inputs)
        return Arm(tag, alias, codec, bound)

    for child in node:
        if child.tag == f"{SCE}arm" and not set(child.attrib) - {"value", "type", "default"}:
            arms.append(arm(int(child.get("value"), 0), child.get("type")))
        elif child.tag == f"{SCE}default" and set(child.attrib) == {"type"}:
            default_arm = arm(None, child.get("type"))
        elif (child.tag == f"{SCE}peek-byte" and set(child.attrib) == {"id", f"{SCE}type"}
              and child.get(f"{SCE}type") == "uint8" and peek is None):
            peek_id = child.get("id")
            peek = read_flags(stem, f"the peeked byte `{peek_id}`", child)
            carrier, _, flag = tag_ref.partition(".")
            if carrier != peek_id or not any(f.name == flag for f in peek):
                raise Unread(f"{stem}: the tag `{tag_ref}` is no flag of the peeked byte")
        else:
            raise Unread(f"{stem}: a variant's <{child.tag}> is not read yet")
    return Field(
        id="body",
        sce_type="variant",
        size="variant",
        bits=0,
        byte=-1,
        bit_offset=None,
        endian="big",
        tag_ref=tag_ref,
        arms=arms,
        default_arm=default_arm,
        peek=peek,
    )


def read_document(path: Path) -> Codec:
    """The codec at `path`, as its document declares it, with each codec it
    imports read in turn."""
    stem = path.stem
    root = ET.parse(path).getroot()
    default_endian = root.get(f"{SCE}default-endian", "big")
    if extra := sorted(set(root.attrib) - READ_ROOT_ATTRIBUTES):
        raise Unread(f"{stem}: the root declares {', '.join(extra)}, which is not read yet")
    if root.get(f"{SCE}kind") != "codec":
        raise Unread(f"{stem}: not a codec")
    imports: dict[str, Codec] = {}
    enums: dict[str, Enum] = {}
    #: For each import, what the parent hands each flag input of it.
    binds: dict[str, dict[str, str]] = {}
    inputs: list[str] = []
    for child in root:
        if child.tag == f"{SCE}flag-inputs":
            for item in child:
                if item.tag != f"{SCE}flag-input" or set(item.attrib) - {"name", "width"}:
                    raise Unread(f"{stem}: a flag input declares what is not read yet")
                if item.get("width", "1") != "1":
                    raise Unread(f"{stem}: the flag input `{item.get('name')}` is wider than a bit")
                inputs.append(item.get("name"))
        elif child.tag == f"{SCE}import":
            kind = child.get("kind")
            if kind not in ("codec", "enum") or set(child.attrib) - {"src", "kind", "as"}:
                raise Unread(f"{stem}: an import other than of a codec or an enum by its file")
            src = child.get("src")
            if src.startswith("sce:"):
                raise Unread(f"{stem}: the standard document {src} is not read yet")
            alias = child.get("as")
            if kind == "enum":
                if len(child):
                    raise Unread(f"{stem}: the import of the enum `{alias}` holds <{child[0].tag}>")
                enums[alias] = read_enum(path.parent / src)
                continue
            imports[alias] = read_document(path.parent / src)
            for bind in child:
                if bind.tag != f"{SCE}flag-bind" or set(bind.attrib) != {"input", "source"}:
                    raise Unread(f"{stem}: the import of `{alias}` holds <{bind.tag}>")
                binds.setdefault(alias, {})[bind.get("input")] = bind.get("source")
        elif child.tag != f"{SCXML}datamodel" and child.tag not in SILENT_CHILDREN:
            raise Unread(f"{stem}: <{child.tag}> is not read yet")
    datamodel = root.find(f"{SCXML}datamodel")
    if datamodel is None:
        raise Unread(f"{stem}: no datamodel")
    fields = []
    for node in datamodel:
        tag = node.tag
        kind = tag.removeprefix(SCE).removeprefix(SCXML)
        kind = "field" if kind in ("flags", "data") else kind
        if kind not in READ_ATTRIBUTES:
            raise Unread(f"{stem}: <{tag}> is not read yet")
        id = node.get("id")
        if extra := sorted(set(node.attrib) - READ_ATTRIBUTES[kind]):
            raise Unread(f"{stem}: `{id}` declares {', '.join(extra)}, which is not read yet")
        if kind == "variant":
            fields.append(read_variant(stem, node, imports, inputs, binds))
            continue
        if kind != "field":
            fields.append(read_held(stem, node, kind, imports, inputs, binds))
            continue
        sce_type = node.get(f"{SCE}type")
        size_text = node.get(f"{SCE}bit-size")
        # A field of an imported enum is written as the enum's carrier.
        enum = None
        written_as = sce_type
        if (sce_type or "").startswith("enum:"):
            enum = enums.get(sce_type.removeprefix("enum:"))
            if enum is None:
                raise Unread(f"{stem}: `{id}` is of `{sce_type}`, no enum the codec imports")
            written_as = enum.carrier
        if sce_type in SEQUENCE:
            if size_text not in ("length-ref", "tail"):
                raise Unread(f"{stem}: `{id}` is {sce_type} of bit-size {size_text!r}")
            size, bits = size_text, 0
        elif written_as in WIDTH:
            if size_text == "vle":
                size, bits = "vle", WIDTH[written_as]
            elif size_text is not None and size_text.isdigit():
                size, bits = "fixed", int(size_text)
                offset = attribute_int(node, "bit-offset") or 0
                if bits == 0 or offset < 0:
                    raise Unread(f"{stem}: `{id}` has {bits} bits at bit {offset}")
                if bits > WIDTH[written_as]:
                    raise Refused(f"{stem}: `{id}` has {bits} bits, more than {written_as} holds")
                if offset + bits > 64:
                    raise Refused(f"{stem}: the word of `{id}` is {offset + bits} bits, past 64")
            else:
                raise Unread(f"{stem}: bit-size {size_text!r} of `{id}` is not read yet")
        else:
            raise Unread(f"{stem}: type {sce_type!r} of `{id}` is not read yet")
        flags = read_flags(stem, f"`{id}`", node)
        length_field = node.get(f"{SCE}length-field")
        if (size == "length-ref") != (length_field is not None):
            raise Unread(f"{stem}: `{id}` pairs length-ref and length-field otherwise")
        endian = node.get(f"{SCE}endian", default_endian)
        if endian not in ("big", "little", "native"):
            raise Unread(f"{stem}: the byte order {endian!r} of `{id}` is not read yet")
        fields.append(
            Field(
                id=id,
                sce_type=sce_type,
                size=size,
                bits=bits,
                byte=int(node.get(f"{SCE}byte")),
                bit_offset=attribute_int(node, "bit-offset"),
                endian=endian,
                carrier=tag == f"{SCE}flags",
                flags=flags,
                gate=read_gate(stem, node, inputs),
                measured_by=length_field,
                arith=attribute_int(node, "length-arith") or 0,
                most=attribute_int(node, "max-size"),
                aligned_at=aligned_at(stem, node),
                enum=enum,
            )
        )
    may_fail = root.get(f"{SCE}encode-may-fail", "false").strip() in ("true", "1")
    codec = Codec(root.get("name"), fields, inputs, may_fail)
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


def chain_of(codec: Codec, group: Group, values: tuple[bool, ...], stated) -> list | None:
    """The entries of the chain an arm of a group of its kinds holds (R6): as
    the arm states them, or for a bare entry's arm the others with the entry
    derived at its place; refused where they disagree with the kinds the arm
    sets and clears."""
    chain = group.chain
    if group.placed is not None and values[0]:
        if not isinstance(stated, dict) or sorted(stated) != ["place", "rest"]:
            raise ValueError(f"{codec.name}: `{chain.id}` is stated as its rest and the place")
        rest, place = stated["rest"], stated["place"]
        if not 0 <= place <= len(rest) or any(
            chain.kind_of(e) == group.placed for e in rest[:place]
        ):
            raise ValueError(f"{codec.name}: the place {place} of `{chain.id}` is not its first")
        entries = rest[:place] + [named_entry(chain, group.placed)] + rest[place:]
    else:
        entries = stated
    kinds = {chain.kind_of(e) for e in entries or []}
    for clause, value in zip(group.clauses, values):
        if clause.is_member() and (clause.key in kinds) != value:
            raise ValueError(f"{codec.name}: `{chain.id}` disagrees with `{clause.label()}`")
    return entries


def gate_holds(
    codec: Codec,
    gate: Gate,
    carriers: dict[str, int],
    inputs: dict[str, int],
    values: dict,
) -> bool:
    """Whether some clause of `gate` holds of the carriers read so far, the
    flag inputs the parent hands the codec, and the kinds of the entries of a
    chain among `values`, the elements so far."""
    for clause in gate.clauses:
        if clause.is_member():
            chain = codec.field(clause.carrier)
            set_ = any(chain.kind_of(e) == clause.key for e in entries_of(values.get(chain.id)))
        elif clause.is_input():
            set_ = inputs[clause.flag] == 1
        else:
            flag = codec.flag(clause.carrier, clause.flag)
            set_ = (carriers[clause.carrier] >> flag.bit) & 1 == 1
        if set_ != clause.when_clear:
            return True
    return False


def content(f: Field, value) -> bytes:
    """The bytes a sequence field's member states."""
    if f.sce_type == "string":
        if not isinstance(value, str):
            raise ValueError(f"`{f.id}` is text, a JSON string")
        return value.encode("utf-8")
    if not isinstance(value, list) or any(not 0 <= b <= 0xFF for b in value):
        raise ValueError(f"`{f.id}` is bytes, an array of numbers in 0..=255")
    return bytes(value)


def said_input(codec: Codec, source: dict, name: str) -> int | None:
    """The value `source` says of the flag input `name` of `codec`, or `None`
    where it says none (`SCE_FORGE.md` §4.6, docs/adr/0065 §6 R4a): a field it
    presents gated on the input alone, present exactly where the gate holds;
    the arm of a group the input decides; or a body, always there, the input is
    handed on to and whose codec says it."""
    grouped = {m.id for g in codec.groups() for m in g.members()}
    for group in codec.groups():
        if [c.label() for c in group.clauses] == [name] and group.clauses[0].is_input():
            held_arm = source[group.label]
            if held_arm is None:
                (side,), _ = group.arms[0]
                return int(not side)
            return int(group.arms[held_arm["arm"]][0][0])
    for f in codec.fields:
        clause = f.gate.one() if f.gate is not None else None
        if (clause is not None and clause.is_input() and clause.flag == name
                and codec.presented(f) and f.id not in grouped):
            return int((source[f.id] is not None) != clause.when_clear)
    for f in codec.fields:
        if f.size == "body" and f.gate is None:
            for child_input, bound in f.binds.items():
                if bound == name:
                    said = said_input(f.child, source[f.id], child_input)
                    if said is not None:
                        return said
    # The start of a list that hands the input to its entries (R8), where the
    # value holds the list, and the arm of a variant the input is handed to.
    for f in codec.fields:
        if f.lawful() and source.get(f.id) is not None:
            for child_input in f.started():
                if f.binds[child_input] == name:
                    return source[f.id]["start"][child_input]
        if f.size == "variant":
            arm = arm_held(codec, f, source[f.id])
            for child_input, bound in arm.binds.items():
                if bound == name:
                    said = said_input(arm.codec, source[f.id]["body"], child_input)
                    if said is not None:
                        return said
    return None


def codec_says(codec: Codec, name: str) -> bool:
    """Whether some value of `codec` says its flag input `name` (R4a): a group
    the input decides, a field gated on it alone, or a body, a list or an arm
    the input is handed to whose codec says it."""
    if any([c.label() for c in g.clauses] == [name] and g.clauses[0].is_input()
           for g in codec.groups()):
        return True
    for f in codec.fields:
        clause = f.gate.one() if f.gate is not None else None
        if clause is not None and clause.is_input() and clause.flag == name and codec.presented(f):
            return True
        holders = [(f.child, f.binds)] if f.child is not None else []
        if f.size == "variant":
            holders = [(a.codec, a.binds) for a in [*f.arms, f.default_arm] if a is not None]
        for child, binds in holders:
            if any(bound == name and codec_says(child, child_input)
                   for child_input, bound in binds.items()):
                return True
    return False


def entries_of(value) -> list[dict]:
    """The entries of a list's value: the array itself, or a lawful list's
    `entries`; none where the list is absent."""
    if value is None:
        return []
    return value["entries"] if isinstance(value, dict) else value


def handed_to(
    codec: Codec,
    child: Codec,
    binds: dict[str, str],
    value: dict,
    source: dict,
    inputs: dict[str, int],
) -> dict[str, int]:
    """What the parent `codec` hands each flag input of the held `child`
    holding `value`, as `binds` binds it: the parent's own input, or the flag
    of its carrier, which the held codec says where it says the input and the
    value presents where it does not."""
    out = {}
    for child_input, bound in binds.items():
        carrier, _, flag = bound.partition(".")
        if not flag:
            out[child_input] = inputs[bound]
            continue
        said = said_input(child, value, child_input)
        if said is None:
            said = (source[carrier] >> codec.flag(carrier, flag).bit) & 1
        out[child_input] = said
    return out


def entries_handed(
    codec: Codec, f: Field, source: dict, inputs: dict[str, int]
) -> dict[str, int]:
    """What the list `f` of `codec` hands every entry for each input it binds
    that no law of the entries runs over (docs/adr/0065 §6 R4a, R8): the
    parent's own input, or the bit of its carrier the value presents, since an
    entry that does not say the input derives nothing of it."""
    out = {}
    started = f.started()
    for child_input, bound in f.binds.items():
        if child_input in started:
            continue
        carrier, _, flag = bound.partition(".")
        out[child_input] = (
            inputs[bound] if not flag else (source[carrier] >> codec.flag(carrier, flag).bit) & 1
        )
    return out


def handed_on(
    codec: Codec, binds: dict[str, str], carriers: dict[str, int], inputs: dict[str, int]
) -> dict[str, int]:
    """What the decode of `codec` hands a held codec for each input `binds`
    binds: its own input, or the flag of a carrier read before it."""
    out = {}
    for child_input, bound in binds.items():
        carrier, _, flag = bound.partition(".")
        out[child_input] = (
            inputs[bound] if not flag else (carriers[carrier] >> codec.flag(carrier, flag).bit) & 1
        )
    return out


def held_codecs(codec: Codec, source: dict) -> list[tuple[Codec, dict[str, str], dict]]:
    """Each codec the value of `codec` holds that its parent binds inputs of:
    a present embedded body and the arm of the variant, with its binds and the
    value it holds."""
    out = []
    for f in codec.fields:
        if f.size == "body" and f.binds and source.get(f.id) is not None:
            out.append((f.child, f.binds, source[f.id]))
        elif f.size == "variant":
            arm = arm_held(codec, f, source[f.id])
            if arm.binds:
                out.append((arm.codec, arm.binds, source[f.id]["body"]))
    return out


def bound_flags(codec: Codec, source: dict) -> dict[tuple[str, str], int]:
    """Each flag of a carrier bound to an input of a codec the value holds and
    says: the encoder derives it from what the held codec says (R4a)."""
    out = {}
    for child, binds, value in held_codecs(codec, source):
        for child_input, bound in binds.items():
            carrier, _, flag = bound.partition(".")
            if flag:
                said = said_input(child, value, child_input)
                if said is not None:
                    out[(carrier, flag)] = said
    # A flag bound to the entries of a list held to a law is its start, where
    # the value holds the list (R8, the plan's `Start` measure).
    for f in codec.fields:
        if f.lawful() and source.get(f.id) is not None:
            for child_input in f.started():
                carrier, _, flag = f.binds[child_input].partition(".")
                if flag:
                    out[(carrier, flag)] = source[f.id]["start"][child_input]
    return out


def holds_bytes(f: Field) -> bool:
    """Whether `f` writes what it holds rather than a number: bytes, text, a
    body or a list."""
    return f.sce_type in SEQUENCE or f.child is not None


def held(f: Field, value, handed: dict[str, int] | None = None) -> bytes | list[bytes]:
    """What a present element that holds bytes writes: the bytes of a sequence,
    the frame of a body, handed the flag inputs `handed`, the frame of each
    entry of a list, each within its bound. A list's entries are handed
    `handed` for each input the list binds that no law of theirs runs over,
    its start for each one that does, and what the entry before is for each
    one the chain supplies."""
    if f.sce_type in SEQUENCE:
        data = content(f, value)
    elif f.size == "body":
        if not isinstance(value, dict):
            raise ValueError(f"`{f.id}` is a body, a JSON object")
        return encode(f.child, value, handed)
    else:
        given = dict(handed or {})
        if f.lawful():
            # A list held to a law (R8) is its start and its entries; each
            # entry is handed the start, or whether the entry before is of a
            # kind, and its encode refuses a value that says otherwise.
            if not isinstance(value, dict) or sorted(value) != ["entries", "start"]:
                raise ValueError(f"`{f.id}` is held to a law: {{\"start\": .., \"entries\": ..}}")
            start, value = value["start"], value["entries"]
            started = f.started()
            if sorted(start) != sorted(started) or any(v not in (0, 1) for v in start.values()):
                raise ValueError(f"`{f.id}`'s start gives each of {sorted(started)} as 0 or 1")
            given |= start
        if not isinstance(value, list):
            raise ValueError(f"`{f.id}` is a list, a JSON array")
        # An input no law runs over is handed as the walk writes the entries:
        # an entry its value cannot say it in is held to it by its own encode.
        handed_each = f.handed_entries(given, value) if f.child.inputs else None
        if f.end_flag is not None:
            # The chain ends on its entries' flag: it is set on every entry
            # but the last, so the chain holds one entry or more.
            if not value:
                raise ValueError(f"`{f.id}` ends on an entry's flag, so it holds one or more")
            carrier, name = f.end_flag
            mask = 1 << f.child.flag(carrier, name).bit
            value = [
                entry | {carrier: (entry[carrier] | mask) if i + 1 < len(value)
                         else (entry[carrier] & ~mask)}
                for i, entry in enumerate(value)
            ]
        data = [
            encode(f.child, entry, handed_each[i] if handed_each is not None else None)
            for i, entry in enumerate(value)
        ]
    if f.most is not None and len(data) > f.most:
        raise ValueError(f"`{f.id}` holds {len(data)}, past its bound of {f.most}")
    return data


def peek_flag(variant: Field) -> Flag:
    """The flag of the peeked byte a variant's tag is in."""
    _, _, name = variant.tag_ref.partition(".")
    return next(f for f in variant.peek if f.name == name)


def tag_width(codec: Codec, variant: Field) -> int:
    """The bits a variant's tag is written in: its field's, or its flag's,
    of a carrier or of the peeked byte."""
    carrier, _, flag = variant.tag_ref.partition(".")
    if variant.peek is not None:
        return peek_flag(variant).width
    if flag:
        return codec.flag(carrier, flag).width
    tag = codec.field(carrier)
    if tag.sce_type not in WIDTH:
        raise Unread(f"{codec.name}: the tag `{carrier}` is not an unsigned number")
    return tag.bits


def arm_held(codec: Codec, variant: Field, value) -> Arm:
    """The arm `value` holds: `{"arm": alias, "body": ..}` for a known arm, or
    `{"tag": n, "body": ..}` for the default arm, which holds a tag no arm
    names and fits the tag's bits."""
    if not isinstance(value, dict) or "body" not in value:
        raise ValueError(f"{codec.name}: a variant is an object with `body`")
    if sorted(value) == ["arm", "body"]:
        arm = next((a for a in variant.arms if a.alias == value["arm"]), None)
        if arm is None:
            raise ValueError(f"{codec.name}: no arm holds `{value['arm']}`")
        return arm
    if sorted(value) == ["body", "tag"]:
        tag = value["tag"]
        if variant.default_arm is None:
            raise ValueError(f"{codec.name}: the variant has no default arm")
        if tag in (a.tag for a in variant.arms) or not 0 <= tag < 1 << tag_width(codec, variant):
            raise ValueError(f"{codec.name}: {tag} is an arm's tag or past the tag's bits")
        return variant.default_arm
    raise ValueError(f"{codec.name}: a variant states `arm` or `tag`, and `body`")


def chosen_arm(
    codec: Codec, variant: Field, value, source: dict, inputs: dict[str, int]
) -> tuple[int, bytes]:
    """The tag and the frame of the arm `value` holds, a known arm's tag the
    document's and the default arm's the one it keeps, the arm's codec handed
    each input as its import binds it."""
    arm = arm_held(codec, variant, value)
    tag = value["tag"] if arm.tag is None else arm.tag
    handed = handed_to(codec, arm.codec, arm.binds, value["body"], source, inputs)
    return tag, encode(arm.codec, value["body"], handed)


def said_length(codec: Codec, name: str, contents: dict[str, bytes | list[bytes]]) -> int | None:
    """What the measure `name` says on the wire, from the contents it measures
    that are present: the bytes of a sequence or a body less the
    `sce:length-arith` offset, or the entries of a list; `None` where none is
    present. Every present one must agree: two lists of one count (R5) are held
    of one count by their type, so a pair that differs is no value of the
    codec; any other contents that differ are two things the value says of one
    number, which the encode refuses."""
    lengths = set()
    for f in codec.measured()[name]:
        if f.id in contents:
            lengths.add(len(contents[f.id]) - f.arith)
    if len(lengths) > 1:
        if codec.aligned(name) is not None:
            raise ValueError(f"{codec.name}: the two lists `{name}` counts differ")
        raise EncodeRefused(INCONSISTENT_SOURCE, f"{codec.name}: the contents `{name}` "
                            "measures disagree")
    if not lengths:
        return None
    wire = lengths.pop()
    # A content that holds a length offset (`sce:length-arith`) holds at least
    # that many bytes by its type (docs/adr/0077 §3), so a shorter one is no
    # value of the codec.
    if wire < 0:
        raise ValueError(f"{codec.name}: a length below what `{name}` can say")
    return wire


def past_measure(codec: Codec, name: str, wire: int, bits: int) -> None:
    """Refuse the number `wire` the measure `name` derives where it is past the
    `bits` that carry it. A byte string, a text or a list is held within what
    its measure can say by its type (docs/adr/0077 §1, §3), so one past it is
    no value of the codec; the encoded length of a body is bounded by nothing
    its type holds, and the encode refuses it (docs/adr/0065 §2)."""
    if wire < 1 << bits:
        return
    if all(m.measure() == BODY_LENGTH for m in codec.measured()[name]):
        raise EncodeRefused(LENGTH_OUT_OF_RANGE, f"{codec.name}: {wire} is past the {bits} bits "
                            f"of `{name}`")
    raise ValueError(f"{codec.name}: {wire} is past the {bits} bits of `{name}`")


def handed(codec: Codec, inputs: dict | None) -> dict[str, int]:
    """The flag inputs a parent hands `codec`, each one bit, every one named."""
    inputs = inputs or {}
    if sorted(inputs) != sorted(codec.inputs) or any(v not in (0, 1) for v in inputs.values()):
        raise ValueError(f"{codec.name}: the parent hands {codec.inputs} as 0 or 1, not {inputs}")
    return inputs


def encode(codec: Codec, source: dict, inputs: dict | None = None) -> bytes:
    """The frame of `source`, `inputs` the flag inputs its parent hands it: the
    fields in declaration order, a field gated off left out, each derived
    element computed from what the value presents, and the fixed fields of a
    positional layout written together as its fixed bytes."""
    if not isinstance(source, dict):
        raise ValueError(f"{codec.name}: a source is a JSON object")
    inputs = handed(codec, inputs)
    groups = codec.groups()
    grouped = {m.id for g in groups for m in g.members()}
    presented = [f.id for f in codec.fields if codec.presented(f) and f.id not in grouped]
    presented += [g.label for g in groups]
    if sorted(source) != sorted(presented):
        raise ValueError(f"{codec.name}: a source states {presented}, not {sorted(source)}")
    # A group's arm says each of its members, the others absent, and the flags
    # of a disjunction: the value of each while the value holds the arm.
    source = dict(source)
    forced: dict[tuple[str, str], bool] = {}
    for group in groups:
        held_arm = source.pop(group.label)
        for m in group.members():
            source[m.id] = None
        values = tuple(False for _ in group.clauses)
        if held_arm is None:
            if not group.optional:
                raise ValueError(f"{codec.name}: `{group.label}` always holds an arm")
        else:
            values, fields = group.arms[held_arm["arm"]]
            if sorted(held_arm) != sorted(["arm"] + [m.id for m in fields]):
                raise ValueError(f"{codec.name}: arm {held_arm['arm']} of `{group.label}` "
                                 f"states {sorted(held_arm)}")
            for m in fields:
                source[m.id] = held_arm[m.id]
        if group.chain is not None:
            source[group.chain.id] = chain_of(codec, group, values, source[group.chain.id])
        if len(group.clauses) > 1 or group.chain is not None:
            for clause, value in zip(group.clauses, values):
                if not clause.is_member():
                    forced[(clause.carrier, clause.flag)] = value
    contents = {
        f.id: held(
            f,
            source[f.id],
            # A body is handed its inputs here; a list hands each of its entries
            # an input its law runs over from its start, and any other it binds
            # from here (`held`).
            handed_to(codec, f.child, f.binds, source[f.id], source, inputs)
            if f.binds and f.size == "body"
            else entries_handed(codec, f, source, inputs) if f.size == "list" else None,
        )
        for f in codec.fields
        if holds_bytes(f) and source[f.id] is not None
    }
    bound = bound_flags(codec, source)
    variant = codec.variant()
    tag = None
    # A tag in a flag of a carrier is that flag, derived from the arm. A tag in
    # a peeked byte is written by no one but the arm, as its own first byte.
    forced_tag = None
    if variant is not None:
        tag, contents[variant.id] = chosen_arm(codec, variant, source[variant.id], source, inputs)
        carrier, _, flag_name = variant.tag_ref.partition(".")
        if flag_name and variant.peek is None:
            forced_tag = (carrier, flag_name)
    carriers: dict[str, int] = {}
    for f in codec.fields:
        if not f.carrier:
            continue
        value = source[f.id]
        for flag in f.flags:
            mask = ((1 << flag.width) - 1) << flag.bit
            gated = codec.gate_flags().get((f.id, flag.name))
            if forced_tag == (f.id, flag.name):
                value = (value & ~mask) | (tag << flag.bit)
            elif (f.id, flag.name) in forced:
                value = (value & ~mask) | (mask if forced[(f.id, flag.name)] else 0)
            elif (f.id, flag.name) in bound:
                value = (value & ~mask) | (mask if bound[(f.id, flag.name)] else 0)
            elif gated is not None:
                # Fields one flag gates that no group holds announce it each;
                # where they disagree the value says two things of one flag.
                sets = {(source[g.id] is not None) != g.gate.when_clear for g in gated}
                if len(sets) > 1:
                    raise EncodeRefused(INCONSISTENT_SOURCE, f"{codec.name}: the fields "
                                        f"`{flag.name}` gates disagree")
                value = (value & ~mask) | (mask if sets.pop() else 0)
            elif f"{f.id}.{flag.name}" in codec.measured():
                wire = said_length(codec, f"{f.id}.{flag.name}", contents)
                if wire is None:
                    continue
                past_measure(codec, f"{f.id}.{flag.name}", wire, flag.width)
                value = (value & ~mask) | (wire << flag.bit)
        carriers[f.id] = value
    out = bytearray()
    # A positional layout's fixed fields lead it, and their bytes are written
    # once the last of them is known (docs/adr/0095 §2).
    fixed = codec.fixed() if codec.positional() else []
    words: dict[str, int] = {}
    for f in codec.fields:
        if f.gate is not None:
            holds = gate_holds(codec, f.gate, carriers, inputs, source)
            if codec.presented(f) and holds != (source[f.id] is not None):
                # A gate the encode cannot derive a flag from is one the value
                # is held to, and the encode refuses a value that contradicts
                # it (docs/adr/0065 §2). Every other gate is derived from what
                # the value presents, or decided by the inputs the case hands,
                # which the value says: there the case is written wrong.
                if codec.checked(f):
                    raise EncodeRefused(PRESENT_IF_MISMATCH, f"{codec.name}: `{f.id}` "
                                        f"disagrees with `{f.gate.text}`")
                raise ValueError(f"{codec.name}: `{f.id}` disagrees with its gate")
            if not holds:
                continue
        if f.aligned_at is not None:
            # A field aligned to a DMA burst starts at its declared byte: zeros
            # fill the frame up to it.
            if len(out) > f.aligned_at:
                raise ValueError(f"{codec.name}: `{f.id}` is past byte {f.aligned_at}")
            out += bytes(f.aligned_at - len(out))
        if f.size == "variant":
            out += contents[f.id]
            continue
        if f.carrier:
            value = carriers[f.id]
        elif codec.is_tag(f):
            value = tag
        elif codec.is_length(f):
            value = said_length(codec, f.id, contents)
            if value is None:
                value = source[f.id] if codec.stated(f) else None
            else:
                # A derived number past its field; a stated one is the value's
                # own number, held to its field's bits below.
                past_measure(codec, f.id, value, f.bits)
                # A number the value states beside content that says it is
                # said twice, and the two must agree (docs/adr/0065 §6 R4b).
                if codec.stated(f) and source[f.id] not in (None, value):
                    raise EncodeRefused(INCONSISTENT_SOURCE, f"{codec.name}: `{f.id}` is "
                                        f"stated as {source[f.id]} beside content of {value}")
            if value is None:
                raise ValueError(f"{codec.name}: nothing says `{f.id}`")
        elif holds_bytes(f):
            data = contents[f.id]
            out += b"".join(data) if f.size == "list" else data
            continue
        elif f.enum is not None:
            value = f.enum.number(source[f.id])
        else:
            value = source[f.id]
        if not isinstance(value, int) or value < 0 or value >= 1 << f.bits:
            raise ValueError(f"{codec.name}: `{f.id}` = {value!r} has more than {f.bits} bits")
        if f.size == "vle":
            out += vle(value)
        elif fixed and f.size == "fixed":
            words[f.id] = value
            if f is fixed[-1]:
                out += codec.fixed_frame(words)
        else:
            # A layout written in turn holds only whole fields (check_bits).
            out += f.word(value)
    return bytes(out)


def decode(codec: Codec, frame: bytes, inputs: dict | None = None) -> dict:
    """The `source` a frame says, the inverse of [`encode`]: a derived element
    is not a member, a stated measure is a member only where its content is
    absent, and a carrier is presented as read."""
    source, at = decode_at(codec, frame, 0, len(frame), handed(codec, inputs))
    if at != len(frame):
        raise ValueError(f"{codec.name}: {len(frame) - at} bytes left over")
    return source


def measure_on_wire(codec: Codec, name: str, raw: dict, carriers: dict[str, int]) -> int:
    """What the measure `name` read so far says: a sibling, or a flag of a
    carrier."""
    carrier, _, flag = name.partition(".")
    if flag:
        bits = codec.flag(carrier, flag)
        return (carriers[carrier] >> bits.bit) & ((1 << bits.width) - 1)
    return raw[carrier]


def decode_at(
    codec: Codec, frame: bytes, at: int, end: int, inputs: dict[str, int]
) -> tuple[dict, int]:
    """The `source` of `codec` read from `frame` at `at`, within a scope that
    ends at `end`, where a `tail` and a list read to the end stop, its parent
    handing it `inputs`; and where the read stopped."""
    raw: dict = {}
    carriers: dict[str, int] = {}

    def within(n: int, what: str) -> None:
        if at + n > end:
            raise ValueError(f"{codec.name}: the frame ends inside `{what}`")

    start = at
    # Where the codec's frame begins, from which a positional layout's fixed
    # fields lie at their declared bytes; they lead the layout.
    origin = at
    fixed = codec.fixed() if codec.positional() else []
    for f in codec.fields:
        if f.gate is not None and not gate_holds(codec, f.gate, carriers, inputs, raw):
            raw[f.id] = None
            continue
        if f.aligned_at is not None:
            # The padding before a field aligned to a DMA burst is skipped
            # unread, as the generated decode skips it.
            if at - start > f.aligned_at:
                raise ValueError(f"{codec.name}: `{f.id}` is past byte {f.aligned_at}")
            within(start + f.aligned_at - at, f.id)
            at = start + f.aligned_at
        if f.sce_type in SEQUENCE:
            if f.measured_by is None:
                n = end - at
            else:
                n = measure_on_wire(codec, f.measured_by, raw, carriers) + f.arith
            if n < 0 or (f.most is not None and n > f.most):
                raise ValueError(f"{codec.name}: `{f.id}` of {n} bytes is past its bound")
            within(n, f.id)
            data = frame[at : at + n]
            at += n
            raw[f.id] = data.decode("utf-8") if f.sce_type == "string" else list(data)
            continue
        if f.size == "body":
            handed = handed_on(codec, f.binds, carriers, inputs)
            if f.measured_by is None:
                raw[f.id], at = decode_at(f.child, frame, at, end, handed)
            else:
                n = measure_on_wire(codec, f.measured_by, raw, carriers)
                within(n, f.id)
                raw[f.id], stop = decode_at(f.child, frame, at, at + n, handed)
                if stop != at + n:
                    raise ValueError(f"{codec.name}: `{f.id}` ends before the bytes it is given")
                at += n
            continue
        if f.size == "variant":
            if f.peek is not None:
                # The arm's own first byte, read here and left for the arm.
                within(1, f.id)
                flag = peek_flag(f)
                tag = (frame[at] >> flag.bit) & ((1 << flag.width) - 1)
            else:
                tag = measure_on_wire(codec, f.tag_ref, raw, carriers)
            arm = next((a for a in f.arms if a.tag == tag), f.default_arm)
            if arm is None:
                raise ValueError(f"{codec.name}: no arm has the tag {tag}")
            handed = handed_on(codec, arm.binds, carriers, inputs)
            body, at = decode_at(arm.codec, frame, at, end, handed)
            raw[f.id] = {"body": body} | (
                {"tag": tag} if arm.tag is None else {"arm": arm.alias}
            )
            continue
        if f.size == "list":
            entries = []
            count = None
            if f.measured_by is not None:
                count = measure_on_wire(codec, f.measured_by, raw, carriers)
            # What the parent hands the entries: its own input, or a flag of a
            # carrier read before the list. A lawful list's start is that of
            # each input its law runs over.
            start = handed_on(codec, f.binds, carriers, inputs)
            more = True
            while more and ((len(entries) < count) if count is not None else (at < end)):
                if f.most is not None and len(entries) == f.most:
                    raise ValueError(f"{codec.name}: `{f.id}` holds more than {f.most} entries")
                handed = dict(start)
                for name, key in f.after_key.items():
                    handed[name] = int(bool(entries) and f.kind_of(entries[-1]) == key)
                entry, stop = decode_at(f.child, frame, at, end, handed)
                if stop == at:
                    raise ValueError(f"{codec.name}: an entry of `{f.id}` reads no byte")
                entries.append(entry)
                at = stop
                if f.end_flag is not None:
                    # Read until an entry whose flag says no other follows.
                    carrier, name = f.end_flag
                    more = (entry[carrier] >> f.child.flag(carrier, name).bit) & 1 == 1
            if f.end_flag is not None and (not entries or more):
                raise ValueError(f"{codec.name}: the frame ends inside `{f.id}`")
            if f.lawful():
                raw[f.id] = {"start": {n: start[n] for n in f.started()}, "entries": entries}
            else:
                raw[f.id] = entries
            continue
        if f.size == "vle":
            within(1, f.id)
            value, at = read_vle(frame[:end], at)
        elif fixed and f.size == "fixed":
            # Each field is read from its word, and a byte no field holds is
            # not read (docs/adr/0095 §2).
            if f is fixed[0]:
                within(codec.fixed_end(), "the fixed bytes")
            value = f.of_word(frame[origin + f.byte : origin + f.byte + f.word_bytes()])
            if f is fixed[-1]:
                at = origin + codec.fixed_end()
        else:
            n = f.word_bytes()
            within(n, f.id)
            value = f.of_word(frame[at : at + n])
            at += n
        raw[f.id] = value
        if f.carrier:
            carriers[f.id] = value
    source: dict = {}
    for f in codec.fields:
        if not codec.presented(f):
            continue
        if codec.stated(f) and any(raw[c.id] is not None for c in codec.measured()[f.id]):
            source[f.id] = None
        elif f.enum is not None and raw[f.id] is not None:
            source[f.id] = f.enum.value_of(raw[f.id])
        else:
            source[f.id] = raw[f.id]
    # A group is the arm its flags and its chain's kinds say, holding its
    # members; none where every flag of an optional group says no arm. A bare
    # entry's arm holds the chain as the others and the place of the first.
    for group in codec.groups():
        values = tuple(
            gate_holds(codec, Gate([clause], clause.label()), carriers, inputs, raw)
            for clause in group.clauses
        )
        read = {m.id: source.pop(m.id) for m in group.members()}
        arm = next((i for i, (v, _) in enumerate(group.arms) if v == values), None)
        if arm is None:
            source[group.label] = None
            continue
        if group.placed is not None and values[0]:
            chain = group.chain
            entries = read[chain.id]
            place = next(i for i, e in enumerate(entries) if chain.kind_of(e) == group.placed)
            read[chain.id] = {"rest": entries[:place] + entries[place + 1:], "place": place}
        source[group.label] = {"arm": arm} | {m.id: read[m.id] for m in group.arms[arm][1]}
    return source, at


@dataclass
class Written:
    """The sources of a fixture with their frames, and its refused cases with
    the refusal of each, each held by the model."""

    sources: list[dict]
    refused: list[dict] = field(default_factory=list)


def write(stem: str, sources: list[tuple], refused: list[tuple] | None = None) -> Written:
    """Each `(source, note)`, or `(source, note, inputs)` for a codec that
    takes flag inputs, with its frame, held by the model. A case of such a
    codec states the inputs its parent hands it, which its decode is given.

    Each refused case `(source, kind, note)`, or `(source, kind, note,
    inputs)`, is a value of the codec's interface whose encode the model holds
    to refusing with `kind` (docs/adr/0065 §2): a value it writes a frame of,
    or refuses otherwise, is a case written wrong, and so is one that is no
    value of the codec at all (`ValueError`), which no encode is called on."""
    codec = read_codec(stem)
    out = []
    for source, note, *rest in sources:
        inputs = rest[0] if rest else None
        frame = encode(codec, source, inputs)
        again = decode(codec, frame, inputs)
        if again != source:
            raise AssertionError(f"{stem}: {source} reads back as {again}")
        case = {"source": source}
        if codec.inputs:
            case["inputs"] = inputs
        out.append(case | {"encoded": list(frame), "note": note})
    refusals = []
    if refused and not codec.may_fail:
        raise AssertionError(f"{stem}: states that its encode refuses nothing, and has refused "
                             "cases (sce:encode-may-fail, docs/adr/0065 §1)")
    for source, kind, note, *rest in refused or []:
        if kind not in REFUSALS:
            raise AssertionError(f"{stem}: `{kind}` is no refusal; one of {REFUSALS}")
        inputs = rest[0] if rest else None
        try:
            frame = encode(codec, source, inputs)
        except EncodeRefused as refusal:
            if refusal.kind != kind:
                raise AssertionError(
                    f"{stem}: {source} is refused as {refusal.kind}, not {kind}: {refusal}"
                ) from refusal
        else:
            raise AssertionError(f"{stem}: {source} is no refused value: its frame is "
                                 f"{list(frame)}")
        case = {"source": source}
        if codec.inputs:
            case["inputs"] = inputs
        refusals.append(case | {"error": kind, "note": note})
    return Written(out, refusals)


@dataclass
class Fixture:
    """A fixture's hand-chosen sources, what its entry says where the model
    writes the entry itself, and its hand-chosen refused cases: each a value
    of the codec's interface, the refusal its encode answers it with and a
    note."""

    sources: list[tuple[dict, str]]
    description: str = ""
    refused: list[tuple[dict, str, str]] = field(default_factory=list)


ABOUT = (
    "docs/adr/0081 §5: each source states the value as the codec's interface presents it, "
    "and its frame is written by tests/forge/conformance/binary_model.py, an independent "
    "reading of the page, which holds each to itself before writing it. "
)

ASCII = list(range(0x61, 0x61 + 26))


def ext_unit(ext_id: int, more: bool) -> dict:
    """A Zenoh unit extension of `codec_zenoh_ext_entry`: `enc` 0 is its arm's
    tag, and `Z` is set where another entry follows, as the chain derives it."""
    return {"header": ext_id | (0x80 if more else 0),
            "body": {"arm": "codec_zenoh_ext_unit", "body": {}}}


def ext_zint(ext_id: int, value: int, more: bool) -> dict:
    """A Zenoh zint extension: `enc` 1 (bits 5 and 6) is its arm's tag."""
    return {"header": ext_id | 0x20 | (0x80 if more else 0),
            "body": {"arm": "codec_zenoh_ext_zint", "body": {"value": value}}}


def ext_zbuf(ext_id: int, value: list, more: bool) -> dict:
    """A Zenoh zbuf extension: `enc` 2 is its arm's tag."""
    return {"header": ext_id | 0x40 | (0x80 if more else 0),
            "body": {"arm": "codec_zenoh_ext_zbuf", "body": {"value": value}}}


def flag_entry(kind: int, value: int, wide: int, more: bool) -> dict:
    """An entry of `codec_chain_flag_entry` handed `wide`: the long value
    where it is set, the short one where it is not; `Z` set where another
    entry follows."""
    arm = {"arm": 0, "long_value": value} if wide else {"arm": 1, "short_value": value}
    return {"header": kind | (0x80 if more else 0), "wide": arm}


def prev_entry(ext_id: int, value: int, after: int, tag: int | None, more: bool) -> dict:
    """An entry of `codec_chain_prev_entry` handed `after_marker` (whether the
    entry before is of kind 4) and `tagged` (whether it has a tag)."""
    arm = {"arm": 0, "long_value": value} if after else {"arm": 1, "short_value": value}
    return {"header": ext_id | (0x80 if more else 0), "after_marker": arm, "tag": tag}


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
    "codec_optional_field_attributes": Fixture(
        [
            ({"status": 0x0C, "reading": 0xABC},
             "ready and fault set; reading is the word 0xABC0 over bytes 1 and 2"),
            ({"status": 0x00, "reading": 0}, "every field zero: three zero bytes"),
            ({"status": 0x04, "reading": 0xFFF},
             "ready alone; reading at its largest, the low four bits of byte 2 no field's"),
            ({"status": 0x00, "reading": 0x00F},
             "the low four bits of reading are the high four of byte 2, and nothing else is set"),
        ],
        "a one-byte flags carrier read little-endian, and a 12-bit big-endian field four bits "
        "into byte 1 that spans bytes 1 and 2 (docs/adr/0095 §1).",
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
             "128 bytes of text: the length takes two VLE bytes"),
            ({"locator": "".join(chr(c) for c in ASCII) * 9 + "abcdefghijklmnopqrstu"},
             "text at its bound of 255 bytes, the u8 length upstream reads it with"),
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
            ({"kind": 0xFF, "data": list(range(16))}, "sixteen bytes of data"),
            ({"kind": 1, "data": list(range(200))}, "200 bytes: the length takes two VLE bytes"),
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
    "codec_ext_encoding_info": Fixture(
        [
            ({"combined_id": 0x00, "combined_id.has_schema": {"arm": 1, "schema_size": 0}},
             "no schema: the value states the size, zero"),
            ({"combined_id": 0x00, "combined_id.has_schema": {"arm": 1, "schema_size": 7}},
             "no schema: the size is what the value states"),
            ({"combined_id": 0x01, "combined_id.has_schema": {"arm": 0, "schema": [1, 2, 3]}},
             "a schema: has_schema and the size are derived from it"),
            ({"combined_id": 0x81, "combined_id.has_schema": {"arm": 0, "schema": []}},
             "an empty schema is present; the carrier takes two VLE bytes"),
        ],
        "a length stated where its gated content is absent is held with that content, on the "
        "side where it is not (R4b): arm 0 holds the schema, arm 1 the size.",
    ),
    "codec_present_if_length_ref": Fixture(
        [
            ({"flags": 0x00, "flags.has_payload": {"arm": 1, "payload_size": 0}},
             "no payload, a stated size of zero"),
            ({"flags": 0x00, "flags.has_payload": {"arm": 1, "payload_size": 9}},
             "no payload, a stated size of nine"),
            ({"flags": 0x01, "flags.has_payload": {"arm": 0, "payload": [5, 6]}},
             "a payload: the size and has_payload are derived"),
            ({"flags": 0x01, "flags.has_payload": {"arm": 0, "payload": list(range(32))}},
             "a payload at its bound"),
        ],
        "a length stated where its gated content is absent is held with that content, on the "
        "side where it is not (R4b): arm 0 holds the payload, arm 1 the size.",
    ),
    "codec_present_if_disjunction": Fixture(
        [
            ({"flags": 0x00, "flags.wants_a || flags.wants_b": None},
             "neither flag: no seq, the frame is the carrier alone"),
            ({"flags": 0x01, "flags.wants_a || flags.wants_b": {"arm": 0, "seq": 0x1234}},
             "wants_a alone: arm 0"),
            ({"flags": 0x02, "flags.wants_a || flags.wants_b": {"arm": 1, "seq": 5}},
             "wants_b alone: arm 1"),
            ({"flags": 0x03, "flags.wants_a || flags.wants_b": {"arm": 2, "seq": 0xFFFF}},
             "both flags: arm 2"),
            ({"flags": 0xFC, "flags.wants_a || flags.wants_b": None},
             "the bits the value presents are kept beside the derived ones"),
        ],
        "a disjunction no other gate reads is a group (R4c): an arm for each assignment of its "
        "flags that satisfies it, the first flag the lowest bit, and the flags derived from it.",
    ),
    "codec_present_if_checked": Fixture(
        [
            ({"flags": 0x00, "seq": None, "trailer": None},
             "neither flag: no seq and no trailer, the frame is the carrier alone"),
            ({"flags": 0x01, "seq": 0x1234, "trailer": 0x07},
             "seq sets has_seq, so the trailer is there by has_seq"),
            ({"flags": 0x02, "seq": None, "trailer": 0xFF},
             "wants_trailer alone: the trailer is there without seq"),
            ({"flags": 0x03, "seq": 0x0000, "trailer": 0x00},
             "both flags: seq and the trailer, each zero and still present"),
            ({"flags": 0xFC, "seq": None, "trailer": None},
             "the bits the value presents beside the two flags are kept"),
        ],
        "a disjunction one of whose flags another gate reads is a check and not a group: "
        "has_seq is derived from seq, wants_trailer is the value's own, and the trailer is "
        "there exactly where either is set; the encode refuses a value whose trailer says "
        "otherwise (docs/adr/0065 §2).",
        [
            ({"flags": 0x00, "seq": None, "trailer": 0x05}, PRESENT_IF_MISMATCH,
             "the trailer given where neither flag is set"),
            ({"flags": 0x00, "seq": 0x0102, "trailer": None}, PRESENT_IF_MISMATCH,
             "the trailer left out where seq sets has_seq, the value's own has_seq bit clear"),
            ({"flags": 0x02, "seq": None, "trailer": None}, PRESENT_IF_MISMATCH,
             "the trailer left out where wants_trailer is set"),
            ({"flags": 0x01, "seq": None, "trailer": 0x09}, PRESENT_IF_MISMATCH,
             "the trailer given where the value's own has_seq bit is set and no seq derives it"),
        ],
    ),
    "codec_zenoh_wireexpr": Fixture(
        [
            ({"id": 5, "suffix": None}, "N clear: the id alone", {"N": 0}),
            ({"id": 0x80, "suffix": "a/b"}, "N set: the suffix and its length follow",
             {"N": 1}),
            ({"id": 1, "suffix": ""}, "an empty suffix is present", {"N": 1}),
        ],
        "the parent hands N, which the value says: a suffix is there exactly where N is set.",
    ),
    "codec_zenoh_decl_kexpr": Fixture(
        [
            ({"id": 3, "wireexpr": {"id": 7, "suffix": None}},
             "N clear, handed on: the key expression is its id alone", {"N": 0}),
            ({"id": 0x80, "wireexpr": {"id": 1, "suffix": "a/b"}},
             "N set, handed on: the suffix follows the id", {"N": 1}),
        ],
        "the body hands its own input N to the key expression it embeds, which says it.",
    ),
    "codec_zenoh_decl_subscriber": Fixture(
        [
            ({"id": 0, "wireexpr": {"id": 0, "suffix": None}},
             "N clear: an id and a bare key expression", {"N": 0}),
            ({"id": 0x4000, "wireexpr": {"id": 2, "suffix": "x"}},
             "N set: a three-byte id, the key expression with its suffix", {"N": 1}),
        ],
        "the body hands its own input N to the key expression it embeds.",
    ),
    "codec_zenoh_decl_token": Fixture(
        [
            ({"id": 9, "wireexpr": {"id": 4, "suffix": None}}, "N clear", {"N": 0}),
            ({"id": 9, "wireexpr": {"id": 4, "suffix": ""}},
             "N set: an empty suffix is still present", {"N": 1}),
        ],
        "the body hands its own input N to the key expression it embeds.",
    ),
    "codec_zenoh_decl_queryable": Fixture(
        [
            ({"id": 1, "wireexpr": {"id": 5, "suffix": None}, "Z": None},
             "N and Z clear: no suffix, no extension", {"N": 0, "Z": 0}),
            ({"id": 1, "wireexpr": {"id": 5, "suffix": "q"},
              "Z": {"arm": 0, "ext_type": 0x21, "ext_value": 0x0301}},
             "N and Z set: the suffix, then the extension", {"N": 1, "Z": 1}),
            ({"id": 2, "wireexpr": {"id": 6, "suffix": None},
              "Z": {"arm": 0, "ext_type": 0x21, "ext_value": 1}},
             "Z set alone: the extension without a suffix", {"N": 0, "Z": 1}),
        ],
        "N is handed on to the key expression; Z, the body's own input, gates the extension.",
    ),
    "codec_zenoh_interest_body": Fixture(
        [
            ({"header": 0x01, "keyexpr": None},
             "not restricted: the flags byte alone, N its own"),
            ({"header": 0x3F, "keyexpr": {"id": 3, "suffix": "k/*"}},
             "restricted, the key expression says N: bit 5 is what it says"),
            ({"header": 0x12, "keyexpr": {"id": 8, "suffix": None}},
             "restricted, the key expression says N clear: bit 5 stays clear"),
        ],
        "the flags byte's N is the key expression's input: derived from the key expression "
        "where the body holds one, the value's own where it does not.",
    ),
    "codec_zenoh_value_zbuf": Fixture(
        [
            ({"after_shm": {"arm": 1, "value": [1, 2, 3]}},
             "after_shm clear: the value and its length", {"after_shm": 0}),
            ({"after_shm": {"arm": 0, "value_len": 7, "encoding": 3,
                            "slices": [{"kind": 1, "data": [9]}, {"kind": 2, "data": []}]}},
             "after_shm set: the stated length, the encoding and two slices, counted",
             {"after_shm": 1}),
            ({"after_shm": {"arm": 0, "value_len": 0, "encoding": 0, "slices": []}},
             "after_shm set and no slice: the count is zero", {"after_shm": 1}),
        ],
        "a group one flag input decides, one side holding a list of slices.",
    ),
    "codec_zenoh_value_entry": Fixture(
        [
            ({"header": 0x43, "body": {"arm": "codec_zenoh_value_zbuf", "body": {
                "after_shm": {"arm": 1, "value": [5]}}}},
             "the zbuf arm, handed after_shm clear by the entry", {"after_shm": 0}),
            ({"header": 0xC3, "body": {"arm": "codec_zenoh_value_zbuf", "body": {
                "after_shm": {"arm": 0, "value_len": 1, "encoding": 2,
                              "slices": [{"kind": 0, "data": [1, 2]}]}}}},
             "the zbuf arm, handed after_shm set, Z the value's own", {"after_shm": 1}),
            ({"header": 0x21, "body": {"arm": "codec_zenoh_ext_zint", "body": {"value": 4}}},
             "the zint arm, which takes no input", {"after_shm": 1}),
        ],
        "an arm handed the entry's own input, as an embedded body is.",
    ),
    "codec_zenoh_interest": Fixture(
        [
            ({"header": 0x19, "id": 1, "header.CURRENT || header.FUTURE": None,
              "extensions": None},
             "final: neither flag, so no body"),
            ({"header": 0x39, "id": 2,
              "header.CURRENT || header.FUTURE": {"arm": 0, "body": {"header": 0x01,
                                                                    "keyexpr": None}},
              "extensions": None},
             "CURRENT alone: the body, not restricted"),
            ({"header": 0x59, "id": 0x80,
              "header.CURRENT || header.FUTURE": {"arm": 1, "body": {
                  "header": 0x3F, "keyexpr": {"id": 3, "suffix": "k/*"}}},
              "extensions": None},
             "FUTURE alone: a restricted body with its key expression"),
            ({"header": 0xF9, "id": 3,
              "header.CURRENT || header.FUTURE": {"arm": 2, "body": {"header": 0x02,
                                                                    "keyexpr": None}},
              "extensions": [{"header": 0x01, "body": {"arm": "codec_zenoh_ext_unit",
                                                       "body": {}}}]},
             "both flags and Z: the body, then one extension"),
        ],
        "a body a disjunction of two flags decides, each arm setting the flags it names.",
    ),
    "codec_dma_aligned_basic": Fixture(
        [
            ({"msg_id": 0x01, "reserved": 0x00, "aligned_payload": list(b"HELLO")},
             "the document's own oracle: 30 zero bytes, then the payload at byte 32"),
            ({"msg_id": 0xFF, "reserved": 0x7E, "aligned_payload": []},
             "no payload: the frame ends at the aligned byte"),
            ({"msg_id": 2, "reserved": 3, "aligned_payload": list(range(64))},
             "a payload at its bound"),
        ],
        "a field aligned to a DMA burst starts at its declared byte, zeros before it.",
    ),
    "codec_repeat_unconditional_count": Fixture(
        [
            ({"options": 0x01, "options.H": {"arm": 0, "links": [{"seq": 1}, {"seq": 2}],
                                             "weights": [{"seq": 0x10}, {"seq": 0x20}]}},
             "H set: the links and the weights, two of each, one count"),
            ({"options": 0x00, "options.H": {"arm": 1, "links": [{"seq": 3}]}},
             "H clear: the links alone"),
            ({"options": 0x01, "options.H": {"arm": 0, "links": [], "weights": []}},
             "H set and no entry: both lists empty, the count zero"),
        ],
        "two lists of one count (R5): the gated one held with the other, of one length.",
    ),
    "codec_chain_flag_carrier": Fixture(
        [
            ({"head": 0x00, "entries": {"start": {"wide": 0},
                                        "entries": [flag_entry(1, 5, 0, True),
                                                    flag_entry(2, 6, 0, False)]}},
             "the start clear: every entry short, W derived clear"),
            ({"head": 0x01, "entries": {"start": {"wide": 1},
                                        "entries": [flag_entry(3, 0x1234, 1, False)]}},
             "the start set: the entry long, W derived set"),
        ],
        "a flag bound to every entry of a chain is its law's start (R8, Uniform).",
    ),
    "codec_chain_flag_forward": Fixture(
        [
            ({"head": 0x01, "entries": {"start": {"wide": 1},
                                        "entries": [flag_entry(1, 0x0102, 1, False)]}},
             "the codec's own input handed on: the start says it", {"wide": 1}),
            ({"head": 0x01, "entries": {"start": {"wide": 0},
                                        "entries": [flag_entry(1, 9, 0, True),
                                                    flag_entry(2, 8, 0, False)]}},
             "the input clear, two short entries", {"wide": 0}),
            ({"head": 0x00, "entries": None},
             "no chain: E clear, and the value says nothing of the input", {"wide": 1}),
        ],
        "the codec's own input bound to every entry: the list's start says it.",
    ),
    "codec_repeat_flag_carrier": Fixture(
        [
            ({"head": 0x01, "entries": {"start": {"wide": 1},
                                        "entries": [flag_entry(1, 0x10, 1, False),
                                                    flag_entry(2, 0x20, 1, False)]}},
             "two long entries, counted"),
            ({"head": 0x00, "entries": {"start": {"wide": 0}, "entries": []}},
             "no entry: the start is the value's, W clear"),
            ({"head": 0x01, "entries": {"start": {"wide": 1}, "entries": []}},
             "no entry and the start set: W follows the start, not the entries"),
        ],
        "a counted list's law: its start is the value's even where it holds no entry.",
    ),
    "codec_repeat_flag_gated": Fixture(
        [
            ({"head": 0x03, "entries": {"start": {"wide": 1},
                                        "entries": [flag_entry(1, 7, 1, False)]}},
             "L set by the list, W by its start"),
            ({"head": 0x00, "entries": None}, "no list: L clear, W the value's own"),
            ({"head": 0x01, "entries": None}, "no list and W set: nothing derives it"),
        ],
        "a gated list's law: the bound flag is its start where the list is there.",
    ),
    "codec_chain_prev_carrier": Fixture(
        [
            ({"head": 0x00, "entries": {"start": {"tagged": 0}, "entries": [
                prev_entry(4, 1, 0, None, True), prev_entry(2, 0x0203, 1, None, False)]}},
             "after an entry of kind 4 the next is handed after_marker set"),
            ({"head": 0x01, "entries": {"start": {"tagged": 1}, "entries": [
                prev_entry(1, 5, 0, 9, False)]}},
             "the start sets tagged, so the entry has its tag; the first is never after"),
        ],
        "two laws run together: tagged from the start (T), after_marker from the entry before.",
    ),
    "codec_chain_prev_gated": Fixture(
        [
            ({"head": 0x03, "entries": {"start": {"tagged": 1}, "entries": [
                prev_entry(4, 1, 0, 1, True), prev_entry(4, 0x0506, 1, 2, True),
                prev_entry(1, 7, 1, 3, False)]}},
             "two of kind 4 in a row: each entry after one is handed after_marker"),
            ({"head": 0x00, "entries": None}, "no chain: E clear, T the value's own"),
        ],
        "the kind is the whole header less the chain's own Z.",
    ),
    "codec_chain_prev_tail": Fixture(
        [
            ({"head": 0x00, "entries": {"start": {"tagged": 0}, "entries": [
                prev_entry(4, 1, 0, None, False), prev_entry(5, 2, 1, None, False)]}},
             "a chain read to the end: Z is each entry's own"),
        ],
        "the entry before's kind, in a chain that ends with the frame.",
    ),
    "codec_chain_prev_tail_gated": Fixture(
        [
            ({"head": 0x02, "entries": {"start": {"tagged": 0}, "entries": [
                prev_entry(4, 9, 0, None, False)]}},
             "one entry: E set by the chain"),
            ({"head": 0x00, "entries": None}, "no chain"),
        ],
        "a gated chain read to the end, its law as an ungated one's.",
    ),
    "codec_zenoh_query_value": Fixture(
        [
            ({"exts": {"start": {}, "entries": [
                {"header": 0x84, "body": {"arm": "codec_zenoh_ext_unit", "body": {}}},
                {"header": 0x43, "body": {"arm": "codec_zenoh_value_zbuf", "body": {
                    "after_shm": {"arm": 0, "value_len": 1, "encoding": 2,
                                  "slices": [{"kind": 0, "data": [1]}]}}}}]}},
             "the shared-memory marker (kind 4), then a zbuf handed after_shm"),
            ({"exts": {"start": {}, "entries": [
                {"header": 0x43, "body": {"arm": "codec_zenoh_value_zbuf", "body": {
                    "after_shm": {"arm": 1, "value": [7]}}}}]}},
             "a zbuf first: nothing before it, so its value inline"),
        ],
        "a law with no start: each entry is handed whether the one before is the marker.",
    ),
    "codec_chain_has_envelope": Fixture(
        [
            ({"header": 0x80, "extensions.has(0x2)": {
                "arm": 0, "extensions": [ext_zbuf(2, [1, 2], False)],
                "slices": [{"kind": 4, "bytes": [7]}]}},
             "an entry of kind 2: the slices, counted; E set by the chain"),
            ({"header": 0x80, "extensions.has(0x2)": {
                "arm": 0, "extensions": [ext_unit(1, True), ext_zbuf(2, [], False)],
                "slices": []}},
             "kind 2 second in the chain, no slice"),
            ({"header": 0x00, "extensions.has(0x2)": {
                "arm": 1, "extensions": None, "payload": [9]}},
             "no chain: E clear, the payload and its length"),
            ({"header": 0x80, "extensions.has(0x2)": {
                "arm": 1, "extensions": [ext_unit(1, False)], "payload": []}},
             "a chain with no entry of kind 2: the payload"),
        ],
        "the fields a chain's kind 2 decides, the chain in each arm: an entry with content.",
    ),
    "codec_chain_has_marker": Fixture(
        [
            ({"header": 0x80, "extensions.has(0x12)": {
                "arm": 0, "extensions": {"rest": [], "place": 0},
                "slices": [{"kind": 4, "data": [1]}]}},
             "the marker alone: derived from its kind, its Z clear as the last"),
            ({"header": 0x80, "extensions.has(0x12)": {
                "arm": 0, "extensions": {"rest": [ext_zint(1, 5, True)], "place": 1},
                "slices": []}},
             "the marker second, after a zint"),
            ({"header": 0x80, "extensions.has(0x12)": {
                "arm": 0, "extensions": {"rest": [ext_unit(3, False)], "place": 0},
                "slices": []}},
             "the marker first: the chain sets its Z, another entry following"),
            ({"header": 0x00, "extensions.has(0x12)": {
                "arm": 1, "extensions": None, "payload": [1, 2]}},
             "no chain: the payload"),
        ],
        "a bare marker (kind 0x12: id 2, M set, unit) is held as its place among the others.",
    ),
    "codec_chain_has_tagged": Fixture(
        [
            ({"header": 0x00, "entries.has(0x7) || entries.has(0x9) || header.wide": {
                "arm": 7, "entries": [{"entry_type": 1, "ctl": 0, "body": 5}],
                "checksum": 0x1234}},
             "no kind 7 or 9 and not wide: the checksum alone"),
            ({"header": 0x00, "entries.has(0x7) || entries.has(0x9) || header.wide": {
                "arm": 4, "entries": [{"entry_type": 7, "ctl": 1, "body": 1},
                                      {"entry_type": 1, "ctl": 0, "body": 2}],
                "priority": 3, "checksum": 0xABCD}},
             "kind 7: the priority and the checksum"),
            ({"header": 0x00, "entries.has(0x7) || entries.has(0x9) || header.wide": {
                "arm": 5, "entries": [{"entry_type": 9, "ctl": 0, "body": 0}]}},
             "kind 9 alone: neither field"),
            ({"header": 0x01, "entries.has(0x7) || entries.has(0x9) || header.wide": {
                "arm": 6, "entries": [{"entry_type": 2, "ctl": 0, "body": 0}],
                "priority": 1, "checksum": 2}},
             "wide alone: the flag derived from the arm"),
            ({"header": 0x01, "entries.has(0x7) || entries.has(0x9) || header.wide": {
                "arm": 0, "entries": [{"entry_type": 9, "ctl": 1, "body": 0},
                                      {"entry_type": 7, "ctl": 0, "body": 0}],
                "priority": 4}},
             "every atom set: the priority, no checksum"),
        ],
        "two kinds and a flag: an arm for each of their eight assignments, the chain in each.",
    ),
    "codec_variant_peek_basic": Fixture(
        [
            ({"body": {"arm": "codec_peek_arm_a", "body": {"header": 0x00, "payload": 5}}},
             "arm a: its header's kind is clear, and the variant writes no tag"),
            ({"body": {"arm": "codec_peek_arm_b", "body": {"header": 0x01, "payload": 0x1234}}},
             "arm b: its header's kind is set"),
            ({"body": {"arm": "codec_peek_arm_a", "body": {"header": 0xFE, "payload": 0}}},
             "only the peeked flag decides: the header's other bits are arm a's own"),
        ],
        "the tag is a flag of the arm's own first byte, read without moving past it.",
    ),
    "codec_zenoh_request": Fixture(
        [
            ({"header": 0x1C, "rid": 1, "keyexpr": {"id": 5, "suffix": None}, "extensions": None,
              "body": {"arm": "codec_zenoh_msg_put", "body": {
                  "header": 0x01, "timestamp": None, "encoding": None, "extensions": None,
                  "payload": [0xAA]}}},
             "a put: its header's mid 1 is the tag"),
            ({"header": 0x3C, "rid": 0x80, "keyexpr": {"id": 5, "suffix": "a"}, "extensions": None,
              "body": {"arm": "codec_zenoh_msg_del", "body": {
                  "header": 0x02, "timestamp": None, "extensions": None}}},
             "a del, the key expression's suffix setting N"),
            ({"header": 0x1C, "rid": 2, "keyexpr": {"id": 0, "suffix": None}, "extensions": None,
              "body": {"arm": "codec_zenoh_query", "body": {
                  "header": 0x03, "consolidation": None, "parameters": None, "extensions": None}}},
             "a query"),
            ({"header": 0x1C, "rid": 3, "keyexpr": {"id": 0, "suffix": None}, "extensions": None,
              "body": {"tag": 0x07, "body": {
                  "header": 0x07, "consolidation": None, "parameters": None, "extensions": None}}},
             "a mid no arm names: the default query keeps it, its header stating it"),
        ],
        "the arm is the one its own header's mid names, peeked and left for the arm.",
    ),
    "codec_zenoh_response": Fixture(
        [
            ({"header": 0x1B, "request_id": 1, "key_id": 2, "suffix": None, "extensions": None,
              "body": {"arm": "codec_zenoh_reply", "body": {
                  "header": 0x04, "consolidation": None, "extensions": None,
                  "body": {"header": 0x01,
                           "body": {"arm": "codec_zenoh_put", "body": {"payload": 3}}}}}},
             "a reply holding a put"),
            ({"header": 0x3B, "request_id": 1, "key_id": 2, "suffix": "x", "extensions": None,
              "body": {"arm": "codec_zenoh_err", "body": {
                  "header": 0x05, "encoding": None, "extensions": None, "payload": [1]}}},
             "an err, the suffix setting N"),
            ({"header": 0x1B, "request_id": 0, "key_id": 0, "suffix": None, "extensions": None,
              "body": {"tag": 0x06, "body": {
                  "header": 0x06, "consolidation": None, "extensions": None,
                  "body": {"header": 0x02, "body": {"arm": "codec_zenoh_del", "body": {}}}}}},
             "a mid no arm names: the default reply keeps it"),
        ],
        "the arm is the one its own header's mid names.",
    ),
    "codec_zenoh_network_envelope": Fixture(
        [
            ({"body": {"arm": "codec_zenoh_interest", "body": {
                "header": 0x19, "id": 1, "header.CURRENT || header.FUTURE": None,
                "extensions": None}}},
             "an interest, mid 0x19"),
            ({"body": {"arm": "codec_zenoh_response_final", "body": {
                "header": 0x1A, "request_id": 7, "extensions": None}}},
             "a final response, mid 0x1a"),
            ({"body": {"arm": "codec_zenoh_response", "body": {
                "header": 0x1B, "request_id": 1, "key_id": 2, "suffix": None, "extensions": None,
                "body": {"arm": "codec_zenoh_err", "body": {
                    "header": 0x05, "encoding": None, "extensions": None, "payload": []}}}}},
             "a response holding an err, mid 0x1b"),
            ({"body": {"arm": "codec_zenoh_request", "body": {
                "header": 0x1C, "rid": 1, "keyexpr": {"id": 5, "suffix": None},
                "extensions": None,
                "body": {"arm": "codec_zenoh_msg_del", "body": {
                    "header": 0x02, "timestamp": None, "extensions": None}}}}},
             "a request holding a del, mid 0x1c"),
            ({"body": {"arm": "codec_zenoh_push", "body": {
                "header": 0x1D, "body": {"arm": "codec_zenoh_push_body", "body": {
                    "header": 0x01,
                    "body": {"arm": "codec_zenoh_put", "body": {"payload": 7}}}}}}},
             "a push, mid 0x1d"),
            ({"body": {"arm": "codec_zenoh_declare", "body": {
                "header": 0x1E, "interest_id": None, "extensions": None,
                "declaration": {"header": 0x1A, "body": {
                    "arm": "codec_zenoh_decl_final", "body": {}}}}}},
             "a declare holding the final declaration, mid 0x1e"),
            ({"body": {"arm": "codec_zenoh_oam", "body": {
                "header": 0x1F, "id": 1, "extensions": None,
                "body": {"arm": "codec_zenoh_ext_unit", "body": {}}}}},
             "an OAM, mid 0x1f"),
            ({"body": {"tag": 0x10, "body": {
                "header": 0x10, "id": 2, "extensions": None,
                "body": {"arm": "codec_zenoh_ext_unit", "body": {}}}}},
             "a mid no arm names: the default OAM keeps it, its header stating it"),
        ],
        "a Zenoh network message: the arm is the one its own header's mid names.",
    ),
    "codec_zenoh_declaration": Fixture(
        [
            ({"header": 0x20, "body": {"arm": "codec_zenoh_decl_kexpr",
                                       "body": {"id": 3, "wireexpr": {"id": 7, "suffix": "a"}}}},
             "a key-expression declaration whose suffix says N: mid 0 and N set"),
            ({"header": 0x84, "body": {"arm": "codec_zenoh_decl_queryable",
                                       "body": {"id": 1, "wireexpr": {"id": 5, "suffix": None},
                                                "Z": {"arm": 0, "ext_type": 0x21,
                                                      "ext_value": 1}}}},
             "a queryable that says Z by its extension and N clear by its key expression"),
            ({"header": 0x03, "body": {"arm": "codec_zenoh_undecl_subscriber",
                                       "body": {"id": 2, "ext_keyexpr": None}}},
             "a subscriber undeclared with no extension: Z clear"),
            ({"header": 0x61, "body": {"arm": "codec_zenoh_undecl_kexpr", "body": {"id": 0}}},
             "an arm that binds nothing: N and M are the value's own"),
            ({"header": 0x1A, "body": {"arm": "codec_zenoh_decl_final", "body": {}}},
             "the final declaration's own tag"),
            ({"header": 0x1B, "body": {"tag": 0x1B, "body": {}}},
             "a tag no arm names: the default arm keeps it"),
        ],
        "each arm's codec is handed the header's N or Z as its import binds it, and the "
        "header's flag is what the arm says: derived where the arm binds and says it, the "
        "value's own where it does not.",
    ),
    "codec_zenoh_declare": Fixture(
        [
            ({"header": 0x1E, "interest_id": None, "extensions": None,
              "declaration": {"header": 0x20, "body": {
                  "arm": "codec_zenoh_decl_kexpr",
                  "body": {"id": 3, "wireexpr": {"id": 7, "suffix": "a"}}}}},
             "no interest id, no extension: the declaration alone"),
            ({"header": 0x3E, "interest_id": 5, "extensions": None,
              "declaration": {"header": 0x06, "body": {
                  "arm": "codec_zenoh_decl_token",
                  "body": {"id": 9, "wireexpr": {"id": 4, "suffix": None}}}}},
             "I set: the interest id, then a token declared with no suffix"),
        ],
        "a declaration embedded whole: its header and the arm it holds.",
    ),
    "codec_transport_envelope": Fixture(
        [
            ({"header": 0x61, "body": {"arm": "codec_zenoh_init_body", "body": {
                "version": 1, "cbyte": 0x31, "zid": [1, 2, 3, 4],
                "S": {"arm": 0, "sn_res": 0x0A, "batch_size": 0x0102}, "cookie": [9, 9]}}},
             "an init that says S by its group and A by its cookie: both set"),
            ({"header": 0x22, "body": {"arm": "codec_zenoh_open_body", "body": {
                "lease": 10, "initial_sn": 0x80, "cookie": None}}},
             "an open with no cookie says A set"),
            ({"header": 0x02, "body": {"arm": "codec_zenoh_open_body", "body": {
                "lease": 10, "initial_sn": 0, "cookie": [1, 2]}}},
             "an open with a cookie says A clear"),
            ({"header": 0x84, "body": {"arm": "codec_zenoh_keep_alive", "body": {}}},
             "an arm that binds nothing: Z is the value's own"),
            ({"header": 0x07, "body": {"arm": "codec_zenoh_join", "body": {
                "version": 1, "cbyte": 0x02, "zid": [5], "S": None, "lease": 1,
                "next_sn_reliable": 2, "next_sn_best_effort": 3}}},
             "a join that says S clear"),
            ({"header": 0x10, "body": {"tag": 0x10, "body": {"reason": 1}}},
             "a tag no arm names: the default close keeps it"),
        ],
        "each arm's codec is handed the header's A or S as its import binds it, and the "
        "header's flag is what the arm says.",
    ),
    "codec_init_syn_envelope": Fixture(
        [
            ({"header": 0x41, "body": {"arm": "codec_init_syn_body", "body": {
                "version": 1, "S": {"arm": 0, "sn_res": 2, "batch_size": 0x1000}}}},
             "the arm says S set by its group"),
            ({"header": 0x01, "body": {"arm": "codec_init_syn_body", "body": {
                "version": 1, "S": None}}},
             "the arm says S clear"),
            ({"header": 0x42, "body": {"tag": 2, "body": {
                "version": 1, "S": {"arm": 0, "sn_res": 0, "batch_size": 0}}}},
             "the default arm binds S too, and keeps its tag"),
        ],
        "one arm, also the default, handed the header's S: the flag is what the arm says.",
    ),
    "codec_init_cookie_body": Fixture(
        [
            ({"version": 1, "cookie": None}, "A clear: no cookie", {"A": 0}),
            ({"version": 1, "cookie": [1, 2, 3]}, "A set: the cookie and its length", {"A": 1}),
            ({"version": 2, "cookie": list(range(64))}, "a cookie at its bound", {"A": 1}),
        ],
        "the parent hands A, which the value says by its cookie.",
    ),
    "codec_init_syn_body": Fixture(
        [
            ({"version": 1, "S": None}, "S clear: the version alone", {"S": 0}),
            ({"version": 1, "S": {"arm": 0, "sn_res": 2, "batch_size": 0x1000}},
             "S set: both fields it decides", {"S": 1}),
        ],
        "two fields one flag input decides are a group (R4b), keyed by the input.",
    ),
    "codec_zenoh_init_body": Fixture(
        [
            ({"version": 1, "cbyte": 0x01, "zid": [7], "S": None, "cookie": None},
             "neither input: a one-byte zid", {"S": 0, "A": 0}),
            ({"version": 1, "cbyte": 0x31, "zid": [1, 2, 3, 4],
              "S": {"arm": 0, "sn_res": 0x0A, "batch_size": 0x0102}, "cookie": [9, 9]},
             "both inputs: the group, little-endian, and the cookie", {"S": 1, "A": 1}),
        ],
        "two flag inputs the value says: one by a group, one by its cookie.",
    ),
    "codec_zenoh_open_body": Fixture(
        [
            ({"lease": 10, "initial_sn": 0, "cookie": [1, 2]}, "A clear: a cookie", {"A": 0}),
            ({"lease": 10, "initial_sn": 0x80, "cookie": None}, "A set: no cookie", {"A": 1}),
            ({"lease": 0, "initial_sn": 0, "cookie": []}, "an empty cookie is present", {"A": 0}),
        ],
        "a negated flag input: the cookie is there exactly where A is clear.",
    ),
    "codec_chain_flag_entry": Fixture(
        [
            ({"header": 0x01, "wide": {"arm": 1, "short_value": 5}}, "wide clear: one byte",
             {"wide": 0}),
            ({"header": 0x82, "wide": {"arm": 0, "long_value": 0x1234}}, "wide set: two bytes",
             {"wide": 1}),
        ],
        "a flag input both of whose sides hold a field is a group of two arms.",
    ),
    "codec_chain_prev_entry": Fixture(
        [
            ({"header": 0x04, "after_marker": {"arm": 1, "short_value": 1}, "tag": None},
             "neither input", {"after_marker": 0, "tagged": 0}),
            ({"header": 0x81, "after_marker": {"arm": 0, "long_value": 2}, "tag": 7},
             "both inputs: the long value and the tag", {"after_marker": 1, "tagged": 1}),
        ],
        "two flag inputs: one a group of two arms, one said by a single field.",
    ),
    "codec_zenoh_join": Fixture(
        [
            ({"version": 1, "cbyte": 0x02, "zid": [5], "S": None, "lease": 1,
              "next_sn_reliable": 2, "next_sn_best_effort": 3},
             "S clear: no sn_res or batch_size", {"S": 0}),
            ({"version": 1, "cbyte": 0x12, "zid": [5, 6],
              "S": {"arm": 0, "sn_res": 8, "batch_size": 0x0200},
              "lease": 0x80, "next_sn_reliable": 0, "next_sn_best_effort": 0},
             "S set: the group, little-endian", {"S": 1}),
        ],
        "a group of two fields one flag input decides, among VLEs.",
    ),
    "codec_zenoh_undecl_queryable": Fixture(
        [
            ({"id": 1, "ext_keyexpr": None}, "Z clear: the id alone", {"Z": 0}),
            ({"id": 0x80, "ext_keyexpr": {"outer_header": 0x0F,
                                          "inner": {"inner_header": 0x00, "id": 5,
                                                    "suffix": None}}},
             "Z set: the key expression extension", {"Z": 1}),
        ],
        "a body gated on a flag input, which the value says by the body.",
    ),
    "codec_zenoh_undecl_subscriber": Fixture(
        [
            ({"id": 2, "ext_keyexpr": None}, "Z clear: the id alone", {"Z": 0}),
            ({"id": 3, "ext_keyexpr": {"outer_header": 0x0F,
                                       "inner": {"inner_header": 0x03, "id": 1,
                                                 "suffix": [0x61]}}},
             "Z set: the extension with a suffix", {"Z": 1}),
        ],
        "a body gated on a flag input, which the value says by the body.",
    ),
    "codec_zenoh_undecl_token": Fixture(
        [
            ({"id": 0, "ext_keyexpr": None}, "Z clear: the id alone", {"Z": 0}),
            ({"id": 9, "ext_keyexpr": {"outer_header": 0x0F,
                                       "inner": {"inner_header": 0x00, "id": 0,
                                                 "suffix": None}}},
             "Z set: the key expression extension", {"Z": 1}),
        ],
        "a body gated on a flag input, which the value says by the body.",
    ),
    "codec_variant_dispatch": Fixture(
        [
            ({"body": {"arm": "codec_variant_session_open", "body": {"version": 0x0102}}},
             "the open arm: its tag, then its body"),
            ({"body": {"arm": "codec_variant_session_close", "body": {"reason": 3}}},
             "the close arm"),
            ({"body": {"tag": 0x7F, "body": {"reason": 9}}},
             "a tag no arm names: the default arm keeps it"),
        ],
        "a variant's tag is its arm's, or the one the default arm holds: no member states it.",
    ),
    "codec_origin_choice": Fixture(
        [
            ({"body": {"arm": "codec_origin_leaf", "body": {"data": [1, 2]}}}, "the leaf arm"),
            ({"body": {"arm": "codec_origin_scalar", "body": {"value": 0xAB}}}, "the scalar arm"),
            ({"body": {"tag": 0, "body": {"value": 1}}}, "tag zero, which no arm names"),
        ],
        "a variant whose arms are a leaf and a scalar.",
    ),
    "codec_zenoh_push_body": Fixture(
        [
            ({"header": 0x01, "body": {"arm": "codec_zenoh_put", "body": {"payload": 7}}},
             "mid 1: a put; the mid bits are derived from the arm"),
            ({"header": 0x22, "body": {"arm": "codec_zenoh_del", "body": {}}},
             "mid 2: a del, the bits beside the tag kept"),
            ({"header": 0xF0, "body": {"tag": 0x10, "body": {"payload": 1}}},
             "a mid no arm names: the default arm keeps it"),
        ],
        "a variant whose tag is a flag of a carrier, derived from the arm.",
    ),
    "codec_zenoh_ext_entry": Fixture(
        [
            ({"header": 0x01, "body": {"arm": "codec_zenoh_ext_unit", "body": {}}},
             "a unit extension: enc 0"),
            ({"header": 0x22, "body": {"arm": "codec_zenoh_ext_zint", "body": {"value": 0x80}}},
             "a zint extension: enc 1"),
            ({"header": 0xC3, "body": {"arm": "codec_zenoh_ext_zbuf", "body": {"value": [1, 2]}}},
             "a zbuf extension with Z set: enc 2"),
            ({"header": 0x65, "body": {"tag": 3, "body": {}}},
             "enc 3, which no arm names: the default arm"),
        ],
        "a Zenoh extension: its encoding, two bits of the header, is the arm's tag.",
    ),
    "codec_peek_arm_a": Fixture(
        [({"header": 0x00, "payload": 5}, "the header presented whole, then one byte")],
        "a flag's `value` is the builder's default and says nothing of the wire.",
    ),
    "codec_peek_arm_b": Fixture(
        [({"header": 0x01, "payload": 0x1234}, "the header presented whole, then two bytes")],
        "a flag's `value` is the builder's default and says nothing of the wire.",
    ),
    "codec_repeat_present_if_basic": Fixture(
        [
            ({"carrier": 0x00, "elems": None}, "has_list clear: no count and no list"),
            ({"carrier": 0x01, "elems": [{"seq": 1}, {"seq": 2}]},
             "has_list and the count derived from the list"),
            ({"carrier": 0x01, "elems": []}, "an empty list is present: its count is zero"),
        ],
        "a gated list whose count is gated with it, and so follows it.",
    ),
    "codec_tlv_chain_present_if_basic": Fixture(
        [
            ({"carrier": 0x00, "entries": None}, "has_chain clear: the carrier alone"),
            ({"carrier": 0x01, "entries": [{"entry_type": 1, "entry_body": [9]}]},
             "a chain read to the end of the frame, its flag derived"),
        ],
        "a gated chain read to the end of the frame.",
    ),
    "codec_chain_depth_plain": Fixture(
        [
            ({"header_flags": 0, "extensions": [{"header": 0x01}]},
             "one entry: Z clear, the last"),
            ({"header_flags": 0, "extensions": [{"header": 0x80 | i} for i in range(1, 8)]
              + [{"header": 0x08}]},
             "eight entries, the depth: Z set on each but the last"),
        ],
        "a chain that ends on its entries' flag: Z is set on every entry but the last.",
    ),
    "codec_chain_depth_gated": Fixture(
        [
            ({"header": 0x00, "extensions": None, "payload": [0xAA]},
             "Z clear: no chain, then the payload"),
            ({"header": 0x80, "extensions": [{"header": 0x81}, {"header": 0x02}],
              "payload": []},
             "Z set: two entries, then an empty payload"),
        ],
        "a gated chain that ends on its entries' flag, followed by a payload.",
    ),
    "codec_zenoh_ext_envelope": Fixture(
        [
            ({"header_flags": 0, "extensions": [ext_unit(1, False)]}, "one unit extension"),
            ({"header_flags": 0, "extensions": [ext_unit(1, True), ext_zint(2, 5, False)]},
             "a unit and a zint: Z on the first only"),
            ({"header_flags": 0x10, "extensions": [ext_zint(1, 0x80, True), ext_unit(4, True),
                                                   ext_zbuf(3, [7, 8], False)]},
             "three extensions of each encoding"),
        ],
        "a Zenoh extension chain: each entry's enc is its arm's, its Z the chain's.",
    ),
    "codec_zenoh_msg_put": Fixture(
        [
            ({"header": 0x01, "timestamp": None, "encoding": None, "extensions": None,
              "payload": [0xAA, 0xBB]}, "a bare put: the header and the payload"),
            ({"header": 0xE1, "timestamp": {"time": 5, "zid": [1]},
              "encoding": {"packed_id": 0x00, "schema": None},
              "extensions": [ext_unit(1, True), ext_zint(2, 9, False)], "payload": []},
             "T, E and Z derived from the timestamp, the encoding and the chain"),
        ],
        "a Zenoh put: gated bodies, a gated extension chain and a payload.",
    ),
    "codec_zenoh_msg_del": Fixture(
        [
            ({"header": 0x02, "timestamp": None, "extensions": None}, "a bare del"),
            ({"header": 0xA2, "timestamp": {"time": 0x80, "zid": []},
              "extensions": [ext_unit(3, False)]}, "T and Z derived"),
        ],
        "a Zenoh del: a gated timestamp and a gated extension chain.",
    ),
    "codec_zenoh_err": Fixture(
        [
            ({"header": 0x05, "encoding": None, "extensions": None, "payload": [1]},
             "a bare err"),
            ({"header": 0xC5, "encoding": {"packed_id": 0x01, "schema": "t"},
              "extensions": [ext_zbuf(1, [], False)], "payload": []},
             "E and Z derived; an encoding with a schema"),
        ],
        "a Zenoh err: a gated encoding, a gated extension chain and a payload.",
    ),
    "codec_zenoh_reply": Fixture(
        [
            ({"header": 0x04, "consolidation": None, "extensions": None,
              "body": {"header": 0x01, "body": {"arm": "codec_zenoh_put", "body": {"payload": 3}}}},
             "a bare reply holding a put"),
            ({"header": 0xA4, "consolidation": 2, "extensions": [ext_unit(2, False)],
              "body": {"header": 0x02, "body": {"arm": "codec_zenoh_del", "body": {}}}},
             "C and Z derived; a reply holding a del"),
        ],
        "a Zenoh reply: a gated consolidation and chain, then a push body.",
    ),
    "codec_zenoh_oam": Fixture(
        [
            ({"header": 0x1F, "id": 1, "extensions": None,
              "body": {"arm": "codec_zenoh_ext_unit", "body": {}}},
             "an OAM of no body: enc 0"),
            ({"header": 0xDF, "id": 0x80, "extensions": [ext_unit(1, False)],
              "body": {"arm": "codec_zenoh_ext_zbuf", "body": {"value": [5]}}},
             "Z derived from the chain, enc 2 from the zbuf arm"),
        ],
        "a Zenoh OAM: a gated chain, then a body whose encoding is in the header.",
    ),
    "codec_zenoh_query": Fixture(
        [
            ({"header": 0x03, "consolidation": None, "parameters": None, "extensions": None},
             "a bare query"),
            ({"header": 0xE3, "consolidation": 1, "parameters": [0x61, 0x3D, 0x31],
              "extensions": [{"header": 0x01, "body": {"arm": "codec_zenoh_ext_unit", "body": {}}},
                             {"header": 0x22, "body": {"arm": "codec_zenoh_ext_zint",
                                                       "body": {"value": 4}}}]},
             "C, P and Z derived; a chain read to the end of the frame"),
        ],
        "a Zenoh query: its chain ends with the frame, so no entry's Z is the chain's.",
    ),
    "codec_zenoh_response_final": Fixture(
        [
            ({"header": 0x1A, "request_id": 7, "extensions": None}, "a bare final response"),
            ({"header": 0x9A, "request_id": 0x80,
              "extensions": [ext_unit(1, True), ext_unit(2, False)]}, "Z derived from the chain"),
        ],
        "a Zenoh final response: a request id and a gated extension chain.",
    ),
    "codec_zenoh_hello": Fixture(
        [
            ({"version": 1, "cbyte": 0x01, "zid": [3], "locators": None},
             "L clear: no locators", {"L": 0}),
            ({"version": 1, "cbyte": 0x12, "zid": [3, 4],
              "locators": [{"locator": "tcp/1"}, {"locator": ""}]},
             "L set: two locators, their count derived", {"L": 1}),
        ],
        "a Zenoh hello: locators the flag input L gates, their count gated with them.",
    ),
    "codec_zenoh_push": Fixture(
        [
            ({"header": 0x1D, "body": {"arm": "codec_zenoh_push_body", "body": {
                "header": 0x01, "body": {"arm": "codec_zenoh_put", "body": {"payload": 7}}}}},
             "mid 0x1d, derived from the arm; a put"),
            ({"header": 0x3E, "body": {"tag": 0x1E, "body": {
                "header": 0x02, "body": {"arm": "codec_zenoh_del", "body": {}}}}},
             "a mid no arm names: the default arm keeps it"),
        ],
        "a Zenoh push: a variant of one arm and a default.",
    ),
    "codec_chain_depth_entry": Fixture(
        [({"header": 0x05}, "an id, Z clear"), ({"header": 0x8F}, "id 15 and Z set")],
        "one header byte, presented whole.",
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
            ({"value": list(range(32))}, "thirty-two bytes"),
            ({"value": list(range(200))}, "200 bytes: the length takes two VLE bytes"),
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
    "codec_embed_basic": Fixture(
        [
            ({"tag": 1, "locator": {"locator": ""}}, "a body of empty text: its length alone"),
            ({"tag": 2, "locator": {"locator": "abc"}}, "the body's frame follows the tag"),
        ],
        "an embedded body is presented as the value of the codec it holds.",
    ),
    "codec_repeat_basic": Fixture(
        [
            ({"frags": []}, "no entry: the count is zero"),
            ({"frags": [{"seq": 1}, {"seq": 0xABCD}]}, "the count is derived from the entries"),
            ({"frags": [{"seq": i} for i in range(32)]}, "a list at its bound of 32"),
        ],
        "a list counted by a field before it: the count is derived from the entries.",
    ),
    "codec_until_eof_basic": Fixture(
        [
            ({"msgs": []}, "no entry: an empty frame"),
            ({"msgs": [{"seq": 7}, {"seq": 8}, {"seq": 0xFFFF}]}, "entries to the end of the frame"),
        ],
        "a list read to the end of the frame.",
    ),
    "codec_declared_bounds": Fixture(
        [
            ({"key": [], "tag": [], "items": []}, "everything empty: the tag's length says one more"),
            ({"key": list(range(8)), "tag": [9, 8, 7, 6], "items": [{"seq": 1}, {"seq": 2}]},
             "every element at its bound"),
        ],
        "lengths and a count derived from what they measure; the tag's length is one more "
        "than its bytes (sce:length-arith=\"-1\").",
    ),
    "codec_declared_bounds_to_end": Fixture(
        [
            ({"items": []}, "no entry"),
            ({"items": [{"seq": 1}, {"seq": 2}]}, "a list read to the end, at its bound"),
        ],
        "a list read to the end of the frame, within its bound.",
    ),
    "codec_origin_envelope": Fixture(
        [
            ({"hdr": 0x00, "note": "", "required": {"data": []}, "optional": None, "items": []},
             "no optional body and no item"),
            ({"hdr": 0x01, "note": "hi", "required": {"data": [1]},
              "optional": {"data": [2, 3]},
              "items": [{"data": []}, {"data": [4]}, {"data": [5, 6]}]},
             "has_opt derived from the optional body; the count from the items, at its bound"),
            ({"hdr": 0xFE, "note": "x", "required": {"data": []}, "optional": None,
              "items": [{"data": [7]}]},
             "the bits the value presents are kept beside the derived one"),
        ],
        "bodies, a gated body and a counted list of bodies, each presented as its codec's value.",
    ),
    "codec_tlv_chain_basic": Fixture(
        [
            ({"header_flags": 0, "extensions": []}, "an empty chain: the header alone"),
            ({"header_flags": 0x80, "extensions": [
                {"entry_type": 1, "entry_body": [0xAA]},
                {"entry_type": 2, "entry_body": []},
            ]}, "two entries read to the end of the frame"),
        ],
        "a chain that ends where the frame does.",
    ),
    "codec_zenoh_decl_ext_keyexpr": Fixture(
        [
            ({"outer_header": 0x0F, "inner": {"inner_header": 0x00, "id": 5, "suffix": None}},
             "the total length is derived from the inner body's frame"),
            ({"outer_header": 0x9F,
              "inner": {"inner_header": 0x03, "id": 0x80, "suffix": [0x61, 0x2F, 0x62]}},
             "an inner body with its suffix: N derived inside it"),
        ],
        "a body whose encoded length a field before it says (sce:length-from).",
    ),
    "codec_zenoh_source_info_ext": Fixture(
        [
            ({"info": {"header": 0x00, "zid": [1], "eid": 0, "sn": 0}},
             "the size is derived from the body's frame"),
            ({"info": {"header": 0xF0, "zid": list(range(16)), "eid": 0x80, "sn": 1}},
             "a body with a zid at its bound"),
        ],
        "a body whose encoded length a field before it says (sce:length-from).",
    ),
    "codec_zenoh_timestamp_ext": Fixture(
        [
            ({"ts": {"time": 0, "zid": []}}, "the size is derived from the body's frame"),
            ({"ts": {"time": 0x4000, "zid": [1, 2, 3]}}, "a body of three VLE bytes and a zid"),
        ],
        "a body whose encoded length a field before it says (sce:length-from).",
    ),
    "codec_variant_session_close": Fixture(
        [({"reason": 3}, "one byte")],
        "one byte.",
    ),
    "codec_variant_session_open": Fixture(
        [({"version": 0x0102}, "two bytes, high first")],
        "two bytes.",
    ),
    "codec_length_shared": Fixture(
        [
            ({"alpha": [], "beta": []}, "two empty byte strings: the length is zero"),
            ({"alpha": [1, 2], "beta": [3, 4]}, "two byte strings of one length, after it"),
            ({"alpha": [0xA5] * 255, "beta": [0x5A] * 255},
             "the length at the most its byte says, the bound of both byte strings"),
        ],
        "one length shared by two byte strings that are always there: each is held within "
        "what the length can say by its type, and the encode refuses two that differ, since "
        "the wire has room for one number (docs/adr/0065 §2). No value of the interface is "
        "past its length, so none is refused as out of range.",
        [
            ({"alpha": [1, 2], "beta": [3]}, INCONSISTENT_SOURCE,
             "the second byte string one byte shorter than the first"),
            ({"alpha": [], "beta": [9]}, INCONSISTENT_SOURCE,
             "the first byte string empty and the second not"),
        ],
    ),
    "codec_length_stated": Fixture(
        [
            ({"flags": 0x00, "span_len": 5, "alpha": None, "beta": None},
             "neither byte string: the value states the length"),
            ({"flags": 0x00, "span_len": 0, "alpha": None, "beta": None},
             "neither byte string, a stated length of zero"),
            ({"flags": 0x01, "span_len": None, "alpha": [0x61, 0x62], "beta": None},
             "alpha alone says the length, and has_alpha is derived"),
            ({"flags": 0x02, "span_len": None, "alpha": None, "beta": [0x7A]},
             "beta alone says the length, and has_beta is derived"),
            ({"flags": 0x03, "span_len": None, "alpha": [0x78, 0x79], "beta": [1, 2]},
             "both, of one length, which both say"),
            ({"flags": 0x03, "span_len": None, "alpha": [], "beta": []},
             "both there and empty: the content says zero"),
            ({"flags": 0xFC, "span_len": 0xFF, "alpha": None, "beta": None},
             "the bits beside the two flags are kept, and the stated length at its largest"),
        ],
        "one length shared by two byte strings each gated on a flag of its own. The length's "
        "content is under two gates, so it joins no group (docs/adr/0065 §6 R4b): it is "
        "presented alone, stated by the value where neither byte string is there and derived "
        "where either is. The encode refuses two byte strings there of two lengths, and a "
        "length the value states beside content that says another.",
        [
            ({"flags": 0x03, "span_len": None, "alpha": [1, 2], "beta": [3]}, INCONSISTENT_SOURCE,
             "both byte strings there, of two lengths"),
            ({"flags": 0x01, "span_len": 7, "alpha": [1, 2], "beta": None}, INCONSISTENT_SOURCE,
             "a stated length of seven beside alpha of two bytes"),
            ({"flags": 0x02, "span_len": 0, "alpha": None, "beta": [4]}, INCONSISTENT_SOURCE,
             "a stated length of zero beside beta of one byte"),
        ],
    ),
    "codec_encoded_length_checked": Fixture(
        [
            ({"inner": {"alpha": [], "beta": []}},
             "the body of two empty byte strings is one byte long"),
            ({"inner": {"alpha": [0x0A], "beta": [0x0B]}},
             "the length is the count of the bytes the body encodes to"),
            ({"inner": {"alpha": [0xA5] * 128, "beta": [0x5A] * 128}},
             "a body of 257 bytes: the length's high byte is set"),
        ],
        "the encoded length of a body whose own encode can refuse, in a 16-bit field. The "
        "body's refusal is passed on before the parent writes its first byte (docs/adr/0065 "
        "§2). The language bounds no encoded length, so the encode can refuse one past the "
        "field; no value of this body reaches it, its two byte strings being of at most 255 "
        "bytes each, so no case is refused as out of range.",
        [
            ({"inner": {"alpha": [1, 2], "beta": [3]}}, INCONSISTENT_SOURCE,
             "the body's two byte strings differ, which the body refuses"),
        ],
    ),
    "codec_group_enum": Fixture(
        [
            ({"flags": 0x00, "flags.has_session": None}, "has_session clear: the carrier alone"),
            ({"flags": 0x01,
              "flags.has_session": {"arm": 0, "state": "defaultSession", "attempts": 0}},
             "the first variant, at its declared 1, and no attempt"),
            ({"flags": 0x01,
              "flags.has_session": {"arm": 0, "state": "programming", "attempts": 5}},
             "programming is written as its carrier 2"),
            ({"flags": 0x01,
              "flags.has_session": {"arm": 0, "state": "safetySystem", "attempts": 0xFF}},
             "safetySystem is written as 64, and attempts at its largest"),
            ({"flags": 0xFF,
              "flags.has_session": {"arm": 0, "state": "extendedDiagnostic", "attempts": 1}},
             "the bits beside has_session are kept"),
            ({"flags": 0xFE, "flags.has_session": None},
             "the bits beside has_session are kept where the group is absent"),
        ],
        "an imported closed enum in a group of one flag (docs/adr/0065 §6 R4b): the value "
        "states it by the name of its variant, and the wire holds the number the variant "
        "declares.",
    ),
    "codec_chain_has_input": Fixture(
        [
            ({"entries": [{"entry_type": 7, "ctl": 0x00, "body": 5}], "priority": 9},
             "an entry of kind 7: priority is there", {"urgent": 0}),
            ({"entries": [{"entry_type": 3, "ctl": 0x00, "body": 5}], "priority": None},
             "no entry of kind 7 and urgent clear: no priority", {"urgent": 0}),
            ({"entries": [{"entry_type": 3, "ctl": 0x00, "body": 5}], "priority": 1},
             "urgent alone puts priority there", {"urgent": 1}),
            ({"entries": [{"entry_type": 3, "ctl": 0x01, "body": 1},
                          {"entry_type": 7, "ctl": 0x00, "body": 2}], "priority": 0xFF},
             "kind 7 second and urgent set: either holds", {"urgent": 1}),
            ({"entries": [{"entry_type": 1, "ctl": 0x01, "body": 0},
                          {"entry_type": 7, "ctl": 0x01, "body": 0},
                          {"entry_type": 2, "ctl": 0xFE, "body": 0}], "priority": 4},
             "three entries, the depth, kind 7 among them; the last ctl keeps its other bits",
             {"urgent": 0}),
        ],
        "a field gated on a chain's kinds or a flag input the caller hands: no group holds it, "
        "since no value holds the input (docs/adr/0065 §6 R6). The encode is handed urgent as "
        "its decode is, tests the gate on the entries the value presents, and refuses a "
        "priority that disagrees with it.",
        [
            ({"entries": [{"entry_type": 3, "ctl": 0x00, "body": 5}], "priority": None},
             PRESENT_IF_MISMATCH, "no priority where urgent is set", {"urgent": 1}),
            ({"entries": [{"entry_type": 7, "ctl": 0x00, "body": 5}], "priority": None},
             PRESENT_IF_MISMATCH, "no priority where an entry is of kind 7", {"urgent": 0}),
            ({"entries": [{"entry_type": 3, "ctl": 0x00, "body": 5}], "priority": 5},
             PRESENT_IF_MISMATCH, "a priority where neither holds", {"urgent": 0}),
        ],
    ),
    "codec_chain_handed_entry": Fixture(
        [
            ({"header": 0x02, "value": None}, "handed after_marker clear, wide clear: no value",
             {"after_marker": 0}),
            ({"header": 0x02, "value": 0x55}, "handed after_marker set: the value is there",
             {"after_marker": 1}),
            ({"header": 0x42, "value": 7}, "its own wide flag: the value is there",
             {"after_marker": 0}),
            ({"header": 0xC4, "value": 0}, "both, and Z set, which it presents",
             {"after_marker": 1}),
            ({"header": 0x30, "value": None}, "the bits no flag names are kept",
             {"after_marker": 0}),
        ],
        "an entry that takes a flag input it does not say: the disjunction reads the input, "
        "so it is no group but a check (docs/adr/0065 §6 R4c). wide is the value's own, the "
        "encode is handed after_marker, and it refuses a value that disagrees with the gate.",
        [
            ({"header": 0x02, "value": 5}, PRESENT_IF_MISMATCH,
             "a value where neither the input nor wide is set", {"after_marker": 0}),
            ({"header": 0x02, "value": None}, PRESENT_IF_MISMATCH,
             "no value where after_marker is handed set", {"after_marker": 1}),
            ({"header": 0x42, "value": None}, PRESENT_IF_MISMATCH,
             "no value where its own wide flag is set", {"after_marker": 0}),
        ],
    ),
    "codec_chain_handed": Fixture(
        [
            ({"entries": [{"header": 0x02, "value": None}]},
             "one entry, the first: handed after_marker clear, no value"),
            ({"entries": [{"header": 0x84, "value": None}, {"header": 0x02, "value": 0x55}]},
             "an entry after one of identifier 4 is handed after_marker set"),
            ({"entries": [{"header": 0xC2, "value": 7}, {"header": 0x03, "value": None}]},
             "an entry's own wide flag, then one after identifier 2, handed it clear"),
            ({"entries": [{"header": 0x84, "value": None}, {"header": 0x84, "value": 1},
                          {"header": 0x82, "value": 2}, {"header": 0x01, "value": None}]},
             "four entries, the depth: each handed what the entry before is"),
        ],
        "a chain that hands each entry whether the entry before has the identifier 4, an "
        "input the entry does not say (docs/adr/0065 §6 R8b): no law holds the entries, the "
        "list is its entries alone, and the walk hands each what the entry before is as it "
        "writes it. An entry that refuses refuses the chain.",
        [
            ({"entries": [{"header": 0x84, "value": None}, {"header": 0x02, "value": None}]},
             PRESENT_IF_MISMATCH, "no value on an entry after one of identifier 4"),
            ({"entries": [{"header": 0x02, "value": 5}]},
             PRESENT_IF_MISMATCH, "a value on the first entry, which is handed it clear"),
            ({"entries": [{"header": 0x82, "value": None}, {"header": 0x04, "value": 3}]},
             PRESENT_IF_MISMATCH, "a value on an entry after identifier 2, wide clear"),
        ],
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


def with_refused(text: str, fixture: str, wanted: bool) -> str:
    """`text` with a `refused` key last in `fixture`'s entry where `wanted`, to
    be filled by [`regenerate`], and with none where not: the refused cases are
    the model's to write, so a fixture that states none carries none."""
    start, end = entry_span(text, fixture)
    key = ',\n      "refused": ['
    at = text.find(key, start, end)
    if wanted:
        return text if at >= 0 else text[:end] + key + "\n      ]" + text[end:]
    if at < 0:
        return text
    closing = text.index("\n      ]", at + len(key)) + len("\n      ]")
    return text[:at] + text[closing:]


def regenerate(text: str) -> tuple[str, str]:
    """`text` with each fixture's sources and refused cases written, and a
    report."""
    reports = []
    for fixture, spec in FIXTURES.items():
        written = write(fixture, spec.sources, spec.refused)
        text = ensure_sources(text, fixture, spec.description)
        start, end = span(text, fixture, "sources")
        body = "\n" + ",\n".join(case_text(item) for item in written.sources)
        text = text[:start] + body + text[end:]
        text = with_refused(text, fixture, bool(written.refused))
        report = f"{fixture}: {len(written.sources)} sources"
        if written.refused:
            start, end = span(text, fixture, "refused")
            body = "\n" + ",\n".join(case_text(item) for item in written.refused)
            text = text[:start] + body + text[end:]
            report += f", {len(written.refused)} refused"
        reports.append(report + "; the model holds each")
    return text, "\n".join(reports)
