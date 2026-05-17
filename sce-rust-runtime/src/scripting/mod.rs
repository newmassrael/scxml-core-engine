// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! Script engine abstraction for ECMAScript evaluation in SCXML.
//!
//! Ports C++ `sce/include/scripting/IScriptEngine.h`.
//!
//! - [`IScriptEngine`]: the trait implementations expose (Lua, QuickJS, etc.)
//! - [`ScriptValue`], [`ScriptError`], [`ScriptResult`]: value/error types for
//!   IScriptEngine calls
//!
//! Engine DI Parity RFC (Path B+): engines are constructed per-`Engine` and
//! passed into generated `Policy::new(engine)`, mirroring Kotlin's
//! per-instance constructor pattern. The pre-cleanup `ScriptEngineProvider`
//! singleton has been removed.

pub mod i_script_engine;

pub use i_script_engine::{
    IScriptEngine, NativeMethod, ScriptError, ScriptResult, ScriptValue, StateQueryCallback,
};
