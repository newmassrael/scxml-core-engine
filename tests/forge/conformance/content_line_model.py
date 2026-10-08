# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""An independent model of the content-line codec, and the cases it writes.

`SCE_FORGE.md` §4.6.4 writes the wire rules of a `sce:encoding="content-line"`
codec once, and six backends implement them. This file is a seventh reading of
the same page, written from the page alone and not from any backend: it never
looks at a runtime, a template or a generated file, and it takes the entries of
the codec from the fixture's own document (`codec_content_line_event.scxml`), so
a change to the document moves the model with it.

It does two things, and `gen_cases.py` runs both:

* It writes the fixture's `cases` and `rejects` into `numerical_reference.json`:
  a hand-chosen set that reaches every rule of the page, and a seeded spread
  around them (round trips of drawn values, inputs that decode but would not be
  written that way, and a single fault put into a valid component). Each case
  carries the component as `encoded` bytes, which the backends compare against,
  and as `text`, which is what a reader can check against the page.
* It holds itself to its own output: every case is decoded and encoded again by
  the model and must come back, and every reject is refused by the model with
  the failure the case names. A case the model disagrees with is a case that was
  not written by it.

The cases are a consequence of the model, so a backend that disagrees with one
is a backend that reads the page differently from this file; which of the two is
wrong is decided against the page.

