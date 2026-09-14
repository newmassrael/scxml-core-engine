// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Which templates the generator registers, and in which syntax it reads each.
//!
//! The population every sweep over the template tree wants, and NOT the same
//! thing as "the `.jinja2` files on disk": a template is read as the language
//! it emits, one file can be registered by more than one backend, and a
//! walk that pairs each file with a syntax of its own choosing measures a tree
//! the generator never sees.
//!
//! # Why it is here rather than in one gate
//!
//! Both encoding gates need it — `a_value_written_into_a_comment_is_encoded`
//! and `a_value_written_into_a_string_literal_is_escaped` — because the
//! generator has two encoding doors and each one's census is over the same
//! registrations. A second copy would be a second answer to what the generator
//! registers, and the answer is derived from the loaders rather than declared,
//! precisely so that it cannot be.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sce_build::generator::{loader_template_files, Language};
use sce_build::template_lexing::Syntax;

/// Where the template tree lives, relative to the workspace root.
pub const TEMPLATE_ROOT: &str = "tools/codegen/templates";

/// Every `(template on disk, syntax it is read in)` the generator registers,
/// mapped to the name it is registered under.
///
/// Read from the statechart loaders of every backend. The C++ and C11 loaders
/// are rooted at the whole template tree, so between them they register every
/// template, and each other backend's loader adds the shared `_macros/` tree
/// under that backend's syntax. The forge and mesh loaders register subsets of
/// the root's templates under names whose extension already fixes the syntax,
/// so they add no pair this does not hold.
pub fn registrations(root: &Path) -> BTreeMap<(PathBuf, Syntax), String> {
    let templates = root.join(TEMPLATE_ROOT);
    let mut out = BTreeMap::new();
    for &language in Language::ALL {
        let dir = templates.join(language.template_subdir());
        for (name, path) in loader_template_files(&dir) {
            let syntax = Syntax::of_template(&name, language);
            out.entry((path, syntax)).or_insert(name);
        }
    }
    out
}
