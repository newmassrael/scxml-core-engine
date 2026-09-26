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

from . import delivery, landing
from .errors import READ_ERRORS, PackError, describe_path
from .pack import SCHEMA_DIR, Pack, _validate


def _keys_that_hold_no_symbol() -> frozenset:
    """Rule keys whose value is never a platform symbol, read from the schema.

    The YAML-boolean guard exists for SYMBOLS: `equals: ON` silently becomes
    True and stops being the name the platform uses. A key the schema types as
    a boolean, or marks `x-sce-document-value` -- it carries the DOCUMENT's
    own value, like `initial` for a boolean output -- holds a real boolean on
    purpose, and refusing it tells the author a symbol was expected where none
    was.

    ⚠ Read from the schema rather than listed here. The list used to be
    written by hand beside a comment saying "these four" while it held five;
    the sixth, `hold_last`, was refused as a bare YES the first time a binding
    used it, and then `initial: true` for a boolean output was refused the
    same way. A guard that has to be told about each new key fails exactly
    when a new key arrives.
    """
    schema = json.loads((SCHEMA_DIR / "binding.v1.schema.json").read_text(
        encoding="utf-8"))
    return frozenset(
        key
        for rule in (schema.get("$defs") or {}).values()
        for key, spec in (rule.get("properties") or {}).items()
        if isinstance(spec, dict)
        and (spec.get("type") == "boolean" or spec.get("x-sce-document-value")))

SCXML_NS = "{http://www.w3.org/2005/07/scxml}"
SCE_NS = "{http://sce.dev/ext}"

# Kinds whose binding may not hand them memory. A transform may KEEP values,
# and says so in the document itself with `previous(<field>)`; the other three
# keep none. Either way a binding that reads an earlier round is memory the
# document does not declare, and the only place that shows is the binding.
STATELESS_KINDS = frozenset({"transform", "lookup", "condition", "interpolation"})

# Kinds that are DRIVEN by events rather than called with their inputs (W3C
# SCXML 3.12). `verify` drives these, and the rules a binding for one may use
# are judged here, once, for both (`driving_refusals`).
STATECHART_KINDS = frozenset({"statechart"})

# What `verify`'s statechart driver reads off an input rule: the event it
# sends, the address whose driving fires it, and the value that address must
# take first (`StatechartRun.drive`). The annotations beside them say nothing
# to the machine. ⚠ Listed as what IS read rather than as what is not, so a
# key the vocabulary grows later is refused on a statechart until the driver
# learns it, instead of being dropped the way every value key was.
_STATECHART_DRIVER_READS = frozenset({"event", "address", "becomes"})
_ANNOTATIONS = frozenset({"unresolved", "assumed", "note"})

