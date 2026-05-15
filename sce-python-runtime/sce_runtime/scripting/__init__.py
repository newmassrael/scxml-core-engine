# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""sce_runtime.scripting — script engine layer (W3C SCXML 5.3 / 5.5 / 5.10).

1:1 family port of the C++ / Rust / Go / Kotlin / C11 ScriptEngine
pattern. Generated `*_sm.py` modules route every author expression
through this layer instead of Python `eval()`. The default backend is
`LuaScriptEngine` (lupa); users can swap implementations via
`set_script_engine` before any state machine is constructed."""

from .i_script_engine import (
    IScriptEngine,
    NativeMethod,
    ReadOnlySystemVariableError,
    ScriptError,
    ScriptRuntimeError,
    ScriptSyntaxError,
    ScriptValue,
    ScriptValueKind,
    SessionNotFoundError,
    StateQueryCallback,
    VariableNotDeclaredError,
)
from .lua_engine import LuaScriptEngine
from .provider import (
    ScriptEngineAlreadyRegisteredError,
    get,
    reset_for_tests,
    set_script_engine,
)

__all__ = [
    "IScriptEngine",
    "LuaScriptEngine",
    "NativeMethod",
    "ReadOnlySystemVariableError",
    "ScriptEngineAlreadyRegisteredError",
    "ScriptError",
    "ScriptRuntimeError",
    "ScriptSyntaxError",
    "ScriptValue",
    "ScriptValueKind",
    "SessionNotFoundError",
    "StateQueryCallback",
    "VariableNotDeclaredError",
    "get",
    "reset_for_tests",
    "set_script_engine",
]
