"""Judging a written document against the model it claims to speak to.

Four refusals, and every one of them exists because its absence was once a
silent wrong answer rather than an error:

  an input bound to an address nothing declares      read a default forever
  an output bound to a field that does not exist     written nowhere
  a symbol outside the field's value space           a value the platform
                                                     cannot represent
  a document value with no binding at all            computed and dropped

A check that only warns about these would be worth little. A default that is
quietly wrong is worse than an answer that is loudly wrong, because the second
one gets fixed.

⚠ And a refusal here has to grow with the binding vocabulary. The kind guard
below was written when memory could only arrive through `state_of`, missed a
latched protocol the moment protocols could carry one, and then reported 16 of
16 documents until it read the `infrastructure` list the pack had already
declared. See `questions.py` for why this keeps happening and what to do about
it before adding anything here.
"""

from __future__ import annotations

import json
import pathlib
import re
import xml.etree.ElementTree as ET
import dataclasses
from dataclasses import dataclass

import yaml

from .errors import READ_ERRORS, PackError, describe_path
from .pack import Pack, _validate

SCXML_NS = "{http://www.w3.org/2005/07/scxml}"
SCE_NS = "{http://sce.dev/ext}"

# Kinds whose definition is an answer from this round's inputs alone. A
# document of one of these that needs memory has been named wrongly, and the
# only place that shows is the binding.
STATELESS_KINDS = frozenset({"transform", "lookup", "condition", "interpolation"})


@dataclass
class Document:
    """What an SCXML declares it receives and produces."""

    path: pathlib.Path
    inputs: tuple[str, ...]
    outputs: tuple[str, ...]
    kind: str
    # Output identifier -> what its author wrote down as an assumption. See
    # `read_document` for why this is carried at all.
    assumed: dict = dataclasses.field(default_factory=dict)
    # Output identifier -> the identifiers its expression mentions. ⚠ Needed
    # because an assumption is rarely on the value that lands at an address:
    # it is usually a step upstream, and attributing only to the direct writer
    # found none of them on the corpus that prompted this.
    reads: dict = dataclasses.field(default_factory=dict)

    def rests_on_an_assumption(self, ident: str) -> str:
        """The assumption this output depends on, transitively, if any."""
        seen, stack = set(), [ident]
        while stack:
            current = stack.pop()
            if current in seen:
                continue
            seen.add(current)
            if current in self.assumed:
                return self.assumed[current]
            stack.extend(self.reads.get(current, ()))
        return ""


def read_document(path: pathlib.Path) -> Document:
    # ⚠ A malformed document is an ANSWER, not a crash. This raised a
    # traceback the first time a document with a comment containing `--`
    # reached it, and a caller that gets a traceback learns only that
    # something went wrong somewhere.
    try:
        root = ET.parse(path).getroot()
    except ET.ParseError as exc:
        raise PackError(
            f"{path}: not well-formed XML ({exc}). A comment cannot contain "
            f"a double hyphen, which is the way this usually happens."
        ) from exc
    inputs, outputs = [], []
    assumed, reads = {}, {}
    for data in root.iter(f"{SCXML_NS}data"):
        ident = data.get("id")
        direction = data.get(f"{SCE_NS}direction")
        if direction == "in":
            inputs.append(ident)
        elif direction == "out":
            outputs.append(ident)
        # ⚠ The author's own record of what they had to decide without being
        # told. `sce:assumed` is the product's non-blocking marker -- the pair
        # of `sce:unresolved`, which refuses the build. An assumption compiles,
        # so nothing downstream ever mentioned it again, and a case that
        # refutes one used to read as "your document is wrong" rather than
        # "the thing you wrote down as a guess is the thing that failed".
        if data.get(f"{SCE_NS}assumed"):
            assumed[ident] = (data.get(f"{SCE_NS}assumed-reason")
                              or data.get(f"{SCE_NS}assumed"))
        if data.get("expr"):
            reads[ident] = frozenset(re.findall(r"[A-Za-z_][A-Za-z0-9_]*",
                                                data.get("expr")))
    return Document(
        path=path,
        inputs=tuple(inputs),
        outputs=tuple(outputs),
        kind=root.get(f"{SCE_NS}kind", ""),
        assumed=assumed,
        reads=reads,
    )


def read_binding(path: pathlib.Path) -> dict:
    # ⚠ Pointing `--binding` at a document that is not one is the ordinary
    # mistake here, and it used to arrive as a YAML parser traceback whose
    # last line was a caret under a column number.
    try:
        text = path.read_text(encoding="utf-8")
    except READ_ERRORS as exc:
        if not path.is_file():
            raise PackError(describe_path(path)) from exc
        raise PackError(f"{path}: cannot be read as text ({exc})") from exc
    try:
        doc = json.loads(text) if path.suffix == ".json" else yaml.safe_load(text)
    except (yaml.YAMLError, json.JSONDecodeError) as exc:
        first = str(exc).strip().splitlines()[0] if str(exc).strip() else exc
        raise PackError(f"{path}: not a well-formed binding ({first})") from exc
    _refuse_yaml_booleans(doc, path)
    _validate(doc, "binding.v1.schema.json", path)
    return doc


