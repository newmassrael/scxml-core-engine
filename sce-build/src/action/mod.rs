// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Generated code as a build output (ADR 0097).
//!
//! A generation is a declared build action: a command, the inputs it may
//! read, the outputs it writes and the generator that runs it. Its key is the
//! digest of an REAPI `Action` over all of them, and its outputs are kept in a
//! content-addressed store under that key, locally and on the fleet, so that a
//! key seen before is never generated again and any machine sharing the store
//! takes the outputs by digest.
//!
//! - [`reapi`]: the REAPI messages a key and a result are made of.
//! - [`input_root`]: the Merkle tree of an action's inputs.
//! - [`store`]: the action cache and the content store, and the local one.
//! - [`identity`]: which generator an action names.
//! - [`generation`]: one generator invocation as an action, looked up by the
//!   files its last run read and run when they moved.
//! - [`tree`]: the manifest of a generated tree, `tools/codegen/trees/<tree>.toml`.
//! - [`materialize`]: a tree written into a directory from its actions, and
//!   compared with a committed copy.

pub mod generation;
pub mod identity;
pub mod input_root;
pub mod materialize;
pub mod reapi;
pub mod store;
pub mod tree;
