"""A binding to start from, written from the interface model.

Half of a binding is not a decision. Which positions a component writes, what
each one is called, what values it admits -- the interface model already says
all of that, and a writer who copies it by hand is doing the one part of the
job a machine can do without being wrong. Measured 2026-09-25: of four models
set to write a document and its binding, the ones that failed failed on that
half -- a missing `version`, an output rule with no `field`, a `sent` without
the `when_nothing_sent` it depends on -- and never reached the part that reads
the specification.

So this writes that half and nothing else:

  - `version` and `document`;
  - one output rule per position the model declares, with its `address` and
    `field`, landing through `map: platform` -- the platform's symbols, each
    under its own name, read from the model rather than typed -- when the field
    is a closed set of symbols, and `passthrough` when it has no value space;
  - one input rule per address the model declares, with the value space
    beside it as a comment.

What it does NOT write is anything that is a reading of the specification:
which condition produces which value, which inputs the document needs, when
the host runs it. `activation` is a fact about the deployment: where the
pack's `host` states it, the binding takes it from there and is not given a
copy; otherwise it is written only when the caller says it, since a default
here would be a guess made on the deployment's behalf.

The result is always a binding `check` can read, so every remaining gap comes
back as a refusal naming the rule it is about, rather than as a file the
checker cannot open.

Asked for a `transform`, it also writes the document's skeleton: the root that
says it is a transform, in SCE's namespace, and one `<data>` per rule with the
same identifier, its direction and a type read off the model -- every output
WITHOUT an expression. The expressions are the reading of the specification,
so they are left out, and `check` names each output that still computes
nothing. Measured 2026-09-26: a model that had the binding skeleton still
wrote the document's shell wrong three rounds running -- no `sce:kind`, the
`sce` prefix bound to a made-up namespace, `<if>` directly inside a state --
and never reached the one condition the specification states. Whether a
component IS a transform is a reading too, so the document is written only
when the caller says so; a statechart's shell is its states and events, which
the model does not know.

A position the platform gives a closed set of symbols is declared an
ENUMERATION of the document's own, in a file of its own beside the document
(`<position>_values.scxml`), with the platform's symbols as its variants and the
document's own numbers, 0 upwards. The document names a value (`Lamp.ON`), the
binding lands it on the symbol of the same name, and the platform's number for
it appears in neither. ⚠ The skeleton used to declare such a position `int64`
and land it through a table keyed by the platform's numbers, and the documents
written from it computed those numbers: measured 2026-10-14 over one corpus,
nearly every document did, and none could be bound to another platform.
"""

from __future__ import annotations

import json
import pathlib
import re
from dataclasses import dataclass

from .errors import AuthoringError
from .landing import PLATFORM_MAP
from .pack import Entry, Field, Pack


class ScaffoldError(AuthoringError):
    """The skeleton cannot be written."""


ACTIVATIONS = ("on-change", "periodic")
KINDS = ("transform",)

SCE_NAMESPACE = "http://sce.dev/ext"

# The `sce:type` a document declares for a position of each model type that is not a closed
# set of symbols (that one is an enumeration of the document's own, `enum_alias`). `number`
# is a model that does not say whole or fraction, so the type that holds both is written; a
# model that knows says `integer`.
_SCE_TYPE = {"integer": "int64", "number": "float64",
             "text": "string", "boolean": "bool"}


def identifier(text: str) -> str:
    """A document identifier from a model name: letters and digits, starting
    with a letter, in lower camel case (`IN_TrainApproach` -> `inTrainApproach`)."""
    parts = [p for p in re.split(r"[^A-Za-z0-9]+", text) if p]
    if not parts:
        return "position"
    head = parts[0]
    # An all-capitals first part is a prefix or an acronym, lowered whole.
    head = head.lower() if head.isupper() else head[:1].lower() + head[1:]
    ident = head + "".join(p[:1].upper() + p[1:] for p in parts[1:])
    return ident if ident[0].isalpha() else "p" + ident


