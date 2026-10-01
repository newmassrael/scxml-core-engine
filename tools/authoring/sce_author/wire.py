"""What crosses between the server and the process that plays a design.

Both sides speak JSON, one object per line, and nothing else. Not `pickle`: the
child runs a design, a design can be hostile, and a parent that unpickles what a
compromised child sends has handed it the parent. JSON can only be a value, so
the worst the child can say is a wrong one.

A value is one of: `None`, a boolean, a number, a string, a list, a tuple, a set,
a dictionary, bytes, a snapshot of a dataclass, or a REFERENCE to an object that
stayed in the child. The first group is copied; the last stays where it is and
the parent holds a number for it (`sandbox.Remote`). Anything else is not sent:
it is a reference, never a guess at a copy. Tags are objects with a key that
starts with `$`; a dictionary of the design's that would collide with one is
sent as a list of pairs, so the two cannot be confused.

    {"$ref": 7, "r": "Engine"}      an object that stayed in the child
    {"$m": [7, "advance_time"]}     a method of one (callable, named on its owner)
    {"$ns": {"type": "...", "fields": {...}}}   a dataclass, copied field by field
    {"$t": [...]}  {"$set": [...]}  {"$bytes": "<base64>"}  {"$float": "nan"}
    {"$items": [[key, value], ...]} a dictionary with keys that are not plain names

Exactly the types in the first group are copied, by exact type: a subclass of
`int` (an enumeration member) is an object with an identity, not a number, and
travels as a reference.
"""

from __future__ import annotations

import base64
import dataclasses
import math
from types import SimpleNamespace

from .errors import VerifyError

#: How deep a value may nest. A design that builds a list inside a list a
#: million times is not describing a result.
MAX_DEPTH = 100


class WireError(VerifyError):
    """A value that cannot cross, or a message that is not what the protocol says."""


def encode(value, hook, _depth: int = 0):
    """`value` as JSON-able data. `hook(object)` answers for anything that is not
    one of the copied types: a tagged reference, a snapshot, or a refusal."""
    if _depth > MAX_DEPTH:
        raise WireError(f"a value nests more than {MAX_DEPTH} levels deep")
    kind = type(value)
    if value is None or kind is bool or kind is int or kind is str:
        return value
    if kind is float:
        if math.isfinite(value):
            return value
        return {"$float": "nan" if math.isnan(value) else ("inf" if value > 0 else "-inf")}
    deeper = _depth + 1
    if kind is list:
        return [encode(item, hook, deeper) for item in value]
    if kind is tuple:
        return {"$t": [encode(item, hook, deeper) for item in value]}
    if kind is set or kind is frozenset:
        return {"$set": sorted((encode(item, hook, deeper) for item in value),
                               key=lambda item: repr(item))}
    if kind is dict:
        if all(type(key) is str and not key.startswith("$") for key in value):
            return {key: encode(item, hook, deeper) for key, item in value.items()}
        return {"$items": [[encode(key, hook, deeper), encode(item, hook, deeper)]
                           for key, item in value.items()]}
    if kind is bytes:
        return {"$bytes": base64.b64encode(value).decode("ascii")}
    return hook(value)


def snapshot(value, hook, _depth: int = 0):
    """A dataclass instance as the tagged copy of its fields, or None when `value`
    is not one. Fields are encoded the same way, so a nested object is a reference."""
    if dataclasses.is_dataclass(value) and not isinstance(value, type):
        return {"$ns": {"type": type(value).__name__,
                        "fields": {field.name: encode(getattr(value, field.name), hook, _depth + 1)
                                   for field in dataclasses.fields(value)}}}
    return None


def decode(data, resolve, _depth: int = 0):
    """The value `encode` described. `resolve(tag, payload)` answers for the tags
    that stand for objects (`$ref`, `$m`); the rest are rebuilt here."""
    if _depth > MAX_DEPTH:
        raise WireError(f"a message nests more than {MAX_DEPTH} levels deep")
    kind = type(data)
    if data is None or kind is bool or kind is int or kind is float or kind is str:
        return data
    deeper = _depth + 1
    if kind is list:
        return [decode(item, resolve, deeper) for item in data]
    if kind is not dict:
        raise WireError(f"the message holds a {kind.__name__}, which JSON does not carry")
    tag = next((key for key in data if key.startswith("$")), None)
    if tag is None:
        return {key: decode(item, resolve, deeper) for key, item in data.items()}
    if tag == "$t":
        return tuple(decode(item, resolve, deeper) for item in data["$t"])
    if tag == "$set":
        return {_hashable(decode(item, resolve, deeper)) for item in data["$set"]}
    if tag == "$bytes":
        return base64.b64decode(data["$bytes"])
    if tag == "$float":
        return float(data["$float"])
    if tag == "$items":
        return {_hashable(decode(key, resolve, deeper)): decode(item, resolve, deeper)
                for key, item in data["$items"]}
    if tag == "$ns":
        body = data["$ns"]
        return SimpleNamespace(**{name: decode(item, resolve, deeper)
                                  for name, item in body["fields"].items()})
    if tag in ("$ref", "$m"):
        return resolve(tag, data)
    raise WireError(f"the message carries a tag this protocol does not have: {tag!r}")


def _hashable(value):
    """A list cannot be a set member or a key; the tuple the design had can."""
    return tuple(_hashable(item) for item in value) if type(value) is list else value