Failure names are the page's (`SCE_FORGE.md` §4.6.4, the table of failures).
"""

from __future__ import annotations

import json
import re
import xml.etree.ElementTree as ET
from dataclasses import dataclass
from pathlib import Path

RESOURCE = Path("tests/forge/resources/codec_content_line_event.scxml")
FIXTURE = "codec_content_line_event"

SCE = "{http://sce.dev/ext}"
SCXML = "{http://www.w3.org/2005/07/scxml}"

FOLD_WIDTH = 75

INT_RANGE = {
    "uint8": (0, 2**8 - 1),
    "uint16": (0, 2**16 - 1),
    "uint32": (0, 2**32 - 1),
    "uint64": (0, 2**64 - 1),
    "int8": (-(2**7), 2**7 - 1),
    "int16": (-(2**15), 2**15 - 1),
    "int32": (-(2**31), 2**31 - 1),
    "int64": (-(2**63), 2**63 - 1),
}

NAME = re.compile(rb"[A-Za-z0-9-]+")


def fails(name: str):
    return ("fails", name)


def is_control(byte: int) -> bool:
    return (byte < 0x20 and byte != 0x09) or byte == 0x7F


@dataclass
class Entry:
    id: str
    prop: str
    param: str | None
    type: str
    text: bool
    required: bool
    max_size: int | None
    max_count: int | None


def load(path: Path = RESOURCE):
    """The component and the entries of the codec document."""
    root = ET.parse(path).getroot()
    component = root.attrib[SCE + "component"]
    entries = []
    for data in root.find(SCXML + "datamodel"):
        if data.attrib.get(SCE + "direction") == "in":
            continue
        size = data.attrib.get(SCE + "max-size")
        count = data.attrib.get(SCE + "max-count")
        entries.append(
            Entry(
                id=data.attrib["id"],
                prop=data.attrib[SCE + "property"],
                param=data.attrib.get(SCE + "param"),
                type=data.attrib[SCE + "type"],
                text=data.attrib.get(SCE + "value") == "text",
                required=data.attrib.get(SCE + "required") == "true",
                max_size=int(size) if size else None,
                max_count=int(count) if count else None,
            )
        )
    return component, entries


# ── Reading ─────────────────────────────────────────────────────────────


def unfold(data: bytes):
    """The logical lines of `data` as (text, terminated). A line break is CRLF or
    LF; a physical line that starts with a space or a tab continues the one
    before it, the break and that one character gone."""
    parts = data.split(b"\n")
    tail = parts.pop()
    physical = [(p[:-1] if p.endswith(b"\r") else p, True) for p in parts]
    if tail:
        physical.append((tail, False))
    lines = []
    for body, terminated in physical:
        if lines and body[:1] in (b" ", b"\t"):
            lines[-1] = (lines[-1][0] + body[1:], terminated)
        else:
            lines.append((body, terminated))
    return lines


def split_head(body: bytes):
    """(name, separator, index after the separator), or None."""
    m = NAME.match(body)
    if not m:
        return None
    sep = body[m.end() : m.end() + 1]
    if sep not in (b";", b":"):
        return None
    return m.group(0), sep, m.end() + 1


def read_text(raw: bytes, text: bool, max_size: int):
    """A string value, byte by byte: the first failure in line order."""
    out = bytearray()
    i = 0
    while i < len(raw):
        c = raw[i]
        i += 1
        if text and c == 0x5C:
            if i >= len(raw):
                return fails("line-bad-escape")
            e = raw[i]
            i += 1
            mapped = {0x5C: 0x5C, 0x3B: 0x3B, 0x2C: 0x2C, 0x6E: 0x0A, 0x4E: 0x0A}.get(e)
            if mapped is None:
                return fails("line-bad-escape")
            c = mapped
        elif is_control(c):
            return fails("line-bad-value")
        if len(out) == max_size:
            return fails("line-too-long")
        out.append(c)
    try:
        return ("ok", bytes(out).decode("utf-8"))
    except UnicodeDecodeError:
        return fails("line-bad-value")


def read_number(raw: bytes, type_: str):
    if not re.fullmatch(rb"[+-]?[0-9]+", raw):
        return fails("line-bad-value")
    low, high = INT_RANGE[type_]
    if raw.startswith(b"-") and low == 0:
        return fails("line-bad-value")
    value = int(raw)
    return ("ok", value) if low <= value <= high else fails("line-bad-value")


def read_bool(raw: bytes):
    word = raw.upper()
    if word == b"TRUE":
        return ("ok", True)
    if word == b"FALSE":
        return ("ok", False)
    return fails("line-bad-value")


def scan_param_values(body: bytes, i: int):
    """The values of one parameter from `i`: ([bytes], index of the `;` or `:`
    that follows), or None when the line does not admit it."""
    values = []
    while True:
        if body[i : i + 1] == b'"':
            close = body.find(b'"', i + 1)
            if close < 0:
                return None
            values.append(body[i + 1 : close])
            i = close + 1
        else:
            j = i
            while j < len(body) and body[j : j + 1] not in (b";", b":", b",", b'"'):
                j += 1
            if body[j : j + 1] == b'"':
                return None
            values.append(body[i:j])
            i = j
        follow = body[i : i + 1]
        if follow == b",":
            i += 1
            continue
        if follow in (b";", b":"):
            return values, i
        return None


class Model:
    def __init__(self, component: str, entries: list[Entry]):
        self.component = component.encode()
        self.entries = entries
        self.values = [e for e in entries if e.param is None]
        self.params_of = {
            v.id: [
                p
                for p in entries
                if p.param is not None and p.prop.upper() == v.prop.upper()
            ]
            for v in self.values
        }

    # -- decode --------------------------------------------------------

    def decode(self, data: bytes):
        lines = unfold(data)
        n = len(lines)
        i = 0
        while True:
            if i >= n:
                return fails("need-more-bytes")
            body, terminated = lines[i]
            i += 1
            if not terminated:
                return fails("need-more-bytes")
            head = split_head(body)
            if (
                head
                and head[1] == b":"
                and head[0].upper() == b"BEGIN"
                and body[head[2] :].upper() == self.component.upper()
            ):
                break
        got = {e.id: ([] if e.max_count else None) for e in self.entries}
        depth = 0
        while True:
            if i >= n:
                return fails("need-more-bytes")
            body, terminated = lines[i]
            i += 1
            head = split_head(body)
            if depth > 0:
                if head and head[1] == b":":
                    name = head[0].upper()
                    if name == b"BEGIN":
                        depth += 1
                    elif name == b"END":
                        depth -= 1
                continue
            if head is None:
                return fails("line-malformed" if terminated else "need-more-bytes")
            name = head[0].upper()
            if head[1] == b":":
                if name == b"END":
                    if body[head[2] :].upper() == self.component.upper():
                        break
                    return fails("line-malformed" if terminated else "need-more-bytes")
                if terminated and name == b"BEGIN":
                    depth = 1
                    continue
            if not terminated:
                return fails("need-more-bytes")
            entry = next((v for v in self.values if v.prop.upper().encode() == name), None)
            if entry is None:
                continue
            failure = self.read_line(entry, body, head, got)
            if failure:
                return failure
        for e in self.entries:
            if e.required and got[e.id] is None:
                return fails("line-required-missing")
        return ("ok", got)

    def read_line(self, entry: Entry, body: bytes, head, got):
        if entry.max_count:
            if len(got[entry.id]) >= entry.max_count:
                return fails("line-too-many")
        elif got[entry.id] is not None:
            return fails("line-too-many")
        declared = {p.param.upper().encode(): p for p in self.params_of[entry.id]}
        i = head[2]
        separator = head[1]
        seen = {}
        while separator == b";":
            m = NAME.match(body, i)
            if not m:
                return fails("line-malformed")
            name = m.group(0).upper()
            i = m.end()
            if body[i : i + 1] != b"=":
                return fails("line-malformed")
            scanned = scan_param_values(body, i + 1)
            if scanned is None:
                return fails("line-malformed")
            values, i = scanned
            separator = body[i : i + 1]
            i += 1
            param = declared.get(name)
            if param is None:
                continue
            if param.id in seen:
                return fails("line-too-many")
            read = read_text(values[0], False, param.max_size)
            if read[0] == "fails":
                return read
            if len(values) > 1:
                return fails("line-bad-value")
            seen[param.id] = read[1]
        raw = body[i:]
        if entry.type == "string":
            read = read_text(raw, entry.text, entry.max_size)
        elif entry.type == "bool":
            read = read_bool(raw)
        else:
            read = read_number(raw, entry.type)
        if read[0] == "fails":
            return read
        if entry.max_count:
            got[entry.id].append(read[1])
        else:
            got[entry.id] = read[1]
        got.update(seen)
        return None

    # -- encode --------------------------------------------------------

    def encode(self, decoded: dict):
        out = bytearray(b"BEGIN:" + self.component + b"\r\n")
        for entry in self.values:
            params = self.params_of[entry.id]
            if entry.max_count:
                values = decoded.get(entry.id) or []
                if len(values) > entry.max_count:
                    return fails("line-too-many")
                for value in values:
                    line = self.line(entry, [], decoded, value)
                    if line[0] == "fails":
                        return line
                    out += line[1]
                continue
            value = decoded.get(entry.id)
            if value is None:
                if entry.required:
                    return fails("line-required-missing")
                if any(decoded.get(p.id) is not None for p in params):
                    return fails("line-required-missing")
                continue
            line = self.line(entry, params, decoded, value)
            if line[0] == "fails":
                return line
            out += line[1]
        out += b"END:" + self.component + b"\r\n"
        return ("ok", bytes(out))

    def line(self, entry: Entry, params, decoded: dict, value):
        units = [bytes([b]) for b in entry.prop.encode()]
        for p in params:
            pv = decoded.get(p.id)
            if pv is None:
                if p.required:
                    return fails("line-required-missing")
                continue
            raw = pv.encode("utf-8")
            if len(raw) > p.max_size:
                return fails("line-too-long")
            if any(b == 0x22 or is_control(b) for b in raw):
                return fails("line-bad-value")
            quoted = any(b in b":;," for b in raw)
            units += [b";"] + [bytes([b]) for b in p.param.encode()] + [b"="]
            if quoted:
                units.append(b'"')
            units += [c.encode("utf-8") for c in pv]
            if quoted:
                units.append(b'"')
        units.append(b":")
        if entry.type == "string":
            raw = value.encode("utf-8")
            if len(raw) > entry.max_size:
                return fails("line-too-long")
            if any(is_control(b) and not (entry.text and b == 0x0A) for b in raw):
                return fails("line-bad-value")
            for c in value:
                if entry.text and c in "\\;,\n":
                    units.append(b"\\" + {"\\": b"\\", ";": b";", ",": b",", "\n": b"n"}[c])
                else:
                    units.append(c.encode("utf-8"))
        elif entry.type == "bool":
            units += [bytes([b]) for b in (b"TRUE" if value else b"FALSE")]
        else:
            low, high = INT_RANGE[entry.type]
            if not low <= value <= high:
                return fails("line-bad-value")
            units += [bytes([b]) for b in str(value).encode()]
        return ("ok", fold(units) + b"\r\n")


def fold(units: list[bytes]) -> bytes:
    """One logical line from its units, cut before the unit that would take a
    physical line past the fold width."""
    out = bytearray()
    column = 0
    for unit in units:
        if column + len(unit) > FOLD_WIDTH:
            out += b"\r\n "
            column = 1
        out += unit
        column += len(unit)
    return bytes(out)


# ── The cases ───────────────────────────────────────────────────────────

BASE = {
    "uid": "evt-1",
    "dtstart": "20261008T090000",
    "dtstartTzid": "Asia/Seoul",
}


def decoded_of(**fields):
    """A full `decoded` object: every entry named, absent as null and an empty
    list."""
    out = {
        "uid": None,
        "dtstart": None,
        "dtstartTzid": None,
        "summary": None,
        "description": None,
        "organizer": None,
        "organizerCn": None,
        "rrule": None,
        "exdate": [],
        "sequence": None,
        "priority": None,
        "allDay": None,
    }
    out.update(BASE)
    out.update(fields)
    return out


def based(**fields):
    """`decoded_of` over the required lines `component()` writes."""
    return decoded_of(
        **{"uid": "u", "dtstart": "20260101T000000", "dtstartTzid": "Z", **fields}
    )


LONG_DESCRIPTION = (
    "Quarterly planning for the platform group: roadmap review, staffing, "
    "budget and the list of risks carried over from the last quarter, "
    "with notes."
)

#: (note, decoded) — each is written by the model and read back.
HAND_CASES = [
    ("the smallest component: the required entries, their parameter, nothing else", decoded_of()),
    (
        "every entry present; the list has as many values as it holds",
        decoded_of(
            summary="Design review",
            description="Agenda attached.",
            organizer="mailto:kim@example.org",
            organizerCn="Kim",
            rrule="FREQ=WEEKLY;BYDAY=MO,WE;COUNT=10",
            exdate=["20261015T090000", "20261022T090000", "20261029T090000"],
            sequence=65535,
            priority=-2147483648,
            allDay=True,
        ),
    ),
    (
        "a TEXT writes its backslash, semicolon, comma and line feed as escapes",
        decoded_of(summary="a;b,c\\d\ne"),
    ),
    (
        "a parameter holding a comma is written between quotes",
        decoded_of(organizer="mailto:kim@example.org", organizerCn="Kim, Lee"),
    ),
    (
        "a parameter holding a colon and a semicolon is written between quotes",
        decoded_of(dtstartTzid="Zone:A;B"),
    ),
    (
        "a value past 75 octets is folded; the cut is before the unit that would pass it",
        decoded_of(description=LONG_DESCRIPTION),
    ),
    (
        "a cut falls between two characters of two octets, and none is split",
        decoded_of(description="\u00e9" * 80),
    ),
    (
        "a cut falls before a character of four octets that would pass the width",
        decoded_of(description="x" * 52 + "\U0001F600" * 20),
    ),
    (
        "a cut falls before an escape, and the escape is not split",
        decoded_of(description="x" * 55 + ";" * 20),
    ),
    (
        "zero, false and an empty string are values like another",
        decoded_of(summary="", sequence=0, priority=0, allDay=False),
    ),
    (
        "the bounds of the widest integer types the document names",
        decoded_of(sequence=1, priority=2147483647, allDay=True),
    ),
    (
        "an unreserved character that is not ASCII is written as it is",
        decoded_of(summary="\ud55c\uae00 \u2713"),
    ),
]

CANON = decoded_of(
    summary="Review",
    organizer="mailto:kim@example.org",
    organizerCn="Kim",
    exdate=["20261015T090000"],
    sequence=2,
    priority=5,
    allDay=False,
)

COMPONENT_HEAD = b"BEGIN:VEVENT\r\n"
COMPONENT_TAIL = b"END:VEVENT\r\n"
BASE_LINES = b"UID:u\r\nDTSTART;TZID=Z:20260101T000000\r\n"


def component(*lines: bytes, base: bytes = BASE_LINES) -> bytes:
    return COMPONENT_HEAD + base + b"".join(lines) + COMPONENT_TAIL


#: (note, decoded, input) — inputs that decode to `decoded` and are not what the
#: encoder writes for it.
HAND_ACCEPTS = [
    (
        "LF line ends and lower-case names are read",
        decoded_of(uid="u", dtstart="20260101T000000", dtstartTzid="Z", summary="s"),
        b"begin:vevent\nuid:u\ndtstart;tzid=Z:20260101T000000\nsummary:s\nend:vevent\n",
    ),
    (
        "a value cut by a fold is read whole, and a tab folds like a space",
        decoded_of(uid="abcdef", dtstart="20260101T000000", dtstartTzid="Z"),
        b"BEGIN:VEVENT\r\nUID:abc\r\n def\r\nDTSTART;TZID=Z:20260101T000000\r\nEND:VEVENT\r\n".replace(
            b"abc\r\n def", b"ab\r\n\tc\r\n d\r\n ef"
        ),
    ),
    (
        "a name cut by a fold is read whole",
        decoded_of(uid="u", dtstart="20260101T000000", dtstartTzid="Z"),
        b"BEGIN:VEVENT\r\nUI\r\n D:u\r\nDTSTART;TZ\r\n ID=Z:20260101T000000\r\nEND:VEVENT\r\n",
    ),
    (
        "a character of two octets cut between them is whole once unfolded",
        based(summary="\u00e9"),
        component(b"SUMMARY:\xc3\r\n \xa9\r\n"),
    ),
    (
        "the lines around the component are not the codec's",
        decoded_of(uid="u", dtstart="20260101T000000", dtstartTzid="Z"),
        b"BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//x//y//EN\r\n"
        + component()
        + b"END:VCALENDAR\r\n",
    ),
    (
        "a property the codec does not declare is skipped, whatever it holds",
        decoded_of(uid="u", dtstart="20260101T000000", dtstartTzid="Z"),
        component(b"X-NOISE;A=\"b:c\";D=e,f:ggg\\xhh\r\n", b"LOCATION:Room 4\r\n"),
    ),
    (
        "a nested component is skipped whole, its own properties included",
        decoded_of(uid="u", dtstart="20260101T000000", dtstartTzid="Z"),
        component(
            b"BEGIN:VALARM\r\n",
            b"UID:inner\r\n",
            b"BEGIN:X-NESTED\r\n",
            b"END:X-NESTED\r\n",
            b"ACTION:DISPLAY\r\n",
            b"END:VALARM\r\n",
        ),
    ),
    (
        "properties come in any order",
        decoded_of(uid="u", dtstart="20260101T000000", dtstartTzid="Z", summary="s"),
        b"BEGIN:VEVENT\r\nSUMMARY:s\r\nDTSTART;TZID=Z:20260101T000000\r\nUID:u\r\nEND:VEVENT\r\n",
    ),
    (
        "a parameter no entry declares is skipped, and one that does is found among them",
        decoded_of(uid="u", dtstart="20260101T000000", dtstartTzid="Z"),
        b"BEGIN:VEVENT\r\nUID:u\r\nDTSTART;VALUE=DATE-TIME;TZID=Z;X-Q=\"a,b\",c:20260101T000000\r\nEND:VEVENT\r\n",
    ),
    (
        "an unescaped semicolon or comma in a TEXT is itself; N is a line feed too",
        based(summary="a;b,c\nd"),
        component(b"SUMMARY:a;b,c\\Nd\r\n"),
    ),
    (
        "a backslash in a value that is not a TEXT is a byte like another",
        based(rrule="a\\nb"),
        component(b"RRULE:a\\nb\r\n"),
    ),
    (
        "a plus sign and leading zeros are read in a number",
        based(sequence=7, priority=-3),
        component(b"SEQUENCE:+007\r\n", b"PRIORITY:-003\r\n"),
    ),
    (
        "true and false are read in either case",
        based(allDay=True),
        component(b"X-ALL-DAY:true\r\n"),
    ),
    (
        "the END line needs no line break after it",
        decoded_of(uid="u", dtstart="20260101T000000", dtstartTzid="Z"),
        COMPONENT_HEAD + BASE_LINES + b"END:VEVENT",
    ),
    (
        "bytes after the END line are the next component's and are not read",
        decoded_of(uid="u", dtstart="20260101T000000", dtstartTzid="Z"),
        component() + b"BEGIN:VEVENT\r\nUID:next\r\n",
    ),
    (
        "a parameter value may be empty, and a value too",
        decoded_of(uid="", dtstart="20260101T000000", dtstartTzid="", summary=""),
        b"BEGIN:VEVENT\r\nUID:\r\nDTSTART;TZID=:20260101T000000\r\nSUMMARY:\r\nEND:VEVENT\r\n",
    ),
]

#: (why, input, failure) — single faults in an otherwise valid component. An
#: empty input is not here: a reject vector carries at least one byte (a C array
#: has no zero length), and every backend's own test holds the empty input.
HAND_REJECTS = [
    ("the input ends before the component begins", b"BEGIN:VCALENDAR\r\nVERSION:2.0\r\n", "need-more-bytes"),
    ("the component begins and nothing follows", b"BEGIN:VEVENT\r\n", "need-more-bytes"),
    ("the END line is missing", COMPONENT_HEAD + BASE_LINES, "need-more-bytes"),
    ("the END line is cut short", COMPONENT_HEAD + BASE_LINES + b"END:VEVE", "need-more-bytes"),
    ("the last property is cut short", COMPONENT_HEAD + BASE_LINES + b"SUMMARY:ab", "need-more-bytes"),
    ("a nested component never ends", component(b"BEGIN:VALARM\r\n", b"ACTION:AUDIO\r\n"), "need-more-bytes"),
    ("an empty line inside the component", component(b"\r\n"), "line-malformed"),
    ("a line with a name and no separator", component(b"SUMMARY\r\n"), "line-malformed"),
    ("a line whose name is not a name", component(b"=x:y\r\n"), "line-malformed"),
    ("a line that begins with a separator", component(b":x\r\n"), "line-malformed"),
    ("an END that is another component's", COMPONENT_HEAD + BASE_LINES + b"END:VTODO\r\n", "line-malformed"),
    ("a parameter with no equals sign", b"BEGIN:VEVENT\r\nUID:u\r\nDTSTART;TZID:20260101T000000\r\nEND:VEVENT\r\n", "line-malformed"),
    ("a quoted parameter value that never closes", b"BEGIN:VEVENT\r\nUID:u\r\nDTSTART;TZID=\"Z:20260101T000000\r\nEND:VEVENT\r\n", "line-malformed"),
    ("text after the closing quote of a parameter value", b"BEGIN:VEVENT\r\nUID:u\r\nDTSTART;TZID=\"Z\"x:20260101T000000\r\nEND:VEVENT\r\n", "line-malformed"),
    ("a quote inside an unquoted parameter value", b"BEGIN:VEVENT\r\nUID:u\r\nDTSTART;TZID=a\"b:20260101T000000\r\nEND:VEVENT\r\n", "line-malformed"),
    ("a parameter with no name", component(b"SUMMARY;=x:y\r\n"), "line-malformed"),
    ("a parameter of a property nobody reads is still a parameter of a declared one: DTSTART's has no colon", b"BEGIN:VEVENT\r\nUID:u\r\nDTSTART;TZID=Z\r\nEND:VEVENT\r\n", "line-malformed"),
    ("the required UID is absent", b"BEGIN:VEVENT\r\nDTSTART;TZID=Z:20260101T000000\r\nEND:VEVENT\r\n", "line-required-missing"),
    ("the required DTSTART is absent", b"BEGIN:VEVENT\r\nUID:u\r\nEND:VEVENT\r\n", "line-required-missing"),
    ("the required TZID of DTSTART is absent", b"BEGIN:VEVENT\r\nUID:u\r\nDTSTART:20260101T000000\r\nEND:VEVENT\r\n", "line-required-missing"),
    ("UID occurs twice", component(b"UID:v\r\n"), "line-too-many"),
    ("TZID is given twice in one line", b"BEGIN:VEVENT\r\nUID:u\r\nDTSTART;TZID=A;TZID=B:20260101T000000\r\nEND:VEVENT\r\n", "line-too-many"),
    ("a fourth EXDATE past the list's three", component(b"EXDATE:1\r\n", b"EXDATE:2\r\n", b"EXDATE:3\r\n", b"EXDATE:4\r\n"), "line-too-many"),
    ("a UID of 25 octets past its 24", b"BEGIN:VEVENT\r\nUID:" + b"x" * 25 + b"\r\nDTSTART;TZID=Z:20260101T000000\r\nEND:VEVENT\r\n", "line-too-long"),
    ("a SUMMARY of 25 octets once unescaped", component(b"SUMMARY:" + b"x" * 25 + b"\r\n"), "line-too-long"),
    ("a TZID of 25 octets past its 24", b"BEGIN:VEVENT\r\nUID:u\r\nDTSTART;TZID=" + b"z" * 25 + b":20260101T000000\r\nEND:VEVENT\r\n", "line-too-long"),
    ("an EXDATE of 21 octets past its 20", component(b"EXDATE:" + b"d" * 21 + b"\r\n"), "line-too-long"),
    ("a backslash before a letter that is not an escape", component(b"SUMMARY:a\\xb\r\n"), "line-bad-escape"),
    ("a backslash that ends the value", component(b"SUMMARY:ab\\\r\n"), "line-bad-escape"),
    ("a control character in a UID", b"BEGIN:VEVENT\r\nUID:a\x01b\r\nDTSTART;TZID=Z:20260101T000000\r\nEND:VEVENT\r\n", "line-bad-value"),
    ("a carriage return alone inside a value", component(b"RRULE:a\rb\r\n"), "line-bad-value"),
    ("a delete character in a value", component(b"RRULE:a\x7fb\r\n"), "line-bad-value"),
    ("bytes that are not UTF-8 in a value", component(b"RRULE:a\xffb\r\n"), "line-bad-value"),
    ("an incomplete UTF-8 sequence at the end of a value", component(b"RRULE:a\xc3\r\n"), "line-bad-value"),
    ("a number past its type: 65536 in a uint16", component(b"SEQUENCE:65536\r\n"), "line-bad-value"),
    ("a minus sign on an unsigned number", component(b"SEQUENCE:-1\r\n"), "line-bad-value"),
    ("a number that is not digits", component(b"SEQUENCE:12a\r\n"), "line-bad-value"),
    ("a number with a space in it", component(b"SEQUENCE: 1\r\n"), "line-bad-value"),
    ("no digits at all", component(b"SEQUENCE:\r\n"), "line-bad-value"),
    ("2147483648 in an int32", component(b"PRIORITY:2147483648\r\n"), "line-bad-value"),
    ("-2147483649 in an int32", component(b"PRIORITY:-2147483649\r\n"), "line-bad-value"),
    ("a number with more digits than any type holds", component(b"PRIORITY:" + b"9" * 60 + b"\r\n"), "line-bad-value"),
    ("a truth value that is not one", component(b"X-ALL-DAY:yes\r\n"), "line-bad-value"),
    ("a truth value with extra letters", component(b"X-ALL-DAY:TRUEE\r\n"), "line-bad-value"),
    ("two values for a parameter that holds one", b"BEGIN:VEVENT\r\nUID:u\r\nDTSTART;TZID=a,b:20260101T000000\r\nEND:VEVENT\r\n", "line-bad-value"),
    ("two quoted values for a parameter that holds one", b"BEGIN:VEVENT\r\nUID:u\r\nDTSTART;TZID=\"a\",\"b\":20260101T000000\r\nEND:VEVENT\r\n", "line-bad-value"),
    ("a control character in a parameter value", b"BEGIN:VEVENT\r\nUID:u\r\nDTSTART;TZID=a\x02b:20260101T000000\r\nEND:VEVENT\r\n", "line-bad-value"),
]


class SplitMix64:
    """Sebastiano Vigna's SplitMix64, so a seed means one stream everywhere."""

    def __init__(self, seed: int) -> None:
        self.state = seed & (2**64 - 1)

    def next(self) -> int:
        self.state = (self.state + 0x9E3779B97F4A7C15) & (2**64 - 1)
        z = self.state
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & (2**64 - 1)
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & (2**64 - 1)
        return z ^ (z >> 31)

    def below(self, bound: int) -> int:
        return self.next() % bound

    def between(self, low: int, high: int) -> int:
        return low + self.below(high - low + 1)

    def pick(self, items):
        return items[self.below(len(items))]