def _base(entry: Entry) -> str:
    # The model's first name is the platform's identifier for the address;
    # without one, the last segment of the address is what is left.
    if entry.names:
        return entry.names[0]
    segments = [s for s in re.split(r"[./]", entry.address) if s]
    return segments[-1] if segments else entry.address


def _unique(name: str, taken: set) -> str:
    candidate, n = name, 2
    while candidate in taken:
        candidate, n = f"{name}{n}", n + 1
    taken.add(candidate)
    return candidate


def _values_comment(fld: Field) -> str:
    if fld.values:
        return ", ".join(f"{s}={n}" for s, n in sorted(fld.values.items(), key=lambda kv: kv[1]))
    return fld.type or "no declared value space"


def _scalar(value) -> str:
    # JSON is YAML, and it quotes exactly what YAML would otherwise misread
    # (`ON`, `OFF`, `yes`, a leading zero).
    return json.dumps(value, ensure_ascii=False)


@dataclass(frozen=True)
class Position:
    """One rule the skeleton writes: its identifier and what the model says of it."""

    name: str
    entry: Entry
    field: Field


def positions(pack: Pack) -> tuple[list[Position], list[Position]]:
    """Every input and output position, named once for binding and document alike.

    One naming for both files is what makes the pair agree: a binding rule and
    the document variable it hands to have the same identifier by construction.
    """
    taken: set = set()
    sides = ([e for e in pack.model.entries if e.role != "output"], pack.model.outputs())
    named: tuple[list[Position], list[Position]] = ([], [])
    for side, entries in zip(named, sides):
        for entry in entries:
            base = identifier(_base(entry))
            for fld in entry.fields:
                name = _unique(base if len(entry.fields) == 1 else
                               identifier(f"{base} {fld.name}"), taken)
                side.append(Position(name, entry, fld))
    return named


def enum_alias(pos: Position) -> str:
    """The alias a document imports a position's enumeration under: the position's
    identifier, capitalised (`lampStat` -> `LampStat`)."""
    return pos.name[:1].upper() + pos.name[1:]


def enum_stem(pos: Position) -> str:
    """The stem of the file a position's enumeration is written to, which is also the
    enumeration's name (`lampStat` -> `lamp_stat_values`). ⚠ No dot: a document's identity is
    its file stem, and the generator cannot give a dotted one back."""
    return re.sub(r"(?<=[a-z0-9])(?=[A-Z])", "_", pos.name).lower() + "_values"


def enum_file(pos: Position) -> str:
    """The file a position's enumeration is written to, beside the document."""
    return f"{enum_stem(pos)}.scxml"


def draft_enum(pos: Position) -> str:
    """The enumeration of the document's own for a position the platform gives a closed set.

    The platform's symbols are its variants, in the platform's order, and the numbers are the
    DOCUMENT's: 0 upwards, whatever the platform numbers them. Nothing reads them across the
    boundary -- a value crosses by its name -- so they are free to be the document's own.
    """
    symbols = [s for s, _n in sorted(pos.field.values.items(), key=lambda kv: kv[1])]
    underlying = "uint8" if len(symbols) <= 256 else "uint16"
    name = enum_stem(pos)
    lines = [
        '<?xml version="1.0" encoding="UTF-8"?>',
        f"<!-- The values {pos.entry.address}{'.' + pos.field.name if pos.field.name else ''}",
        "     takes, as this document names them. The names are the platform's symbols;",
        "     the numbers are the document's own and never cross the boundary. -->",
        f'<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="{SCE_NAMESPACE}"',
        f'       version="1.0" sce:kind="enum" name={_xml_attr(name)} '
        f'sce:underlying-type="{underlying}">',
        "  <datamodel>",
        '    <data id="variants">',
    ]
    lines += [f'      <sce:variant name={_xml_attr(symbol)} value="{number}"/>'
              for number, symbol in enumerate(symbols)]
    lines += ["    </data>", "  </datamodel>", "</scxml>"]
    return "\n".join(lines) + "\n"


