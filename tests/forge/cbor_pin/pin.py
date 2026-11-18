#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The bytes every backend's CBOR encode writes today, pinned (docs/adr/0101, step 0).

ADR 0101 changes what a CBOR codec's entry may hold, and section 8 claims that no
existing CBOR document changes meaning: for every value of every committed CBOR
document, each encode writes the bytes it writes today. This directory turns that
claim into a check. `cases.json` states the values, at a smallest and a largest
value of each document and every optional entry both ways; `captured.json` holds
what each of the seven backends wrote for them, captured from the encoders as
they are before any step of the record moves them.

How a backend is asked. Each case's value is written here in core deterministic
CBOR (RFC 8949 section 4.2.1) by an independent model of the documents' types:
the codec's own map of present entries in ascending key order, a list of records
an array of maps keyed by field id in the order of their encoded keys, every head
in its shortest form. Each backend's driver decodes those bytes with the codec it
generates and encodes the value it read again, and writes what its encode wrote,
or the refusal its decode or encode answered. No driver builds a value by hand,
so one driver is the same few lines for every document, and what is captured is
the encode's output for the value the decode reads.

How the capture becomes the pin. `merge` reads the seven drivers' outputs and
writes `captured.json`: where the seven agree, the bytes once; where they do not,
each backend's own, and the case is named among the disagreements, a finding the
record keeps rather than resolves. A case `captured.json` already holds is held,
not rewritten: a later capture whose bytes for it differ on any backend fails and
writes nothing, which is how every later step of ADR 0101 shows that it moved no
pinned byte. A step that moves one on purpose (a negative variant of a signed enum,
section 8.2) runs `release` for that case in its own commit, with its reason, and
captures it again.

Usage, from the repository root:

    python3 tests/forge/cbor_pin/pin.py check           # validate cases.json
    python3 tests/forge/cbor_pin/pin.py codecs          # the documents, one a line
    python3 tests/forge/cbor_pin/pin.py inputs FILE     # the drivers' input
    python3 tests/forge/cbor_pin/pin.py merge DIR       # fold DIR/<backend>.txt in
    python3 tests/forge/cbor_pin/pin.py release ID...   # unpin cases by id

