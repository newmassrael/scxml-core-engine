"""Loading a pack: the interface model and the conventions.

A pack is the only route by which a subject matter reaches this package. The
loader therefore does two things beyond parsing: it validates against the
published schema, and it refuses silently-empty input. A pack that loads to
nothing would make every later answer vacuously clean, which reads exactly like
success.
"""

from __future__ import annotations

import functools
import json
import pathlib
import re
from dataclasses import dataclass, field

import yaml

from .errors import READ_ERRORS, AuthoringError, PackError, describe_path
from .expression import LITERALS, ExpressionError
from .expression import names as expression_names
from .structured import RepeatedKey, read_json, read_yaml

try:
    import jsonschema
except ImportError:  # pragma: no cover - the caller is told, not worked around
    jsonschema = None

SCHEMA_DIR = pathlib.Path(__file__).resolve().parent.parent / "schema"

MODEL_FILES = ("interface-model.yaml", "interface-model.yml", "interface-model.json")
CONVENTION_FILES = ("conventions.yaml", "conventions.yml", "conventions.json")


class Problems:
    """Where a loader says what is wrong with a pack.

    ONE implementation serves two callers. The loader every command uses
    refuses at the first problem, in the sentence it always did. The pack check
    keeps going and lists every one, because a pack with three mistakes used to
    cost three round trips to learn about, and a person preparing a pack for a
    specification owner needs all three at once.

    A loader site never decides which it is serving: it states the problem and
    carries on with the value that cannot mislead a later site (nothing, where
    there is nothing to read). `skipped` is for what a check could not hold the
    pack to because something it depends on did not load: a list of problems
    that does not say so reads as a pack that passed everything else."""

    def __init__(self, collect: bool = False) -> None:
        self.collect = collect
        self.found: list[AuthoringError] = []
        self.skipped: list[str] = []

    def refuse(self, error: AuthoringError, cause: BaseException | None = None) -> None:
        """Raise `error` at once, or keep it and let the loader go on."""
        if self.collect:
            self.found.append(error)
        elif cause is None:
            raise error
        else:
            raise error from cause

    def skip(self, what: str) -> None:
        self.skipped.append(what)


#: The default every caller outside the pack check gets: the first problem raises.
FIRST_PROBLEM = Problems()


def _read(path: pathlib.Path, problems: Problems = FIRST_PROBLEM):
    """Read a pack file, or refuse in a sentence that names it. None when the
    refusal was kept and not raised, since there is then nothing to read.

    ⚠ Everything here used to travel to the caller as whatever the library
    raised. A pack whose model was binary reached the command line as a YAML
    scanner traceback whose last line was `in "<unicode string>", position 0`
    -- which does not name the file, let alone the pack.
    """
    try:
        text = path.read_text(encoding="utf-8")
    except READ_ERRORS as exc:
        if not path.is_file():
            problems.refuse(PackError(describe_path(path)), exc)
        else:
            problems.refuse(PackError(f"{path}: cannot be read as text ({exc})"), exc)
        return None
    try:
        return read_json(text) if path.suffix == ".json" else read_yaml(text)
    except RepeatedKey as exc:
        # ⚠ Refused, not resolved: the library keeps the last value and says
        # nothing, which is how a value space, an entry's role and a phrase's
        # reading each changed with no error (reproduced for both formats).
        problems.refuse(PackError(f"{path}: {exc}"), exc)
    except (yaml.YAMLError, json.JSONDecodeError) as exc:
        first = str(exc).strip().splitlines()[0] if str(exc).strip() else exc
        problems.refuse(PackError(f"{path}: not well-formed ({first})"), exc)
    return None


def _validate(doc, schema_name: str, path: pathlib.Path,
              error: type[AuthoringError] = PackError,
              problems: Problems = FIRST_PROBLEM) -> bool:
    """Refuse `doc` unless it validates against `schema_name`, as `error`.
    True when it does. Where the refusal is kept and not raised, EVERY place the
    document departs from the schema is kept, in the order the document has them,
    and the answer is False: a document that does not fit its schema cannot be
    read further, since what its parts mean is what the schema says they are.

    `error` because not every file this core validates is part of a pack: the
    owner's decision record is read against its own schema, and a refusal of
    it saying "pack" would send its reader to the wrong file."""
    if jsonschema is None:
        problems.refuse(error(
            f"jsonschema is not installed, so {path} cannot be validated. "
            "Refusing rather than loading an unchecked file."
        ))
        return False
    schema = json.loads((SCHEMA_DIR / schema_name).read_text(encoding="utf-8"))
    validator = jsonschema.Draft202012Validator(
        schema, format_checker=jsonschema.Draft202012Validator.FORMAT_CHECKER)
    errors = sorted(validator.iter_errors(doc), key=lambda e: list(e.path))
    for departure in errors:
        where = " -> ".join(str(p) for p in departure.path) or "(document root)"
        problems.refuse(error(f"{path}: {where}: {departure.message}"))
    return not errors


@dataclass(frozen=True)
class Field:
    """One writable position on an address, with the values it admits."""

    name: str
    values: dict[str, int] | None
    type: str | None
    # (minimum, maximum) the platform can carry, either None where unsaid.
    range: tuple | None = None

    def admits(self, symbol: str) -> bool:
        return self.values is None or symbol in self.values


#: The roles a document writes. `stored` is also READ (every role but `output` is), which is the
#: point of it: the platform keeps a value the component reads at start and writes back.
WRITTEN_ROLES = ("output", "stored")