def draft(pack: Pack, document: str, activation: str | None = None) -> str:
    """The skeleton as YAML text. Deterministic: the model's order, always."""
    if activation is not None and activation not in ACTIVATIONS:
        raise ScaffoldError(f"activation {activation!r}: the schema admits "
                            f"{' or '.join(ACTIVATIONS)}")
    lines = [
        "# A binding skeleton written from the interface model by `scaffold`.",
        "# The addresses, fields and value spaces below are the model's; what is",
        "# left is what the specification decides: which condition gives which",
        "# value, and which inputs the document reads. Rename each rule to the",
        "# identifier your document uses, delete what it does not use, and run",
        "# `check` -- it names every rule that still does not fit the document.",
        "version: 1",
        f"document: {_scalar(document)}",
    ]
    platform = (pack.conventions.host or {}).get("activation")
    if activation and platform and activation != platform:
        raise ScaffoldError(f"activation {activation!r}: the pack's `host` says "
                            f"this platform runs a document {platform!r}, and "
                            f"`check` refuses a binding that says otherwise")
    if platform:
        # Not copied: a binding that leaves it out takes the platform's, so
        # there is one place it is written and nothing to drift from it.
        lines += [
            f"# activation: {platform} -- the pack's `host` says it for the whole",
            "# platform, and this binding takes it from there.",
        ]
    elif activation:
        lines.append(f"activation: {activation}")
    else:
        lines += [
            "# activation: when the host runs the document -- `on-change` (once each",
            "# time an input changes) or `periodic`. A fact about the deployment, so",
            "# it is left for whoever knows it.",
        ]

    inputs, outputs = positions(pack)

    lines.append("")
    lines.append("inputs:" if inputs else "inputs: {}")
    for pos in inputs:
        lines.append(f"  # {pos.entry.role}: {_values_comment(pos.field)}")
        if pos.field.values:
            # A closed set crosses by name: compared here, or handed to an enumeration of
            # the document's own. Never as the platform's number.
            lines.append("  #   a closed set: compare it (`equals: <symbol>`), or leave the rule as it")
            lines.append(f"  #   is for a document input declared `enum:{enum_alias(pos)}` -- it is handed")
            lines.append("  #   the variant of the same name, never the platform's number.")
        lines.append(f"  {pos.name}:")
        lines.append(f"    address: {_scalar(pos.entry.address)}")
        if pos.field.name:
            lines.append(f"    field: {_scalar(pos.field.name)}")

    lines.append("")
    lines.append("outputs:" if outputs else "outputs: {}")
    for pos in outputs:
        fld = pos.field
        lines.append(f"  # {_values_comment(fld)}")
        lines.append(f"  {pos.name}:")
        lines.append(f"    address: {_scalar(pos.entry.address)}")
        if fld.name:
            lines.append(f"    field: {_scalar(fld.name)}")
        if fld.values:
            # The platform's symbols under their own names, read from the model: the
            # document's enumeration (`enum:<alias>`) lands each variant on the symbol of
            # the same name, and nothing is typed that could be copied wrongly. A document
            # that names its values otherwise replaces this line with the table it needs,
            # keyed by its variants.
            lines.append(f"    map: {PLATFORM_MAP}")
        else:
            lines.append("    passthrough: true")
    return "\n".join(lines) + "\n"


def _sce_type(pos: Position) -> str:
    fld = pos.field
    if fld.values:
        # A closed set of symbols: an enumeration of the document's own (`draft_enum`).
        return f"enum:{enum_alias(pos)}"
    if fld.type not in _SCE_TYPE:
        raise ScaffoldError(
            f"field {fld.name or '(the address)'}: the model gives it "
            f"{'no type' if fld.type is None else f'type {fld.type!r}'}, and a document "
            f"variable needs one of {', '.join(sorted(_SCE_TYPE))} or a closed set of symbols")
    return _SCE_TYPE[fld.type]


