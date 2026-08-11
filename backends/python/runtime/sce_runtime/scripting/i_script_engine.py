# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""IScriptEngine — script engine contract, 1:1 port of C++ `IScriptEngine.h`.

Mirrors `sce_rust_runtime::IScriptEngine` and the Go / Kotlin / C11 equivalents.
Generated state machine code calls into this trait through the
`ScriptEngineProvider` singleton (no runtime dependency injection); each
`Engine` instance creates / destroys its own session id.

The Lua-backed default implementation lives in `lua_engine.py`. The trait
is intentionally engine-neutral so a future QuickJS-backed impl could
register itself in the same provider slot.
"""

from __future__ import annotations

from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Callable, Dict, List, Optional

from ..io_processors import IoProcessorDescriptor


class ScriptValueKind(Enum):
    """Discriminator for `ScriptValue`. Matches the C++ `std::variant`
    case set so DOM payloads can round-trip without losing type."""

    NULL = "null"
    UNDEFINED = "undefined"
    BOOL = "bool"
    INT = "int"
    DOUBLE = "double"
    STRING = "string"
    ARRAY = "array"
    OBJECT = "object"
    DOM = "dom"


@dataclass
class ScriptValue:
    """A value that can cross the script-engine boundary.

    Matches `backends/rust/runtime/src/scripting/i_script_engine.rs::ScriptValue`
    field-for-field. Generated code converts datamodel values to/from
    `ScriptValue` at assignment and guard-evaluation sites. The kind tag
    is explicit so JavaScript's `null` vs `undefined` distinction
    survives across the Lua boundary (Lua collapses both to `nil`)."""

    kind: ScriptValueKind = ScriptValueKind.NULL
    bool_val: bool = False
    int_val: int = 0
    double_val: float = 0.0
    string_val: str = ""
    array_val: List["ScriptValue"] = field(default_factory=list)
    object_val: Dict[str, "ScriptValue"] = field(default_factory=dict)
    dom_val: str = ""

    @staticmethod
    def null() -> "ScriptValue":
        return ScriptValue(kind=ScriptValueKind.NULL)

    @staticmethod
    def undefined() -> "ScriptValue":
        return ScriptValue(kind=ScriptValueKind.UNDEFINED)

    @staticmethod
    def of(value: Any) -> "ScriptValue":
        """W3C SCXML 5.10 — coerce a Python value into a `ScriptValue`
        for the script-engine boundary. `None` always maps to NULL (Lua
        collapses null/undefined to `nil` so the round-trip is lossy at
        that side; callers needing the distinction must construct
        `ScriptValue.undefined()` explicitly)."""
        if value is None:
            return ScriptValue.null()
        if isinstance(value, bool):
            return ScriptValue(kind=ScriptValueKind.BOOL, bool_val=value)
        if isinstance(value, int):
            return ScriptValue(kind=ScriptValueKind.INT, int_val=value)
        if isinstance(value, float):
            return ScriptValue(kind=ScriptValueKind.DOUBLE, double_val=value)
        if isinstance(value, str):
            return ScriptValue(kind=ScriptValueKind.STRING, string_val=value)
        if isinstance(value, list):
            return ScriptValue(
                kind=ScriptValueKind.ARRAY,
                array_val=[ScriptValue.of(v) for v in value],
            )
        if isinstance(value, dict):
            return ScriptValue(
                kind=ScriptValueKind.OBJECT,
                object_val={str(k): ScriptValue.of(v) for k, v in value.items()},
            )
        # Fallback: stringify any other Python type so it round-trips
        # through the Lua engine without losing data.
        return ScriptValue(kind=ScriptValueKind.STRING, string_val=str(value))

    def to_python(self) -> Any:
        """W3C SCXML 5.10 — unwrap to the equivalent Python type for
        consumption by the engine (`_event.data` and friends)."""
        if self.kind is ScriptValueKind.NULL or self.kind is ScriptValueKind.UNDEFINED:
            return None
        if self.kind is ScriptValueKind.BOOL:
            return self.bool_val
        if self.kind is ScriptValueKind.INT:
            return self.int_val
        if self.kind is ScriptValueKind.DOUBLE:
            return self.double_val
        if self.kind is ScriptValueKind.STRING:
            return self.string_val
        if self.kind is ScriptValueKind.ARRAY:
            return [v.to_python() for v in self.array_val]
        if self.kind is ScriptValueKind.OBJECT:
            return {k: v.to_python() for k, v in self.object_val.items()}
        if self.kind is ScriptValueKind.DOM:
            return self.dom_val
        return None

    def to_lua_literal(self) -> str:
        """W3C SCXML 5.5: Render this value as Lua source text. Mirrors
        Rust `ScriptValue::to_lua_literal` (used by the AOT donedata
        emit). The output is re-eval'd by `set_current_event` when the
        parent's `done.state.<id>` / `done.invoke.<id>` event is
        dispatched, so the round trip must preserve string identity
        (test294: `<content>'foo'</content>` must surface on
        `_event.data` as the string `"foo"`, not the global lookup of
        an identifier `foo`)."""
        if self.kind is ScriptValueKind.NULL or self.kind is ScriptValueKind.UNDEFINED:
            return "nil"
        if self.kind is ScriptValueKind.BOOL:
            return "true" if self.bool_val else "false"
        if self.kind is ScriptValueKind.INT:
            return str(self.int_val)
        if self.kind is ScriptValueKind.DOUBLE:
            return repr(self.double_val)
        if self.kind is ScriptValueKind.STRING:
            escaped = self.string_val.replace("\\", "\\\\").replace("'", "\\'")
            return f"'{escaped}'"
        if self.kind is ScriptValueKind.ARRAY:
            return "{" + ", ".join(v.to_lua_literal() for v in self.array_val) + "}"
        if self.kind is ScriptValueKind.OBJECT:
            return (
                "{"
                + ", ".join(
                    f'["{k}"] = {v.to_lua_literal()}'
                    for k, v in self.object_val.items()
                )
                + "}"
            )
        return "nil"

    def to_bool(self) -> bool:
        """W3C SCXML B.2.3 — ECMAScript truthiness. Falsy: null,
        undefined, false, 0, NaN, empty string. Everything else truthy.
        Used when a guard's `evaluate_expression` returns a non-bool
        type."""
        if self.kind is ScriptValueKind.NULL or self.kind is ScriptValueKind.UNDEFINED:
            return False
        if self.kind is ScriptValueKind.BOOL:
            return self.bool_val
        if self.kind is ScriptValueKind.INT:
            return self.int_val != 0
        if self.kind is ScriptValueKind.DOUBLE:
            return self.double_val != 0.0 and self.double_val == self.double_val
        if self.kind is ScriptValueKind.STRING:
            return bool(self.string_val)
        return True


class ScriptError(Exception):
    """Base class for script-engine failures (W3C SCXML 5.9 maps these
    onto `error.execution`)."""


class ScriptSyntaxError(ScriptError):
    """Expression / script body failed to parse."""


class ScriptRuntimeError(ScriptError):
    """Evaluation raised at runtime (TypeError, ReferenceError, …)."""


class SessionNotFoundError(ScriptError):
    """Operation referenced an unknown session id."""


class VariableNotDeclaredError(ScriptError):
    """W3C SCXML 5.4 — `<assign location>` referenced a name absent from
    the document's static datamodel schema. The generated engine
    translates this into `error.execution`."""


class ReadOnlySystemVariableError(ScriptError):
    """Caller attempted to overwrite `_event` / `_sessionid` / etc."""


NativeMethod = Callable[[List[ScriptValue]], ScriptValue]
StateQueryCallback = Callable[[str], bool]


@dataclass
class SetCurrentEventArgs:
    """Parameter object for the W3C SCXML 5.10 `set_current_event` boundary.

    Bundles the seven `_event.*` metadata fields (name + 6 metadata) that
    every script engine impl must surface before guard evaluation / action
    execution. Cross-language siblings: `SCE::SetCurrentEventArgs` in C++
    and `sce_rust_runtime::SetCurrentEventArgs` in Rust."""

    event_name: str
    event_data: str = ""
    event_type: str = "internal"
    send_id: str = ""
    origin: str = ""
    origin_type: str = ""
    invoke_id: str = ""


class IScriptEngine(ABC):
    """Script engine contract — 1:1 port of C++ `IScriptEngine.h` and
    `sce_rust_runtime::IScriptEngine`. Implementations (`LuaScriptEngine`
    in `lua_engine.py`, future QuickJS-backed) provide ECMAScript
    evaluation for the W3C SCXML B.2 datamodel. Generated `*_sm.py`
    modules invoke this through `ScriptEngineProvider.get()` — there is
    no runtime DI."""

    # ── Core script execution ──────────────────────────────────────

    @abstractmethod
    def execute_script(self, session_id: str, script: str) -> ScriptValue:
        """W3C SCXML 5.5 — run a `<script>` body. Returns the value of
        the last expression (or NULL when the script is statement-only).
        Syntax / runtime errors raise the matching `ScriptError`
        subclass."""

    @abstractmethod
    def evaluate_expression(self, session_id: str, expression: str) -> ScriptValue:
        """W3C SCXML 5.3 — evaluate an expression (cond / `<assign expr>`
        / `<param expr>` / `<send eventexpr>` / …) and return the
        result."""

    @abstractmethod
    def validate_expression(self, session_id: str, expression: str) -> bool:
        """Parse-only check used by the datamodel loader to surface
        syntax errors during `<data>` registration."""

    # ── Variable management ────────────────────────────────────────

    @abstractmethod
    def declare_variable(
        self, session_id: str, name: str, initial_value: ScriptValue
    ) -> None:
        """W3C SCXML 5.3 — register `<data id>` schema and bind the
        initial value. Called once per `<data>` element during
        datamodel init. Subsequent `<assign>` operations target the
        same name via `set_variable`."""

    @abstractmethod
    def set_variable(
        self, session_id: str, name: str, value: ScriptValue
    ) -> None:
        """W3C SCXML 5.3 — assign a `ScriptValue` to a declared
        location. Raises `VariableNotDeclaredError` when the name is
        absent from the session schema (mirrors C++ behaviour)."""

    @abstractmethod
    def get_variable(self, session_id: str, name: str) -> ScriptValue:
        """W3C SCXML 5.3 — read a variable. Returns UNDEFINED when the
        name is declared but unassigned, NULL when the lookup follows a
        dotted path into an absent member."""

    @abstractmethod
    def set_variable_as_dom(
        self, session_id: str, name: str, xml_content: str
    ) -> None:
        """W3C SCXML B.2 — parse an XML document and bind it as a
        DOM-style variable. Used by `<data src="…xml">` and
        `<content>` with XML payloads."""

    @abstractmethod
    def has_variable(self, session_id: str, name: str) -> bool:
        """Variable declared in the session scope (W3C SCXML 4.6 / 6.4
        — `<foreach>` distinguishes declared-but-empty from undeclared)."""

    # ── SCXML-specific features ────────────────────────────────────

    @abstractmethod
    def setup_system_variables(
        self,
        session_id: str,
        session_name: str,
        io_processors: List[IoProcessorDescriptor],
    ) -> None:
        """W3C SCXML 5.10 — bind `_sessionid`, `_name`, `_ioprocessors`
        into the session scope. Called once per session right after
        `create_session`.

        The descriptors arrive fully resolved from
        `sce_runtime.io_processors.build`. An implementation files each one
        under its name with its location and invents neither, so
        `_ioprocessors` reads identically whichever engine backs the
        session."""

    @abstractmethod
    def set_current_event(
        self,
        session_id: str,
        args: "SetCurrentEventArgs",
    ) -> None:
        """W3C SCXML 5.10 — bind the `_event` system variable for the
        currently-processing event. Called by the runtime before guard
        evaluation and action execution for each event. The 7 metadata
        fields are bundled into a [SetCurrentEventArgs] mirroring the
        C++ `SCE::SetCurrentEventArgs` struct and the Rust
        `SetCurrentEventArgs<'a>` parameter object."""

    # ── Global functions / native bindings ─────────────────────────

    @abstractmethod
    def register_global_function(
        self, function_name: str, callback: NativeMethod
    ) -> bool:
        """Expose a native Python callback at script-engine scope.
        Used for `log_hook` integration and similar."""

    @abstractmethod
    def set_state_query_callback(
        self, session_id: str, callback: Optional[StateQueryCallback]
    ) -> None:
        """W3C SCXML 5.9.2 — register the `In(state)` predicate
        resolver for the session. Passing `None` unregisters."""

    # ── Engine + session lifecycle ─────────────────────────────────

    @abstractmethod
    def initialize(self) -> bool:
        """Initialise the script engine. Idempotent; returns `True`
        once the engine is ready."""

    @abstractmethod
    def shutdown(self) -> None:
        """Release all sessions and engine resources."""

    @abstractmethod
    def is_initialized(self) -> bool: ...

    @abstractmethod
    def reset(self) -> None:
        """Destroy all sessions, clear registered callbacks, then
        re-initialise. Used for test isolation."""

    @abstractmethod
    def create_session(self, session_id: str) -> None: ...

    @abstractmethod
    def destroy_session(self, session_id: str) -> None: ...

    @abstractmethod
    def has_session(self, session_id: str) -> bool: ...

    # ── Engine info ────────────────────────────────────────────────

    @abstractmethod
    def get_engine_info(self) -> str: ...