# `previous(<field>)` in a transform output's expression, as the product reads
# it (`forge::previous_value::reads`): one bare name, in a call nothing
# qualifies. String literals are removed first so a text that spells the call
# is not read as one.
_PREVIOUS_READ = re.compile(r"(?<![\w.])previous\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)\s*\)")
_STRING_LITERAL = re.compile(r"'(?:[^'\\]|\\.)*'|\"(?:[^\"\\]|\\.)*\"")


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
    # Output identifier -> the reason its author gave for leaving it
    # `sce:unresolved`: a value nobody has decided, written as a question
    # rather than a guess. The product refuses to BUILD such a document for
    # shipping, which is right; `verify` builds a copy that withholds it.
    unresolved: dict = dataclasses.field(default_factory=dict)
    # Output identifier -> the identifiers its expression mentions. ⚠ Needed
    # because an assumption is rarely on the value that lands at an address:
    # it is usually a step upstream, and attributing only to the direct writer
    # found none of them on the corpus that prompted this.
    reads: dict = dataclasses.field(default_factory=dict)
    # `(event name, processor type)` for every `<send>` the document addresses
    # to a host-served processor -- what it produces that anything outside can
    # read. Empty for a document whose answers are all datamodel values.
    sends: tuple = ()
    # Every event descriptor any transition listens for.
    events: frozenset = frozenset()
    # State id -> every descriptor an event reaching that state could be
    # acted on through, while it is active. `*` stands for a state that acts
    # on ANY event: one forwarding to an invoked child (W3C SCXML 6.4
    # `autoforward`), or one with an eventless transition whose condition may
    # read `_event` -- which every dequeued event rebinds, selected or not
    # (W3C SCXML 5.10). A state with no id is keyed `None`, because nothing
    # can ask whether it is active.
    listeners: dict = dataclasses.field(default_factory=dict)
    # Input and output identifier -> the `sce:type` the document declares for
    # it. What a binding hands an input is read as this type (`delivery`).
    types: dict = dataclasses.field(default_factory=dict)
    # Import alias -> the variants an imported enumeration declares, each to
    # its number. An `enum:<alias>` input takes the platform's number, and
    # this is what says which variant that number is.
    enums: dict = dataclasses.field(default_factory=dict)
    # Every field a transform reads through `previous()`: what it keeps from
    # one activation to the next. Empty for any other kind, as the product
    # has it -- only a transform has a holder. `verify` asks the product
    # itself (the manifest's `holder`); this is the same answer read without
    # building, so `check` can ask the binding for what such a document needs.
    keeps: frozenset = frozenset()
    # Transitions that restart every other region of a `<parallel>` as a side
    # effect (`_parallel_reentries`). Refused by `check` for a statechart.
    reentries: tuple = ()
    # Whether `kind` was written (`sce:kind`) or is the default. A document
    # that never chose is read as a statechart, and a refusal that only speaks
    # a statechart's terms ("no event") steers its author further from a
    # transform it may have meant -- measured 2026-09-26, a writer given that
    # refusal three rounds running went on adding state around `<if>`.
    kind_declared: bool = True
    # Outputs declared with no `expr` and not left `sce:unresolved`. In a
    # transform the expression IS the output -- the product refuses the
    # document without one ("Transform output field ... must have an 'expr'
    # attribute") -- and this passed a skeleton that computed nothing.
    uncomputed: tuple = ()
    # The namespace URI the document binds the `sce` prefix to, as written, or
    # None if it binds none. ⚠ Any other URI than SCE's makes every `sce:`
    # attribute a different, unknown attribute that the reader passes over in
    # silence -- `sce:kind`, `sce:direction` and `sce:type` included.
    sce_prefix_uri: str | None = None

    def variants_of(self, ident: str) -> dict | None:
        """The enumeration an `enum:` input or output names, if one is imported."""
        declared = self.types.get(ident) or ""
        if not declared.startswith("enum:"):
            return None
        return self.enums.get(declared[len("enum:"):])

    def listens_for(self, event_name: str) -> bool:
        """Whether any transition would be selected by this event."""
        return any(descriptor_matches(d, event_name) for d in self.events)

    def states_acting_on(self, event_name: str) -> list:
        """Every state that could act on this event while it is active."""
        return [state for state, descriptors in self.listeners.items()
                if any(descriptor_matches(d, event_name) for d in descriptors)]

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


def descriptor_matches(descriptor: str, event_name: str) -> bool:
    """Whether one transition descriptor selects this event name.

    ⚠ W3C SCXML 3.12.1 matches a descriptor against a PREFIX of the event
    name, token by token, so `error` selects `error.execution` and an equality
    test would report a document that plainly answers the event as ignoring
    it. `error`, `error.` and `error.*` are the same descriptor there.
    """
    token = descriptor[:-2] if descriptor.endswith(".*") else descriptor.rstrip(".")
    return (token == "*" or event_name == token
            or event_name.startswith(token + "."))


_STATE_TAGS = frozenset(f"{SCXML_NS}{t}" for t in ("state", "parallel", "final", "history"))


