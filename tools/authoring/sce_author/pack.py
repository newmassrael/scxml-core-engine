"""Loading a pack: the interface model and the conventions.

A pack is the only route by which a subject matter reaches this package. The
loader therefore does two things beyond parsing: it validates against the
published schema, and it refuses silently-empty input. A pack that loads to
nothing would make every later answer vacuously clean, which reads exactly like
success.
"""

from __future__ import annotations

import json
import pathlib
import re
from dataclasses import dataclass, field

import yaml

from .errors import READ_ERRORS, PackError, describe_path

try:
    import jsonschema
except ImportError:  # pragma: no cover - the caller is told, not worked around
    jsonschema = None

SCHEMA_DIR = pathlib.Path(__file__).resolve().parent.parent / "schema"

MODEL_FILES = ("interface-model.yaml", "interface-model.yml", "interface-model.json")
CONVENTION_FILES = ("conventions.yaml", "conventions.yml", "conventions.json")


def _read(path: pathlib.Path):
    """Read a pack file, or refuse in a sentence that names it.

    ⚠ Everything here used to travel to the caller as whatever the library
    raised. A pack whose model was binary reached the command line as a YAML
    scanner traceback whose last line was `in "<unicode string>", position 0`
    -- which does not name the file, let alone the pack.
    """
    try:
        text = path.read_text(encoding="utf-8")
    except READ_ERRORS as exc:
        if not path.is_file():
            raise PackError(describe_path(path)) from exc
        raise PackError(f"{path}: cannot be read as text ({exc})") from exc
    try:
        if path.suffix == ".json":
            return json.loads(text)
        return yaml.safe_load(text)
    except (yaml.YAMLError, json.JSONDecodeError) as exc:
        first = str(exc).strip().splitlines()[0] if str(exc).strip() else exc
        raise PackError(f"{path}: not well-formed ({first})") from exc


def _validate(doc, schema_name: str, path: pathlib.Path) -> None:
    if jsonschema is None:
        raise PackError(
            "jsonschema is not installed, so a pack cannot be validated. "
            "Refusing rather than loading an unchecked pack."
        )
    schema = json.loads((SCHEMA_DIR / schema_name).read_text(encoding="utf-8"))
    validator = jsonschema.Draft202012Validator(schema)
    errors = sorted(validator.iter_errors(doc), key=lambda e: list(e.path))
    if errors:
        first = errors[0]
        where = " -> ".join(str(p) for p in first.path) or "(document root)"
        raise PackError(f"{path}: {where}: {first.message}")


@dataclass(frozen=True)
class Field:
    """One writable position on an address, with the values it admits."""

    name: str
    values: dict[str, int] | None
    type: str | None

    def admits(self, symbol: str) -> bool:
        return self.values is None or symbol in self.values


@dataclass(frozen=True)
class Entry:
    address: str
    role: str
    names: tuple[str, ...]
    fields: tuple[Field, ...]
    note: str = ""

    def field(self, name: str) -> Field | None:
        for f in self.fields:
            if f.name == name:
                return f
        return None


@dataclass
class Model:
    """Every address a document may touch, indexed the three ways it is asked for."""

    entries: list[Entry] = field(default_factory=list)
    by_address: dict[str, Entry] = field(default_factory=dict)
    by_name: dict[str, list[Entry]] = field(default_factory=dict)

    def outputs(self) -> list[Entry]:
        return [e for e in self.entries if e.role == "output"]

    def owning(self, address: str) -> Entry | None:
        """The entry an address belongs to, following it up to its record.

        An address may name a field, and a field may itself be nested
        (`…Pressure.MonitorPage.Stat` under a declared `…Pressure`). Stripping
        one segment covers the first case and not the second, which is how
        four addresses a pack DOES declare were reported as undeclared.
        Walking up is sound: if any ancestor is declared, the address is a
        position within it.
        """
        parts = address.split(".")
        for cut in range(len(parts), 0, -1):
            entry = self.by_address.get(".".join(parts[:cut]))
            if entry is not None:
                return entry
        return None

    def resolve(self, name: str) -> list[Entry]:
        """Entries the prose could mean by this name.

        Returns a list rather than one entry: a name that reaches two addresses
        is an ambiguity the pack has to settle, and hiding it behind a first
        match would make the document silently read the wrong one.
        """
        return self.by_name.get(name, [])

    def numbered(self, name: str) -> list[str]:
        """Addresses whose last segment is this name followed by a number.

        A platform commonly publishes what a specification writes as one
        subscripted name -- `Setting_Tolerance[i]` -- as a numbered family,
        `Setting_Tolerance1` through `Setting_Tolerance15`. The two are the same thing
        said twice, and nothing in either document connects them, so an author
        told merely that their name does not exist goes to ask for something
        the platform already has.

        Sorted by the number rather than the text, so `2` comes before `10`.
        """
        found = []
        for address in self.by_address:
            leaf = address.rsplit(".", 1)[-1]
            if leaf.startswith(name) and leaf[len(name):].isdigit():
                found.append((int(leaf[len(name):]), leaf))
        return [leaf for _index, leaf in sorted(found)]