SEED = 0xE116_0001

#: Characters a drawn string takes from: plain, the three a TEXT escapes, a line
#: feed, a space, a quote, a colon, and characters of two, three and four octets.
PLAIN = list("abcXYZ019-_./ ") + [";", ",", ":", "\\", "\u00e9", "\u20ac", "\U0001F600", "\ud55c"]
#: A TEXT may hold a line feed, which it writes as an escape.
TEXT = PLAIN + ["\n", '"']
#: A value carried as written may hold a quote; a parameter value may not.
VALUE = PLAIN + ['"']
PARAM = PLAIN


def draw_text(rng: SplitMix64, limit: int, alphabet=VALUE) -> str:
    """A string of at most `limit` octets."""
    out = ""
    for _ in range(rng.between(0, limit)):
        c = rng.pick(alphabet)
        if len((out + c).encode("utf-8")) > limit:
            break
        out += c
    return out


def draw_decoded(rng: SplitMix64, entries: list[Entry]) -> dict:
    out = decoded_of()
    out["uid"] = draw_text(rng, 24)
    out["dtstart"] = draw_text(rng, 20)
    out["dtstartTzid"] = draw_text(rng, 24, PARAM)
    for e in entries:
        if e.id in ("uid", "dtstart", "dtstartTzid", "organizerCn"):
            continue
        if e.max_count:
            out[e.id] = [draw_text(rng, e.max_size) for _ in range(rng.between(0, e.max_count))]
        elif rng.below(3) == 0:
            continue
        elif e.type == "string":
            out[e.id] = draw_text(rng, e.max_size, TEXT if e.text else VALUE)
        elif e.type == "bool":
            out[e.id] = bool(rng.below(2))
        else:
            low, high = INT_RANGE[e.type]
            out[e.id] = rng.pick([low, high, 0, 1, rng.between(low, high)])
    if out["organizer"] is not None and rng.below(2):
        out["organizerCn"] = draw_text(rng, 24, PARAM)
    return out