def _parallel_reentries(root) -> tuple:
    """Transitions that leave a `<parallel>` and re-enter it with a region no
    target is in, as `(source, event, target, parallel, below_source,
    restarted regions)`.

    W3C SCXML 3.13 takes a transition's domain from the nearest proper
    ancestor that is a compound state or the `<scxml>` element -- and a
    `<parallel>` is neither, so it is stepped over. A transition written on a
    region, or between regions, therefore exits the whole `<parallel>` and
    enters it again, and every OTHER region restarts at its initial state. The
    document says "this latch forgets"; the machine does "every latch
    forgets". Nothing downstream mentions it: the document is valid SCXML,
    it generates, and a platform's tests pass it whenever they never change
    one region's input between another region's edges.

    ⚠ Measured 2026-09-23: a model-written document reset all four latches on
    any latch's ERROR and failed a shipped case; the reference document, which
    passed every shipped case, reset them on any change of a fault input --
    written by an author who had already recorded this very trap.

    Reported once per transition, at the WIDEST `<parallel>` it restarts a
    region of. Not reported: a transition whose target is the `<parallel>`
    itself (a restart that says so); one whose targets name a state in EVERY
    region it re-enters (a joint move -- measured the same day, a document
    moved two latches together with `target="rl_lock rr_lock"` under a
    compound state that kept the domain there, and nothing restarted that it
    did not name); `type="internal"` on a compound source whose targets are
    inside it (W3C SCXML 3.13 then takes the source as the domain); and
    anything inside an `<invoke>`, which is another session.
    """
    parent: dict = {}

    def walk(element) -> None:
        for child in element:
            if child.tag == f"{SCXML_NS}invoke":
                continue
            parent[child] = element
            walk(child)

    walk(root)
    by_id = {e.get("id"): e for e in parent if e.tag in _STATE_TAGS and e.get("id")}

    def ancestors(element):
        while element in parent:
            element = parent[element]
            yield element

    def compound_or_root(element) -> bool:
        return element is root or (element.tag == f"{SCXML_NS}state"
                                   and any(c.tag in _STATE_TAGS for c in element))

    found = []
    for transition in (e for e in parent if e.tag == f"{SCXML_NS}transition"):
        source = parent[transition]
        if source.tag not in (f"{SCXML_NS}state", f"{SCXML_NS}parallel"):
            continue  # an <initial>'s transition exits nothing
        targets = [by_id.get(i) for i in (transition.get("target") or "").split()]
        if not targets or None in targets:
            continue  # targetless exits nothing; an unknown target is refused elsewhere
        below_source = all(source in ancestors(t) for t in targets)
        if (transition.get("type") == "internal" and compound_or_root(source)
                and below_source):
            continue
        domain = next(a for a in ancestors(source) if compound_or_root(a)
                      and all(a in ancestors(t) for t in targets))
        widest, restarted = None, ()
        for a in ancestors(source):
            if a is domain:
                break
            if a.tag != f"{SCXML_NS}parallel" or not all(a in ancestors(t) for t in targets):
                continue
            # A region holding no target is entered at its initial state.
            untargeted = tuple(r.get("id") for r in a if r.tag in _STATE_TAGS
                               and not any(t is r or r in ancestors(t) for t in targets))
            if untargeted:
                widest, restarted = a, untargeted
        if widest is not None:
            found.append((source.get("id"), transition.get("event") or "",
                          transition.get("target"), widest.get("id"), below_source,
                          restarted))
    return tuple(found)