def draft_transform(pack: Pack, name: str) -> str:
    """A transform document's skeleton, as XML text, matching `draft`'s names.

    Every output is declared and none is computed: its `expr` is the
    specification's, and `check` reports each one still missing.
    """
    inputs, outputs = positions(pack)
    lines = [
        '<?xml version="1.0" encoding="UTF-8"?>',
        "<!-- A transform skeleton written from the interface model by `scaffold`.",
        "     Its identifiers are the binding skeleton's. Each output still needs",
        "     `expr`: its value computed from the inputs, which is what the",
        "     specification says. Delete the inputs the document does not read,",
        "     and the output rules the binding does not bind. A closed set of symbols",
        "     is an enumeration of the document's own (the imports below): write a",
        "     value by its name, `<Alias>.<SYMBOL>`, never by a number. -->",
        f'<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="{SCE_NAMESPACE}"',
        f'       version="1.0" sce:kind="transform" name={_xml_attr(name)}>',
    ]
    lines += [f'  <sce:import as="{enum_alias(pos)}" src="{enum_file(pos)}" kind="enum"/>'
              for pos in (*inputs, *outputs) if pos.field.values]
    lines.append("  <datamodel>")
    for direction, side in (("in", inputs), ("out", outputs)):
        for pos in side:
            lines.append(f"    <!-- {pos.entry.address}"
                         f"{'.' + pos.field.name if pos.field.name else ''}: "
                         f"{_values_comment(pos.field)} -->")
            lines.append(f'    <data id="{pos.name}" sce:type="{_sce_type(pos)}" '
                         f'sce:direction="{direction}"/>')
    lines += ["  </datamodel>", "</scxml>"]
    return "\n".join(lines) + "\n"


def _xml_attr(value: str) -> str:
    return '"' + (value.replace("&", "&amp;").replace('"', "&quot;")
                  .replace("<", "&lt;")) + '"'


def write(pack: Pack, document: str, out: pathlib.Path,
          activation: str | None = None, kind: str | None = None) -> str:
    """Write the binding skeleton to `out`, which must not exist yet.

    With `kind="transform"`, the document the binding names is written too,
    beside it, with the enumeration of each position that is a closed set
    (`draft_enum`), and none of them may exist either. Nothing is written
    unless all can be.
    """
    if kind is not None and kind not in KINDS:
        raise ScaffoldError(f"kind {kind!r}: a skeleton is written only for "
                            f"{' or '.join(KINDS)} -- a statechart's shell is its "
                            f"states and events, which the model does not know")
    if out.exists():
        raise ScaffoldError(f"{out}: a binding is already there, and a skeleton "
                            f"written over it would throw away whatever it decides")
    text = draft(pack, document, activation)
    doc_path = doc_text = None
    enums: list[tuple[pathlib.Path, str]] = []
    if kind == "transform":
        doc_path = (out.parent / document)
        if doc_path.exists():
            raise ScaffoldError(f"{doc_path}: a document is already there, and a "
                                f"skeleton written over it would throw it away")
        doc_text = draft_transform(pack, pathlib.PurePath(document).stem)
        inputs, outputs = positions(pack)
        enums = [(doc_path.parent / enum_file(pos), draft_enum(pos))
                 for pos in (*inputs, *outputs) if pos.field.values]
        for path, _text in enums:
            if path.exists():
                raise ScaffoldError(f"{path}: an enumeration is already there, and a "
                                    f"skeleton written over it would throw it away")
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(text, encoding="utf-8")
    if doc_path is not None:
        doc_path.parent.mkdir(parents=True, exist_ok=True)
        doc_path.write_text(doc_text, encoding="utf-8")
        text += f"\n# and {document}:\n" + doc_text
        for path, enum_text in enums:
            path.write_text(enum_text, encoding="utf-8")
            text += f"\n# and {path.name}:\n" + enum_text
    return text
