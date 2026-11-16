// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! `sce-codegen`, the command line of the SCE code generator.
//!
//! The arguments and every command are the library's (`sce_build::cli`), so
//! that a build script linking the library runs the same generator as this
//! binary does (ADR 0097).

fn main() {
    sce_build::cli::main()
}
