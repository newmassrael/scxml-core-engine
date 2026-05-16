# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""LuaScriptEngine — lupa-backed implementation of `IScriptEngine`.

Cross-backend parity:
- C++ uses Lua 5.4 directly via the C API + ECMAScript→Lua transformer.
- Rust uses `mlua` (PUC Lua bindings) + the same transformer.
- Go uses `gopher-lua` + the same transformer.
- Kotlin uses JNI bindings + the same transformer.
- C11 uses Lua 5.4 directly.
- Python AOT now uses `lupa` (PyPI: lupa) + the same transformer
  (`to_lua_expr` / `to_lua_guard` / `to_lua_script` filters in
  `sce-build/src/filters.rs`).

Generated `*_sm.py` modules call `IScriptEngine.evaluate_expression(...)`
with **Lua text** (already transformed at codegen time). The Lua engine
runs that text against a per-session global table. Round-trip conversion
between Python values and Lua values is centralised in `_python_to_lua`
/ `_lua_to_python` so the rest of the runtime stays engine-agnostic.
"""

from __future__ import annotations

from threading import RLock
from typing import Any, Callable, Dict, List, Optional

try:
    import lupa  # type: ignore[import-not-found]
except ImportError as _exc:  # pragma: no cover — surfaces during dependency install
    raise ImportError(
        "lupa is required for the SCE Python AOT runtime — install with `pip install lupa`"
    ) from _exc

from .i_script_engine import (
    IScriptEngine,
    NativeMethod,
    ScriptError,
    ScriptRuntimeError,
    ScriptSyntaxError,
    ScriptValue,
    ScriptValueKind,
    SessionNotFoundError,
    StateQueryCallback,
    VariableNotDeclaredError,
)


class _LuaSession:
    """Per-state-machine Lua isolation. Holds the lupa runtime + the
    set of declared variable names + the registered `In()` callback.

    Each `Engine` instance owns one session id; the script engine's
    session table maps that id to one of these. Sessions are created
    by `create_session` and destroyed by `destroy_session`."""

    __slots__ = ("runtime", "declared_vars", "state_query")

    def __init__(self) -> None:
        # lupa.LuaRuntime is the entry point — defaults to PUC Lua 5.x
        # when the runtime can load it (Lua 5.4 in this project,
        # matching other backends per `[[lua_engine_default]]`).
        # `unpack_returned_tuples=True` so multi-return Lua functions
        # (e.g., `string.find`) decompose naturally on the Python side.
        self.runtime = lupa.LuaRuntime(unpack_returned_tuples=True)
        self.declared_vars: set = set()
        self.state_query: Optional[StateQueryCallback] = None
        _install_ecmascript_builtins(self.runtime)


# ── Undeclared-identifier detection (ECMAScript ReferenceError parity) ──

_LUA_KEYWORDS = frozenset({
    "and", "break", "do", "else", "elseif", "end", "false", "for",
    "function", "goto", "if", "in", "local", "nil", "not", "or",
    "repeat", "return", "then", "true", "until", "while",
})


def _is_undeclared_simple_variable(expr: str, session: "_LuaSession") -> bool:
    """Detect references to undeclared globals in the way C++ /
    Rust / Go / Kotlin / C11 do (W3C B.2 ReferenceError parity).
    Handles bare identifiers (`Var1`) and member access
    (`Var1.foo`, `Var1["x"]`) by examining the base name only.
    Returns False for expressions whose base name is a Lua keyword,
    a declared SCXML variable, or a Lua standard-library global."""
    if not expr:
        return False
    first = expr[0]
    if not (first.isalpha() or first == "_"):
        return False
    base_end = len(expr)
    for i, ch in enumerate(expr):
        if not (ch.isalnum() or ch == "_"):
            base_end = i
            break
    if base_end == 0:
        return False
    base = expr[:base_end]
    if base in _LUA_KEYWORDS:
        return False
    if base in session.declared_vars:
        return False
    # Lua standard-library globals (`table`, `string`, `math`, …) and
    # SCE-installed builtins (`_scxml_truthy`, `_event`, `_ioprocessors`,
    # …) are non-nil in the session's global table.
    return session.runtime.globals()[base] is None


# ── ECMAScript builtins (W3C SCXML B.2 semantics over Lua 5.4) ─────


def _install_ecmascript_builtins(runtime: Any) -> None:
    """Port of `sce-rust-lua::setup_builtins` (which itself is a port of
    C++ `LuaEngine::setupBuiltins()`). Registers the global helpers the
    ECMAScript→Lua transformer's emitted text expects to find:
    `_scxml_truthy`, `_typeof`, `_isArray`, `_indexOf`, `_concat`,
    `parseInt`, `parseFloat`, `_NULL` / `_UNDEFINED` sentinels, the
    string `__add` concatenation metatable, and `Object.keys`. JSON
    builtins are loaded from the shared `sce/include/scripting/
    json_builtins.lua` file so Lua-side behaviour matches every other
    backend bit-for-bit."""
    globals_ = runtime.globals()

    def _scxml_truthy(val: Any) -> bool:
        if val is None or val is False:
            return False
        if isinstance(val, bool):
            return val
        if isinstance(val, int):
            return val != 0
        if isinstance(val, float):
            return val != 0.0 and val == val  # NaN check
        if isinstance(val, str):
            return bool(val)
        return True
    globals_["_scxml_truthy"] = _scxml_truthy

    def _typeof(val: Any) -> str:
        if val is None:
            return "undefined"
        if isinstance(val, bool):
            return "boolean"
        if isinstance(val, (int, float)):
            return "number"
        if isinstance(val, str):
            return "string"
        # lupa exposes Lua functions as callable Python objects
        if callable(val):
            return "function"
        return "object"
    globals_["_typeof"] = _typeof

    def _is_array(val: Any) -> bool:
        if val is None:
            return False
        try:
            keys = list(val.keys()) if hasattr(val, "keys") else None
        except Exception:
            return False
        if keys is None:
            return False
        if not keys:
            return True
        return all(isinstance(k, int) for k in keys) and sorted(keys) == list(
            range(1, len(keys) + 1)
        )
    globals_["_isArray"] = _is_array

    def _index_of(*args: Any) -> int:
        if not args:
            return -1
        haystack = args[0]
        needle = args[1] if len(args) > 1 else None
        start = int(args[2]) if len(args) > 2 and args[2] is not None else 0
        if isinstance(haystack, str):
            needle_str = "" if needle is None else str(needle)
            if not needle_str:
                return 0
            try:
                return haystack.index(needle_str, start)
            except ValueError:
                return -1
        # Lua table (array) — 1-indexed, return 0-based result
        if hasattr(haystack, "keys"):
            try:
                length = len(haystack)
            except TypeError:
                length = 0
                for _ in haystack:
                    length += 1
            for i in range(start + 1, length + 1):
                if haystack[i] == needle:
                    return i - 1
        return -1
    globals_["_indexOf"] = _index_of

    def _concat(arr1: Any, *rest: Any) -> Any:
        out = []
        if arr1 is not None and hasattr(arr1, "keys"):
            for i in range(1, len(arr1) + 1):
                out.append(arr1[i])
        for arr2 in rest:
            if arr2 is None:
                continue
            if hasattr(arr2, "keys"):
                for i in range(1, len(arr2) + 1):
                    out.append(arr2[i])
            else:
                out.append(arr2)
        return runtime.table_from(out)
    globals_["_concat"] = _concat

    def _parse_int(val: Any, *rest: Any) -> Any:
        radix = int(rest[0]) if rest and rest[0] is not None else 10
        if isinstance(val, (int, float)):
            return int(val)
        if not isinstance(val, str):
            return float("nan")
        text = val.strip()
        prefix = ""
        if (radix == 16 or radix == 0) and (
            text.lower().startswith("0x")
        ):
            prefix = text[:2]
            text = text[2:]
            radix = 16
        elif radix == 0:
            radix = 10
        sign = ""
        if text.startswith(("+", "-")):
            sign = text[0]
            text = text[1:]
        digits = ""
        for ch in text:
            try:
                int(ch, radix)
                digits += ch
            except ValueError:
                break
        if not digits:
            return float("nan")
        try:
            return int(sign + digits, radix)
        except ValueError:
            return float("nan")
    globals_["parseInt"] = _parse_int

    def _parse_float(val: Any) -> Any:
        if isinstance(val, (int, float)):
            return float(val)
        if not isinstance(val, str):
            return float("nan")
        text = val.strip()
        try:
            return float(text)
        except ValueError:
            return float("nan")
    globals_["parseFloat"] = _parse_float

    # ES `null` / `undefined` collapse to nil here (matches Rust port's
    # documented "_NULL = nil, _UNDEFINED = nil"). The dedicated
    # lightuserdata sentinels in C++ exist for array-element identity
    # preservation in nested literals; lupa doesn't expose lightuserdata
    # so we accept the same lossy-on-distinction trade-off Rust takes.
    globals_["_NULL"] = None
    globals_["_UNDEFINED"] = None

    # ECMAScript string concatenation via `+` — install __add on the
    # string metatable so `"a" + "b"` produces `"ab"` (Lua's native `+`
    # is numeric only).
    runtime.execute(
        """
        local mt = getmetatable("")
        if mt then
            mt.__add = function(a, b)
                return tostring(a) .. tostring(b)
            end
        end
        """
    )

    # Object.keys(table) → array of string keys.
    def _object_keys(tbl: Any) -> Any:
        if tbl is None or not hasattr(tbl, "keys"):
            return runtime.table_from([])
        return runtime.table_from([str(k) for k in tbl.keys()])
    object_table = runtime.table_from({"keys": _object_keys})
    globals_["Object"] = object_table

    # JSON builtins shared with every other backend (single source of
    # truth at sce/include/scripting/json_builtins.lua). Loaded
    # opportunistically; absent from non-monorepo deployments is
    # acceptable — the helpers are tested by JSON-using fixtures only.
    import os
    here = os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(__file__))))
    json_lua = os.path.join(here, "sce", "include", "scripting", "json_builtins.lua")
    if os.path.exists(json_lua):
        with open(json_lua, "r", encoding="utf-8") as fh:
            runtime.execute(fh.read())


class LuaScriptEngine(IScriptEngine):
    """W3C SCXML 5.5 / 5.3 — Lua-backed ECMAScript datamodel.

    Thread-safety: a single engine instance multiplexes across sessions.
    Each method takes the session id and looks up the corresponding
    `_LuaSession`. A coarse `RLock` guards the session registry; Lua
    runtimes themselves are single-threaded (lupa pins one Lua state
    to one OS thread under the hood, so multi-threaded callers should
    keep one engine per thread or guard externally)."""

    def __init__(self) -> None:
        self._sessions: Dict[str, _LuaSession] = {}
        self._global_funcs: Dict[str, NativeMethod] = {}
        self._initialized: bool = False
        self._lock = RLock()

    # ── Engine lifecycle ───────────────────────────────────────────

    def initialize(self) -> bool:
        with self._lock:
            self._initialized = True
        return True

    def shutdown(self) -> None:
        with self._lock:
            self._sessions.clear()
            self._global_funcs.clear()
            self._initialized = False

    def is_initialized(self) -> bool:
        return self._initialized

    def reset(self) -> None:
        self.shutdown()
        self.initialize()

    # ── Session lifecycle ─────────────────────────────────────────

    def create_session(self, session_id: str) -> None:
        with self._lock:
            if session_id in self._sessions:
                return
            session = _LuaSession()
            self._sessions[session_id] = session
            # Surface previously-registered global functions into the
            # new session — matches the C++ pattern where
            # `registerGlobalFunction` applies to every existing AND
            # future session.
            for name, callback in self._global_funcs.items():
                self._install_native_function(session, name, callback)

    def destroy_session(self, session_id: str) -> None:
        with self._lock:
            self._sessions.pop(session_id, None)

    def has_session(self, session_id: str) -> bool:
        with self._lock:
            return session_id in self._sessions

    # ── Core script execution ──────────────────────────────────────

    def execute_script(self, session_id: str, script: str) -> ScriptValue:
        """W3C SCXML 5.5 — execute a `<script>` body. The text is
        already Lua (transformed at codegen time via `to_lua_script`)
        so we hand it straight to `lupa.execute`."""
        session = self._require_session(session_id)
        if not script:
            return ScriptValue.null()
        try:
            result = session.runtime.execute(script)
        except lupa.LuaSyntaxError as exc:
            raise ScriptSyntaxError(str(exc)) from exc
        except lupa.LuaError as exc:
            raise ScriptRuntimeError(str(exc)) from exc
        return _lua_to_script_value(result)

    def evaluate_expression(self, session_id: str, expression: str) -> ScriptValue:
        """W3C SCXML 5.3 — evaluate a single expression. Empty input
        returns NULL (matches the C++ convention for omitted attrs).

        ECMAScript semantics for undeclared globals (W3C B.2): JavaScript
        throws ReferenceError, Lua silently returns nil. C++ / Rust /
        Go / Kotlin / C11 all detect the simple-identifier case before
        eval and surface ReferenceError. Mirrors
        `sce-rust-lua/src/lib.rs::is_undeclared_simple_variable` so
        `<send namelist=...>` / `<donedata><param expr=...>` / `<assign
        expr=...>` raise `error.execution` on undeclared reads (test343,
        test553, etc.)."""
        session = self._require_session(session_id)
        if not expression:
            return ScriptValue.null()
        if _is_undeclared_simple_variable(expression, session):
            raise ScriptRuntimeError(
                f"ReferenceError: {expression} is not defined"
            )
        try:
            result = session.runtime.eval(expression)
        except lupa.LuaSyntaxError as exc:
            raise ScriptSyntaxError(str(exc)) from exc
        except lupa.LuaError as exc:
            raise ScriptRuntimeError(str(exc)) from exc
        return _lua_to_script_value(result)

    def validate_expression(self, session_id: str, expression: str) -> bool:
        session = self._require_session(session_id)
        if not expression:
            return True
        # lupa exposes `compile` via the underlying `load` builtin —
        # wrap the expression as a return statement so we validate
        # parse-only without running side effects.
        try:
            session.runtime.eval(f"function() return {expression} end")
            return True
        except (lupa.LuaSyntaxError, lupa.LuaError):
            return False

    # ── Variable management ────────────────────────────────────────

    def declare_variable(
        self, session_id: str, name: str, initial_value: ScriptValue
    ) -> None:
        session = self._require_session(session_id)
        session.runtime.globals()[name] = _script_value_to_lua(
            session, initial_value
        )
        session.declared_vars.add(name)

    def set_variable(
        self, session_id: str, name: str, value: ScriptValue
    ) -> None:
        """W3C SCXML 5.4 — bind a value to a datamodel location. Matches
        the Rust + C++ + Go Lua engines: `set_variable` is always
        allowed (any identifier is a legal location under ECMAScript
        rules), and the side-effect of declaring the name happens here
        too. The W3C "undeclared identifier raises ReferenceError" case
        is handled on the read side (`evaluate_expression`) — see the
        C++ `isUndeclaredSimpleVariable` short-circuit."""
        session = self._require_session(session_id)
        session.runtime.globals()[name] = _script_value_to_lua(session, value)
        session.declared_vars.add(name)

    def get_variable(self, session_id: str, name: str) -> ScriptValue:
        session = self._require_session(session_id)
        value = session.runtime.globals()[name]
        return _lua_to_script_value(value)

    def set_variable_as_dom(
        self, session_id: str, name: str, xml_content: str
    ) -> None:
        # W3C SCXML B.2 — DOM datamodel support stub. Storing the raw
        # XML text on the Lua side lets future XPath helpers run
        # against it; an actual DOM tree binding lands with the B.2
        # atomic. The variable name does not need to be in
        # `declared_vars` if it was registered as DOM-only.
        session = self._require_session(session_id)
        session.runtime.globals()[name] = xml_content
        session.declared_vars.add(name)

    def has_variable(self, session_id: str, name: str) -> bool:
        session = self._require_session(session_id)
        return name in session.declared_vars

    # ── SCXML-specific features ────────────────────────────────────

    def setup_system_variables(
        self,
        session_id: str,
        session_name: str,
        io_processors: List[str],
    ) -> None:
        """W3C SCXML 5.10 — `_sessionid` / `_name` / `_ioprocessors`."""
        session = self._require_session(session_id)
        globals_ = session.runtime.globals()
        globals_["_sessionid"] = session_id
        globals_["_name"] = session_name
        # `_ioprocessors` is a table keyed on the processor URI with a
        # `location` field per W3C C.2. The lupa `table_from` builder
        # produces a real Lua table the generated expressions can
        # dot-access.
        io_table = session.runtime.table_from(
            {uri: session.runtime.table_from({"location": uri}) for uri in io_processors}
        )
        globals_["_ioprocessors"] = io_table

    def set_current_event(
        self,
        session_id: str,
        event_name: str,
        event_data: str,
        event_type: str,
        send_id: str,
        origin: str,
        origin_type: str,
        invoke_id: str,
    ) -> None:
        """W3C SCXML 5.10 — bind the `_event` table for the current
        microstep. Mirrors the C++ `setCurrentEvent` signature and the
        Rust `set_current_event` parsing chain: Lua-eval the payload
        first (so JSON-like dict text becomes a Lua table whose `.foo`
        members are dot-accessible from generated expressions); fall
        back to whitespace-normalised string when neither parse path
        succeeds. Empty payloads bind no `data` field at all so
        guards reading `_event.data` get Lua nil (== ES `undefined`)."""
        session = self._require_session(session_id)
        event_table = session.runtime.table()
        event_table["name"] = event_name
        if event_data:
            data_value: Any = _coerce_event_data_to_lua(
                session.runtime, event_data
            )
            event_table["data"] = data_value
        if event_type:
            event_table["type"] = event_type
        if send_id:
            event_table["sendid"] = send_id
        # W3C SCXML 5.10.1 — always set origin/origintype so
        # `targetexpr="_event.origin"` evaluates to "" (not nil) when
        # origin is unset (test 336).
        event_table["origin"] = origin
        event_table["origintype"] = origin_type
        if invoke_id:
            event_table["invokeid"] = invoke_id
        session.runtime.globals()["_event"] = event_table

    # ── Native bindings ────────────────────────────────────────────

    def register_global_function(
        self, function_name: str, callback: NativeMethod
    ) -> bool:
        with self._lock:
            self._global_funcs[function_name] = callback
            for session in self._sessions.values():
                self._install_native_function(session, function_name, callback)
        return True

    def set_state_query_callback(
        self, session_id: str, callback: Optional[StateQueryCallback]
    ) -> None:
        session = self._require_session(session_id)
        session.state_query = callback
        if callback is None:
            session.runtime.globals()["In"] = None
        else:
            def _in_predicate(state_id: str) -> bool:
                return bool(callback(state_id))
            session.runtime.globals()["In"] = _in_predicate

    # ── Engine info ────────────────────────────────────────────────

    def get_engine_info(self) -> str:
        try:
            version = self._sessions[next(iter(self._sessions))].runtime.eval("_VERSION") if self._sessions else "lua"
        except Exception:
            version = "lua"
        return f"LuaScriptEngine (lupa, {version})"

    # ── Internals ──────────────────────────────────────────────────

    def _require_session(self, session_id: str) -> _LuaSession:
        with self._lock:
            session = self._sessions.get(session_id)
        if session is None:
            raise SessionNotFoundError(session_id)
        return session

    @staticmethod
    def _install_native_function(
        session: _LuaSession, name: str, callback: NativeMethod
    ) -> None:
        def _bridge(*lua_args: Any) -> Any:
            args = [_lua_to_script_value(arg) for arg in lua_args]
            result = callback(args)
            return _script_value_to_lua(session, result)
        session.runtime.globals()[name] = _bridge


# ── Event payload coercion ───────────────────────────────────────


def _coerce_event_data_to_lua(runtime: Any, event_data: str) -> Any:
    """W3C SCXML 5.10 — turn the raw `event_data` string into the Lua
    value `_event.data` should expose.

    Mirrors the Rust `set_current_event` parsing chain
    (`sce_rust_lua::lib::set_current_event`): try `return <text>` as Lua
    first (so a Lua/JSON-like table literal becomes a real table); when
    that fails, fall back to JSON-to-Lua syntax rewriting (so payloads
    that round-tripped through `json.dumps` still produce a table); when
    that also fails, treat the bytes as a string with whitespace
    normalised (W3C B.2 test562). The chain order is identical across
    backends so `_event.data.foo` resolves consistently."""
    if not event_data:
        return None
    text = event_data.strip()
    if not text:
        return event_data
    # Path 1 — Lua eval (covers `{a=1}` produced by a previous
    # roundtrip + bare numbers + bare strings the user already
    # quoted in the payload).
    try:
        return runtime.eval(text)
    except Exception:
        pass
    # Path 2 — JSON-style `{"key": val}` rewritten into Lua
    # syntax `{["key"] = val}`. The transform is intentionally
    # narrow: it swaps `:` for `=` only inside object literals
    # and quotes the key. Handles the common case where the
    # event payload was serialised by `json.dumps(dict)`.
    converted = _json_to_lua_table(text)
    if converted is not None:
        try:
            return runtime.eval(converted)
        except Exception:
            pass
    # Path 3 — whitespace-normalised string fallback (W3C B.2).
    return " ".join(event_data.split())


def _json_to_lua_table(text: str) -> Optional[str]:
    """Best-effort JSON→Lua table-literal rewriter. Returns None when
    the input doesn't look like a JSON object/array, so the caller
    knows to fall through to the string fallback."""
    stripped = text.strip()
    if not stripped:
        return None
    if not (stripped.startswith("{") or stripped.startswith("[")):
        return None
    try:
        import json

        parsed = json.loads(stripped)
    except (json.JSONDecodeError, ValueError):
        return None
    return _python_to_lua_literal(parsed)


def _python_to_lua_literal(value: Any) -> str:
    """Render a Python value as the corresponding Lua source text. Used
    by `_coerce_event_data_to_lua` for the JSON-to-Lua fallback so the
    Lua runtime sees a real table when the source payload was
    JSON-encoded."""
    if value is None:
        return "nil"
    if value is True:
        return "true"
    if value is False:
        return "false"
    if isinstance(value, (int, float)):
        return repr(value)
    if isinstance(value, str):
        return '"' + value.replace("\\", "\\\\").replace('"', '\\"') + '"'
    if isinstance(value, list):
        return "{" + ", ".join(_python_to_lua_literal(v) for v in value) + "}"
    if isinstance(value, dict):
        parts: List[str] = []
        for k, v in value.items():
            key = (
                f"[{_python_to_lua_literal(k)}]"
                if not isinstance(k, str)
                or not k.replace("_", "").isalnum()
                or (k[:1].isdigit() if k else True)
                else k
            )
            parts.append(f"{key} = {_python_to_lua_literal(v)}")
        return "{" + ", ".join(parts) + "}"
    return _python_to_lua_literal(str(value))


# ── ScriptValue ↔ Lua type bridge ─────────────────────────────────


def _script_value_to_lua(session: _LuaSession, value: ScriptValue) -> Any:
    """Convert a `ScriptValue` into the Lua representation lupa expects.

    Primitives ride the lupa auto-coercion path; arrays and objects are
    built as real Lua tables via `runtime.table_from` so dotted-path
    and ipairs/pairs access work from generated expressions."""
    if value.kind is ScriptValueKind.NULL or value.kind is ScriptValueKind.UNDEFINED:
        return None
    if value.kind is ScriptValueKind.BOOL:
        return value.bool_val
    if value.kind is ScriptValueKind.INT:
        return value.int_val
    if value.kind is ScriptValueKind.DOUBLE:
        return value.double_val
    if value.kind is ScriptValueKind.STRING:
        return value.string_val
    if value.kind is ScriptValueKind.ARRAY:
        return session.runtime.table_from(
            [_script_value_to_lua(session, v) for v in value.array_val]
        )
    if value.kind is ScriptValueKind.OBJECT:
        return session.runtime.table_from(
            {k: _script_value_to_lua(session, v) for k, v in value.object_val.items()}
        )
    if value.kind is ScriptValueKind.DOM:
        return value.dom_val
    return None


def _lua_to_script_value(value: Any) -> ScriptValue:
    """Convert whatever lupa hands back into a `ScriptValue`. Tables are
    detected via the lupa-specific type — keyed tables become OBJECT,
    array-shaped tables become ARRAY (best-effort heuristic mirrors how
    the C++ engine round-trips Lua values into `ScriptValue`)."""
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
    # lupa exposes Lua tables as objects answering to `.keys()` /
    # iteration. Test if every key is an integer 1..N (array shape) or
    # arbitrary (object shape).
    try:
        keys = list(value.keys()) if hasattr(value, "keys") else list(value)
    except Exception:
        return ScriptValue(kind=ScriptValueKind.STRING, string_val=str(value))
    if not keys:
        # Empty Lua table — treat as empty object (matches C++ default).
        return ScriptValue(kind=ScriptValueKind.OBJECT, object_val={})
    if all(isinstance(k, int) for k in keys) and sorted(keys) == list(
        range(1, len(keys) + 1)
    ):
        return ScriptValue(
            kind=ScriptValueKind.ARRAY,
            array_val=[_lua_to_script_value(value[i]) for i in sorted(keys)],
        )
    return ScriptValue(
        kind=ScriptValueKind.OBJECT,
        object_val={str(k): _lua_to_script_value(value[k]) for k in keys},
    )
