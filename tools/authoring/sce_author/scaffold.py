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
    `field`, landing through a `map` keyed by the platform's own numbers when
    the field has a value space, and `passthrough` when it has none;
  - one input rule per address the model declares, with the value space
    beside it as a comment.

What it does NOT write is anything that is a reading of the specification:
which condition produces which value, which inputs the document needs, when
the host runs it. `activation` is written only when the caller says it -- it
is a fact about the deployment, and a default here would be a guess made on
the deployment's behalf.

The result is always a binding `check` can read, so every remaining gap comes
back as a refusal naming the rule it is about, rather than as a file the
checker cannot open.
"""

from __future__ import annotations

import json
import pathlib
import re

from .errors import AuthoringError
from .pack import Entry, Field, Pack


class ScaffoldError(AuthoringError):
    """The skeleton cannot be written."""


ACTIVATIONS = ("on-change", "periodic")


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


def draft(pack: Pack, document: str, activation: str | None = None) -> str:
    """The skeleton as YAML text. Deterministic: the model's order, always."""
    if activation is not None and activation not in ACTIVATIONS:
        raise ScaffoldError(f"activation {activation!r}: the schema admits "
                            f"{' or '.join(ACTIVATIONS)}")
    model = pack.model
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
    if activation:
        lines.append(f"activation: {activation}")
    else:
        lines += [
            "# activation: when the host runs the document -- `on-change` (once each",
            "# time an input changes) or `periodic`. A fact about the deployment, so",
            "# it is left for whoever knows it.",
        ]

    taken: set = set()
    inputs = [e for e in model.entries if e.role != "output"]
    outputs = model.outputs()

    lines.append("")
    lines.append("inputs:" if inputs else "inputs: {}")
    for entry in inputs:
        base = identifier(_base(entry))
        for fld in entry.fields:
            name = _unique(base if len(entry.fields) == 1 else
                           identifier(f"{base} {fld.name}"), taken)
            lines.append(f"  # {entry.role}: {_values_comment(fld)}")
            lines.append(f"  {name}:")
            lines.append(f"    address: {_scalar(entry.address)}")
            if fld.name:
                lines.append(f"    field: {_scalar(fld.name)}")

    lines.append("")
    lines.append("outputs:" if outputs else "outputs: {}")
    for entry in outputs:
        base = identifier(_base(entry))
        for fld in entry.fields:
            name = _unique(base if len(entry.fields) == 1 else
                           identifier(f"{base} {fld.name}"), taken)
            lines.append(f"  # {_values_comment(fld)}")
            lines.append(f"  {name}:")
            lines.append(f"    address: {_scalar(entry.address)}")
            if fld.name:
                lines.append(f"    field: {_scalar(fld.name)}")
            if fld.values:
                # Keyed by the platform's own number: a document that computes
                # the platform's code lands it unchanged. Re-key it to whatever
                # the document produces.
                pairs = ", ".join(f"{n}: {_scalar(s)}" for s, n in
                                  sorted(fld.values.items(), key=lambda kv: kv[1]))
                lines.append(f"    map: {{{pairs}}}")
            else:
                lines.append("    passthrough: true")
    return "\n".join(lines) + "\n"


def write(pack: Pack, document: str, out: pathlib.Path,
          activation: str | None = None) -> str:
    """Write the skeleton to `out`, which must not exist yet."""
    if out.exists():
        raise ScaffoldError(f"{out}: a binding is already there, and a skeleton "
                            f"written over it would throw away whatever it decides")
    text = draft(pack, document, activation)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(text, encoding="utf-8")
    return text