# What YAML 1.1 turns into booleans without being asked. The interface model's
# loader already refuses these as value-space KEYS; a binding meets them as
# values, in exactly the places a platform symbol belongs.
_YAML_BOOLEANS = ("ON", "OFF", "YES", "NO", "Y", "N", "TRUE", "FALSE")


def _refuse_yaml_booleans(doc, path: pathlib.Path) -> None:
    """Say what happened, rather than that a string was expected.

    ⚠ `equals: ON` and `map: {0: OFF}` both parse to booleans, and the schema
    then reports `True is not of type 'string'` -- true, and it names neither
    the cause nor the fix. Writing this rule cost six encounters with the same
    trap across two subject matters; nobody else should have to pay that.
    """
    def walk(node, trail):
        if isinstance(node, bool):
            where = " -> ".join(trail) or "(document root)"
            raise PackError(
                f"{path}: {where}: the value is the boolean {node!r}, and a "
                f"platform symbol was expected. YAML 1.1 reads a bare "
                f"{', '.join(_YAML_BOOLEANS[:4])} as a boolean -- quote it "
                f'("ON") so it stays the name the platform uses.')
        if isinstance(node, dict):
            for key, value in node.items():
                walk(value, [*trail, str(key)])
        elif isinstance(node, list):
            for index, value in enumerate(node):
                walk(value, [*trail, str(index)])

    for side in ("inputs", "outputs"):
        for name, rule in ((doc or {}).get(side) or {}).items():
            if not isinstance(rule, dict):
                continue
            for key, value in rule.items():
                # These four are genuinely booleans in the published schema.
                if key in {"number", "absent", "passthrough", "internal",
                           "clock"}:
                    continue
                walk(value, [side, name, key])


@dataclass
class Finding:
    where: str
    detail: str

    def __str__(self) -> str:
        return f"{self.where}: {self.detail}"


