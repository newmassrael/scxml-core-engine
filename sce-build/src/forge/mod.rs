// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Forge: Extended SCXML kind system for multi-pattern code generation.
//
// Extends W3C SCXML with `sce:kind` attribute to support code generation
// beyond state machines: transforms, lookups, conditions, codecs, and more.

pub mod ast_export;
pub mod buffer_pool_fsm;
pub mod codegen_markers;
pub mod codegen_matrix;
pub mod const_fold;
pub mod cross_doc_registry;
pub mod cross_kind_check;
pub mod diagnostic;
pub mod drift;
pub mod error;
pub mod event_schema_check;
pub mod expr;
pub mod extern_emit;
pub mod extern_validator;
pub mod generator;
pub mod intrinsic_registry;
pub mod limits;
pub mod manifest;
pub mod model;
pub mod native_action;
pub mod parser;
pub mod pool_registry;
pub mod provenance;
pub mod quantity;
pub mod quantity_check;
pub mod quantity_codegen;
pub mod sourcemap;
pub mod symbol_mangling;
pub mod target_plugin;
pub mod type_ctx;
pub mod types;
pub mod validate;
pub mod variant_default_overlay;
pub mod xsd_validator;
