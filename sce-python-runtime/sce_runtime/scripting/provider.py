# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""ScriptEngineProvider — 1:1 port of C++ `ScriptEngineProvider`.

A single process-wide script engine singleton, registered at import time
(default = `LuaScriptEngine`). Mirrors the Rust `OnceLock` and C++
static-singleton patterns: callers acquire the engine via `get()` with
zero overhead — no runtime DI, no factory chain.

The hook `set_script_engine(...)` lets advanced users swap to a custom
implementation (e.g. a QuickJS-backed engine, a test stub) before any
state machine is created. After the first `get()` call the singleton is
locked."""

from __future__ import annotations

from threading import Lock
from typing import Optional

from .i_script_engine import IScriptEngine
from .lua_engine import LuaScriptEngine


class ScriptEngineAlreadyRegisteredError(RuntimeError):
    """Raised when `set_script_engine` is called after `get()`. The
    singleton is intentionally immutable for the program's lifetime so
    generated state machines never observe a mid-flight engine swap."""


_engine: Optional[IScriptEngine] = None
_locked: bool = False
_lock = Lock()


def set_script_engine(engine: IScriptEngine) -> None:
    """Replace the default Lua engine with a custom impl. Must be
    called before any `Engine` is constructed (i.e., before the first
    `get()` call). Raises if the singleton is already locked."""
    global _engine, _locked
    with _lock:
        if _locked:
            raise ScriptEngineAlreadyRegisteredError(
                "ScriptEngineProvider.set_script_engine must be called before "
                "the first get() — the singleton is locked once a state machine "
                "queries the provider."
            )
        _engine = engine
        _engine.initialize()


def get() -> IScriptEngine:
    """Return the registered engine, instantiating the default
    `LuaScriptEngine` on first access. Subsequent calls return the same
    instance with zero overhead (matches C++ + Rust singleton pattern)."""
    global _engine, _locked
    with _lock:
        if _engine is None:
            _engine = LuaScriptEngine()
            _engine.initialize()
        _locked = True
        return _engine


def reset_for_tests() -> None:
    """Test-only hook — destroy the singleton so the next `get()` builds
    a fresh engine. NOT for production use; mirrors C++
    `ScriptEngineProvider::resetForTests`."""
    global _engine, _locked
    with _lock:
        if _engine is not None:
            _engine.shutdown()
        _engine = None
        _locked = False