The drivers read FILE, one case a line, `<id> <hex>`, where `<id>` is
`<codec>/<case>` and `<hex>` the input's bytes in lower-case hexadecimal, and
write one line a case, `<id> ok <hex>` or `<id> refused <words>`.
`scripts/capture_cbor_pin.sh` runs the seven and the merge.
"""

from __future__ import annotations

import hashlib
import json
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO_ROOT = HERE.parents[2]
CASES = HERE / "cases.json"
CAPTURED = HERE / "captured.json"

# The seven backends whose encoders are captured, in the order the capture runs
# them and `captured.json` lists them.
BACKENDS = ("rust", "cpp", "c11", "go", "kotlin", "python", "swift")

# Bytes past this length are kept as their length and SHA-256 rather than spelled
# out: a largest envelope is some nineteen million bytes.
INLINE_LIMIT = 4096

SCXML_NS = "{http://www.w3.org/2005/07/scxml}"
SCE_NS = "{http://sce.dev/ext}"

UNSIGNED_WIDTHS = {"uint8": 8, "uint16": 16, "uint32": 32, "uint64": 64}
SIGNED_WIDTHS = {"int8": 8, "int16": 16, "int32": 32, "int64": 64}


class PinError(Exception):
    """A case, a document or a capture this script cannot accept."""


# ---------------------------------------------------------------------------
# The documents, as far as the CBOR form of a value reads them.
# ---------------------------------------------------------------------------


def resolve_import(document: Path, src: str) -> Path:
    """Where an `<sce:import src>` of `document` points."""
    if src.startswith("sce:std/"):
        return REPO_ROOT / "stdlib" / src[len("sce:std/") :]
    return (document.parent / src).resolve()


def parse_document(path: Path) -> ET.Element:
    try:
        return ET.parse(path).getroot()
    except (OSError, ET.ParseError) as e:
        raise PinError(f"{path}: {e}") from e


def data_elements(root: ET.Element) -> list[ET.Element]:
    datamodel = root.find(f"{SCXML_NS}datamodel")
    if datamodel is None:
        return []
    return list(datamodel.findall(f"{SCXML_NS}data"))


class Enum:
    """An enum document: its variants by name, its carrier and whether it is open."""

    def __init__(self, path: Path) -> None:
        root = parse_document(path)
        self.path = path
        carrier = root.get(f"{SCE_NS}underlying-type", "uint8")
        self.bits, self.signed = integer_width(carrier, path)
        self.open = root.get(f"{SCE_NS}strict-variants") == "false"
        self.variants: dict[str, int] = {}
        for variant in root.iter(f"{SCE_NS}variant"):
            self.variants[variant.get("name", "")] = int(variant.get("value", "0"), 0)

    def number(self, value: object, where: str) -> int:
        if isinstance(value, str):
            if value not in self.variants:
                raise PinError(f"{where}: {self.path.name} declares no variant '{value}'")
            return self.variants[value]
        if isinstance(value, int) and not isinstance(value, bool):
            if not self.open and value not in self.variants.values():
                raise PinError(f"{where}: {value} names no variant of the closed {self.path.name}")
            check_integer(value, self.bits, self.signed, where)
            return value
        raise PinError(f"{where}: an enum's value is a variant's name or, for an open enum, a number")


def integer_width(type_name: str, where: object) -> tuple[int, bool]:
    if type_name in UNSIGNED_WIDTHS:
        return UNSIGNED_WIDTHS[type_name], False
    if type_name in SIGNED_WIDTHS:
        return SIGNED_WIDTHS[type_name], True
    raise PinError(f"{where}: '{type_name}' is not an integer type")


def check_integer(value: int, bits: int, signed: bool, where: str) -> None:
    low, high = (-(1 << (bits - 1)), (1 << (bits - 1)) - 1) if signed else (0, (1 << bits) - 1)
    if not low <= value <= high:
        raise PinError(f"{where}: {value} is outside {low}..{high}")


class Leaf:
    """A scalar, text or byte string type, with its bound."""

    def __init__(self, kind: str, bits: int = 0, signed: bool = False,
                 most: int | None = None, exact: int | None = None,
                 enum: Enum | None = None) -> None:
        self.kind = kind
        self.bits = bits
        self.signed = signed
        self.most = most
        self.exact = exact
        self.enum = enum


class RecordList:
    """`list<record:R>[M..N]`: the record's fields by id, in declaration order."""

    def __init__(self, fields: list[tuple[str, Leaf]], least: int, most: int) -> None:
        self.fields = fields
        self.least = least
        self.most = most


def bracket_bound(type_text: str) -> tuple[str, int | None]:
    """`string[16]` as (`string`, 16); a type with no bracket keeps None."""
    if type_text.endswith("]") and "[" in type_text:
        base, bound = type_text[:-1].split("[", 1)
        return base, int(bound)
    return type_text, None


def leaf_type(type_text: str, element: ET.Element, enums: dict[str, Enum], where: str) -> Leaf:
    base, bound = bracket_bound(type_text)
    most = element.get(f"{SCE_NS}max-size")
    length = element.get(f"{SCE_NS}length")
    if base == "bool":
        return Leaf("bool")
    if base in UNSIGNED_WIDTHS or base in SIGNED_WIDTHS:
        bits, signed = integer_width(base, where)
        return Leaf("int", bits=bits, signed=signed)
    if base in ("string", "bytes"):
        stated = bound if bound is not None else (int(most) if most is not None else None)
        exact = int(length) if length is not None else None
        return Leaf("text" if base == "string" else "bytes", most=stated, exact=exact)
    if base.startswith("enum:"):
        alias = base[len("enum:") :]
        if alias not in enums:
            raise PinError(f"{where}: no enum is imported as '{alias}'")
        return Leaf("enum", enum=enums[alias])
    raise PinError(f"{where}: '{type_text}' is a type no committed CBOR codec holds today")


class Entry:
    def __init__(self, name: str, key: int, optional: bool, shape: Leaf | RecordList) -> None:
        self.name = name
        self.key = key
        self.optional = optional
        self.shape = shape