def _listeners(root) -> dict:
    """State id -> the descriptors an event could be acted on through there.

    Read for `verify`, which has to know whether an event nobody can say was
    sent could have changed anything -- so this errs toward ACTING. A state
    listed here is one where the event might matter; one left out is one
    where, by W3C SCXML, it cannot.
    """
    # ⚠ A condition can call a function a script defines, so a script that
    # reads `_event` makes every eventless condition a possible reader of it.
    scripts_read_event = any(
        script.get("src") or "_event" in "".join(script.itertext())
        for script in root.iter(f"{SCXML_NS}script"))
    found: dict = {}

    def walk(element) -> None:
        descriptors: set = set()
        for child in element:
            if child.tag == f"{SCXML_NS}invoke":
                # ⚠ Not descended into. An inline child document is another
                # session, which sees this one's events only when they are
                # forwarded -- and a forwarding state acts on every event.
                if child.get("autoforward") == "true":
                    descriptors.add("*")
                continue
            if child.tag == f"{SCXML_NS}transition":
                if child.get("event"):
                    descriptors.update(child.get("event").split())
                elif child.get("cond") and ("_event" in child.get("cond")
                                            or scripts_read_event):
                    descriptors.add("*")
            walk(child)
        if descriptors and element.tag in (f"{SCXML_NS}state",
                                           f"{SCXML_NS}parallel"):
            found.setdefault(element.get("id"), set()).update(descriptors)

    walk(root)
    return {state: frozenset(d) for state, d in found.items()}


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
    inputs, outputs, uncomputed = [], [], []
    assumed, reads, unresolved, types = {}, {}, {}, {}
    kept: set = set()
    for data in root.iter(f"{SCXML_NS}data"):
        ident = data.get("id")
        direction = data.get(f"{SCE_NS}direction")
        if direction == "in":
            inputs.append(ident)
        elif direction == "out":
            outputs.append(ident)
            if data.get("expr") is None and data.get(f"{SCE_NS}unresolved") is None:
                uncomputed.append(ident)
            kept.update(_PREVIOUS_READ.findall(
                _STRING_LITERAL.sub("''", data.get("expr") or "")))
        if direction in ("in", "out") and data.get(f"{SCE_NS}type"):
            types[ident] = data.get(f"{SCE_NS}type")
        # ⚠ The author's own record of what they had to decide without being
        # told. `sce:assumed` is the product's non-blocking marker -- the pair
        # of `sce:unresolved`, which refuses the build. An assumption compiles,
        # so nothing downstream ever mentioned it again, and a case that
        # refutes one used to read as "your document is wrong" rather than
        # "the thing you wrote down as a guess is the thing that failed".
        if data.get(f"{SCE_NS}assumed"):
            assumed[ident] = (data.get(f"{SCE_NS}assumed-reason")
                              or data.get(f"{SCE_NS}assumed"))
        if direction == "out" and data.get(f"{SCE_NS}unresolved"):
            # Both halves: the marker is the question's handle, the reason is
            # what to go and ask. The product's refusal named both.
            marker = data.get(f"{SCE_NS}unresolved")
            reason = data.get(f"{SCE_NS}unresolved-reason")
            unresolved[ident] = f"{marker} -- {reason}" if reason else marker
        if data.get("expr"):
            reads[ident] = frozenset(re.findall(r"[A-Za-z_][A-Za-z0-9_]*",
                                                data.get("expr")))
    # ⚠ What a STATECHART produces is not in `<data sce:direction="out">` at
    # all. It leaves as a `<send>` to a host-served processor, which is the W3C
    # channel for reaching outside the machine (W3C SCXML 6.2). Reading only the
    # datamodel declarations reported every such document as computing nothing,
    # so a binding naming a real output was refused for naming it.
    sends = []
    for send in root.iter(f"{SCXML_NS}send"):
        processor = send.get("type")
        if not processor:
            # A send with no `type` is the SCXML event processor's, and stays
            # inside this session's own world (W3C SCXML 6.2.4). Nothing
            # outside reads it, so it is not a position a binding can name.
            continue
        sends.append((send.get("event") or "", processor))
    events = {name
              for transition in root.iter(f"{SCXML_NS}transition")
              for name in (transition.get("event") or "").split()}
    # ⚠ No `sce:kind` is a STATECHART, as the product reads it: a forge kind
    # is opt-in, and a plain SCXML document goes the statechart way
    # (`forge::parser::parse_forge` answers "not a forge document"). This read
    # an absent kind as '' -- measured 2026-09-22, an author writing plain
    # SCXML from a brief had a document `check` accepted and `verify` then
    # refused as a kind it could not drive.
    kind = root.get(f"{SCE_NS}kind") or "statechart"
    # The parser resolves prefixes and keeps no record of them, so the binding
    # is read from the text; the first declaration is the root's.
    bound = re.search(r'\bxmlns:sce\s*=\s*(["\'])(.*?)\1', path.read_text(encoding="utf-8"))
    return Document(
        path=path,
        inputs=tuple(inputs),
        outputs=tuple(outputs),
        kind=kind,
        kind_declared=root.get(f"{SCE_NS}kind") is not None,
        uncomputed=tuple(uncomputed),
        sce_prefix_uri=bound.group(2) if bound else None,
        keeps=frozenset(kept) if kind == "transform" else frozenset(),
        assumed=assumed,
        unresolved=unresolved,
        reads=reads,
        sends=tuple(sends),
        events=frozenset(events),
        listeners=_listeners(root),
        types=types,
        enums=_imported_enums(root, path),
        reentries=_parallel_reentries(root),
    )


def _imported_enums(root, path: pathlib.Path) -> dict:
    """Import alias -> {variant: number} for every enumeration this imports.

    Resolved against the importing document, as the generator resolves it. An
    enumeration that cannot be read is refused here, naming it, for the reason
    `imports_of` gives: the alternative is a type nobody can say the numbers of.
    """
    found = {}
    for node in root.iter(f"{SCE_NS}import"):
        if node.get("kind") != "enum" or not node.get("as") or not node.get("src"):
            continue
        source = (pathlib.Path(path).parent / node.get("src")).resolve()
        try:
            enum_root = ET.parse(source).getroot()
        except (ET.ParseError, OSError) as exc:
            raise PackError(f"{path}: imports the enumeration {node.get('as')!r} "
                            f"from {source}, which cannot be read ({exc})") from exc
        variants = {}
        for variant in enum_root.iter(f"{SCE_NS}variant"):
            try:
                variants[variant.get("name")] = int(variant.get("value"), 0)
            except (TypeError, ValueError) as exc:
                raise PackError(
                    f"{source}: variant {variant.get('name')!r} declares no "
                    f"number, and an enumeration input takes one") from exc
        found[node.get("as")] = variants
    return found


