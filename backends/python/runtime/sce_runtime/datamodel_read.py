# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""Typed reads of a live datamodel variable.

The counterpart to the datamodel initialisation `initialize_datamodel`
performs. These take a value back out in the host's own type, so a generated
machine can answer a question about its own datamodel without the caller
holding a script engine, a session id and the variable's name spelled as a
string.

ARCHITECTURE.md: Zero Duplication -- the same four readers back the C++
``DataModelReadHelper``, the Rust ``helpers::datamodel_read``, the Go
``ReadDatamodel*`` and the Kotlin ``DatamodelRead`` surface, and the C11
template inlines the same rules, so every backend's accessor answers alike.
Three hand a value back in a host type; the fourth hands a structured one over
as JSON, because there is no host type six languages share for it.

Why the read goes to the engine rather than to a copy: a ``<data>`` variable
with an initialiser is owned by the script engine for the life of the session
-- ``<assign>`` writes there and guards read from there. Anything the generated
policy kept alongside it would be a second representation of one variable,
wrong from the first ``<assign>`` onwards.

Why the answer is optional: the session may not be initialised yet, the
variable may have been assigned a value of another type mid-run, or the engine
may refuse. All three mean the same thing to a consumer -- the machine cannot
answer that right now.
"""

from typing import Any, Optional

from .scripting.i_script_engine import ScriptValue, ScriptValueKind

__all__ = ["read_int", "read_string", "read_bool", "read_json"]


def _current(engine: Any, session_id: Optional[str], name: str) -> Optional[ScriptValue]:
    """Fetch a variable's value, or ``None`` if it cannot be read."""
    if engine is None or not session_id:
        return None
    try:
        return engine.get_variable(session_id, name)
    except Exception:
        return None


def read_int(engine: Any, session_id: Optional[str], name: str) -> Optional[int]:
    """Read an integer-declared datamodel variable.

    A whole-valued double is accepted as well as an int, and that leniency is
    about engines rather than about types: Lua 5.2-family bindings have no
    integer subtype at all, so the same authored ``40`` crosses back as an int
    from one engine and as a double from another. Refusing the second would
    make the accessor's answer depend on which engine the deployment injected,
    which is exactly what a typed accessor exists to hide. A fractional value
    is a different number and is refused.
    """
    # §scxml-5.3: the declared variable's current value, in the host's type.
    value = _current(engine, session_id, name)
    if value is None:
        return None
    if value.kind is ScriptValueKind.INT:
        return value.int_val
    if value.kind is ScriptValueKind.DOUBLE and value.double_val.is_integer():
        return int(value.double_val)
    return None


def read_string(engine: Any, session_id: Optional[str], name: str) -> Optional[str]:
    """Read a string-declared datamodel variable.

    Strict: a number that happens to print as text is not a string, and
    coercing it would let a consumer read a value the datamodel never held.
    """
    # §scxml-5.3: the declared variable's current value, in the host's type.
    value = _current(engine, session_id, name)
    if value is None or value.kind is not ScriptValueKind.STRING:
        return None
    return value.string_val


def read_bool(engine: Any, session_id: Optional[str], name: str) -> Optional[bool]:
    """Read a boolean-declared datamodel variable.

    Strict, and deliberately not the SCXML truthiness rule: that rule answers a
    question every value has an answer to. This one answers whether the
    variable is holding a boolean, and a consumer inspecting a declared flag
    wants to be told when it is not.
    """
    # §scxml-5.3: the declared variable's current value, in the host's type.
    value = _current(engine, session_id, name)
    if value is None or value.kind is not ScriptValueKind.BOOL:
        return None
    return value.bool_val


def read_json(engine: Any, session_id: Optional[str], name: str) -> Optional[str]:
    """Read an array- or object-declared datamodel variable, as JSON text.

    Why the engine serialises it rather than this function: every engine SCE
    can be given carries ``JSON.stringify`` -- the clause cited in the body is
    what requires it -- and that one serialiser is the answer. Walking the
    ``ScriptValue`` tree here would be a
    second serialiser disagreeing with the first, and it would have to agree
    with five other backends' walkers besides. What the engine produces is
    stable for that engine (the shared Lua builtin sorts object keys; an
    ECMAScript engine emits property order), and stability is what a consumer
    diffing two reads needs. It is the engine's encoding, not a normal form
    across engines, which is the same shape of promise ``read_int`` makes
    about numeric width.

    Why this expression survives either engine family: ``evaluate_expression``
    takes the ENGINE's language, not the document's -- a Lua-backed session is
    handed Lua. ``JSON.stringify(x)`` is spelled the same in both, member
    access and a call, in a language the datamodel clause requires that exact
    name to exist in.

    Why the answer is strict: the scalar readers refuse a value of another type
    and so does this one. A variable declared ``[...]`` and later assigned
    ``5`` answers ``None``, not ``"5"``. The test is the first character of the
    serialiser's output, where JSON's grammar puts the type -- ``[`` opens an
    array and ``{`` an object, and nothing else stringifies to either.
    """
    # §scxml-5.3: the declared variable's value, in the encoding §scxml-B-2
    # already requires the engine to produce. `name` reaches here only for a
    # name the classifier confirmed is a bare identifier -- see
    # `analyzer::reachable_as_an_expression`.
    if engine is None or not session_id:
        return None
    try:
        value = engine.evaluate_expression(session_id, f"JSON.stringify({name})")
    except Exception:
        return None
    if value is None or value.kind is not ScriptValueKind.STRING:
        return None
    json = value.string_val
    if not json or json[0] not in "[{":
        return None
    return json