# ⚠ A trap every pack author will meet, so the refusal names it rather than
# letting the reader find out from an AttributeError three modules away.
# YAML 1.1 reads bare ON, OFF, YES, NO, TRUE and FALSE as booleans, so a value
# space written `ON: 1` arrives with a boolean where a symbol should be. It
# was met twice in one day: once in a pack's supply ladder, once in the first
# value space a second subject matter declared.
_YAML_BOOLEANS = "ON OFF YES NO TRUE FALSE Y N"


def _check_symbols(where: str, values) -> None:
    for symbol in values or ():
        if isinstance(symbol, str):
            continue
        raise PackError(
            f"{where}: a value space is keyed by {symbol!r}, which is not a "
            f"symbol. YAML reads bare {_YAML_BOOLEANS} as booleans -- quote "
            f"them (\"ON\": 1) so they stay the names the platform uses."
        )


def _entry(raw: dict) -> Entry:
    fields: list[Field] = []
    if "fields" in raw:
        for fname, fspec in raw["fields"].items():
            fields.append(Field(fname, fspec.get("values"), fspec.get("type")))
    else:
        fields.append(Field("", raw.get("values"), raw.get("type")))
    return Entry(
        address=raw["address"],
        role=raw["role"],
        names=tuple(raw.get("names") or ()),
        fields=tuple(fields),
        note=raw.get("note", ""),
    )


def load_model(paths: list[pathlib.Path]) -> Model:
    """Merge any number of interface-model files.

    Several files because a platform's model is usually published in pieces and
    joining them by hand is how a piece goes missing. Merging is a union; a
    repeated address is an error rather than a last-one-wins, because the two
    copies would disagree about a value space and nothing would say so.
    """
    model = Model()
    for path in paths:
        doc = _read(path)
        _validate(doc, "interface-model.v1.schema.json", path)
        for raw in doc["entries"]:
            entry = _entry(raw)
            for fld in entry.fields:
                _check_symbols(f"{path}: {entry.address}", fld.values)
            if entry.address in model.by_address:
                raise PackError(
                    f"{path}: address {entry.address!r} is already declared. "
                    "Two declarations of one address cannot both be believed."
                )
            model.by_address[entry.address] = entry
            model.entries.append(entry)
            for name in entry.names:
                model.by_name.setdefault(name, []).append(entry)
    if not model.entries:
        raise PackError(
            "the interface model is empty. An empty model answers every "
            "question cleanly, which is indistinguishable from a correct one."
        )
    return model


@dataclass(frozen=True)
class NameClass:
    pattern: re.Pattern
    role: str


@dataclass
class Conventions:
    name_classes: list[NameClass]
    precondition_inputs: dict[str, str]
    precondition_phrases: dict[str, str]
    normalise: list[tuple[str, str]]
    gate_off: list[dict]
    neutral_symbols: tuple[str, ...]
    absence_tokens: tuple[str, ...]
    # Address prefixes that are plumbing rather than subject matter.
    infrastructure: tuple[str, ...]
    time_inputs: tuple[str, ...]
    duration_pattern: re.Pattern | None
    # What the pack knows about how often its own cascade is right. Printed
    # beside every proposal the cascade makes, because a procedure with a
    # measured hit rate is not an oracle and must not read like one.
    gate_off_note: str
    # How THIS kind of document writes a comparison. None means the default in
    # `prose.DEFAULT_COMPARISON`, which is programming notation. This is the
    # only thing about the shape of prose the core would otherwise assume.
    comparison_pattern: re.Pattern | None
    # Ways of reading an address that the pack names and the core does not
    # interpret. This is the seam: a platform's signalling idioms stay data.
    protocols: dict[str, dict]

    def classify(self, name: str) -> str | None:
        for nc in self.name_classes:
            if nc.pattern.fullmatch(name) or nc.pattern.match(name):
                return nc.role
        return None

    def names_in(self, text: str) -> dict[str, str]:
        """Every name the prose writes, to its role. Order of classes decides."""
        found: dict[str, str] = {}
        for nc in self.name_classes:
            for hit in nc.pattern.findall(text):
                token = hit if isinstance(hit, str) else hit[0]
                found.setdefault(token, nc.role)
        return found

    def phrase_expression(self, phrase: str) -> str | None:
        key = phrase.strip().lower()
        for frm, to in self.normalise:
            key = re.sub(frm, to, key)
        return self.precondition_phrases.get(key.strip())


