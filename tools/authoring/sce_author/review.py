"""Judging the PACK, which every other command assumes is right.

`brief`, `questions`, `check` and `verify` all compare a document against the
pack. None of them can be more right than the pack is, and the pack is written
by hand or by an adapter that nothing in this tree ever runs -- a platform's
own format is not ours, so the code that converts it lives with whoever owns
that format, outside any suite here.

⚠ SO THE ONE PIECE OF THE WORKFLOW WITH NO TESTS IS THE ONE EVERY ANSWER
RESTS ON. That is not fixable by testing somebody else's converter. What is
fixable is this: the two ways a pack goes wrong are known, they were written
down in the README before anything computed them, and both are measurable
from the pack and the prose alone.

    the spellings an address goes by   every class that skips an address it
                                       thinks unmentioned goes quiet
    which text belongs to which        `gate-off-unstated` above all

⚠⚠ THIS REPORTS NUMBERS AND REFUSES TO GIVE A VERDICT. A pack cannot be
"correct" -- it is a claim about a platform, and nothing here has the platform
to check it against. What a reader needs is the handful of figures that say
whether the answers are worth trusting, with the shapes that are always wrong
called out. A green tick over an unmeasurable claim is the thing this whole
tool exists to refuse.

⚠⚠⚠ The measured failure mode of a real adapter, twice, was a DROPPED SUBSET:
a reader that discarded the platform's 823 enum-less signals, so an address
the platform has read as an address the platform lacks. Nothing in a pack can
reveal that on its own -- absence looks like absence. The examples are the
second opinion that can, which is why `driven_undeclared` is here and why a
pack with no examples is reported as the weakest case rather than the cleanest.
"""

from __future__ import annotations

from dataclasses import dataclass, field

from .pack import Model
from .prose import Prose


@dataclass
class Review:
    """What is measurable about a pack, without claiming it is right."""

    addresses: int = 0
    outputs: int = 0
    # An address the prose never writes under any spelling it is given. This
    # is the population `no-decision-logic` lives in: every one of them is
    # either a genuine silence or a spelling the pack does not know.
    unmentioned: list[str] = field(default_factory=list)
    # An address given exactly one spelling. Not wrong -- most are right --
    # but a missing alternative cannot be seen, so this is the surface the
    # row above is drawn from.
    single_spelling: int = 0
    # An address with neither a value space nor a type: nothing can be
    # compared against it, so every value check passes over it in silence.
    unspecified: list[str] = field(default_factory=list)
    # Lines of prose the partition gives to some address, over lines in all.
    attributed_lines: int = 0
    total_lines: int = 0
    # Blocks of one or two lines. A block that short usually means the
    # partition changed owner on a passing mention rather than a heading.
    thin_blocks: int = 0
    blocks_built: int = 0
    # Addresses the examples drive that the model does not declare. The only
    # evidence in a pack that the model is INCOMPLETE rather than small.
    driven_undeclared: int = 0
    has_examples: bool = False
    # Output positions the model declares that no case ever expects. `verify`
    # reports the same thing against the BINDING, once one exists; this is the
    # view a pack author has before that, and it is the number that says what
    # a future pass would be worth.
    unasserted_outputs: list[str] = field(default_factory=list)
    asserted_outputs: int = 0

    @property
    def attribution(self) -> float:
        return (self.attributed_lines / self.total_lines) if self.total_lines else 0.0

    def alarms(self) -> list[str]:
        """The shapes that are wrong whatever the platform turns out to be.

        Not a verdict on the pack -- a list of things that cannot be right.
        An empty list means none of THESE, never "the pack is good".
        """
        out = []
        if not self.addresses:
            out.append("the model declares no addresses, so every question "
                       "answers itself clean")
        if not self.outputs:
            out.append("the model declares no output, so nothing in the "
                       "document is judged as deciding anything")
        if not self.has_examples:
            out.append("no examples, so nothing can notice a signal the "
                       "specification fails to mention, and nothing runs")
        if self.unspecified:
            out.append(f"{len(self.unspecified)} address(es) carry neither a "
                       f"value space nor a type, so every value compared "
                       f"against them passes")
        if self.has_examples and not self.asserted_outputs:
            out.append("no case expects any output this model declares, so a "
                       "run of them would judge nothing and still pass")
        if self.total_lines and self.attribution < 0.25:
            out.append(f"the partition attributes {self.attribution:.0%} of "
                       f"the prose, so most of the document is read by no "
                       f"address and the gated-output class is nearly blind")
        return out


def review(pack, prose: Prose) -> Review:
    """Measure the pack against the document it is about to be used on.

    ⚠ It takes the PROSE as well, and has to: half of what can be wrong with a
    pack is only visible against a document. "The names are wrong" has no
    meaning until there is a text that writes them.
    """
    model: Model = pack.model
    out = Review()
    out.addresses = len(model.entries)
    out.outputs = len(model.outputs())
    out.has_examples = pack.examples is not None and pack.examples.present

    from .questions import mentions

    for entry in model.entries:
        if len(entry.names) <= 1:
            out.single_spelling += 1
        if not mentions(prose, entry.names):
            out.unmentioned.append(entry.address)
        # ⚠ NOT `not entry.fields`. An address that declares neither is given
        # one field carrying nothing, so it is never fieldless -- the shape to
        # look for is a field that admits everything, which is what
        # `Field.admits` returns True for whatever it is handed.
        if all(f.values is None and f.type is None for f in entry.fields):
            out.unspecified.append(entry.address)

    universe = {e.address: e.names for e in model.outputs() if e.names}
    blocks = prose.blocks(universe) if universe else {}
    out.blocks_built = sum(1 for text in blocks.values() if text.strip())
    out.thin_blocks = sum(1 for text in blocks.values()
                          if 0 < len(text.splitlines()) <= 2)
    out.attributed_lines = sum(len(text.splitlines())
                               for text in blocks.values() if text.strip())
    out.total_lines = sum(1 for line in prose.text.splitlines() if line.strip())

    if out.has_examples:
        out.driven_undeclared = sum(
            1 for address in pack.examples.driven
            if model.owning(address) is None
            and not address.startswith(pack.conventions.infrastructure)
        )
        # ⚠ Every WRITABLE POSITION, not every address. A record address with
        # four fields is four things a case can be wrong about, and counting
        # it once reports a pack that asserts one field of four as complete.
        expected = {a for case in pack.examples.cases for a in case.expect}
        positions = [entry.address + (f".{f.name}" if f.name else "")
                     for entry in model.outputs() for f in entry.fields]
        out.asserted_outputs = sum(1 for p in positions if p in expected)
        out.unasserted_outputs = sorted(p for p in positions
                                        if p not in expected)
    return out
