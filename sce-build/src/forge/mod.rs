// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Forge: Extended SCXML kind system for multi-pattern code generation.
//
// Extends W3C SCXML with `sce:kind` attribute to support code generation
// beyond state machines: transforms, lookups, conditions, codecs, and more.

pub mod diagnostic;
pub mod error;
pub mod model;
pub mod parser;
pub mod types;
pub mod type_ctx;
pub mod expr;
pub mod generator;
pub mod manifest;
pub mod xsd_validator;