def load_conventions(paths: list[pathlib.Path]) -> Conventions:
    classes: list[NameClass] = []
    inputs: dict[str, str] = {}
    phrases: dict[str, str] = {}
    normalise: list[tuple[str, str]] = []
    gate_off: list[dict] = []
    neutral: list[str] = []
    absence: list[str] = []
    plumbing: list[str] = []
    time_inputs: list[str] = []
    duration = None
    comparison = None
    gate_off_note = ""
    protocols: dict[str, dict] = {}

    for path in paths:
        doc = _read(path)
        _validate(doc, "conventions.v1.schema.json", path)
        for nc in doc["name_classes"]:
            classes.append(NameClass(re.compile(nc["pattern"]), nc["role"]))
        pre = doc.get("preconditions") or {}
        inputs.update(pre.get("inputs") or {})
        phrases.update({k.strip().lower(): v for k, v in (pre.get("phrases") or {}).items()})
        normalise += [(n["from"], n["to"]) for n in (pre.get("normalise") or [])]
        gate_off += doc.get("gate_off") or []
        gate_off_note = doc.get("gate_off_note") or gate_off_note
        neutral += doc.get("neutral_symbols") or []
        absence += doc.get("absence_tokens") or []
        plumbing += doc.get("infrastructure") or []
        time_inputs += doc.get("time_inputs") or []
        protocols.update(doc.get("protocols") or {})
        if doc.get("duration_pattern"):
            duration = re.compile(doc["duration_pattern"])
        if doc.get("comparison_pattern"):
            comparison = re.compile(doc["comparison_pattern"])
            missing = {"name", "op", "token"} - set(comparison.groupindex)
            if missing:
                raise PackError(
                    f"{path}: comparison_pattern needs the named group(s) "
                    f"{', '.join(sorted(missing))} — without them the core "
                    f"cannot say what was compared against what"
                )

    return Conventions(
        name_classes=classes,
        precondition_inputs=inputs,
        precondition_phrases=phrases,
        normalise=normalise,
        gate_off=gate_off,
        neutral_symbols=tuple(dict.fromkeys(neutral)),
        absence_tokens=tuple(dict.fromkeys(absence)),
        infrastructure=tuple(dict.fromkeys(plumbing)),
        time_inputs=tuple(dict.fromkeys(time_inputs)),
        duration_pattern=duration,
        gate_off_note=gate_off_note,
        comparison_pattern=comparison,
        protocols=protocols,
    )


@dataclass(frozen=True)
class Case:
    """One exercise: what was driven, and what was required of the result."""

    name: str
    given: dict
    expect: dict
    # How long the situation had held when this case was observed, in
    # milliseconds, or None when the records did not say. ⚠ A DURATION, not a
    # timestamp: it restarts with the situation. And not in `given`, because
    # time has no address and nothing drives it.
    elapsed_ms: float | None = None
    # Addresses this case SET, as opposed to ones that merely still held a
    # value. ⚠ Not the same as "what differs from the case before": a counter
    # restated at the same reading is still a statement that the thing
    # happened again, and a subtraction reads that as nothing happening.
    drove: tuple = ()
    # Which variant of the subject matter this case was taken from. ⚠ Not a
    # signal: nothing drives it and it has no value space, so it sits here
    # beside the clock rather than in `given`.
    variant: str = ""