def reshape(rng: SplitMix64, text: bytes) -> bytes:
    """A reading of `text` that means the same: LF for CRLF, other case in the
    names, a fold at drawn places (even inside a character), extra lines the
    codec ignores around and inside the component."""
    lines = [body for body, _ in unfold(text)]
    out = []
    for line in lines:
        name_end = min((line.find(c) for c in (b";", b":") if line.find(c) >= 0), default=0)
        if rng.below(2):
            line = line[:name_end].lower() + line[name_end:]
        out.append(line)
    lines = out
    lines.insert(rng.between(1, len(lines) - 1), b"X-NOISE;K=\"v:w\":some, text")
    if rng.below(2):
        at = rng.between(1, len(lines) - 1)
        lines[at:at] = [b"BEGIN:VALARM", b"UID:inner", b"END:VALARM"]
    folded = []
    for line in lines:
        pieces = []
        start = 0
        while len(line) - start > 3 and rng.below(3) == 0:
            cut = rng.between(start + 1, len(line) - 1)
            pieces.append(line[start:cut])
            start = cut
        pieces.append(line[start:])
        folded.append(b"\r\n ".join(pieces) if rng.below(2) else b"\r\n\t".join(pieces))
    ending = b"\n" if rng.below(2) else b"\r\n"
    body = ending.join(folded) + ending
    if rng.below(2):
        body = b"BEGIN:VCALENDAR" + ending + b"VERSION:2.0" + ending + body + b"END:VCALENDAR" + ending
    return body


