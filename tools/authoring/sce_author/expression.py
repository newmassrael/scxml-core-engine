"""The expression a precondition phrase is read as.

A pack's table says what a phrase of the specification means as an expression
over the inputs it declares (`"not powered": "!poweredUp"`). The table used to
be read as a bag of identifiers: whatever named a declared input was read, and
everything else -- an operator, a stray bracket, a misspelt input -- was
ignored without a word. A phrase whose input was misspelt read NO input, and
the condition the specification states was missing from every check with
nothing to say so.

The language is the smallest one the table has ever written, and it is the only
one this package reads:

    expression  :=  or
    or          :=  and ( "||" and )*
    and         :=  not ( "&&" not )*
    not         :=  "!" not  |  atom
    atom        :=  name  |  "(" expression ")"
    name        :=  [A-Za-z_][A-Za-z0-9_]*

`true` and `false`, in any case, are the two names that read no input.
"""

from __future__ import annotations

import re

LITERALS = frozenset({"true", "false"})

_TOKEN = re.compile(r"\s*(?:(?P<name>[A-Za-z_][A-Za-z0-9_]*)|(?P<op>&&|\|\||[!()]))")


class ExpressionError(ValueError):
    """The expression is not in the language above, and where it stops being."""


def _tokens(expression: str) -> list[tuple[str, str, int]]:
    out, at = [], 0
    while True:
        rest = expression[at:]
        stripped = rest.lstrip()
        if not stripped:
            break
        match = _TOKEN.match(expression, at)
        if match is None:
            column = at + (len(rest) - len(stripped)) + 1
            raise ExpressionError(
                f"{expression!r} has {stripped[0]!r} at column {column}, "
                f"which is neither a name nor one of ! && || ( )")
        kind = "name" if match.group("name") else "op"
        out.append((kind, match.group(kind), match.start(kind) + 1))
        at = match.end()
    return out


def names(expression: str) -> tuple[str, ...]:
    """Every name the expression writes, in the order it writes them, or
    `ExpressionError` when it is not an expression."""
    tokens = _tokens(expression)
    if not tokens:
        raise ExpressionError("the expression is empty")
    found: list[str] = []
    position = 0

    def fail(wanted: str) -> ExpressionError:
        if position < len(tokens):
            where = f"at column {tokens[position][2]} ({tokens[position][1]!r})"
        else:
            where = "at its end"
        return ExpressionError(f"{expression!r} wants {wanted} {where}")

    def parse_or():
        nonlocal position
        parse_and()
        while position < len(tokens) and tokens[position][1] == "||":
            position += 1
            parse_and()

    def parse_and():
        nonlocal position
        parse_not()
        while position < len(tokens) and tokens[position][1] == "&&":
            position += 1
            parse_not()

    def parse_not():
        nonlocal position
        while position < len(tokens) and tokens[position][1] == "!":
            position += 1
        parse_atom()

    def parse_atom():
        nonlocal position
        if position >= len(tokens):
            raise fail("a name or ( ")
        kind, text, _ = tokens[position]
        if kind == "name":
            found.append(text)
            position += 1
        elif text == "(":
            position += 1
            parse_or()
            if position >= len(tokens) or tokens[position][1] != ")":
                raise fail(") to close the bracket")
            position += 1
        else:
            raise fail("a name or ( ")

    parse_or()
    if position != len(tokens):
        raise fail("an operator, or nothing more")
    return tuple(found)
