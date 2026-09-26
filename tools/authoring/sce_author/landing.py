"""Where an output rule lands a document value, and whether it can land every one.

The output side's peer of `delivery`. An output rule takes what the document
produced and writes it at an address: through `map`, which names each value's
symbol, or through `passthrough`, which writes it as it is. `verify` does that
once per case and refuses a case whose value has nowhere to go; `check` asks
the same question of the binding before any case runs, with the functions
below, so the two cannot disagree about which bindings are well formed.

⚠ Why `check` asks at all. Each refusal here used to exist in `verify` alone,
so a binding `check` had passed came back with every case unjudged -- "the
binding says neither 'map' nor 'passthrough'", "`hold_last` holds a MAPPED
value", "the binding's map has no entry" -- a verdict about the binding that
arrived as a failure to run. Measured 2026-09-22 by writing each shape into a
binding the fixture otherwise accepts: `check` returned nothing for all of
them, and `verify` stopped on every one.
"""

from __future__ import annotations

import re


def names_value(key, value) -> bool:
    """Whether a binding key -- in a `map` or a `when` -- names the value the
    document produced.

    ⚠ A document output declared `bool` arrives as True or False, and a
    binding writes its two cases as `0`/`1`, as `true`/`false`, or as YAML's
    own booleans: three spellings of the same two cases, all of which the
    schema admits and `check` accepts. Matching one spelling made a boolean
    output unmappable under the other two. Measured 2026-09-22: two authors
    writing from a brief both wrote `true`/`false`, `check` passed both
    bindings, and every case of both was then unjudged with "the binding's
    map has no entry" -- a verdict about the verifier, reported as one about
    the binding.
    """
    if isinstance(value, bool):
        if isinstance(key, bool):
            return key is value
        return str(key).strip().lower() in (("1", "true") if value else ("0", "false"))
    if isinstance(key, bool):
        # A truth value names no number and no symbol.
        return False
    return str(key) == str(value)


def mapped(table: dict, value) -> tuple[bool, object]:
    """The entry of a binding's `map` that names `value`: (True, what it
    writes), or (False, None) when no key names it."""
    for key, written in table.items():
        if names_value(key, value):
            return True, written
    return False, None


def form_refusal(rule: dict) -> str:
    """Why this output rule cannot land any value at all, or ''.

    A rule with no address yet, or one that lands nowhere by declaration
    (`internal`), is not asked: the first is reported as unresolved on its
    own, and the second writes nothing.
    """
    if rule.get("internal") or rule.get("unresolved"):
        return ""
    if rule.get("hold_last") and "map" not in rule:
        return ("`hold_last` holds a MAPPED value, and this rule has no map -- "
                "there is nothing a value could fail to be found in")
    if "map" not in rule and not rule.get("passthrough"):
        return ("the binding says neither 'map' nor 'passthrough', so where "
                "its value goes is undefined")
    return ""


_NUMBER = re.compile(r"-?(\d+\.\d*|\.\d+|\d+)([eE][-+]?\d+)?")


def expression_values(expr: str | None) -> list | None:
    """Every value an output expression can produce, when that is all literals.

    Known exactly for a literal, and for a conditional `c ? a : b` whose two
    arms are known the same way, at any depth: a value table written as nested
    conditionals. Anything else -- an arm that reads an input, calls a
    function, does arithmetic -- is None, and nothing is claimed about it.

    ⚠ Why at all: measured 2026-09-26, a writer produced an event code and `''`
    and keyed its map by the INPUT's symbol (`FAILURE`), so no value it
    produced had an entry. `check` passed it, and `verify` then left every
    case unjudged -- the refusal was there to make before any case ran.
    """
    if expr is None:
        return None
    body = _unwrap(expr.strip())
    literal = _literal(body)
    if literal is not _NOT_LITERAL:
        return [literal]
    split = _conditional(body)
    if split is None:
        return None
    arms = [expression_values(arm) for arm in split]
    if any(arm is None for arm in arms):
        return None
    return list(dict.fromkeys(v for arm in arms for v in arm))


_NOT_LITERAL = object()


