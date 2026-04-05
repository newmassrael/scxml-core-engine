// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! Script engine abstraction for ECMAScript evaluation in SCXML.
//!
//! Ports C++ `sce/include/scripting/IScriptEngine.h` and
//! `sce/include/scripting/ScriptEngineProvider.h`.
//!
//! - [`IScriptEngine`]: the trait implementations expose (Lua, QuickJS, etc.)
//! - [`ScriptEngineProvider`]: compile-time engine selection via Cargo features
//!   (matches C++ `ScriptEngineProvider` with CMake `-DSCE_SCRIPT_ENGINE=...`)
//! - [`ScriptValue`], [`ScriptError`], [`ScriptResult`]: value/error types for
//!   IScriptEngine calls

pub mod i_script_engine;
pub mod provider;

pub use i_script_engine::{IScriptEngine, NativeMethod, ScriptError, ScriptResult, ScriptValue};
pub use provider::{has_script_engine, set_script_engine, ScriptEngineProvider};