@dataclass(frozen=True)
class Entry:
    address: str
    role: str
    names: tuple[str, ...]
    fields: tuple[Field, ...]
    note: str = ""
    #: The component moves this event slot to another event by first publishing the old one off.
    announces_old_off: bool = False

    @property
    def written(self) -> bool:
        """Whether a document writes this address: an `output`, or a `stored` value that is
        read AND written back."""
        return self.role in WRITTEN_ROLES

    def field(self, name: str) -> Field | None:
        for f in self.fields:
            if f.name == name:
                return f
        return None

    def read_space(self) -> dict[str, int] | None:
        """The value space an input rule reading this address compares against:
        the unnamed field's, else the first. None where the model declares none.

        ⚠ The one place that answers it. The binding check and the pack's rules
        both ask which symbols an address admits, and two answers would let a
        pack accept a rule the binding check refuses."""
        return (self.field("") or self.fields[0]).values


#: The keys of an input rule that name a symbol the address has to take.
RULE_SYMBOL_KEYS = ("equals", "not_equals", "becomes")


def rule_symbols(rule: dict) -> list[tuple[str, str]]:
    """Every `(key, symbol)` an input rule compares an address against, in the
    order written: the single-symbol keys, then each member of `equals_any`."""
    named = [(key, rule[key]) for key in RULE_SYMBOL_KEYS if rule.get(key) is not None]
    named += [("equals_any", symbol) for symbol in rule.get("equals_any") or ()]
    return named


@dataclass
class Model:
    """Every address a document may touch, indexed the three ways it is asked for."""

    entries: list[Entry] = field(default_factory=list)
    by_address: dict[str, Entry] = field(default_factory=dict)
    by_name: dict[str, list[Entry]] = field(default_factory=dict)

    def outputs(self) -> list[Entry]:
        """The entries a document writes: an `output`, and a `stored` value that is read and
        written back."""
        return [e for e in self.entries if e.written]

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

    def field_at(self, key: str) -> Field | None:
        """The field an `address` or `address.field` key names.

        ⚠ The named field is tried FIRST. Looking for the unnamed one first
        returned nothing for every address that has named fields, so the value
        space was never found and a document computing the right answer
        everywhere was reported as failing every case.

        ⚠⚠ And the split is tried at EVERY dot, not just the last one. A field
        name can itself contain a dot -- `LinkedSound.Type` is one field of an
        event record, not a field `Type` of an address ending in
        `LinkedSound` -- and splitting only at the last dot looked for a pair
        that does not exist. That single wrong split accounted for 121 of 259
        failures on one corpus: the document had produced `REPEAT_COUNT` where
        the record said `1`, which are the same value, and the report blamed
        the document.

        ⚠⚠⚠ The whole remainder after the owning address is tried before any
        shorter tail. `owning` walks UP to a declared ancestor, so the loop
        below, cutting at the last dot first, reached an entry through
        `<address>.Offset` and asked it for the field `Stat` -- discarding
        `Offset` -- and a record that declares both `Stat` and `Offset.Stat`
        answered the second key with the first's value space (measured on a
        real corpus: two enumerations of one record differed in their last
        number, and a position was judged against the wrong one).
        """
        parts = key.split(".")
        owner = self.owning(key)
        if owner is not None:
            rest = ".".join(parts[len(owner.address.split(".")):])
            exact = owner.field(rest) if rest else None
            if exact is not None:
                return exact
        for cut in range(len(parts) - 1, 0, -1):
            entry = self.owning(".".join(parts[:cut]))
            if entry is None:
                continue
            found = entry.field(".".join(parts[cut:]))
            if found is not None:
                return found
        entry = self.owning(key)
        if entry is not None:
            for found in entry.fields:
                if not found.name:
                    return found
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


def _check_symbols(where: str, values, problems: Problems = FIRST_PROBLEM) -> None:
    for symbol in values or ():
        if isinstance(symbol, str):
            continue
        problems.refuse(PackError(
            f"{where}: a value space is keyed by {symbol!r}, which is not a "
            f"symbol. YAML reads bare {_YAML_BOOLEANS} as booleans -- quote "
            f"them (\"ON\": 1) so they stay the names the platform uses."
        ))


def _range(spec: dict, where: str = "", problems: Problems = FIRST_PROBLEM) -> tuple | None:
    bounds = spec.get("range")
    if not bounds:
        return None
    low, high = bounds.get("minimum"), bounds.get("maximum")
    if low is not None and high is not None and low > high:
        problems.refuse(PackError(
            f"{where + ': ' if where else ''}range minimum {low} is above its maximum {high}"))
        return None
    return (low, high)


def _entry(raw: dict, where: str = "", problems: Problems = FIRST_PROBLEM) -> Entry:
    fields: list[Field] = []
    here = f"{where}: {raw['address']}" if where else ""
    if "fields" in raw:
        for fname, fspec in raw["fields"].items():
            fields.append(Field(fname, fspec.get("values"), fspec.get("type"),
                                _range(fspec, f"{here} field {fname!r}" if here else "",
                                       problems)))
    else:
        fields.append(Field("", raw.get("values"), raw.get("type"),
                            _range(raw, here, problems)))
    announces = bool(raw.get("announces_old_off"))
    if announces and (raw["role"] not in WRITTEN_ROLES or "Stat" not in raw.get("fields", {})):
        problems.refuse(PackError(
            f"{here + ': ' if here else ''}announces_old_off is said of an address that is "
            f"{'not written by a document' if raw['role'] not in WRITTEN_ROLES else 'without a Stat field'}"
            f": only an `output` or `stored` event record with a `Stat` has an old event to "
            f"publish off"))
    return Entry(
        address=raw["address"],
        role=raw["role"],
        names=tuple(raw.get("names") or ()),
        fields=tuple(fields),
        note=raw.get("note", ""),
        announces_old_off=announces,
    )


