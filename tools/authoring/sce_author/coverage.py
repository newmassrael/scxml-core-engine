"""Which output positions no document writes, and which two documents claim.

`check` and `verify` each take ONE binding, so each answers about one
document. A subject matter is several documents, and the question nobody
could ask is the one about the SET. A conversion that needed five components
and produced three leaves every command green: each of the three is correct,
each of their bindings checks, each of their runs passes, and nothing
enumerates the two that were never written. The report a reader gets is
"green", and the sentence it leaves them to supply themselves is "...for the
parts somebody remembered to write".

⚠ THE TWO FIGURES HERE ARE NOT THE SAME KIND OF THING, so they are reported
differently. A position no document writes is a STATUS: a conversion in
progress looks exactly like that, and alarming on it would be an alarm every
unfinished piece of work trips, which is an alarm people learn to scroll past.
A position TWO documents write cannot be right whatever the platform turns out
to be -- one field would receive two answers, and which one stood would be
settled by whichever component ran last.

⚠⚠ This is a THIRD axis and not a third spelling of an existing one.
`review.unasserted_outputs` is model positions no EXAMPLE expects, and
`verify.unasserted` is positions one BINDING writes that no case expects.
Both ask whether the testing is complete. This one asks whether the
DECOMPOSITION is, and nothing else could: the other two are handed a single
document and cannot see the shape of the set it belongs to.
"""

from __future__ import annotations

import pathlib
from dataclasses import dataclass, field

from .check import read_binding
from .pack import Pack


def position(address: str, field_name: str) -> str:
    """The key a model position and a binding's target are both spelled as.

    ⚠ One rule, written once. `review` counts model positions and `verify`
    counts what a binding writes, and the two figures are only comparable
    because they agree on this -- a record address with four fields is four
    things, and folding it into one reports a component that writes a quarter
    of an address as covering it.
    """
    return address + (f".{field_name}" if field_name else "")


@dataclass
class Coverage:
    """What the set of documents does and does not reach."""

    # Every output position the interface model declares.
    positions: list[str] = field(default_factory=list)
    # Position -> the bindings that write it, by file name.
    written: dict = field(default_factory=dict)
    # Positions some binding writes that the model does not declare. Not this
    # command's business to judge -- `check` refuses each one against its own
    # binding -- but counting them here stops a mistyped field from arriving
    # as a mysterious addition to `unwritten`, where a reader would go looking
    # for a document that was never missing.
    undeclared: list[str] = field(default_factory=list)

    @property
    def unwritten(self) -> list[str]:
        return sorted(p for p in self.positions if p not in self.written)

    @property
    def covered(self) -> int:
        return sum(1 for p in self.positions if p in self.written)

    def contested(self) -> list[tuple]:
        """Positions more than one document writes, with who writes them."""
        return sorted((p, sorted(who)) for p, who in self.written.items()
                      if len(who) > 1)

    def alarms(self) -> list[str]:
        """Only the shapes that cannot be right. See the module docstring."""
        return [f"{p} is written by {len(who)} documents: {', '.join(who)}"
                for p, who in self.contested()]


def coverage(pack: Pack, binding_paths) -> Coverage:
    """Read every binding and say what the set of them reaches."""
    out = Coverage()
    out.positions = [position(entry.address, f.name)
                     for entry in pack.model.outputs()
                     for f in entry.fields]
    declared = set(out.positions)
    undeclared = set()
    for path in binding_paths:
        path = pathlib.Path(path)
        binding = read_binding(path)
        for rule in (binding.get("outputs") or {}).values():
            # ⚠ Neither of these occupies a position. `internal` is a value
            # the document carries and never publishes, and `unresolved` is
            # the author saying nobody has yet said where it lands -- counting
            # either as coverage would report a position as reached by a
            # document that does not reach it.
            if rule.get("internal") or rule.get("unresolved"):
                continue
            address = rule.get("address")
            if not address:
                continue
            key = position(address, rule.get("field") or "")
            if key not in declared:
                undeclared.add(key)
                continue
            out.written.setdefault(key, []).append(path.name)
    out.undeclared = sorted(undeclared)
    return out
