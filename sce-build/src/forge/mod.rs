// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
//
// SCE Forge: Extended SCXML kind system for multi-pattern code generation.
//
// Extends W3C SCXML with `sce:kind` attribute to support code generation
// beyond state machines: transforms, lookups, conditions, codecs, and more.

pub mod model;
pub mod parser;
pub mod types;
pub mod type_ctx;
pub mod expr;
pub mod generator;
pub mod manifest;
