// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Forge: Extended SCXML kind system for multi-pattern code generation.
//
// Extends W3C SCXML with `sce:kind` attribute to support code generation
// beyond state machines: transforms, lookups, conditions, codecs, and more.

pub mod buffer_pool_fsm;
pub mod codegen_matrix;
pub mod const_fold;
pub mod diagnostic;
pub mod error;
pub mod limits;
pub mod link_registry;
pub mod model;
pub mod parser;
pub mod pool_registry;
pub mod types;
pub mod type_ctx;
pub mod expr;
pub mod generator;
pub mod manifest;
pub mod validate;
pub mod xsd_validator;