class Codec:
    """One CBOR codec document, its entries by id."""

    def __init__(self, path: Path) -> None:
        root = parse_document(path)
        if root.get(f"{SCE_NS}encoding") != "cbor":
            raise PinError(f"{path}: not an sce:encoding=\"cbor\" codec")
        self.path = path
        self.name = root.get("name", path.stem)
        enums: dict[str, Enum] = {}
        records: dict[str, list[tuple[str, Leaf]]] = {}
        for imp in root.findall(f"{SCE_NS}import"):
            target = resolve_import(path, imp.get("src", ""))
            alias = imp.get("as", "")
            if imp.get("kind") == "enum":
                enums[alias] = Enum(target)
            elif imp.get("kind") == "event-schema":
                schema = parse_document(target)
                records[alias] = [
                    (d.get("id", ""), leaf_type(d.get(f"{SCE_NS}type", ""), d, enums, f"{target.name}:{d.get('id')}"))
                    for d in data_elements(schema)
                ]
        self.entries: dict[str, Entry] = {}
        for d in data_elements(root):
            key = d.get(f"{SCE_NS}key")
            if key is None:
                continue
            name = d.get("id", "")
            where = f"{path.name}:{name}"
            type_text = d.get(f"{SCE_NS}type", "")
            optional = type_text.startswith("optional<") and type_text.endswith(">")
            if optional:
                type_text = type_text[len("optional<") : -1]
            shape: Leaf | RecordList
            if type_text.startswith("list<record:"):
                inner, _, counts = type_text[len("list<record:") :].partition(">[")
                least_text, _, most_text = counts.rstrip("]").rpartition("..")
                if inner not in records:
                    raise PinError(f"{where}: no event schema is imported as '{inner}'")
                shape = RecordList(records[inner], int(least_text) if least_text else 0, int(most_text))
            else:
                shape = leaf_type(type_text, d, enums, where)
            self.entries[name] = Entry(name, int(key), optional, shape)


# ---------------------------------------------------------------------------
# The deterministic CBOR of a value (RFC 8949 section 4.2.1).
# ---------------------------------------------------------------------------


def head(major: int, argument: int) -> bytes:
    if argument < 24:
        return bytes([major << 5 | argument])
    for extra, width in ((24, 1), (25, 2), (26, 4), (27, 8)):
        if argument < 1 << (8 * width):
            return bytes([major << 5 | extra]) + argument.to_bytes(width, "big")
    raise PinError(f"{argument} needs more than eight bytes")


def integer(value: int) -> bytes:
    return head(0, value) if value >= 0 else head(1, -1 - value)


def spelled_text(value: object, where: str) -> str:
    if isinstance(value, str):
        return value
    if isinstance(value, dict) and set(value) == {"fill", "count"}:
        return str(value["fill"]) * int(value["count"])
    raise PinError(f"{where}: a text is a string or {{\"fill\": text, \"count\": n}}")


def spelled_bytes(value: object, where: str) -> bytes:
    try:
        if isinstance(value, str):
            return bytes.fromhex(value)
        if isinstance(value, dict) and set(value) == {"fill", "count"}:
            return bytes.fromhex(str(value["fill"])) * int(value["count"])
    except ValueError as e:
        raise PinError(f"{where}: {e}") from e
    raise PinError(f"{where}: a byte string is hex or {{\"fill\": hex, \"count\": n}}")


def spelled_list(value: object, where: str) -> list[object]:
    if isinstance(value, list):
        return value
    if isinstance(value, dict) and set(value) == {"repeat", "count"}:
        return [value["repeat"]] * int(value["count"])
    raise PinError(f"{where}: a list is an array or {{\"repeat\": element, \"count\": n}}")