def load_model(paths: list[pathlib.Path], problems: Problems = FIRST_PROBLEM) -> Model:
    """Merge any number of interface-model files.

    Several files because a platform's model is usually published in pieces and
    joining them by hand is how a piece goes missing. Merging is a union; a
    repeated address is an error rather than a last-one-wins, because the two
    copies would disagree about a value space and nothing would say so.
    """
    model = Model()
    before = len(problems.found)
    for path in paths:
        doc = _read(path, problems)
        if doc is None or not _validate(doc, "interface-model.v1.schema.json", path,
                                        problems=problems):
            continue
        for raw in doc["entries"]:
            entry = _entry(raw, str(path), problems)
            for fld in entry.fields:
                _check_symbols(f"{path}: {entry.address}", fld.values, problems)
            if entry.address in model.by_address:
                problems.refuse(PackError(
                    f"{path}: address {entry.address!r} is already declared. "
                    "Two declarations of one address cannot both be believed."
                ))
                continue
            model.by_address[entry.address] = entry
            model.entries.append(entry)
            for name in entry.names:
                model.by_name.setdefault(name, []).append(entry)
    # ⚠ Said only when nothing else was: a model that is empty because its file
    # did not load is that file's problem, and listing it twice makes a pack
    # with one mistake look like one with two.
    if not model.entries and len(problems.found) == before:
        problems.refuse(PackError(
            "the interface model is empty. An empty model answers every "
            "question cleanly, which is indistinguishable from a correct one."
        ))
    return model


@dataclass(frozen=True)
class NameClass:
    pattern: re.Pattern
    role: str


@dataclass(frozen=True)
class HeldRule:
    """Fields of an output that stay described while its gate is off (`held_when_off`).

    The pack's CLAIM about the platform, read and said, never checked: the core knows
    no platform, and a field it was told stays described is one it cannot test.
    """

    pattern: re.Pattern
    gate: str
    fields: tuple[str, ...]
    measured: str = ""


@dataclass(frozen=True)
class CompanionRule:
    """A field that holds one symbol of its own value space while a companion field of
    the same output is in use (`companion_symbol`).

    The pack's CLAIM about the platform, read and said. What the core does check is the
    rule against the interface model, because a symbol a value space does not admit is one
    no document could write; it checks no document against the claim.
    """

    pattern: re.Pattern
    field: str
    companion: str
    symbol: str
    companion_off: str
    measured: str = ""


@dataclass(frozen=True)
class DefaultRule:
    """Fields of an output that hold a value unless the specification states another
    (`field_defaults`).

    The pack's CLAIM about the platform, read and said. What the core does check is the
    rule against the interface model, because a value a field does not admit is one no
    document could write; it checks no document against the claim.
    """

    pattern: re.Pattern
    fields: tuple[tuple[str, object], ...]
    measured: str = ""

    def value_of(self, field_name: str):
        for name, value in self.fields:
            if name == field_name:
                return value
        return None


@dataclass
class Conventions:
    name_classes: list[NameClass]
    precondition_inputs: dict[str, str]
    precondition_phrases: dict[str, str]
    # Phrases whose reading is an ASSUMPTION about the platform, to the reason
    # the pack gives. ⚠ Kept beside the phrase rather than in a comment next
    # to it: a reason that lives only in the pack's source is lost at the
    # first tool that copies the table, and after that the assumption reads
    # as a fact to every report downstream.
    precondition_assumed: dict[str, str]
    # Where THIS kind of document writes a precondition, with a named group
    # `phrase`. None means the core cannot find them, and the questions that
    # need to are declined out loud rather than answered empty.
    precondition_pattern: re.Pattern | None
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
    # Precondition input name -> the binding rule that reads it on this
    # platform, for the inputs whose pack says so. What `check` holds a
    # binding to; an input with only a description is known by name alone.
    precondition_rules: dict[str, dict] = field(default_factory=dict)
    # What the host does with the document's outputs (`host.writes`), or
    # empty when the pack does not say. See the schema.
    host: dict = field(default_factory=dict)
    # Fields that stay described while an output's gate is off, as the pack says it.
    held_when_off: tuple[HeldRule, ...] = ()
    # A symbol a field holds while its companion is in use, as the pack says it.
    companion_symbol: tuple[CompanionRule, ...] = ()
    # The value a field holds unless the specification says another, as the pack says it.
    field_defaults: tuple[DefaultRule, ...] = ()

    def default_rule(self, address: str, field_name: str) -> DefaultRule | None:
        """The rule that gives `field_name` of the output at `address` a default value, or
        None. The first rule that names the field decides."""
        for rule in self.field_defaults:
            if rule.pattern.search(address) and rule.value_of(field_name) is not None:
                return rule
        return None

    def held_rule(self, address: str, field_name: str) -> HeldRule | None:
        """The rule that says `field_name` of the output at `address` stays described
        while its gate is off, or None. The first rule that names it decides."""
        for rule in self.held_when_off:
            if field_name in rule.fields and rule.pattern.search(address):
                return rule
        return None

    def companion_rule(self, address: str, field_name: str) -> CompanionRule | None:
        """The rule that says `field_name` of the output at `address` holds a symbol while
        its companion is in use, or None. The first rule that names the field decides."""
        for rule in self.companion_symbol:
            if rule.field == field_name and rule.pattern.search(address):
                return rule
        return None

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

    def normalise_phrase(self, phrase: str) -> str:
        """The spelling a phrase is looked up under in the table."""
        key = phrase.strip().lower()
        for frm, to in self.normalise:
            key = re.sub(frm, to, key)
        return key.strip()

    def phrase_expression(self, phrase: str) -> str | None:
        return self.precondition_phrases.get(self.normalise_phrase(phrase))

    def precondition_mentions(self, prose) -> dict[str, tuple[str, int, int]]:
        """Every precondition the prose writes where this kind of document
        writes one, by its table spelling: (file, first line, times written).

        Empty when the pack has no pattern for where preconditions are written;
        a caller that must not read that as "none" asks `precondition_pattern`.
        """
        seen: dict[str, list] = {}
        if self.precondition_pattern is None:
            return {}
        for src in prose.sources:
            for lineno, line in enumerate(src.text.splitlines(), 1):
                for match in self.precondition_pattern.finditer(line):
                    key = self.normalise_phrase(match.group("phrase"))
                    if not key:
                        continue
                    entry = seen.setdefault(key, [str(src.path), lineno, 0])
                    entry[2] += 1
        return {key: tuple(entry) for key, entry in seen.items()}

    def precondition_reads(self, expression: str) -> set[str]:
        """The precondition inputs an expression from the table names."""
        return set(expression_names(expression)) & set(self.precondition_inputs)