def imports_of(path: pathlib.Path) -> list[pathlib.Path]:
    """Every document this one imports, and every one those import, once each.

    ⚠ The product's generator builds ONE document per run. The others it names
    only as build dependencies, while the code it writes expects each
    `<sce:import>` as a sibling beside it. A caller that builds a document in
    order to RUN it therefore owes the whole closure -- and `verify` built the
    root alone, so a document importing so much as an enumeration died on its
    own import line before a single case had run.

    Paths are resolved against the importing document, as the generator
    resolves them. An import the core cannot find or read is refused here,
    naming the document that asked for it, rather than surfacing later as a
    module the runtime cannot load.
    """
    root_path = pathlib.Path(path).resolve()
    found: list[pathlib.Path] = []
    visited = {root_path}
    pending = [root_path]
    while pending:
        current = pending.pop(0)
        try:
            root = ET.parse(current).getroot()
        except (ET.ParseError, OSError) as exc:
            raise PackError(f"{current}: cannot be read as a document ({exc})") from exc
        for node in root.iter(f"{SCE_NS}import"):
            src = node.get("src")
            if not src:
                raise PackError(
                    f"{current}: an <sce:import> names no `src`, so there is "
                    f"no document to build beside this one")
            target = (current.parent / src).resolve()
            if target in visited:
                continue
            if not target.is_file():
                raise PackError(
                    f"{current}: imports {src!r}, and there is no document at "
                    f"{target}")
            visited.add(target)
            found.append(target)
            pending.append(target)
    return found


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
                # Never a symbol, per the published schema -- read from it.
                if key in _keys_that_hold_no_symbol():
                    continue
                walk(value, [side, name, key])


def driving_refusals(document: Document, inputs: dict) -> list[tuple[str, str]]:
    """What stops a run before its first case, as `(where, why)` pairs.

    Rules no case could get past, judged from the binding's inputs against the
    document alone. `check` reports each one, and `verify` refuses the run in
    the same words before building anything, so the two cannot disagree about
    which bindings can be run.

    A COMPUTATION is called with its inputs by name, so a rule naming an input
    the document does not declare has nothing to hand its value to.

    A STATECHART is handed nothing but events (W3C SCXML 3.12). The driver
    sends a rule's `event` when the case drove its `address` -- to the value
    `becomes` names, if it names one -- and sends it BARE. So:

      a declared input   nothing outside the machine writes its datamodel;
                         the generated code offers the host a reader for each
                         variable and a writer for none
      a value key        (`equals`, `protocol`, `when_absent`, ...) computes
                         something no part of the machine receives
      no event           reaches no part of the machine at all

    ⚠ The first two used to be DROPPED by the run rather than refused, so a
    guard comparing a level compared the value it was declared with, and the
    case failed as though the document were wrong. Measured 2026-09-22 on a
    machine that flashes only above a level of 3: driven at 5, it stayed dark,
    and `verify` reported "expected FLASHING, got DARK" against the document.
    """
    out: list[tuple[str, str]] = []
    if document.kind not in STATECHART_KINDS:
        for name in sorted(inputs):
            if name not in document.inputs:
                out.append((f"input {name}",
                            f"the document declares no input {name!r}, so this "
                            f"rule has nothing to hand its value to -- the "
                            f"document is called with the inputs it declares, "
                            f"by name"))
        return out

    for ident in document.inputs:
        out.append((
            f"document {document.path.name}",
            f"declares {ident!r} `sce:direction=\"in\"`, and nothing outside a "
            f"statechart writes its datamodel: the generated machine offers "
            f"the host a reader for each variable and a writer for none, so "
            f"every guard reading {ident!r} reads the value it was declared "
            f"with, whatever a case drove. What reaches a statechart from "
            f"outside is an event, and a value travels as that event's data "
            f"(`_event.data`), which a binding cannot attach yet. A component "
            f"that compares levels is a transform, which is handed its inputs "
            f"every activation and keeps what it needs with `previous()`."))
    drives = any(rule.get("event") for rule in inputs.values())
    if not drives:
        undeclared = "" if document.kind_declared else (
            " ⚠ The document does not say what kind it is (no `sce:kind` on "
            "its root), so it is read as a statechart, which only an event "
            "moves. If every output is a function of the inputs' current "
            "values, it is a transform instead: `sce:kind=\"transform\"` on "
            "the root, and each output a `<data sce:direction=\"out\" "
            "expr=\"...\">` computed from the `sce:direction=\"in\"` inputs, "
            "with no states and no events. `scaffold` writes that skeleton "
            "from the interface model.")
        out.append(("binding",
                    "no input rule names an `event`, so no case can drive the "
                    "machine. Every case would be judged against a document "
                    "sitting in its initial configuration, which is a verdict "
                    "about nothing." + undeclared))
    for name, rule in sorted(inputs.items()):
        if not rule.get("event"):
            # A rule still waiting for its address is reported as such, and
            # with no rule driving at all the sentence above covers this one.
            if drives and not rule.get("unresolved"):
                out.append((f"input {name}",
                            "names no `event`, and a statechart is handed "
                            "nothing but events: this rule would reach no part "
                            "of the machine in any case"))
            continue
        extra = sorted(k for k in rule
                       if k not in _STATECHART_DRIVER_READS | _ANNOTATIONS)
        if extra:
            keys = ", ".join(f"`{k}`" for k in extra)
            one = len(extra) == 1
            out.append((
                f"input {name}",
                f"{keys} {'computes' if one else 'compute'} a value for the "
                f"machine to read, and a statechart is handed only the event "
                f"{rule['event']!r}, with no data: what "
                f"{'it computes' if one else 'they compute'} reaches no part of "
                f"the machine, so a guard comparing it compares whatever the "
                f"variable started as"))
    return out