def encode_leaf(leaf: Leaf, value: object, where: str) -> bytes:
    if leaf.kind == "bool":
        if not isinstance(value, bool):
            raise PinError(f"{where}: expected true or false")
        return b"\xf5" if value else b"\xf4"
    if leaf.kind == "int":
        if not isinstance(value, int) or isinstance(value, bool):
            raise PinError(f"{where}: expected an integer")
        check_integer(value, leaf.bits, leaf.signed, where)
        return integer(value)
    if leaf.kind == "enum":
        assert leaf.enum is not None
        return integer(leaf.enum.number(value, where))
    raw = spelled_text(value, where).encode("utf-8") if leaf.kind == "text" else spelled_bytes(value, where)
    if leaf.exact is not None and len(raw) != leaf.exact:
        raise PinError(f"{where}: {len(raw)} bytes where the length is {leaf.exact}")
    if leaf.most is not None and len(raw) > leaf.most:
        raise PinError(f"{where}: {len(raw)} bytes past the bound {leaf.most}")
    return head(3 if leaf.kind == "text" else 2, len(raw)) + raw


def encode_record(fields: list[tuple[str, Leaf]], value: object, where: str) -> bytes:
    if not isinstance(value, dict):
        raise PinError(f"{where}: a record is an object of its fields")
    names = [name for name, _ in fields]
    unknown = sorted(set(value) - set(names))
    missing = [name for name in names if name not in value]
    if unknown or missing:
        raise PinError(f"{where}: fields {unknown} are not the record's, {missing} are missing")
    pairs = sorted(
        (head(3, len(name.encode("utf-8"))) + name.encode("utf-8"), encode_leaf(leaf, value[name], f"{where}.{name}"))
        for name, leaf in fields
    )
    return head(5, len(pairs)) + b"".join(k + v for k, v in pairs)


def encode_entry(entry: Entry, value: object, where: str) -> bytes:
    if isinstance(entry.shape, RecordList):
        items = spelled_list(value, where)
        if not entry.shape.least <= len(items) <= entry.shape.most:
            raise PinError(f"{where}: {len(items)} records outside {entry.shape.least}..{entry.shape.most}")
        return head(4, len(items)) + b"".join(
            encode_record(entry.shape.fields, item, f"{where}[{i}]") for i, item in enumerate(items)
        )
    return encode_leaf(entry.shape, value, where)


def encode_codec(codec: Codec, value: dict[str, object], where: str) -> bytes:
    unknown = sorted(set(value) - set(codec.entries))
    if unknown:
        raise PinError(f"{where}: {unknown} are not entries of {codec.name}")
    present = []
    for entry in codec.entries.values():
        if entry.name not in value:
            if not entry.optional:
                raise PinError(f"{where}: the required entry '{entry.name}' is absent")
            continue
        present.append((entry.key, encode_entry(entry, value[entry.name], f"{where}.{entry.name}")))
    present.sort()
    return head(5, len(present)) + b"".join(head(0, key) + item for key, item in present)


# ---------------------------------------------------------------------------
# The case list.
# ---------------------------------------------------------------------------


class Case:
    def __init__(self, codec: Codec, name: str, value: dict[str, object], note: str) -> None:
        self.codec = codec
        self.name = name
        self.value = value
        self.note = note
        self._input: bytes | None = None

    @property
    def id(self) -> str:
        return f"{self.codec.name}/{self.name}"

    def input(self) -> bytes:
        """The value in deterministic CBOR, the bytes every driver decodes."""
        if self._input is None:
            self._input = encode_codec(self.codec, self.value, self.id)
        return self._input


def load_cases() -> list[Case]:
    try:
        spec = json.loads(CASES.read_text(encoding="utf-8"))
    except (OSError, ValueError) as e:
        raise PinError(f"{CASES}: {e}") from e
    cases: list[Case] = []
    seen_codecs: set[str] = set()
    for group in spec.get("codecs", []):
        codec = Codec(REPO_ROOT / group["document"])
        if codec.name in seen_codecs:
            raise PinError(f"{CASES}: {codec.name} is listed twice")
        seen_codecs.add(codec.name)
        by_name: dict[str, dict[str, object]] = {}
        for item in group.get("cases", []):
            name = item["name"]
            if name in by_name:
                raise PinError(f"{CASES}: {codec.name}/{name} is given twice")
            value: dict[str, object] = {}
            if "base" in item:
                if item["base"] not in by_name:
                    raise PinError(f"{CASES}: {codec.name}/{name} names no earlier case '{item['base']}'")
                value.update(by_name[item["base"]])
            value.update(item.get("with", {}))
            by_name[name] = value
            cases.append(Case(codec, name, value, item.get("note", "")))
    if not cases:
        raise PinError(f"{CASES}: no case")
    return cases


