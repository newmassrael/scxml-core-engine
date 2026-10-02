"""Reading a YAML or JSON file the way its author meant it.

Both formats let a mapping write the same key twice, and both libraries answer
by keeping the LAST value and saying nothing. A value space written
`{ACTIVE: 1, ACTIVE: 0}` is read as `ACTIVE: 0`, an entry that writes `role`
twice is read as whichever came second, and two readings of one phrase become
the second -- each of them a file that loads cleanly and means something its
author did not write. Every file this package reads from an author goes
through here, so a key written twice is refused where it is written, with the
key and its line, rather than resolved by whichever library read it.

A key repeated BETWEEN files is not this module's concern: merging pieces of a
pack is a decision the loader makes and states, one file at a time.
"""

from __future__ import annotations

import json

import yaml


class RepeatedKey(ValueError):
    """A mapping wrote one key twice. `where` says where, or is empty."""

    def __init__(self, key: object, where: str = ""):
        self.key = key
        self.where = where
        super().__init__(f"{where}{key!r} is written twice in one mapping; a repeated "
                         f"key keeps only the last value and says nothing, so say "
                         f"which one is meant")


class _StrictLoader(yaml.SafeLoader):
    """`yaml.safe_load` that refuses a mapping writing one key twice."""

    def construct_mapping(self, node, deep=False):
        if isinstance(node, yaml.MappingNode):
            first: dict = {}
            for key_node, _ in node.value:
                if key_node.tag == "tag:yaml.org,2002:merge":
                    # `<<: *base` is how YAML says "these, unless overridden".
                    continue
                key = self.construct_object(key_node, deep=True)
                try:
                    hash(key)
                except TypeError:
                    continue  # the base class refuses an unhashable key, in its own words
                if key in first:
                    raise RepeatedKey(
                        key, f"line {key_node.start_mark.line + 1} (first written on "
                             f"line {first[key] + 1}): ")
                first[key] = key_node.start_mark.line
        return super().construct_mapping(node, deep=deep)


def _no_repeats(pairs: list[tuple[str, object]]) -> dict:
    seen: set = set()
    for key, _ in pairs:
        if key in seen:
            raise RepeatedKey(key)
        seen.add(key)
    return dict(pairs)


def read_yaml(text: str):
    """The document `text` holds. Raises `yaml.YAMLError` when it is not
    well-formed and `RepeatedKey` when a mapping writes a key twice."""
    return yaml.load(text, Loader=_StrictLoader)


def read_json(text: str):
    """The document `text` holds. Raises `json.JSONDecodeError` when it is not
    well-formed and `RepeatedKey` when an object writes a key twice."""
    return json.loads(text, object_pairs_hook=_no_repeats)


MALFORMED = (yaml.YAMLError, json.JSONDecodeError, RepeatedKey)
"""What reading a file for an author can raise about its CONTENT."""
