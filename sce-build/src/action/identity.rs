// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Which generator an action names (ADR 0097, answer 1): the digest of its
//! source closure, the compiler that builds it and how it is built, never the
//! digest of its binary.
//!
//! A binary's bytes are not reproducible across hosts, so a key over them
//! would give every machine a cache of its own; the sources, the toolchain and
//! the build settings are what decide what the binary does, and two machines
//! that agree on them run the same generator. A key computed from them needs
//! no generator built, so a machine with a warm cache takes every output
//! without building one.

use std::path::Path;
use std::process::Command;

use super::reapi::Property;
use crate::generator_witness;

/// The features `sce-codegen` is built with: the crate's defaults and `cli`,
/// the build `scripts/lib/sce_codegen.sh` and every route that builds the
/// generator uses (`cargo build --bin sce-codegen --features cli -p
/// sce-build`). The defaults themselves are in `sce-build/Cargo.toml`, a
/// witness member, so a change to them moves the source digest.
pub const GENERATOR_FEATURES: &str = "default,cli";

/// The profile the generator is built in: debug, the profile every build path
/// of this repository produces and the one the locators look in first
/// (`scripts/lib/sce_codegen.sh`). Named because `debug_assertions` differ
/// between profiles and can change what a run does.
pub const GENERATOR_PROFILE: &str = "debug";

/// The generator an action is run by.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeneratorIdentity {
    /// [`generator_witness::digest_hex`] of the checkout.
    pub source_digest: String,
    /// The compiler's `release`, `commit-hash` and `host`, from `rustc -vV`.
    pub rustc: String,
    /// The operating system the generator runs on (REAPI's `OSFamily`).
    pub os_family: String,
}

/// Why the generator of a checkout cannot be named.
#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    #[error("the generator's source closure could not be read: {0}")]
    Witness(#[from] generator_witness::WitnessError),
    #[error(
        "`{rustc} -vV` could not be run ({detail}); the compiler that builds the generator \
         is part of every action key, and a machine without one cannot compute a key"
    )]
    NoCompiler { rustc: String, detail: String },
    #[error("`{rustc} -vV` printed no `{field}:` line, so the compiler cannot be named")]
    CompilerUnnamed { rustc: String, field: &'static str },
}

impl GeneratorIdentity {
    /// The generator the checkout at `root` builds, with the compiler `cargo`
    /// would use: `$RUSTC` when it is set, else `rustc`.
    pub fn of_checkout(root: &Path) -> Result<GeneratorIdentity, IdentityError> {
        let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
        let output =
            Command::new(&rustc)
                .arg("-vV")
                .output()
                .map_err(|e| IdentityError::NoCompiler {
                    rustc: rustc.clone(),
                    detail: e.to_string(),
                })?;
        if !output.status.success() {
            return Err(IdentityError::NoCompiler {
                rustc,
                detail: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }
        let text = String::from_utf8_lossy(&output.stdout);
        Ok(GeneratorIdentity {
            source_digest: generator_witness::digest_hex(root)?,
            rustc: compiler_name(&text).map_err(|field| IdentityError::CompilerUnnamed {
                rustc: rustc.clone(),
                field,
            })?,
            os_family: std::env::consts::OS.to_string(),
        })
    }

    /// The identity as REAPI platform properties, the part of an action that
    /// says what runs it.
    pub fn properties(&self) -> Vec<Property> {
        let property = |name: &str, value: &str| Property {
            name: name.to_string(),
            value: value.to_string(),
        };
        vec![
            property("OSFamily", &self.os_family),
            property("sce.generator.features", GENERATOR_FEATURES),
            property("sce.generator.profile", GENERATOR_PROFILE),
            property("sce.generator.rustc", &self.rustc),
            property("sce.generator.source", &self.source_digest),
        ]
    }
}

/// The compiler named by the `release`, `commit-hash` and `host` lines of
/// `rustc -vV`, or the first of them that is missing.
fn compiler_name(verbose_version: &str) -> Result<String, &'static str> {
    let field = |name: &'static str| {
        verbose_version
            .lines()
            .find_map(|line| line.strip_prefix(name)?.strip_prefix(": "))
            .map(str::trim)
            .ok_or(name)
    };
    Ok(format!(
        "{} {} {}",
        field("release")?,
        field("commit-hash")?,
        field("host")?
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const RUSTC_VV: &str = "rustc 1.91.0 (f8297e351 2025-10-28)\n\
        binary: rustc\n\
        commit-hash: f8297e351a40c1439a467bbbb6879088047f50b3\n\
        commit-date: 2025-10-28\n\
        host: x86_64-unknown-linux-gnu\n\
        release: 1.91.0\n\
        LLVM version: 21.1.2\n";

    #[test]
    fn the_compiler_is_named_by_release_commit_and_host() {
        assert_eq!(
            compiler_name(RUSTC_VV).unwrap(),
            "1.91.0 f8297e351a40c1439a467bbbb6879088047f50b3 x86_64-unknown-linux-gnu"
        );
    }

    /// A toolchain built from source prints `commit-hash: unknown`; that is
    /// still a name. A line that is absent is not, and is refused rather than
    /// folded into a key as an empty string two compilers would share.
    #[test]
    fn a_missing_line_is_named() {
        let without_host: String = RUSTC_VV
            .lines()
            .filter(|l| !l.starts_with("host"))
            .map(|l| format!("{l}\n"))
            .collect();
        assert_eq!(compiler_name(&without_host), Err("host"));
    }

    #[test]
    fn the_identity_names_every_part_as_a_property() {
        let identity = GeneratorIdentity {
            source_digest: "s".into(),
            rustc: "r".into(),
            os_family: "linux".into(),
        };
        let names: Vec<String> = identity.properties().into_iter().map(|p| p.name).collect();
        assert_eq!(
            names,
            [
                "OSFamily",
                "sce.generator.features",
                "sce.generator.profile",
                "sce.generator.rustc",
                "sce.generator.source"
            ]
        );
    }
}
