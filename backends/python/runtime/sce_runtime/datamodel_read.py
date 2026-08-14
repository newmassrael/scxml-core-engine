# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""Typed reads of a live datamodel variable.

The counterpart to the datamodel initialisation `initialize_datamodel`
performs. These take a value back out in the host's own type, so a generated
machine can answer a question about its own datamodel without the caller
holding a script engine, a session id and the variable's name spelled as a
string.

ARCHITECTURE.md: Zero Duplication -- the same three coercions back the C++
``DataModelReadHelper``, the Rust ``helpers::datamodel_read``, the Go
``ReadDatamodel*`` and the Kotlin ``DatamodelRead`` surface, so every backend's
accessor answers alike.

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

__all__ = ["read_int", "read_string", "read_bool"]


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