def check(pack: Pack, binding_path: pathlib.Path) -> list[Finding]:
    binding = read_binding(binding_path)
    document = read_document((binding_path.parent / binding["document"]).resolve())
    model, conv = pack.model, pack.conventions
    out: list[Finding] = []

    declared_inputs = dict(binding.get("inputs") or {})
    declared_outputs = dict(binding.get("outputs") or {})

    for name, rule in sorted(declared_inputs.items()):
        address = rule.get("address")
        if rule.get("protocol"):
            proto = conv.protocols.get(rule["protocol"]) if conv.protocols else None
            if proto is None:
                out.append(Finding(f"input {name}", f"protocol {rule['protocol']!r} is not declared by the pack"))
                continue
            for param in proto["parameters"]:
                addr = (rule.get("parameters") or {}).get(param)
                if addr is None:
                    out.append(Finding(f"input {name}", f"protocol {rule['protocol']} needs parameter {param!r}"))
                elif addr not in model.by_address:
                    out.append(Finding(f"input {name}", f"{param} names {addr!r}, which the interface model does not declare"))
            continue
        if rule.get("previous_of"):
            if rule["previous_of"] not in declared_inputs:
                out.append(Finding(f"input {name}", f"previous_of names {rule['previous_of']!r}, which is not an input"))
            continue
        if rule.get("state_of"):
            if rule["state_of"] not in declared_outputs:
                out.append(Finding(f"input {name}", f"state_of names {rule['state_of']!r}, which is not an output — it would read a default every round"))
            continue
        if address is None:
            out.append(Finding(f"input {name}", "no address and no protocol"))
            continue
        entry = model.by_address.get(address)
        if entry is None:
            out.append(Finding(f"input {name}", f"{address!r} is not in the interface model"))
            continue
        space = (entry.field("") or entry.fields[0]).values
        for key in ("equals", "not_equals"):
            sym = rule.get(key)
            if sym is not None and space is not None and sym not in space:
                out.append(Finding(f"input {name}", f"{address} does not admit {sym!r}; it admits " + ", ".join(sorted(space))))
        for sym in rule.get("equals_any") or ():
            if space is not None and sym not in space:
                out.append(Finding(f"input {name}", f"{address} does not admit {sym!r}"))

    for name, rule in sorted(declared_outputs.items()):
        if rule.get("internal"):
            continue
        address = rule.get("address")
        entry = model.by_address.get(address) if address else None
        if entry is None:
            out.append(Finding(f"output {name}", f"{address!r} is not in the interface model"))
            continue
        fld = entry.field(rule.get("field", ""))
        if fld is None:
            have = ", ".join(f.name or "(scalar)" for f in entry.fields)
            out.append(Finding(f"output {name}", f"{address} has no field {rule.get('field', '')!r}; it has {have}"))
            continue
        for value in (rule.get("map") or {}).values():
            if isinstance(value, str) and not fld.admits(value):
                out.append(Finding(f"output {name}", f"{address}.{fld.name} does not admit {value!r}"))
        for group in list((rule.get("when") or {}).values()) + [rule.get("also") or {}]:
            for fname, value in group.items():
                other = entry.field(fname)
                if other is None:
                    out.append(Finding(f"output {name}", f"{address} has no field {fname!r}"))
                elif isinstance(value, str) and not other.admits(value):
                    out.append(Finding(f"output {name}", f"{address}.{fname} does not admit {value!r}"))

    # ⚠ A document can declare itself one thing and be another, and neither
    # the document nor the platform model can tell. The binding can.
    #
    # A kind whose whole definition is "input, computation, output" is a
    # statement that this round's answer depends on nothing but this round's
    # inputs. Feeding an output back as an input breaks that -- and it breaks
    # it INVISIBLY, because the feedback lives in the binding and the document
    # stays syntactically pure. Measured over one corpus of 29 documents,
    # eleven declared themselves pure and were not; every one still worked,
    # because a driver was quietly remembering for them.
    #
    # What that costs: the document alone stops being the component. Anything
    # that reads it -- a reviewer, a generator, a second implementation -- is
    # told it needs no memory, and is wrong.
    # ⚠ Remembering a previous INPUT counts too. It is tempting to call that
    # edge detection and let it pass -- a specification writes `A then B` and
    # something has to see the `A`. But a kind that answers from this round's
    # inputs alone cannot see the previous round at all, whichever side of the
    # document the remembered value sits on. Counting only fed-back outputs
    # put the figure at 8 of 29; counting memory as memory puts it at 11.
    # ⚠ A LATCHED PROTOCOL is memory too, and the guard did not know it. A
    # latch answers by which parameter changed most recently, which is a fact
    # about the round before -- so a document declaring itself a pure
    # computation while its binding reads one is the same lie this check was
    # written for, arriving through a key that did not exist when it was.
    #
    # ⚠⚠ A protocol the pack NAMES and never DEFINES is not assumed either
    # way: the core has nothing to read, so it says it cannot tell rather than
    # calling the document honest or dishonest on a guess.
    #
    # `prev_output_of` used to be listed here and is now dropped: the binding
    # schema sets `additionalProperties: false`, so a binding carrying it is
    # refused before this line runs. It was checking for something that could
    # not arrive.
    protocols = conv.protocols or {}
    memory, undecidable = [], []
    for name, rule in sorted(declared_inputs.items()):
        if rule.get("state_of") or rule.get("previous_of"):
            memory.append(name)
        elif rule.get("protocol"):
            spec = protocols.get(rule["protocol"]) or {}
            if spec.get("latch"):
                # ⚠ Unless every address it latches over is one the pack
                # already calls INFRASTRUCTURE. A supply ladder is the
                # platform's own plumbing, handed to every document alike --
                # the author did not choose to remember it, and reporting it
                # says one thing once per document instead of once. The pack
                # declares which addresses those are and the core was not
                # reading the list, which is the same defect as `gate-off`
                # never reading `absence_tokens`.
                over = tuple((rule.get("parameters") or {}).values())
                plumbing = bool(over) and all(
                    a.startswith(conv.infrastructure) for a in over)
                if not plumbing:
                    memory.append(name)
            else:
                undecidable.append((name, rule["protocol"]))

    for name, protocol in undecidable:
        if document.kind in STATELESS_KINDS:
            out.append(
                Finding(
                    f"input {name}",
                    f"reads the protocol {protocol!r}, which the pack names "
                    f"and never defines, so whether this document needs "
                    f"memory cannot be decided. Give the protocol a "
                    f"definition, or say the kind is not a pure computation.",
                )
            )

    if memory and document.kind in STATELESS_KINDS:
        out.append(
            Finding(
                f"document {document.path.name}",
                f"declares kind {document.kind!r}, which answers from this "
                f"round's inputs alone, but the binding feeds its own output "
                f"back as {', '.join(memory)}. The component is this document "
                f"plus something that remembers, and the kind does not say so.",
            )
        )

    for ident in document.inputs:
        if ident not in declared_inputs:
            out.append(Finding(f"document {document.path.name}", f"input {ident!r} has no binding — it would be a default every round"))
    for ident in document.outputs:
        if ident not in declared_outputs:
            out.append(Finding(f"document {document.path.name}", f"output {ident!r} has no binding — it is computed and dropped"))
    for name in declared_outputs:
        if name not in document.outputs:
            out.append(Finding(f"output {name}", "the document does not compute it"))

    return out
