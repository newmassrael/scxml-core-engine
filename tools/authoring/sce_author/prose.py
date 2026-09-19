"""The prose side: any number of files, read as one body of text.

Several files because one feature is regularly written across several
documents, and a name introduced in one is used in another. Resolving names
per file would report the second file's use of the first file's name as
unknown, which is a question about the reader rather than about the
specification.
"""

from __future__ import annotations

import pathlib
import re
from dataclasses import dataclass
from functools import lru_cache


@lru_cache(maxsize=None)
def token(symbol: str) -> re.Pattern:
    """This symbol WRITTEN, rather than these letters appearing somewhere.

    `\\b` is not enough: the boundary between `_` and a letter is not a word
    boundary, so `\\bOFF\\b` still matches inside `DISPLAY_OFF`. The guards
    below treat an underscore as part of the name, which is what a symbol is.

    ⚠ It lives here, beside the text it reads, because three separate readers
    have needed it and the two that did not have it were both wrong in the
    same direction -- quietly, by finding a name that was not written.
    """
    return re.compile(r"(?<![A-Za-z0-9_])" + re.escape(str(symbol))
                      + r"(?![A-Za-z0-9_])")

# The default way a comparison is written, in programming notation.
#
# ⚠ This is the ONE thing about the SHAPE of prose that lives here rather than
# in a pack, and it is a DEFAULT, not an assumption: `conventions.
# comparison_pattern` replaces it. A document that says "X is HIGH", or that
# attaches a grammatical particle to the name as an agglutinative language
# does, so that the name's end is not a word boundary, writes its own pattern
# there and this module never learns about it.
#
# It is a default rather than a required field because the specifications
# measured so far all use this notation, and a pack that must restate the
# obvious is a pack nobody finishes. The measurement is the reason, and when a
# subject matter arrives that writes comparisons another way, the field is
# already there.
DEFAULT_COMPARISON = re.compile(
    r"(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*(?P<op>==|!=|<>|=)\s*(?P<token>[A-Za-z_][A-Za-z0-9_]*)"
)


@dataclass(frozen=True)
class Source:
    path: pathlib.Path
    text: str
    reader: str = "plain"
    # What the reader could not carry from this file. Kept on the source
    # rather than logged, so a brief can print it beside the text it is
    # missing from.
    notes: tuple[str, ...] = ()


@dataclass
class Prose:
    sources: tuple[Source, ...]

    @property
    def text(self) -> str:
        return "\n".join(s.text for s in self.sources)

    def locate(self, needle: str) -> tuple[pathlib.Path, int] | None:
        """Which file and line first writes this, so a question can point at it."""
        for src in self.sources:
            for lineno, line in enumerate(src.text.splitlines(), 1):
                if needle in line:
                    return src.path, lineno
        return None

    def names_of(self, conventions) -> dict[str, str]:
        """Every name this prose writes, to its role.

        The conventions own the patterns; this method owns the fact that the
        whole body is one text. Asking file by file would call the second
        file's use of the first file's name unknown.
        """
        return conventions.names_in(self.text)

    def blocks(self, universe: dict[str, tuple[str, ...]]) -> dict[str, str]:
        """What the prose says about each subject, as a block rather than a line.

        A subject owns the line that names it and every line after it until
        another subject is named. Two readings were wrong before this one:

          the whole body    one subject's answer stood in for every subject's,
                            so the question never fired at all
          the matching line only, which misses a table's continuation rows --
                            and a continuation row is exactly where a
                            specification puts the other case

        `universe` is subject to the names it may be written as. Ownership
        changes only on a name in that universe, so prose between two subjects
        belongs to the first, which is where it was written.

        ⚠ A NAME IS MATCHED AS A TOKEN, AND THE LONGEST ONE TAKES THE LINE.
        Both halves were measured rather than argued, because this partition
        is the one thing a whole question class is built on and it was
        deciding ownership by two accidents.

        The first was a substring: `n in line` gave a line naming
        `OUT_LampState` to an address called `Lamp`, which is the same defect
        `token` exists for and which this tree has now fixed in four places.
        The second was iteration order: the first key whose name appeared
        won, and the keys arrive in address order, so which address owned a
        line naming two of them was settled by the alphabet.

        Measured 2026-09-20 over 129 subject packs, counting only the class
        this feeds. The test is whether the partition survives an address
        gaining a name it does not need -- a model that publishes every
        spelling rather than the ones its document happens to use:

            substring, first wins      132 findings move
            token, first wins           16
            token, longest name wins     4

        ⚠⚠ The point of that last column is not tidiness. An interface model
        is supposed to be a property of the PLATFORM, published once and read
        by every specification written against it; under the old rule it could
        not be, because adding a spelling moved 132 findings. The rule below
        is most of what stood in the way.
        """
        blocks: dict[str, list[str]] = {key: [] for key in universe}
        # Longest first, so a specific name beats a general one that happens
        # to be written on the same line. Ties keep the universe's order.
        ranked = sorted(
            ((name, key) for key, names in universe.items() for name in names),
            key=lambda pair: -len(pair[0]),
        )
        patterns = [(token(name), key) for name, key in ranked]
        for src in self.sources:
            owner = None
            for line in src.text.splitlines():
                for pattern, key in patterns:
                    if pattern.search(line):
                        owner = key
                        break
                if owner is not None:
                    blocks[owner].append(line)
        return {key: "\n".join(lines) for key, lines in blocks.items()}

    def comparisons(self, conventions=None) -> list[tuple[str, str, str]]:
        """(name, operator, token) for every comparison the prose writes.

        The pattern comes from the pack when it declares one. Passing no
        conventions falls back to programming notation, which is what the
        default exists for.
        """
        pattern = getattr(conventions, "comparison_pattern", None) or DEFAULT_COMPARISON
        return [
            (m.group("name"), m.group("op"), m.group("token"))
            for m in pattern.finditer(self.text)
        ]


def load_prose(paths: list[pathlib.Path]) -> Prose:
    """Any number of documents, in any format the core has a reader for.

    Reading is delegated to `ingest`, which is where formats live: a format is
    not a subject matter, so a reader for one is general and belongs in the
    core rather than in each pack.
    """
    from .ingest import ingest

    sources = []
    for path in paths:
        got = ingest(pathlib.Path(path))
        sources.append(
            Source(pathlib.Path(path), got.text, got.reader, tuple(got.notes))
        )
    if not sources:
        raise ValueError(
            "no prose files. With none, every question answers itself clean, "
            "which is indistinguishable from a specification that says everything."
        )
    return Prose(tuple(sources))