def _literal(body: str):
    if len(body) >= 2 and body[0] == body[-1] and body[0] in "'\"" \
            and _string_end(body, 0) == len(body) - 1:
        return body[1:-1]
    if _NUMBER.fullmatch(body):
        number = float(body)
        return int(number) if number.is_integer() and not re.search(r"[.eE]", body) else number
    if body in ("true", "false"):
        return body == "true"
    return _NOT_LITERAL


def _string_end(text: str, start: int) -> int:
    """Index of the quote closing the string opened at `start`, or -1."""
    quote, i = text[start], start + 1
    while i < len(text):
        if text[i] == "\\":
            i += 2
            continue
        if text[i] == quote:
            return i
        i += 1
    return -1


def _top_level(text: str):
    """(index, character) for each character outside strings and brackets,
    and for each bracket that opens or closes at the outermost level."""
    depth, i = 0, 0
    while i < len(text):
        ch = text[i]
        if ch in "'\"":
            end = _string_end(text, i)
            if end < 0:
                return
            i = end + 1
            continue
        if ch in "([{":
            if depth == 0:
                yield i, ch
            depth += 1
        elif ch in ")]}":
            depth -= 1
            if depth == 0:
                yield i, ch
        elif depth == 0:
            yield i, ch
        i += 1


def _unwrap(body: str) -> str:
    """The expression without brackets that enclose all of it.

    `(a) ? (b) : (c)` starts and ends with a bracket that encloses nothing
    whole: it is unwrapped only when the bracket at the start is the one that
    closes at the end, which is when no character is outside it.
    """
    while body.startswith("(") and body.endswith(")"):
        # Enclosed whole, the only outermost characters are the two brackets.
        outside = [i for i, _ in _top_level(body)]
        if outside != [0, len(body) - 1]:
            return body
        body = body[1:-1].strip()
    return body


def _conditional(body: str) -> tuple[str, str] | None:
    """The two arms of a top-level `c ? a : b`, or None if it is not one.

    The first top-level `?` opens it; its `:` is the one that closes as many
    nested `?` as opened after it, which is how `a ? b ? x : y : z` reads.
    """
    marks = list(_top_level(body))
    question = next((i for i, ch in marks if ch == "?"), None)
    if question is None:
        return None
    pending = 0
    for i, ch in marks:
        if i <= question:
            continue
        if ch == "?":
            pending += 1
        elif ch == ":":
            if pending == 0:
                return body[question + 1:i].strip(), body[i + 1:].strip()
            pending -= 1
    return None


def produces(rule: dict, sce_type: str | None, sends, expr: str | None = None) -> list | None:
    """Every value this rule can be handed, when the document says; else None.

    `sce_type` is what the document declares the output to be, `sends` its
    `(event name, processor)` pairs, and `expr` the output's expression, if
    it has one. Three answers are known before a run: a `bool` output is True
    or False, a rule reading a send's EVENT NAME is handed one of the names
    the document writes literally, or its `when_nothing_sent`, and an output
    whose expression yields only literals is one of them
    (`expression_values`). A computed number has no list of values, and
    neither does a send's `param` or `content`.

    ⚠ The send side is read the way `verify` reads it: the LAST send to the
    rule's processor in a case is the one it takes, whatever its name, so
    every name the document sends there is one this rule can be handed. A
    send whose name is computed (`eventexpr`) is left out, since nothing says
    which names it has; the literal ones are still owed an entry.
    """
    sent = rule.get("sent")
    if sent is not None:
        if sent.get("param") or sent.get("content"):
            return None
        wanted = sent.get("processor")
        names = [event for event, processor in sends
                 if event and (wanted is None or processor == wanted)]
        return list(dict.fromkeys([*names, rule.get("when_nothing_sent")]))
    literals = expression_values(expr)
    if literals is not None:
        return literals
    if sce_type == "bool":
        return [True, False]
    return None


def unmapped(rule: dict, values) -> list:
    """The values in `values` this rule's map has no entry for.

    Empty when the rule does not map, when nothing says what it can be
    handed, and under `hold_last` -- where a value with no entry is the one
    that keeps what the position held, on purpose.
    """
    if "map" not in rule or values is None or rule.get("hold_last"):
        return []
    return [v for v in values if not mapped(rule["map"], v)[0]]