def activation_unsaid(document_name: str) -> str:
    """Why a binding for a document that keeps values is incomplete without
    `activation`. `check` reports it, and `verify` refuses in these words."""
    return (f"{document_name} keeps values from one activation to the next (it "
            f"reads previous()), and the binding does not say how the host runs "
            f"it. What `previous(x)` means is the value one ACTIVATION ago, so "
            f"it depends on the host's schedule: say `activation: on-change` "
            f"(once each time its inputs change) or `activation: periodic` "
            f"(once per period).")


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

    # ⚠ A binding with NO output rule has no behaviour anyone can observe:
    # nothing the document computes is placed anywhere, so no case can compare
    # a value and nothing reaches the platform. That is a property of this one
    # document, which makes it this command's question -- whether a SET of
    # documents writes every position is `coverage`'s, and a position no rule
    # of this binding claims stays allowed for exactly that reason. Only zero
    # rules is refused; one marked `internal` or `unresolved` still counts.
    # Measured 2026-09-25: a writer reached "0 refusals" by deleting every
    # output rule, and `verify` then passed cases none of which read a value.
    if not declared_outputs:
        out.append(Finding(
            "binding",
            "declares no output rule, so the document has no behaviour anyone "
            "can observe: nothing it computes is placed at an address, no case "
            "can compare a value, and nothing reaches the platform. Bind at "
            "least one output (marking it `internal` or `unresolved` where "
            "that is the truth)."))

    sce_uri = SCE_NS.strip("{}")
    if document.sce_prefix_uri is not None and document.sce_prefix_uri != sce_uri:
        out.append(Finding(
            f"document {document.path.name}",
            f"binds the prefix `sce` to {document.sce_prefix_uri!r}, not to "
            f"{sce_uri!r}. The prefix is only a spelling: what an attribute IS "
            f"is its namespace, so every `sce:` attribute in this document is "
            f"some other, unknown attribute and is passed over -- `sce:kind`, "
            f"`sce:direction` and `sce:type` among them. Write "
            f"`xmlns:sce=\"{sce_uri}\"` on the root."))

    # ⚠ Declared unknowns are reported FIRST and as their own thing. A binding
    # written before the platform's list exists is an ordinary state -- the
    # document is already platform-free, so the logic can be authored long
    # before the addresses are known -- and the right report for it names what
    # is still missing rather than complaining that the addresses are not real.
    # It still fails: an incomplete binding cannot reach the platform, which is
    # exactly what this command answers.
    for side, rules in (("input", declared_inputs), ("output", declared_outputs)):
        for name, rule in sorted(rules.items()):
            if isinstance(rule, dict) and rule.get("unresolved"):
                out.append(Finding(
                    f"{side} {name}",
                    f"no address yet — {rule['unresolved']}"))

    for name, rule in sorted(declared_inputs.items()):
        if rule.get("unresolved"):
            continue
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
        # ⚠ Read off the CASE, not off an address -- the schema says so of
        # both, and `verify` reads them that way. This refused them as "no
        # address and no protocol", so a binding `verify` ran was one `check`
        # would not pass.
        if rule.get("clock") or "variant_is" in rule:
            continue
        if address is None:
            out.append(Finding(f"input {name}", "no address and no protocol"))
            continue
        entry = model.by_address.get(address)
        if entry is None:
            out.append(Finding(f"input {name}", f"{address!r} is not in the interface model"))
            continue
        space = (entry.field("") or entry.fields[0]).values
        # ⚠ `becomes` joins the pair rather than getting a check of its own.
        # It names a symbol the address must take, so it is wrong in exactly
        # the way the other two are, and a second site would be one more place
        # to forget when a value space grows.
        for key in ("equals", "not_equals", "becomes"):
            sym = rule.get(key)
            if sym is not None and space is not None and sym not in space:
                out.append(Finding(f"input {name}", f"{address} does not admit {sym!r}; it admits " + ", ".join(sorted(space))))
        for sym in rule.get("equals_any") or ():
            if space is not None and sym not in space:
                out.append(Finding(f"input {name}", f"{address} does not admit {sym!r}"))

    for name, rule in sorted(declared_outputs.items()):
        if rule.get("internal") or rule.get("unresolved"):
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
        # ⚠ The refusal names the FIX, because there is now one in the
        # document: a transform keeps a value by reading `previous(<field>)`,
        # with the field's `sce:initial` for the first activation. A refusal
        # that said only "the kind does not say so" left the author choosing
        # between a statechart and the same binding again.
        moves = []
        for name in memory:
            rule = declared_inputs[name]
            kept = rule.get("previous_of") or rule.get("state_of")
            if not kept:
                continue
            first = (f" (the binding's `initial`, {rule['initial']!r})"
                     if "initial" in rule else "")
            moves.append(
                f"where it reads {name!r}, read `previous({kept})`, give "
                f"{kept!r} the `sce:initial` it holds before the first "
                f"activation{first}, and drop the input {name!r}")
        if not moves:
            fix = ""
        elif document.kind == "transform":
            fix = (" Declare the memory in the document instead: "
                   + "; ".join(moves) + ".")
        else:
            fix = (f" A {document.kind} keeps no value; write it as a "
                   f"transform, which can: " + "; ".join(moves) + ".")
        out.append(
            Finding(
                f"document {document.path.name}",
                f"declares kind {document.kind!r}, whose binding may not hand "
                # ⚠ "Reads an earlier round", not "feeds its own output back":
                # the list holds an input's previous value and a latched
                # protocol as well as the document's own last output, and the
                # narrower sentence sent an author looking for a feedback loop
                # their binding did not have.
                f"it memory, but the binding reads an earlier round through "
                f"{', '.join(memory)}. The component is this document plus "
                f"something that remembers, and the document does not say so."
                + fix,
            )
        )

    # ⚠ The document says what type each input is and the model says what
    # each address carries, so a rule is checked against BOTH -- by the same
    # judgement `verify` reads with, so the two cannot disagree about which
    # bindings are well formed. Nothing did this before: `number: true`
    # restated the type, and a binding saying "number" into a `bool` input,
    # or a comparison into an `int32` one, was found only when a run failed.
    for name, rule in sorted(declared_inputs.items()):
        if name not in document.types or rule.get("unresolved"):
            continue
        remembered = rule.get("previous_of") or rule.get("state_of")
        why = delivery.refusal(
            name, rule, document.types[name],
            model.field_at(rule["address"]) if rule.get("address") else None,
            remembered_type=document.types.get(remembered) if remembered else None,
            variants=document.variants_of(name))
        if why:
            out.append(Finding(f"input {name}", why))

    # A statechart's declared inputs are refused as such below: asking for a
    # binding would send the author to write a rule nothing can deliver.
    for ident in (() if document.kind in STATECHART_KINDS else document.inputs):
        if ident not in declared_inputs:
            out.append(Finding(f"document {document.path.name}", f"input {ident!r} has no binding — it would be a default every round"))
    for ident in document.outputs:
        if ident not in declared_outputs:
            out.append(Finding(f"document {document.path.name}", f"output {ident!r} has no binding — it is computed and dropped"))
    for name, rule in sorted(declared_outputs.items()):
        # ⚠ An output bound to a SEND is answered by the document containing
        # one, not by a datamodel declaration it will never carry. Asking the
        # single question for both channels refused every statechart: the rule
        # named a position the document really does write, and the check could
        # only see one of the two ways a document writes anything.
        if rule.get("sent"):
            wanted = (rule["sent"] or {}).get("processor")
            if not [s for s in document.sends if wanted is None or s[1] == wanted]:
                of_type = f" of type {wanted!r}" if wanted else ""
                out.append(Finding(
                    f"output {name}",
                    f"binds a host-served send{of_type} and the document "
                    f"sends none"))
            continue
        if name not in document.outputs:
            out.append(Finding(f"output {name}", "the document does not compute it"))
        elif document.kind == "transform" and name in document.uncomputed:
            out.append(Finding(
                f"output {name}",
                f"the document declares it and computes nothing: a transform "
                f"output is its `expr`, the value computed from the inputs "
                f"every activation, and the product refuses a document with "
                f"an output that has none. Write the expression the "
                f"specification gives, or mark it `sce:unresolved` with the "
                f"question if the specification does not say."))

    # ⚠ Each of the next three was refused by `verify` alone, so a binding this
    # command had passed came back with every case unjudged, or not run at
    # all. Measured 2026-09-22 by writing each shape into a binding this
    # fixture otherwise accepts: nothing here, and `verify` stopped on every
    # one. Each is now one judgement both commands make.
    #
    # Where an output lands, and whether it can land every value the document
    # says it can produce (`landing`).
    for name, rule in sorted(declared_outputs.items()):
        why = landing.form_refusal(rule)
        if why:
            out.append(Finding(f"output {name}", why))
            continue
        values = landing.produces(rule, document.types.get(name), document.sends)
        missing = landing.unmapped(rule, values)
        if missing:
            out.append(Finding(
                f"output {name}",
                f"the map has no entry for {', '.join(map(repr, missing))}, "
                f"which the document can produce, so a case producing it has "
                f"nowhere to land"))

    # Whether a run can start at all (`driving_refusals`).
    for where, why in driving_refusals(document, declared_inputs):
        out.append(Finding(where, why))

    # What a document that keeps values needs the binding to say.
    if document.keeps and not binding.get("activation"):
        out.append(Finding("binding", activation_unsaid(document.path.name)))

    # A statechart replayed from records that restate a value is judged on
    # the host's delivery rule; `verify` refuses in these words when the
    # binding does not state it, and the two must not disagree.
    if document.kind in STATECHART_KINDS and pack.examples.cases:
        from .verify import restatement_needs_activation  # verify imports this module
        why = restatement_needs_activation(pack.examples.cases, pack.model,
                                           binding.get("activation"))
        if why:
            out.append(Finding("binding", why))

    # A transition that restarts every other region of a <parallel> as a side
    # effect. Valid SCXML that generates, and that a platform's tests pass
    # whenever they never move one region between another region's edges --
    # which is why it has to be said here (`_parallel_reentries`).
    for source, event, target, par, below, restarted in document.reentries:
        fix = (f"write type=\"internal\" to restart only {source!r} (W3C "
               f"SCXML 3.13 then takes it as the domain), or put the "
               f"transition on its own states" if below else
               f"a transition from one region into another restarts every "
               f"region it does not name as a target")
        out.append(Finding(
            f"state {source}",
            f"the transition on {event!r} to {target!r} leaves and re-enters "
            f"<parallel id={par!r}>, so {', '.join(map(repr, restarted))} "
            f"restart{'s' if len(restarted) == 1 else ''} at the initial state "
            f"(W3C SCXML 3.13: a <parallel> is never a transition's domain). "
            f"{fix}; to restart all of {par!r} on purpose, target {par!r} "
            f"itself"))

    # ⚠ An event nothing answers is the statechart shape of a silent pass. The
    # case would send it, the machine would ignore it, every later reading
    # would be of a machine that was never driven -- and each case would still
    # be judged, against whatever the document happened to hold.
    for name, rule in sorted(declared_inputs.items()):
        event = rule.get("event")
        if event and not document.listens_for(event):
            out.append(Finding(
                f"input {name}",
                f"sends {event!r} and no transition listens for it — a case "
                f"driving this input would leave the machine untouched"))

    return out
