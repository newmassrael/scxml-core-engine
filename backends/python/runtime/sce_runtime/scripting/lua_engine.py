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

import xml.dom.minidom as _minidom
from threading import RLock
from typing import Any, Callable, Dict, List, Optional

from ..io_processors import IoProcessorDescriptor

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
    SetCurrentEventArgs,
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


# ── ECMAScript builtins (§scxml-B-2 semantics over Lua 5.4) ─────


def _install_ecmascript_builtins(runtime: Any) -> None:
    """Install what this backend still owns, and load what it shares.

    `_scxml_truthy`, `_typeof`, `_isArray`, `_indexOf`, `_concat`,
    `parseInt` and `parseFloat` used to be written out here in Python, one
    implementation among six. They are in the shared
    `sce/include/scripting/ecma_semantics.lua` now, loaded at the bottom of
    this function, because six implementations drifted exactly as predicted:
    measured 2026-08-16 against `tests/ecmascript/ecma262_semantics.json`,
    this one called `typeof [1,2,3]` "function" — lupa hands a Lua table to
    Python as a callable object, so `callable(val)` answered before the
    object arm was reached — while Go's `_indexOf` had no Array branch at
    all and Rust's dropped the second argument.

    What remains here is what Lua cannot express or lupa needs from the host
    side: the `_NULL` / `_UNDEFINED` sentinels and the string `__add`
    concatenation metatable."""
    globals_ = runtime.globals()

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

    # `Object.keys` is in the shared semantics file loaded below, with the
    # rest of the engine vocabulary. This copy returned lupa's key order,
    # which is the Lua hash layout rather than an order; the shared one sorts,
    # so the six backends answer the same array.

    import os
    # …/backends/python/runtime/sce_runtime/scripting/lua_engine.py → repo root
    # is six directories up. It used to be four, which named
    # `backends/python/sce/include/scripting/` — a path that has never
    # existed, so the JSON load below silently did nothing on every run. The
    # `os.path.exists` guard is what kept that invisible; it stays, because a
    # packaged deployment really can lack the file, but it now guards a path
    # that resolves in this repository.
    here = os.path.dirname(
        os.path.dirname(
            os.path.dirname(
                os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
            )
        )
    )

    # §scxml-B-2: the ECMAScript operators Lua does not share — `+`,
    # `==` and the bitwise family. Single source of truth at
    # sce/include/scripting/ecma_semantics.lua, shared with every other
    # backend. Unlike the JSON helpers below this one is required: the
    # generated code calls `_scxml_add` and `_scxml_eq` by name, so a
    # deployment without the file would fail at the first `+`.
    ecma_lua = os.path.join(here, "sce", "include", "scripting", "ecma_semantics.lua")
    with open(ecma_lua, "r", encoding="utf-8") as fh:
        runtime.execute(fh.read())

    # JSON builtins shared with every other backend (single source of
    # truth at sce/include/scripting/json_builtins.lua). Loaded
    # opportunistically; absent from non-monorepo deployments is
    # acceptable — the helpers are tested by JSON-using fixtures only.
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
        `backends/rust/lua/src/lib.rs::is_undeclared_simple_variable` so
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
        # §scxml-B-2 — parse `xml_content` and bind the resulting DOM
        # root at `name`. The DOM is reachable from Lua expressions via
        # `:getElementsByTagName(tag)` / `:getAttribute(name)` /
        # `:getTagName()` per the ECMAScript data-model contract
        # (test557, test561). Mirrors `sce-rust-lua::set_variable_as_dom`
        # which wraps the parsed tree in an `XmlRef` userdata — the
        # Python side uses `_DomElement` whose bound methods give
        # identical observable behaviour via lupa's colon-call sugar.
        # Parse failure leaves the variable bound to a whitespace-
        # normalised string per W3C B.2 (matches cpp `LuaDOMBinding::
        # pushDOMObject` on `XMLDocument::isValid()` = false).
        session = self._require_session(session_id)
        dom = _parse_xml_to_dom(xml_content, session.runtime)
        if dom is None:
            session.runtime.globals()[name] = " ".join(xml_content.split())
        else:
            session.runtime.globals()[name] = dom
        session.declared_vars.add(name)

    def has_variable(self, session_id: str, name: str) -> bool:
        session = self._require_session(session_id)
        return name in session.declared_vars

    # ── SCXML-specific features ────────────────────────────────────

    def setup_system_variables(
        self,
        session_id: str,
        session_name: str,
        io_processors: List[IoProcessorDescriptor],
    ) -> None:
        """W3C SCXML 5.10 — `_sessionid` / `_name` / `_ioprocessors`."""
        session = self._require_session(session_id)
        globals_ = session.runtime.globals()
        globals_["_sessionid"] = session_id
        globals_["_name"] = session_name
        # §scxml-C-1-1 / §scxml-C-2-3: one entry per processor the deployment
        # supports, each with a `location` field holding the address that
        # reaches this session through it. Names and locations are decided by
        # `sce_runtime.io_processors.build`, so this engine's view of
        # `_ioprocessors` matches every other backend's. The lupa `table_from`
        # builder produces a real Lua table the generated expressions can
        # dot-access.
        io_table = session.runtime.table_from(
            {
                processor.name: session.runtime.table_from(
                    {"location": processor.location}
                )
                for processor in io_processors
            }
        )
        globals_["_ioprocessors"] = io_table

    def set_current_event(
        self,
        session_id: str,
        args: SetCurrentEventArgs,
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
        event_table["name"] = args.event_name
        if args.event_data:
            data_value: Any = _coerce_event_data_to_lua(
                session.runtime, args.event_data
            )
            event_table["data"] = data_value
        if args.event_type:
            event_table["type"] = args.event_type
        if args.send_id:
            event_table["sendid"] = args.send_id
        # §scxml-5.10.1 — always set origin/origintype so
        # `targetexpr="_event.origin"` evaluates to "" (not nil) when
        # origin is unset (test 336).
        event_table["origin"] = args.origin
        event_table["origintype"] = args.origin_type
        if args.invoke_id:
            event_table["invokeid"] = args.invoke_id
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
    """W3C SCXML B.2.8.1 — turn the raw `event_data` string into the Lua
    value `_event.data` should expose.

    The clause gives three readings and no fourth: XML becomes a DOM,
    JSON becomes the value, anything else becomes a space-normalized
    string. Mirrors `sce_rust_lua::lib::set_current_event` and the cpp
    `parseEventData`, so `_event.data.foo` resolves the same everywhere.

    There used to be a rung above all three — `runtime.eval(text)`,
    running the payload as Lua source before anything looked at it — and
    it decided all three of the following, measured 2026-08-17 on the
    sibling Rust engine that carried the same rung:

    * `2 + 3` from a host arrived as the number 5, and as the string
      "2 + 3" on the cpp and Rhino engines that read the clause instead.
      One payload, two answers.
    * a payload that is a function call RAN, in the session's globals.
      `_event.data` is the one field a document takes from outside.
    * it was load-bearing: `<send>` shipped Lua source, so this rung was
      the deserializer for every param a document sent.

    The sender now ships JSON (§scxml-B-2-9: data that leaves the data
    model is serialized to JSON), which is what cpp always shipped."""
    if not event_data:
        return None
    text = event_data.strip()
    if not text:
        return event_data
    # Path 1 — XML payload (§scxml-B-2-8-1's first reading).
    # `<send><content>XML</content></send>` lands `_event.data` as a
    # DOM-style object exposing `getElementsByTagName` / `getAttribute`
    # (test561). Mirrors the Rust `set_current_event` DOM path —
    # `sce-rust-lua::lib::set_current_event` builds an `XmlRef`
    # userdata for the same case.
    if text.startswith("<"):
        dom = _parse_xml_to_dom(text, runtime)
        if dom is not None:
            return dom
    # Path 2 — JSON, rewritten into Lua table syntax. The transform is
    # intentionally narrow: it swaps `:` for `=` only inside object
    # literals and quotes the key.
    converted = _json_to_lua_table(text)
    if converted is not None:
        try:
            return runtime.eval(converted)
        except Exception:
            pass
    # Path 3 — whitespace-normalised string fallback (W3C B.2 test562).
    return " ".join(event_data.split())


# ── XML DOM (§scxml-B-2 ECMAScript data model) ─────────────────


class _DomElement:
    """W3C SCXML B.2 — Lua-facing wrapper around a `xml.dom.minidom`
    node, carrying DOM Level 1 Core's read surface.

    §scxml-B-2-1 obliges the Processor to create *"the corresponding DOM
    structure"*, so what a handle answers is that interface and not the
    two calls the W3C IRP suite happens to read. Measured 2026-08-18,
    three methods were all any backend had: `d.tagName`, `d.childNodes`
    and `d.firstChild` reached the engine as a nil index, on all seven
    backends, with the suite green.

    * methods — `getElementsByTagName(tag)` (Document refkind matches the
      root inclusively, mirroring cpp `XMLDocument::getElementsByTagName`;
      Element refkind only descends into proper descendants, mirroring
      cpp `XMLElement::getElementsByTagName`), `getAttribute(name)`,
      `hasAttribute(name)`, `getTagName()`, `hasChildNodes()`.
    * properties — `nodeType`, `nodeName`, `nodeValue`, `data`,
      `tagName`, `textContent`, `childNodes`, `firstChild`, `lastChild`,
      `nextSibling`, `previousSibling`, `parentNode`,
      `documentElement`.

    A Document refkind answers the Node interface as the document it is
    and the Element vocabulary for its document element — the delegation
    the three shipped methods already performed.

    Lupa's bound-method calling convention lets generated Lua text
    `dom:getElementsByTagName('book')` resolve to the Python method (the
    colon-passed `self` is absorbed by the bound method), and a
    `@property` resolves to a Lua field read, which is what the frontend
    emits for a member. Mirrors `sce-rust-lua::dom::XmlRef`."""

    __slots__ = ("_node", "_is_document", "_runtime")

    # DOM Level 1 Core node types. Four of the twelve, because four is
    # what these trees hold: comments and processing instructions are
    # dropped by `_normalize_dom` to match pugixml's `parse_default`.
    ELEMENT = 1
    TEXT = 3
    CDATA_SECTION = 4
    DOCUMENT = 9

    def __init__(self, node: Any, is_document: bool, runtime: Any) -> None:
        self._node = node
        self._is_document = is_document
        self._runtime = runtime

    # ── methods ────────────────────────────────────────────────────

    def getElementsByTagName(self, tag: str) -> Any:
        if self._is_document:
            matches = list(self._node.getElementsByTagName(tag))
        else:
            matches = [
                found
                for child in self._node.childNodes
                if child.nodeType == self.ELEMENT
                for found in ([child] if child.tagName == tag else [])
                + list(child.getElementsByTagName(tag))
            ]
        return self._nodelist(matches)

    def getAttribute(self, name: str) -> str:
        if self._node.nodeType != self.ELEMENT:
            return ""
        return self._node.getAttribute(name) or ""

    def hasAttribute(self, name: str) -> bool:
        return self._node.nodeType == self.ELEMENT and self._node.hasAttribute(name)

    def getTagName(self) -> str:
        return self._node.tagName if self._node.nodeType == self.ELEMENT else ""

    def hasChildNodes(self) -> bool:
        # A document always has one child: its document element.
        return True if self._is_document else bool(self._node.childNodes)

    # ── the Node interface, as properties ──────────────────────────

    @property
    def nodeType(self) -> int:
        return self.DOCUMENT if self._is_document else int(self._node.nodeType)

    @property
    def nodeName(self) -> str:
        return "#document" if self._is_document else str(self._node.nodeName)

    @property
    def nodeValue(self) -> Optional[str]:
        # DOM Level 1 Core gives an element and a document a null
        # nodeValue.
        if self._is_document or self._node.nodeType == self.ELEMENT:
            return None
        return self._node.nodeValue

    @property
    def data(self) -> Optional[str]:
        """CharacterData's own name for `nodeValue`."""
        return self.nodeValue

    @property
    def tagName(self) -> Optional[str]:
        if not self._is_document and self._node.nodeType != self.ELEMENT:
            return None  # character data has no tag name
        return self.getTagName()

    @property
    def textContent(self) -> str:
        return self._text_content(self._node)

    @property
    def childNodes(self) -> Any:
        if self._is_document:
            return self._nodelist([self._node])
        return self._nodelist(list(self._node.childNodes))

    @property
    def firstChild(self) -> Optional["_DomElement"]:
        if self._is_document:
            return self._wrap(self._node)
        return self._wrap(self._node.firstChild)

    @property
    def lastChild(self) -> Optional["_DomElement"]:
        if self._is_document:
            return self._wrap(self._node)
        return self._wrap(self._node.lastChild)

    @property
    def nextSibling(self) -> Optional["_DomElement"]:
        return None if self._is_document else self._wrap(self._node.nextSibling)

    @property
    def previousSibling(self) -> Optional["_DomElement"]:
        return None if self._is_document else self._wrap(self._node.previousSibling)

    @property
    def parentNode(self) -> Optional["_DomElement"]:
        if self._is_document:
            return None
        parent = self._node.parentNode
        if parent is None:
            return None
        # The document element's parent is the document — DOM Level 1
        # Core 1.3 — which is the handle the variable already holds.
        if parent.nodeType == self.DOCUMENT:
            return _DomElement(self._node, True, self._runtime)
        return self._wrap(parent)

    @property
    def documentElement(self) -> Optional["_DomElement"]:
        # Only the document handle carries this, which is how a document
        # can tell the two kinds apart without reading nodeType.
        return self._wrap(self._node) if self._is_document else None

    # ── helpers ────────────────────────────────────────────────────

    def _wrap(self, node: Any) -> Optional["_DomElement"]:
        return None if node is None else _DomElement(node, False, self._runtime)

    def _nodelist(self, nodes: List[Any]) -> Any:
        """A NodeList: a 1-based Lua array, because the frontend rewrites
        `[0]` to `[1]` and `length` to Lua's `#`. Every backend hands
        back its language's array for that reason, which is why `item(i)`
        is refused by the frontend rather than implemented here."""
        return self._runtime.table_from(
            [_DomElement(node, False, self._runtime) for node in nodes]
        )

    def _text_content(self, node: Any) -> str:
        """DOM Level 3 Core `textContent` — every descendant
        character-data node's content, in document order."""
        if node.nodeType in (self.TEXT, self.CDATA_SECTION):
            return node.nodeValue or ""
        return "".join(self._text_content(child) for child in node.childNodes)


def _normalize_dom(node: Any) -> None:
    """Drop what pugixml's `parse_default` never puts in the tree.

    The cpp reference backend parses with `parse_default`, which omits
    `parse_ws_pcdata`, `parse_comments` and `parse_pi` — so a
    whitespace-only text run, a comment and a processing instruction are
    not nodes there. minidom keeps all three. While
    `getElementsByTagName` was the only reader the difference could not be
    seen; it decides every traversal the moment `childNodes` and
    `firstChild` are readable, so the trees are made to agree here."""
    for child in list(node.childNodes):
        if child.nodeType in (8, 7):  # Comment, ProcessingInstruction
            node.removeChild(child)
            child.unlink()
            continue
        if child.nodeType == _DomElement.TEXT and not (child.nodeValue or "").strip():
            node.removeChild(child)
            child.unlink()
            continue
        _normalize_dom(child)


def _parse_xml_to_dom(xml_text: str, runtime: Any) -> Optional["_DomElement"]:
    """Parse `xml_text` and return a Document-refkind `_DomElement`
    wrapping the root, bound to `runtime` for downstream
    `table_from` calls. Returns `None` on parse failure so callers fall
    through to the W3C B.2 string fallback (matches cpp
    `XMLDocument::isValid()`-false behaviour).

    minidom rather than ElementTree, and the reason is `nodeType`:
    ElementTree has no CDATA node at all — it folds a `<![CDATA[…]]>`
    section into the surrounding text — so this backend could not tell
    the two character-data kinds apart while every other backend can."""
    if not xml_text:
        return None
    stripped = xml_text.strip()
    if not stripped.startswith("<"):
        return None
    try:
        document = _minidom.parseString(stripped)
    except Exception:
        # ExpatError and its friends: a document that does not parse is
        # the string reading's case, not an error to raise here.
        return None
    _normalize_dom(document)
    root = document.documentElement
    if root is None:
        return None
    return _DomElement(root, True, runtime)


def _json_to_lua_table(text: str) -> Optional[str]:
    """JSON→Lua literal rewriter. Returns None when the input is not
    JSON, so the caller falls through to the string reading.

    Scalars count. §scxml-B-2-8-1's JSON rung is about the payload being
    JSON, not about it being a container, and a `<donedata><content>'foo'
    </content>` arrives here as the JSON document `"foo"` (W3C test294) —
    while this only accepted `{`/`[`, that payload fell to the string
    rung and reached the datamodel WITH its quotes, so a guard reading
    `_event.data == 'foo'` was false. It was invisible while the sender
    shipped Lua source, because the rung above ran the text instead.

    A bare word like `hold the line` is still not JSON and still lands on
    the string rung, which is what makes the two readings distinguishable
    at all."""
    stripped = text.strip()
    if not stripped:
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