# ---------------------------------------------------------------------------
# The captures.
# ---------------------------------------------------------------------------


def bytes_form(raw: bytes) -> object:
    if len(raw) <= INLINE_LIMIT:
        return raw.hex()
    return {"length": len(raw), "sha256": hashlib.sha256(raw).hexdigest()}


def read_capture(path: Path, ids: set[str]) -> dict[str, dict[str, object]]:
    """One backend's lines, by case id: `{"encoded": bytes}` or `{"refused": words}`."""
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except OSError as e:
        raise PinError(f"{path}: {e}") from e
    results: dict[str, dict[str, object]] = {}
    for number, line in enumerate(lines, start=1):
        if not line.strip():
            continue
        parts = line.split(" ", 2)
        if len(parts) < 2 or parts[0] not in ids:
            raise PinError(f"{path}:{number}: not a line of a case: {line[:80]!r}")
        case_id, verdict = parts[0], parts[1]
        rest = parts[2] if len(parts) == 3 else ""
        if case_id in results:
            raise PinError(f"{path}:{number}: {case_id} is answered twice")
        if verdict == "ok":
            try:
                results[case_id] = {"encoded": bytes.fromhex(rest)}
            except ValueError as e:
                raise PinError(f"{path}:{number}: {e}") from e
        elif verdict == "refused":
            results[case_id] = {"refused": rest.strip() or "(unnamed)"}
        else:
            raise PinError(f"{path}:{number}: '{verdict}' is neither ok nor refused")
    unanswered = sorted(ids - set(results))
    if unanswered:
        raise PinError(f"{path}: no answer for {', '.join(unanswered)}")
    return results


def outcome(result: dict[str, object]) -> object:
    """What a pin holds of one backend's result: its bytes' form, or that it refused."""
    if "encoded" in result:
        encoded = result["encoded"]
        return bytes_form(encoded) if isinstance(encoded, bytes) else encoded
    return "refused"


def entry_for(case: Case, by_backend: dict[str, dict[str, object]]) -> tuple[dict[str, object], bool]:
    """The entry `captured.json` keeps for one case, and whether the backends disagree."""
    outcomes = {backend: outcome(by_backend[backend]) for backend in BACKENDS}
    distinct = {json.dumps(o, sort_keys=True) for o in outcomes.values()}
    if len(distinct) == 1:
        first = by_backend[BACKENDS[0]]
        if "encoded" in first:
            encoded = first["encoded"]
            assert isinstance(encoded, bytes)
            return {"encoded": bytes_form(encoded), "equals_input": encoded == case.input()}, False
        return {"refused": {b: by_backend[b]["refused"] for b in BACKENDS}}, False
    kept: dict[str, object] = {}
    for backend in BACKENDS:
        result = by_backend[backend]
        if "encoded" in result:
            encoded = result["encoded"]
            assert isinstance(encoded, bytes)
            kept[backend] = {"encoded": bytes_form(encoded), "equals_input": encoded == case.input()}
        else:
            kept[backend] = {"refused": result["refused"]}
    return {"by_backend": kept}, True


def pinned_outcomes(entry: dict[str, object]) -> dict[str, object]:
    """Each backend's outcome as an entry of `captured.json` states it."""
    if "encoded" in entry:
        return {backend: entry["encoded"] for backend in BACKENDS}
    if "refused" in entry:
        return {backend: "refused" for backend in BACKENDS}
    by_backend = entry["by_backend"]
    assert isinstance(by_backend, dict)
    return {
        backend: (result["encoded"] if "encoded" in result else "refused")
        for backend, result in by_backend.items()
    }


def load_captured() -> dict[str, object]:
    if not CAPTURED.exists():
        return {}
    try:
        return json.loads(CAPTURED.read_text(encoding="utf-8"))
    except ValueError as e:
        raise PinError(f"{CAPTURED}: {e}") from e