@dataclass
class Examples:
    """What the subject matter is shown DOING.

    The interface model says what exists; examples say what is exercised, and
    those are different questions. A specification that names something the
    platform has not got is caught by the model. A specification that never
    names something the platform USES cannot be caught by anything the text is
    compared against -- there is no absence to notice until something else
    expects it.
    """

    origin: str = ""
    driven: frozenset = frozenset()     # addresses the cases set
    expected: frozenset = frozenset()   # addresses the cases read back
    case_count: int = 0
    # The cases themselves, kept because the ADDRESSES alone cannot answer
    # every question. Comparing two cases to each other needs their values.
    cases: tuple[Case, ...] = ()
    # Whether each case's `given` is the WHOLE input. When it is not, a case is
    # a delta on the one before it, two cases that look alike were not run
    # alike, and any check that compares one case to another is unsound. It is
    # declared rather than guessed: a delta and a whole input are the same
    # shape, so nothing here can tell them apart by looking.
    independent: bool = False
    # Whether the cases are in the order they happened. ⚠ NOT the same
    # question as independence, and keeping one flag for both cost a real
    # capability: a shipped test log is both -- every entry carries the whole
    # input AND the entries are a timeline -- and one flag could only say one.
    # Independence decides whether two cases may be COMPARED; order decides
    # whether they may be REPLAYED through something that carries state.
    ordered: bool = False

    @property
    def present(self) -> bool:
        return bool(self.driven or self.expected)

    @property
    def valued(self) -> bool:
        """Do the cases carry values, or only the addresses they touch?

        A pack may withhold values -- the addresses alone are enough to say
        "the specification never names this signal" -- and then every check
        that needs a value has to say it could not run rather than read the
        withheld `None` as a value in its own right.
        """
        return any(v is not None
                   for c in self.cases
                   for v in list(c.given.values()) + list(c.expect.values()))


def load_examples(paths: list[pathlib.Path]) -> Examples:
    origin, driven, expected, count = "", set(), set(), 0
    cases: list[Case] = []
    independent = ordered = False
    for path in paths:
        doc = _read(path)
        _validate(doc, "examples.v1.schema.json", path)
        origin = doc.get("origin") or origin
        independent = independent or bool(doc.get("independent_cases"))
        ordered = ordered or bool(doc.get("ordered"))
        for case in doc["cases"]:
            count += 1
            given = dict(case.get("given") or {})
            expect = dict(case.get("expect") or {})
            driven |= set(given)
            expected |= set(expect)
            cases.append(Case(str(case.get("name") or ""), given, expect,
                              case.get("elapsed_ms"),
                              tuple(case.get("drove") or ()),
                              str(case.get("variant") or "")))
    return Examples(origin, frozenset(driven), frozenset(expected), count,
                    tuple(cases), independent, ordered)


@dataclass
class Pack:
    root: pathlib.Path
    model: Model
    conventions: Conventions
    examples: Examples = field(default_factory=Examples)


def _pick(root: pathlib.Path, candidates: tuple[str, ...], what: str) -> list[pathlib.Path]:
    found = [root / c for c in candidates if (root / c).is_file()]
    extra = sorted(root.glob(f"{what}.d/*.yaml")) + sorted(root.glob(f"{what}.d/*.json"))
    if not found and not extra:
        raise PackError(
            f"{root}: no {what} file. Expected one of {', '.join(candidates)}, "
            f"or a {what}.d/ directory of them."
        )
    return found + extra


EXAMPLE_FILES = ("examples.yaml", "examples.yml", "examples.json")


def load_pack(root: pathlib.Path) -> Pack:
    root = pathlib.Path(root)
    if not root.is_dir():
        raise PackError(f"{root}: not a directory")
    # Examples are optional: a specification being written for the first time
    # has none, and that is the normal case rather than a broken pack. What
    # the questions must never do is pretend the silence of an absent example
    # set is the silence of a clean one -- see `examples_are_absent`.
    example_paths = [root / c for c in EXAMPLE_FILES if (root / c).is_file()]
    return Pack(
        root=root,
        model=load_model(_pick(root, MODEL_FILES, "interface-model")),
        conventions=load_conventions(_pick(root, CONVENTION_FILES, "conventions")),
        examples=load_examples(example_paths) if example_paths else Examples(),
    )


def gate_off_value(conventions: Conventions, values: dict[str, int] | None) -> str | None:
    """The value an output takes when its precondition is false.

    The cascade is ordered and the first clause that applies wins. It is a
    cascade rather than a rule set because a generator has to choose one value,
    and a rule set that offers three candidates has answered nothing.
    """
    if not values:
        return None
    real = [s for s in values if s not in conventions.neutral_symbols]
    for clause in conventions.gate_off:
        when, use = clause["when"], clause["use"]
        matched = None
        if when == "has_symbol":
            if clause["symbol"] not in values:
                continue
            matched = clause["symbol"]
        elif when == "unique_suffix":
            hits = [s for s in values if s.endswith(clause["suffix"])]
            if len(hits) != 1:
                continue
            matched = hits[0]
        elif when == "binary":
            if len(real) != 2:
                continue
        elif when != "always":
            continue
        if use == "matched":
            return matched
        if use == "first":
            return real[0] if real else None
        if use == "last":
            return real[-1] if real else None
        return use
    return None