# What a binding rule says about how it READS is everything but these: what
# its author wrote beside it.
RULE_COMMENTARY = frozenset({"note", "assumed"})


def rule_text(rule: dict) -> str:
    """A binding rule as one line of YAML flow style, the way a binding writes it."""
    return yaml.safe_dump({k: v for k, v in rule.items() if k not in RULE_COMMENTARY},
                          default_flow_style=True, sort_keys=False, width=10_000).strip()


def reads_no_input(expression: str) -> bool:
    """Does this expression name nothing a case could drive?

    ⚠ Decided on the expression's own names rather than against the declared
    inputs. Asking "does it name a declared input" would also catch an
    expression naming an UNDECLARED one, which is a different defect with a
    different remedy (declare the input, or mend its spelling), and reporting
    it here would send its author to write a reason for a constant they never
    wrote. That one is refused on its own, in `load_conventions`.
    """
    return all(n.lower() in LITERALS for n in expression_names(expression))


def _compiled(path: pathlib.Path, where: str, pattern: str,
              problems: Problems = FIRST_PROBLEM) -> re.Pattern | None:
    """A regular expression the pack wrote, or a refusal that names it. None
    where the refusal was kept and not raised.

    ⚠ `re.error` used to leave here as itself: a pack with one malformed
    pattern reached the command line as a traceback out of the `re` module,
    naming neither the file nor the field."""
    try:
        return re.compile(pattern)
    except re.error as exc:
        problems.refuse(PackError(f"{path}: {where} {pattern!r} is not a regular "
                                  f"expression ({exc})"), exc)
        return None