def write_captured(cases_out: dict[str, object], disagreements: list[str]) -> None:
    document = {
        "description": (
            "What each backend's CBOR encode wrote for the values of cases.json, captured before any step "
            "of docs/adr/0101 (its step 0, section 8.3) and held by every later capture: an entry the seven "
            "agree on keeps the bytes once, one they disagree on keeps each backend's under by_backend and is "
            "named under disagreements. Bytes past 4096 are kept as their length and SHA-256. equals_input "
            "says whether the bytes are the deterministic form of the value (pin.py's model), which the "
            "drivers decoded. Written by tests/forge/cbor_pin/pin.py merge; never edited by hand."
        ),
        "backends": list(BACKENDS),
        "cases": cases_out,
        "disagreements": disagreements,
    }
    CAPTURED.write_text(json.dumps(document, indent=1, sort_keys=False) + "\n", encoding="utf-8")


def merge(capture_dir: Path) -> int:
    cases = load_cases()
    ids = {case.id for case in cases}
    results = {backend: read_capture(capture_dir / f"{backend}.txt", ids) for backend in BACKENDS}
    previous = load_captured()
    held = previous.get("cases", {})
    assert isinstance(held, dict)
    removed = sorted(set(held) - ids)
    moved: list[str] = []
    cases_out: dict[str, object] = {}
    disagreements: list[str] = []
    for case in cases:
        entry, disagree = entry_for(case, {b: results[b][case.id] for b in BACKENDS})
        if case.id in held:
            before = pinned_outcomes(held[case.id])
            after = pinned_outcomes(entry)
            changed = [b for b in BACKENDS if before.get(b) != after.get(b)]
            if changed:
                moved.append(f"{case.id}: {', '.join(changed)}")
        cases_out[case.id] = entry
        if disagree:
            disagreements.append(case.id)
    if removed or moved:
        for case_id in removed:
            print(f"pin: {case_id} is pinned and gone from cases.json; release it first", file=sys.stderr)
        for line in moved:
            print(f"pin: the bytes of {line} moved from what captured.json pins", file=sys.stderr)
        print("pin: captured.json is left as it was", file=sys.stderr)
        return 1
    write_captured(cases_out, disagreements)
    added = sorted(ids - set(held))
    print(f"pin: {len(cases)} case(s), {len(added)} newly pinned, {len(disagreements)} disagreement(s)")
    for case_id in disagreements:
        print(f"pin: the backends disagree on {case_id}")
    for case_id, entry in cases_out.items():
        if isinstance(entry, dict) and entry.get("equals_input") is False:
            print(f"pin: {case_id}: the bytes the backends wrote are not the deterministic form of its value")
    return 0


def release(case_ids: list[str]) -> int:
    previous = load_captured()
    held = previous.get("cases", {})
    disagreements = previous.get("disagreements", [])
    assert isinstance(held, dict) and isinstance(disagreements, list)
    unknown = [case_id for case_id in case_ids if case_id not in held]
    if unknown:
        print(f"pin: not pinned: {', '.join(unknown)}", file=sys.stderr)
        return 1
    for case_id in case_ids:
        del held[case_id]
    write_captured(held, [d for d in disagreements if d not in case_ids])
    print(f"pin: released {len(case_ids)} case(s)")
    return 0


def main(argv: list[str]) -> int:
    if not argv:
        print(__doc__, file=sys.stderr)
        return 2
    command, rest = argv[0], argv[1:]
    try:
        if command == "check" and not rest:
            cases = load_cases()
            for case in cases:
                case.input()
            print(f"pin: {len(cases)} case(s) over {len({c.codec.name for c in cases})} codec(s) are values of their documents")
            return 0
        if command == "codecs" and not rest:
            listed: list[str] = []
            for case in load_cases():
                document = case.codec.path.relative_to(REPO_ROOT).as_posix()
                if document not in listed:
                    listed.append(document)
            print("\n".join(listed))
            return 0
        if command == "inputs" and len(rest) == 1:
            with open(rest[0], "w", encoding="utf-8") as out:
                for case in load_cases():
                    out.write(f"{case.id} {case.input().hex()}\n")
            return 0
        if command == "merge" and len(rest) == 1:
            return merge(Path(rest[0]))
        if command == "release" and rest:
            return release(rest)
    except PinError as e:
        print(f"pin: {e}", file=sys.stderr)
        return 1
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