def fault(rng: SplitMix64, base: bytes):
    """(why, input, failure) with one fault put into the canonical component."""
    lines = base.split(b"\r\n")[:-1]
    kind = rng.below(8)
    at = rng.between(1, len(lines) - 2)
    if kind == 0:
        return "a property line repeated", b"\r\n".join(lines[: at + 1] + [lines[1]] + lines[at + 1 :]) + b"\r\n", "line-too-many"
    if kind == 1:
        return "the END line dropped", b"\r\n".join(lines[:-1]) + b"\r\n", "need-more-bytes"
    if kind == 2:
        return "a control character put into the UID", base.replace(b"UID:", b"UID:\x07", 1), "line-bad-value"
    if kind == 3:
        return "the UID removed", b"\r\n".join(lines[:1] + lines[2:]) + b"\r\n", "line-required-missing"
    if kind == 4:
        return "the UID grown past its bound", base.replace(b"UID:", b"UID:" + b"u" * 40, 1), "line-too-long"
    if kind == 5:
        return "an empty line put among the properties", b"\r\n".join(lines[:at] + [b""] + lines[at:]) + b"\r\n", "line-malformed"
    if kind == 6:
        return "a stray backslash put into a TEXT", base.replace(b"SUMMARY:", b"SUMMARY:\\q", 1), "line-bad-escape"
    return "the sequence made not a number", base.replace(b"SEQUENCE:", b"SEQUENCE:x", 1), "line-bad-value"