def load_conventions(paths: list[pathlib.Path],
                     problems: Problems = FIRST_PROBLEM) -> Conventions:
    classes: list[NameClass] = []
    inputs: dict[str, str] = {}
    phrases: dict[str, str] = {}
    assumed: dict[str, str] = {}
    phrase_pattern = None
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
    host: dict = {}
    held: list[HeldRule] = []
    companions: list[CompanionRule] = []
    defaults: list[DefaultRule] = []
    rules: dict[str, dict] = {}
    phrase_files: dict[str, pathlib.Path] = {}

    for path in paths:
        doc = _read(path, problems)
        if doc is None or not _validate(doc, "conventions.v1.schema.json", path,
                                        problems=problems):
            continue
        for nc in doc["name_classes"]:
            pattern = _compiled(path, "name_classes pattern", nc["pattern"], problems)
            if pattern is not None:
                classes.append(NameClass(pattern, nc["role"]))
        pre = doc.get("preconditions") or {}
        for name, described in (pre.get("inputs") or {}).items():
            if isinstance(described, dict):
                inputs[name] = described["note"]
                rules[name] = described["rule"]
            else:
                inputs[name] = described
                # A later file that only describes the input withdraws the
                # rule an earlier one gave for it.
                rules.pop(name, None)
        spelt: dict[str, str] = {}
        for raw, reading in (pre.get("phrases") or {}).items():
            key = raw.strip().lower()
            # ⚠ Two spellings of one phrase in ONE file are one key once case
            # and spacing are set aside, and the table keeps the later: the
            # same silent last-wins as a key written twice, one step past what
            # the file reader can see. Across files a later file replacing an
            # earlier reading is the layering this loader documents.
            if key in spelt:
                problems.refuse(PackError(
                    f"{path}: preconditions.phrases {spelt[key]!r} and {raw!r} are "
                    f"one phrase once case and spacing are set aside, and the "
                    f"table would keep only the later reading"))
                continue
            spelt[key] = raw
            expression = reading["expression"] if isinstance(reading, dict) else reading
            try:
                expression_names(expression)
            except ExpressionError as exc:
                problems.refuse(
                    PackError(f"{path}: preconditions.phrases {raw!r}: {exc}"), exc)
                continue
            phrase_files[key] = path
            if isinstance(reading, dict):
                phrases[key] = reading["expression"]
                assumed[key] = reading["assumed"]
            else:
                phrases[key] = reading
                # A later file that reads the phrase plainly withdraws the
                # assumption an earlier one recorded for it.
                assumed.pop(key, None)
            # ⚠ Refused at the pack, where it is written once, rather than
            # reported downstream, where it would be read hundreds of times
            # and believed. A phrase read as a constant is a condition the
            # prose states and nothing will ever test: every case passes the
            # same way whether the reading is right or not. The object form
            # costs one line and makes every report that reaches the phrase
            # say why it was allowed.
            if reads_no_input(phrases[key]) and key not in assumed:
                problems.refuse(PackError(
                    f"{path}: preconditions.phrases {raw!r} reads as "
                    f"{phrases[key]!r}, which names no input -- the condition "
                    f"the prose states is removed from everything a case can "
                    f"exercise, so a pass says nothing about it. Write it as "
                    f"{{expression: {phrases[key]!r}, assumed: \"<why this "
                    f"holds on this platform>\"}} so the reason travels with "
                    f"it and every report that reaches the phrase can name it."
                ))
        if pre.get("pattern"):
            compiled = _compiled(path, "preconditions.pattern", pre["pattern"], problems)
            if compiled is not None and "phrase" not in compiled.groupindex:
                problems.refuse(PackError(
                    f"{path}: preconditions.pattern needs the named group "
                    f"`phrase` -- without it the core cannot say which part "
                    f"of a match is the precondition to look up"
                ))
                compiled = None
            # A pattern that was refused is not kept: it would replace a good
            # one an earlier file gave, with one that cannot be used.
            if compiled is not None:
                phrase_pattern = compiled
        for n in pre.get("normalise") or []:
            from_pattern = _compiled(path, "preconditions.normalise from", n["from"], problems)
            if from_pattern is None:
                continue
            # ⚠ A replacement that names a group the pattern lacks (or is not a
            # replacement at all) fails when the first phrase it matches is
            # rewritten, in the middle of reading a specification, as an
            # `IndexError` out of `re`. Python parses the template before it looks
            # for a match, so an empty subject is enough to find that out here.
            try:
                from_pattern.sub(n["to"], "")
            except (re.error, IndexError) as exc:
                problems.refuse(PackError(
                    f"{path}: preconditions.normalise to {n['to']!r} is not a replacement "
                    f"for {n['from']!r} ({exc})"), exc)
                continue
            normalise.append((n["from"], n["to"]))
        gate_off += doc.get("gate_off") or []
        gate_off_note = doc.get("gate_off_note") or gate_off_note
        neutral += doc.get("neutral_symbols") or []
        absence += doc.get("absence_tokens") or []
        plumbing += doc.get("infrastructure") or []
        time_inputs += doc.get("time_inputs") or []
        protocols.update(doc.get("protocols") or {})
        host.update(doc.get("host") or {})
        for number, held_rule in enumerate(doc.get("held_when_off") or [], start=1):
            where = f"held_when_off[{number}]"
            # ⚠ A rule that holds its own gate says the field that switches the output
            # off stays described while it is off, which is not a sentence about anything.
            if held_rule["gate"] in held_rule["fields"]:
                problems.refuse(PackError(
                    f"{path}: {where} names its own gate {held_rule['gate']!r} among the "
                    f"fields that stay described -- the gate is what switches the output "
                    f"off, so it cannot be one of them"))
                continue
            compiled = _compiled(path, f"{where}.address_pattern",
                                 held_rule["address_pattern"], problems)
            if compiled is not None:
                held.append(HeldRule(compiled, held_rule["gate"],
                                     tuple(held_rule["fields"]),
                                     held_rule.get("measured") or ""))
        for number, companion in enumerate(doc.get("companion_symbol") or [], start=1):
            where = f"companion_symbol[{number}]"
            # ⚠ A field cannot be its own companion: "holds S while it holds anything but
            # OFF" says nothing about when, and the rule would read as a sentence anyway.
            if companion["field"] == companion["companion"]:
                problems.refuse(PackError(
                    f"{path}: {where} names {companion['field']!r} as its own companion "
                    f"-- the companion is the OTHER field whose use switches the symbol on"))
                continue
            compiled = _compiled(path, f"{where}.address_pattern",
                                 companion["address_pattern"], problems)
            if compiled is not None:
                companions.append(CompanionRule(compiled, companion["field"],
                                                companion["companion"], companion["symbol"],
                                                companion["companion_off"],
                                                companion.get("measured") or ""))
        for number, default in enumerate(doc.get("field_defaults") or [], start=1):
            compiled = _compiled(path, f"field_defaults[{number}].address_pattern",
                                 default["address_pattern"], problems)
            if compiled is not None:
                defaults.append(DefaultRule(compiled, tuple(default["fields"].items()),
                                            default.get("measured") or ""))
        if doc.get("duration_pattern"):
            compiled = _compiled(path, "duration_pattern", doc["duration_pattern"], problems)
            if compiled is not None:
                duration = compiled
        if doc.get("comparison_pattern"):
            compiled = _compiled(path, "comparison_pattern", doc["comparison_pattern"],
                                 problems)
            missing = {"name", "op", "token"} - set(compiled.groupindex) if compiled else None
            if missing:
                problems.refuse(PackError(
                    f"{path}: comparison_pattern needs the named group(s) "
                    f"{', '.join(sorted(missing))} — without them the core "
                    f"cannot say what was compared against what"
                ))
            elif compiled is not None:
                comparison = compiled

    # ⚠ Held to the inputs once every file has said its part: a later file may
    # declare an input an earlier one's phrase reads. A name no file declares
    # reads NOTHING, so the condition the prose states is missing from every
    # check with nothing to say so; the likeliest cause is a misspelling, and
    # the declared inputs are named so that it can be seen.
    for key, expression in phrases.items():
        unknown = [n for n in dict.fromkeys(expression_names(expression))
                   if n.lower() not in LITERALS and n not in inputs]
        if unknown:
            problems.refuse(PackError(
                f"{phrase_files[key]}: preconditions.phrases {key!r} reads "
                f"{expression!r}, and {', '.join(map(repr, unknown))} "
                f"{'is' if len(unknown) == 1 else 'are'} not declared in "
                f"preconditions.inputs ({', '.join(sorted(inputs)) or 'none declared'}) "
                f"-- a name nobody declares reads nothing, so the condition would "
                f"be missing from every check"))

    return Conventions(
        name_classes=classes,
        precondition_inputs=inputs,
        precondition_phrases=phrases,
        precondition_assumed=assumed,
        precondition_pattern=phrase_pattern,
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
        precondition_rules=rules,
        host=host,
        held_when_off=tuple(held),
        companion_symbol=tuple(companions),
        field_defaults=tuple(defaults),
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
    # The steps that set the case up, driven in order BEFORE it and never
    # judged -- each one a case with nothing expected of it. ⚠ A case that
    # moves an input from one value to another has to say where it moved FROM,
    # and `given` holds only where it ended. Folding the setup into the final
    # values lost exactly that: a document reading "A becomes B" literally was
    # never shown the A, failed, and a document reading it loosely passed.
    before: tuple = ()
    # The same time as bounds: (lo, hi), with lo == hi when `elapsed_ms` is
    # exact, or None when the record said nothing. `elapsed_ms` stays the
    # exact value only -- a reading that needs THE number (a clock input)
    # must not be handed one end of a window as though it were the time.
    elapsed_window: tuple | None = None
    # How the record read its expectation in that window: "any" moment, or
    # the "first" announcement after the drive (the schema says why).
    observed: str = "any"
    # Which of `drove` the platform delivered as a change, or None where the
    # record does not say (the schema says why a record may know better).
    delivered: tuple | None = None

    def __post_init__(self):
        # One source of truth: an exact time IS a window of width zero, so a
        # Case built with `elapsed_ms` alone is never read as untimed.
        if self.elapsed_window is None and self.elapsed_ms is not None:
            object.__setattr__(self, "elapsed_window",
                               (float(self.elapsed_ms), float(self.elapsed_ms)))


def _elapsed(raw, where: str, problems: Problems = FIRST_PROBLEM) -> tuple:
    """A record's `elapsed_ms` as (exact value or None, window or None)."""
    if raw is None:
        return None, None
    if isinstance(raw, dict):
        lo, hi = float(raw["min"]), float(raw["max"])
        if lo > hi:
            problems.refuse(PackError(
                f"{where}: elapsed_ms window has min {lo} above max {hi}"))
            return None, None
        return (lo if lo == hi else None), (lo, hi)
    return float(raw), (float(raw), float(raw))


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


def _delivered(step: dict, where: str, problems: Problems = FIRST_PROBLEM) -> tuple | None:
    """The step's `delivered`, or None where the record does not say.

    Refused where it names an address the step did not drive: a delivery is
    of a drive, and one of something nobody drove is a record contradicting
    itself."""
    if "delivered" not in step:
        return None
    delivered = tuple(step["delivered"])
    stray = sorted(set(delivered) - set(step.get("drove") or ()))
    if stray:
        problems.refuse(PackError(f"{where}: `delivered` names {stray}, which the step did "
                                  f"not drive -- a delivery is of a drive"))
        return None
    return delivered


def load_examples(paths: list[pathlib.Path], problems: Problems = FIRST_PROBLEM) -> Examples:
    origin, driven, expected, count = "", set(), set(), 0
    cases: list[Case] = []
    independent = ordered = False
    for path in paths:
        doc = _read(path, problems)
        if doc is None or not _validate(doc, "examples.v1.schema.json", path,
                                        problems=problems):
            continue
        origin = doc.get("origin") or origin
        independent = independent or bool(doc.get("independent_cases"))
        ordered = ordered or bool(doc.get("ordered"))
        for case in doc["cases"]:
            count += 1
            name = str(case.get("name") or "")
            variant = str(case.get("variant") or "")
            given = dict(case.get("given") or {})
            expect = dict(case.get("expect") or {})
            driven |= set(given)
            expected |= set(expect)
            before = []
            for index, step in enumerate(case.get("before") or (), 1):
                step_given = dict(step.get("given") or {})
                driven |= set(step_given)
                step_name = f"{name} (before {index})"
                exact, window = _elapsed(step.get("elapsed_ms"), f"{path}: {step_name}",
                                         problems)
                delivered = _delivered(step, f"{path}: {step_name}", problems)
                repeat = step.get("repeat")
                if repeat is None:
                    before.append(Case(step_name, step_given, {}, exact,
                                       tuple(step.get("drove") or ()), variant,
                                       elapsed_window=window, delivered=delivered))
                    continue
                # A cyclic drive is as many rounds as the cycle ran, each observed one
                # period after its drive. A step's own `elapsed_ms` would contradict
                # that, so the two are refused together rather than one read silently.
                if step.get("elapsed_ms") is not None:
                    problems.refuse(PackError(
                        f"{path}: {step_name}: `repeat` and `elapsed_ms` both say when the "
                        f"step was observed -- a cyclic step is observed one period after "
                        f"each drive, so say only `repeat`"))
                    continue
                period = float(repeat["every_ms"])
                times = max(1, int(float(repeat["for_ms"]) // period))
                for turn in range(1, times + 1):
                    before.append(Case(f"{step_name}, repeat {turn} of {times}",
                                       dict(step_given), {}, period,
                                       tuple(step.get("drove") or ()), variant,
                                       elapsed_window=(period, period),
                                       delivered=delivered))
            exact, window = _elapsed(case.get("elapsed_ms"), f"{path}: {name}", problems)
            cases.append(Case(name, given, expect, exact,
                              tuple(case.get("drove") or ()),
                              variant, tuple(before), elapsed_window=window,
                              observed=case.get("observed") or "any",
                              delivered=_delivered(case, f"{path}: {name}", problems)))
    return Examples(origin, frozenset(driven), frozenset(expected), count,
                    tuple(cases), independent, ordered)


@dataclass
class Pack:
    root: pathlib.Path
    model: Model
    conventions: Conventions
    examples: Examples = field(default_factory=Examples)


def _pick(root: pathlib.Path, candidates: tuple[str, ...], what: str,
          problems: Problems = FIRST_PROBLEM) -> list[pathlib.Path]:
    found = [root / c for c in candidates if (root / c).is_file()]
    extra = sorted(root.glob(f"{what}.d/*.yaml")) + sorted(root.glob(f"{what}.d/*.json"))
    if not found and not extra:
        problems.refuse(PackError(
            f"{root}: no {what} file. Expected one of {', '.join(candidates)}, "
            f"or a {what}.d/ directory of them."
        ))
    return found + extra


EXAMPLE_FILES = ("examples.yaml", "examples.yml", "examples.json")


def _load(root: pathlib.Path, problems: Problems) -> Pack | None:
    """The pack at `root`, read once for both callers (`load_pack` refuses at the
    first problem, `check_pack` lists them all). None only where problems were
    kept and the pack could not be assembled from what loaded."""
    root = pathlib.Path(root)
    if not root.is_dir():
        problems.refuse(PackError(f"{root}: not a directory"))
        return None
    # Examples are optional: a specification being written for the first time
    # has none, and that is the normal case rather than a broken pack. What
    # the questions must never do is pretend the silence of an absent example
    # set is the silence of a clean one -- see `examples_are_absent`.
    example_paths = [root / c for c in EXAMPLE_FILES if (root / c).is_file()]
    # ⚠ The order is the order a refusal is met in: the loader that raises at the
    # first problem names whichever of these comes first, and a pack with two
    # mistakes should keep telling its author about the same one first.
    model_paths = _pick(root, MODEL_FILES, "interface-model", problems)
    before = len(problems.found)
    model = load_model(model_paths, problems) if model_paths else None
    model_clean = model is not None and len(problems.found) == before
    convention_paths = _pick(root, CONVENTION_FILES, "conventions", problems)
    conventions = load_conventions(convention_paths, problems) if convention_paths else None
    if model is not None and conventions is not None:
        if model_clean:
            _hold_rules_to_the_model(root, conventions, model, problems)
            _hold_companion_rules_to_the_model(root, conventions, model, problems)
            _hold_default_rules_to_the_model(root, conventions, model, problems)
        else:
            # ⚠ Not held to a model that did not load whole: an address it should
            # declare may be the one in the file that failed, and the check would
            # then accuse a rule of reading what the pack does declare.
            problems.skip("the rules in preconditions.inputs were not held to the "
                          "interface model, which did not load cleanly")
            if conventions.companion_symbol:
                problems.skip("the rules in companion_symbol were not held to the "
                              "interface model, which did not load cleanly")
            if conventions.field_defaults:
                problems.skip("the rules in field_defaults were not held to the "
                              "interface model, which did not load cleanly")
    examples = load_examples(example_paths, problems) if example_paths else Examples()
    if model is None or conventions is None:
        return None
    return Pack(root=root, model=model, conventions=conventions, examples=examples)


def load_pack(root: pathlib.Path) -> Pack:
    pack = _load(root, FIRST_PROBLEM)
    assert pack is not None  # the first problem raised; nothing was kept
    return pack


@dataclass
class PackReport:
    """Everything wrong with a pack at once, and what could not be checked.

    `problems` are the sentences `load_pack` would have refused with, one each,
    in the order the loader met them; `skipped` is what a check could not hold
    the pack to because something it depends on did not load. A pack is clean
    only when both are empty."""

    root: pathlib.Path
    problems: list[AuthoringError]
    skipped: list[str]

    @property
    def clean(self) -> bool:
        return not self.problems and not self.skipped


def check_pack(root: pathlib.Path) -> PackReport:
    """Read the pack at `root` the way every command does, and keep going."""
    problems = Problems(collect=True)
    _load(root, problems)
    return PackReport(pathlib.Path(root), problems.found, problems.skipped)


def _hold_companion_rules_to_the_model(root: pathlib.Path, conventions: Conventions,
                                       model: Model,
                                       problems: Problems = FIRST_PROBLEM) -> None:
    """Refuse a `companion_symbol` rule that names a symbol no document could write.

    The rule is a claim about the platform and is not checked against it. What the core can
    hold it to is the interface model: on every output the rule is about -- its pattern
    finds the address and the output has both fields -- the field's value space must admit
    the symbol it holds, and the companion's must admit the symbol that means it is not in
    use. A rule that fails that is not one a document could follow, and it loaded without a
    word.

    A rule about outputs the pack does not have, or whose output lacks one of the two
    fields, applies to nothing and is not refused: a pack's conventions may be written once
    for outputs only some of its models carry.
    """
    for number, rule in enumerate(conventions.companion_symbol, start=1):
        where = f"{root}: companion_symbol[{number}]"
        for entry in model.outputs():
            if not rule.pattern.search(entry.address):
                continue
            held, companion = entry.field(rule.field), entry.field(rule.companion)
            if held is None or companion is None:
                continue
            for owner, symbol, says in ((held, rule.symbol, "holds"),
                                        (companion, rule.companion_off, "is not in use at")):
                if owner.values is None:
                    problems.refuse(PackError(
                        f"{where} says the {owner.name!r} field {says} {symbol!r} on "
                        f"{entry.address!r}, which declares no value space for it -- there "
                        f"is no symbol there to name"))
                elif symbol not in owner.values:
                    problems.refuse(PackError(
                        f"{where} says the {owner.name!r} field {says} {symbol!r} on "
                        f"{entry.address!r}, which that field does not admit; it admits "
                        f"{', '.join(sorted(owner.values))}"))


def _hold_default_rules_to_the_model(root: pathlib.Path, conventions: Conventions,
                                     model: Model, problems: Problems = FIRST_PROBLEM) -> None:
    """Refuse a `field_defaults` rule that gives a field a value no document could write.

    The rule is a claim about the platform and is not checked against it. What the core can
    hold it to is the interface model: on every output the rule is about -- its pattern
    finds the address and the output has the field -- a field with a value space must admit
    the symbol, and a numeric field must be given a number inside its range. A default the
    field cannot carry is not one a document could follow.

    A rule about outputs the pack does not have, or whose output lacks the field, applies
    to nothing and is not refused: a pack's conventions may be written once for outputs
    only some of its models carry.
    """
    for number, rule in enumerate(conventions.field_defaults, start=1):
        where = f"{root}: field_defaults[{number}]"
        for entry in model.outputs():
            if not rule.pattern.search(entry.address):
                continue
            for name, value in rule.fields:
                fld = entry.field(name)
                if fld is None:
                    continue
                if fld.values is not None:
                    if str(value) not in fld.values:
                        problems.refuse(PackError(
                            f"{where} gives the {name!r} field {value!r} on {entry.address!r}, "
                            f"which that field does not admit; it admits "
                            f"{', '.join(sorted(fld.values))}"))
                elif isinstance(value, bool) or not isinstance(value, (int, float)):
                    if fld.type in ("integer", "number"):
                        problems.refuse(PackError(
                            f"{where} gives the {name!r} field {value!r} on {entry.address!r}, "
                            f"which is a number field"))
                elif fld.range is not None:
                    low, high = fld.range
                    if (low is not None and value < low) or (high is not None and value > high):
                        problems.refuse(PackError(
                            f"{where} gives the {name!r} field {value!r} on {entry.address!r}, "
                            f"outside the range {low}..{high} that field carries"))


def _hold_rules_to_the_model(root: pathlib.Path, conventions: Conventions,
                             model: Model, problems: Problems = FIRST_PROBLEM) -> None:
    """Refuse a precondition rule that reads what the pack does not declare.

    A rule is a binding input as the platform writes it, and `check` holds a
    document's binding to the same facts when it meets one: the rule says what
    the binding vocabulary can say (known keys, the right kinds of value), a
    protocol is one the pack declares, an address is one the model declares, and
    a symbol it compares against is one that address admits. A rule that fails
    them is not a rule `check` could hold a binding to -- the precondition would
    be compared with a reading that reads nothing here -- and it loaded without
    a word. (An `event`, `previous_of` or `state_of` rule names things of the
    binding, which a pack cannot see.)
    """
    for name, rule in sorted(conventions.precondition_rules.items()):
        where = f"{root}: preconditions.inputs {name!r} rule"
        # ⚠ The shape first, from the binding schema: the one place that says what
        # an input rule may contain. A key it does not know (`equalss`) read as no
        # condition at all until a binding copied it, and a `parameters` that was
        # a list reached the code below as an `AttributeError`. A rule of the wrong
        # shape is not held to anything further.
        shape = _input_rule_departures(rule)
        for departure in shape:
            problems.refuse(PackError(f"{where}: {departure}"))
        if shape:
            continue
        protocol = rule.get("protocol")
        if protocol:
            declared = conventions.protocols.get(protocol)
            if declared is None:
                problems.refuse(PackError(
                    f"{where} reads through protocol {protocol!r}, which "
                    f"the pack's `protocols` does not declare"))
                continue
            for parameter in declared.get("parameters") or ():
                address = (rule.get("parameters") or {}).get(parameter)
                if address is None:
                    problems.refuse(PackError(
                        f"{where} reads through protocol {protocol!r}, "
                        f"which needs parameter {parameter!r}"))
                elif address not in model.by_address:
                    problems.refuse(PackError(
                        f"{where} gives {parameter!r} the address "
                        f"{address!r}, which the interface model does "
                        f"not declare"))
            continue
        address = rule.get("address")
        if address and address not in model.by_address:
            problems.refuse(PackError(
                f"{where} reads address {address!r}, which the "
                f"interface model does not declare"))
            continue
        space = model.by_address[address].read_space() if address else None
        for key, symbol in rule_symbols(rule):
            if space is not None and symbol not in space:
                problems.refuse(PackError(
                    f"{where} compares {address!r} with {key} {symbol!r}, which that "
                    f"address does not admit; it admits {', '.join(sorted(space))}"))


@functools.lru_cache(maxsize=1)
def _input_rule_validator():
    """A validator for ONE input rule, built from the binding schema's own
    definition of it.

    `previous_of` and `state_of` need `caller_keeps` in a binding, and that key
    is commentary the BINDING supplies (`RULE_COMMENTARY` is what a rule leaves
    out when it is compared), so a pack stating the rule is not asked for it."""
    if jsonschema is None:
        return None
    binding = json.loads((SCHEMA_DIR / "binding.v1.schema.json").read_text(encoding="utf-8"))
    rule = {key: value for key, value in binding["$defs"]["input"].items()
            if key != "dependentRequired"}
    return jsonschema.Draft202012Validator(
        {"$schema": binding["$schema"], "$defs": binding["$defs"], **rule})


def _input_rule_departures(rule: dict) -> list[str]:
    """Where `rule` departs from what a binding's input rule may say, one
    sentence each, or nothing. Silent where `jsonschema` is missing: `_validate`
    has already refused to load any pack without it."""
    validator = _input_rule_validator()
    if validator is None:
        return []
    return [(" -> ".join(str(p) for p in error.path) + ": " if error.path else "")
            + error.message
            for error in sorted(validator.iter_errors(rule), key=lambda e: list(e.path))]


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
