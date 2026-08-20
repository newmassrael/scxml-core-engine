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


def _json_string(text: str) -> str:
    """A JSON string literal. Ports C++ `DoneDataHelper::escapeJsonString`.

    Spelled out rather than delegating to `json.dumps` so the escape set
    is the one the other five backends apply — `json.dumps` also escapes
    non-ASCII by default, which would make the same payload differ
    between backends byte for byte.
    """
    escaped = (
        text.replace("\\", "\\\\")
        .replace('"', '\\"')
        .replace("\n", "\\n")
        .replace("\r", "\\r")
        .replace("\t", "\\t")
        .replace("\b", "\\b")
        .replace("\f", "\\f")
    )
    return f'"{escaped}"'


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

    def to_wire_string(self) -> str:
        """W3C SCXML C.2 — this value as the text a form-encoded param
        carries.

        The BasicHTTP Event I/O Processor sends each `<param>` as one
        `name=value` pair, so the value crosses as *text* and the
        receiving end hands that text to `_event.data`; no script engine
        reads it at either end. That is why this is neither of its two
        neighbours: `to_json_literal` would wrap a string in quotes that
        are not part of it, and an engine literal (this channel's lives
        inside its own engine, `lua_engine._python_to_lua_literal`)
        would put the sender's *language* on the wire.

        The rendering is ECMAScript's `String(value)` (§scxml-B-1 makes
        the data model ECMAScript) with the two amendments C++
        `ScriptResultUtils::resultToString` already made: absence renders
        empty rather than as a word (§scxml-C-1), and a structured value
        renders as JSON, because a receiver that is not a script engine
        has no other reading of it.

        What this replaced was `str(value.to_python())`, which is
        Python's spelling and not the document's — `True` for a boolean
        the other channels spell `true`, `None` for an absence the wire
        reads as empty, `[1, 2]` with a space JSON does not have.
        """
        if self.kind is ScriptValueKind.NULL or self.kind is ScriptValueKind.UNDEFINED:
            return ""
        if self.kind is ScriptValueKind.BOOL:
            return "true" if self.bool_val else "false"
        if self.kind is ScriptValueKind.INT:
            return str(self.int_val)
        if self.kind is ScriptValueKind.DOUBLE:
            value = self.double_val
            if value != value:
                return "NaN"
            if value == float("inf"):
                return "Infinity"
            if value == float("-inf"):
                return "-Infinity"
            if value == int(value) and abs(value) < 1e15:
                # ECMAScript String(5) is "5"; a `.0` tail is Python's
                # spelling of the number, not the document's.
                return str(int(value))
            return repr(value)
        if self.kind is ScriptValueKind.STRING:
            # Already text: quoting it would deliver characters the
            # document never wrote.
            return self.string_val
        if self.kind is ScriptValueKind.DOM:
            # The receiving end parses XML from document text.
            return self.dom_val or ""
        return self.to_json_literal()

    def to_json_literal(self) -> str:
        """Render this value for a wire that leaves the ECMAScript data
        model.

        The clause cited in the body names JSON as that serialisation —
        it is what the BasicHTTP Event I/O Processor sends — and an event
        payload always leaves the data model: the reader is another
        dequeue, often another session, in a mesh another process running
        another backend.

        This is the counterpart of an engine literal, and the difference
        is the point. A literal is *source*, so reading it back requires
        the receiver to RUN it — which made `_event.data` mean one thing
        on a Lua backend and another on a JavaScript one, and made any
        payload executable at the far end. Ports the C++
        `scriptValueToJson` in `sce/src/common/EventDataHelper.cpp`.

        Object keys are sorted so equal content produces equal bytes,
        matching the C++ `std::map` original and the Rust/Go ports.
        """
        # §scxml-B-2-9: a value that has to leave the ECMAScript data model
        # is serialized to JSON, which reconstructs it in full rather than
        # falling back to a lossy platform format.
        if self.kind is ScriptValueKind.NULL or self.kind is ScriptValueKind.UNDEFINED:
            # JSON has no `undefined`; the C++ port maps both to null.
            return "null"
        if self.kind is ScriptValueKind.BOOL:
            return "true" if self.bool_val else "false"
        if self.kind is ScriptValueKind.INT:
            return str(self.int_val)
        if self.kind is ScriptValueKind.DOUBLE:
            value = self.double_val
            if value != value or value in (float("inf"), float("-inf")):
                # RFC 8259 has no spelling for NaN or the infinities.
                return "null"
            if value == int(value) and abs(value) < 1e15:
                return str(int(value))
            return repr(value)
        if self.kind is ScriptValueKind.STRING:
            return _json_string(self.string_val)
        if self.kind is ScriptValueKind.ARRAY:
            return "[" + ",".join(v.to_json_literal() for v in self.array_val) + "]"
        if self.kind is ScriptValueKind.OBJECT:
            return (
                "{"
                + ",".join(
                    f"{_json_string(k)}:{self.object_val[k].to_json_literal()}"
                    for k in sorted(self.object_val)
                )
                + "}"
            )
        if self.kind is ScriptValueKind.DOM:
            # The data model reads XML into a DOM at the *receiving* end,
            # from the document text — so a DOM that reaches here crosses
            # as that text.
            return _json_string(self.dom_val or "")
        return "null"

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