def array(bytes_: bytes) -> list[int]:
    return list(bytes_)


def readable(bytes_: bytes):
    try:
        return bytes_.decode("utf-8")
    except UnicodeDecodeError:
        return None


class Written:
    """The cases and rejects the model writes, each held to the model."""

    def __init__(self, path: Path = RESOURCE):
        component_name, entries = load(path)
        self.model = Model(component_name, entries)
        self.entries = entries
        self.cases: list[dict] = []
        self.rejects: list[dict] = []
        self.accepts = 0
        self.build()

    def round_trip(self, note: str, decoded: dict):
        written = self.model.encode(decoded)
        if written[0] != "ok":
            raise SystemExit(f"content-line model: {note}: encode answers {written}")
        back = self.model.decode(written[1])
        if back != ("ok", decoded):
            raise SystemExit(f"content-line model: {note}: decode answers {back}, wanted {decoded}")
        self.cases.append(
            {"decoded": decoded, "encoded": array(written[1]), "text": readable(written[1]), "note": note}
        )

    def accept(self, note: str, decoded: dict, data: bytes):
        back = self.model.decode(data)
        if back != ("ok", decoded):
            raise SystemExit(f"content-line model: {note}: decode answers {back}, wanted {decoded}")
        self.cases.append(
            {
                "decoded": decoded,
                "encoded": array(data),
                "text": readable(data),
                "decode_only": True,
                "note": note,
            }
        )
        self.accepts += 1

    def reject(self, why: str, data: bytes, failure: str):
        answer = self.model.decode(data)
        if answer != fails(failure):
            raise SystemExit(f"content-line model: {why}: decode answers {answer}, wanted {failure}")
        entry = {"why": why, "encoded": array(data), "error": failure}
        text = readable(data)
        if text is not None:
            entry["text"] = text
        self.rejects.append(entry)

    def build(self):
        for note, decoded in HAND_CASES:
            self.round_trip(note, decoded)
        for note, decoded, data in HAND_ACCEPTS:
            self.accept(note, decoded, data)
        for why, data, failure in HAND_REJECTS:
            self.reject(why, data, failure)
        rng = SplitMix64(SEED)
        for _ in range(40):
            self.round_trip("fuzz: a drawn value written and read back", draw_decoded(rng, self.entries))
        canonical = self.model.encode(CANON)[1]
        for _ in range(30):
            decoded = draw_decoded(rng, self.entries)
            written = self.model.encode(decoded)[1]
            self.accept("fuzz: the same component read from a reshaped input", decoded, reshape(rng, written))
        for _ in range(30):
            why, data, failure = fault(rng, canonical)
            self.reject("fuzz: " + why, data, failure)


# ── Splicing into numerical_reference.json ──────────────────────────────


def case_text(case: dict) -> str:
    return "        " + json.dumps(case, ensure_ascii=True)


def span(text: str, fixture: str, key: str):
    """(start, end) of the contents of `fixture`'s `key` array."""
    anchor = text.index(f'"{fixture}": {{')
    opening = text.index(f'"{key}": [', anchor) + len(f'"{key}": [')
    closing = text.index("\n      ]", opening)
    return opening, closing


def regenerate(text: str) -> tuple[str, str]:
    """`text` with the fixture's cases and rejects written, and a one-line report."""
    written = Written()
    for key, items in (("rejects", written.rejects), ("cases", written.cases)):
        start, end = span(text, FIXTURE, key)
        body = "\n" + ",\n".join(case_text(item) for item in items)
        text = text[:start] + body + text[end:]
    report = (
        f"{FIXTURE}: {len(written.cases)} cases ({written.accepts} read only), "
        f"{len(written.rejects)} rejects; the model holds each"
    )
    return text, report
