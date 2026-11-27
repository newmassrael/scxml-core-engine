// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The tools the formal judges run, pinned by the judge (docs/adr/0146, D3).
//!
//! `sce-build/src/formal/toolchain.json` names each tool's release, where it is
//! fetched from and the sha256 of what is fetched. It is compiled into the
//! binary, so a binary of a revision knows its pins without a tree, and a
//! consumer that pins the revision pins the tools with it: the judge and its
//! tools are what this repository's gates ran together.
//!
//! A judge never fetches; `sce-codegen formal install` does, by the installer
//! beside the pins (`install_tool.sh`, compiled in as `INSTALLER`), which
//! fetches by `curl` and admits the bytes by their digest. A build machine's
//! declaration runs the same script from the tree, so the procedure is written
//! once, and the pins are read from this one file by both. The transport is not trusted, the digest is
//! (the fixed-output derivations of Nix, Dolstra 2006). A judge admits what it
//! finds by the same digest before it runs it, so a tool replaced under the same
//! name is refused as `formal/tool-unavailable`, never run.
//!
//! The Java runtime TLC and Apalache run on is pinned with them: a tool pinned by
//! its digest that runs on whatever virtual machine a machine has is not pinned.
//!
//! Storm, the probabilistic checker of the level of quantity (docs/adr/0119,
//! D3), is published as a Debian package and no static build: the package holds
//! the program and the libraries built with it, and asks the system for the
//! arithmetic and solver libraries (GMP, CLN, GiNaC, GLPK, Z3, libarchive). Those
//! are named where a gate's system packages are, in the host image
//! (docs/adr/0145) and in `bx.toml` for a build machine; the package is
//! unpacked, never installed, and its program is run with its own libraries.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::FormalError;

/// The surface's stability (docs/adr/0146, D5; `SCE_WIRE_CONTRACTS.md`), which the
/// header of `schemas/sce-formal-toolchain.v1.schema.json` repeats.
pub const TOOLCHAIN_SCHEMA_STATUS: &str = "pre-release";

/// The pins, as the file spells them.
pub const TOOLCHAIN_JSON: &str = include_str!("toolchain.json");

/// TLC, the model checker of levels 2, 4 and 5. A stable release: `v1.8.0` is a
/// rolling pre-release whose asset is replaced (its digest moved between
/// 2026-10-17 and 2026-11-19), so a machine that fetched it later got another
/// program under the same name.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TlcPin {
    pub version: String,
    pub url: String,
    pub sha256: String,
}

/// Verus, the checker of level 6, with the Rust release it was built with and
/// runs on.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VerusPin {
    pub version: String,
    pub toolchain: String,
    pub url: String,
    pub sha256: String,
}

/// Apalache, the symbolic checker of level 3 (docs/adr/0115, D3): the
/// release's archive, which holds its launcher, its jar and the standard module
/// `Apalache` that a model extends to be read by it, and TLC with it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApalachePin {
    pub version: String,
    pub url: String,
    pub sha256: String,
}

/// The Java runtime TLC and Apalache run on: the archive of Eclipse Temurin's
/// runtime for Linux on x64, which unpacks as `jdk-<version>-jre/`. Apalache
/// 0.63.0 is built for Java 21 (class file 65): on 2026-11-27 a build machine
/// whose `java` was 17 answered `UnsupportedClassVersionError` to every
/// inductive run, a run that judged nothing.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JavaPin {
    pub version: String,
    pub url: String,
    pub sha256: String,
}

/// Storm, the probabilistic model checker of the level of quantity
/// (docs/adr/0119, D3): the release's Debian 12 package for x64, whose
/// `usr/bin/storm-<version>` is run with the libraries of its `usr/lib`. The
/// Debian 12 build and not the Debian 13 one: it asks the C library for no
/// symbol past GLIBC_2.34, which the Ubuntu 24.04 of the host image has, and the
/// Debian 13 build asks for a newer one.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StormPin {
    pub version: String,
    pub url: String,
    pub sha256: String,
}

/// Every pinned tool.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Toolchain {
    pub tlc: TlcPin,
    pub verus: VerusPin,
    pub apalache: ApalachePin,
    pub java: JavaPin,
    pub storm: StormPin,
}

/// The pins this binary was built with.
pub fn pinned() -> Toolchain {
    serde_json::from_str(TOOLCHAIN_JSON)
        .expect("sce-build/src/formal/toolchain.json is held to its shape by the unit tests")
}

fn unavailable(tool: &str, detail: impl Into<String>) -> FormalError {
    FormalError::ToolUnavailable {
        tool: tool.to_string(),
        detail: detail.into(),
    }
}

fn home() -> Result<PathBuf, FormalError> {
    std::env::var_os("HOME")
        .filter(|home| !home.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| {
            unavailable(
                "a home directory",
                "HOME is not set, and the tools are kept under it",
            )
        })
}

/// Where the pinned TLC is kept: `~/.cache/sce-formal/tla2tools-<version>.jar`,
/// or the file `TLA2TOOLS` names. Either is admitted by the pin's digest.
pub fn tlc_place(pin: &TlcPin) -> Result<PathBuf, FormalError> {
    if let Some(named) = std::env::var_os("TLA2TOOLS").filter(|named| !named.is_empty()) {
        return Ok(PathBuf::from(named));
    }
    Ok(home()?
        .join(".cache")
        .join("sce-formal")
        .join(format!("tla2tools-{}.jar", pin.version)))
}

/// Where the pinned Verus is unpacked: `~/.local/verus/<version>`, the place
/// `bx.toml` declares for a build machine.
pub fn verus_place(pin: &VerusPin) -> Result<PathBuf, FormalError> {
    Ok(home()?.join(".local").join("verus").join(&pin.version))
}

/// Where the Rust release Verus runs on is: `~/.local/rust/<release>`, where
/// `scripts/install_rust.sh` puts every declared release.
pub fn verus_rust_place(pin: &VerusPin) -> Result<PathBuf, FormalError> {
    Ok(home()?.join(".local").join("rust").join(&pin.toolchain))
}

/// Where the pinned Apalache is unpacked: `~/.local/apalache/<version>`, the
/// place `bx.toml` declares for a build machine.
pub fn apalache_place(pin: &ApalachePin) -> Result<PathBuf, FormalError> {
    Ok(home()?.join(".local").join("apalache").join(&pin.version))
}

/// Where the pinned Java runtime is unpacked: `~/.local/jre/<version>`.
pub fn java_place(pin: &JavaPin) -> Result<PathBuf, FormalError> {
    Ok(home()?.join(".local").join("jre").join(&pin.version))
}

/// Where the pinned Storm is unpacked: `~/.local/storm/<version>`, the place
/// `bx.toml` declares for a build machine.
pub fn storm_place(pin: &StormPin) -> Result<PathBuf, FormalError> {
    Ok(home()?.join(".local").join("storm").join(&pin.version))
}

/// The program `bin/java` of the Java runtime whose home is `home`.
pub fn java_program(home: &Path) -> PathBuf {
    home.join("bin").join("java")
}

/// What a refusal names as the tool when the Java runtime is missing.
const JAVA: &str = "the Java runtime TLC and Apalache run on";

/// The directory under an unpacked Apalache that holds the standard module
/// `Apalache`, taken out of the release's jar by `formal install`.
const APALACHE_MODULES: &str = "modules";

/// The lowercase hex sha256 of a file's bytes.
pub fn sha256_of(path: &Path) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

/// The pinned TLC, found and admitted by its digest.
pub fn admitted_tlc(pin: &TlcPin) -> Result<PathBuf, FormalError> {
    let jar = tlc_place(pin)?;
    if !jar.is_file() {
        return Err(unavailable(
            "TLC",
            format!("no TLC at {}: sce-codegen formal install", jar.display()),
        ));
    }
    admit_tlc(pin, &jar)?;
    Ok(jar)
}

/// Refuse the TLC at `jar` unless its bytes are the ones `pin` names: a jar is
/// run as it is found, with nothing unpacked from it, so it is admitted by its
/// digest each time it is found.
fn admit_tlc(pin: &TlcPin, jar: &Path) -> Result<(), FormalError> {
    let actual = sha256_of(jar)
        .map_err(|e| unavailable("TLC", format!("cannot read {}: {e}", jar.display())))?;
    if actual != pin.sha256 {
        return Err(unavailable(
            "TLC",
            format!(
                "{} is not the TLC this judge pins (sha256 {actual}, pinned {})",
                jar.display(),
                pin.sha256
            ),
        ));
    }
    Ok(())
}

/// The pinned Verus and the Rust release it runs on, found. Verus is not
/// admitted by digest here: what was admitted is the archive it was unpacked
/// from, by `formal install`, and the program answers its release by itself.
pub fn admitted_verus(pin: &VerusPin) -> Result<PathBuf, FormalError> {
    let tool = verus_place(pin)?.join("bin").join("verus");
    if !tool.is_file() {
        return Err(unavailable(
            "Verus",
            format!("no Verus at {}: sce-codegen formal install", tool.display()),
        ));
    }
    let rust = verus_rust_place(pin)?;
    if !rust.join("lib").is_dir() {
        return Err(unavailable(
            "the Rust release Verus runs on",
            format!(
                "no Rust {} at {}: scripts/install_rust.sh --declared {} in this repository installs it there",
                pin.toolchain,
                rust.display(),
                pin.toolchain
            ),
        ));
    }
    Ok(tool)
}

/// The pinned Apalache, found: the program `bin/apalache` under the unpacked
/// release. It is not admitted by digest here, as Verus is not: what was
/// admitted is the archive it was unpacked from, by `formal install`.
pub fn admitted_apalache(pin: &ApalachePin) -> Result<PathBuf, FormalError> {
    let tool = apalache_place(pin)?.join("bin").join("apalache");
    if !tool.is_file() {
        return Err(unavailable(
            "Apalache",
            format!(
                "no Apalache at {}: sce-codegen formal install",
                tool.display()
            ),
        ));
    }
    Ok(tool)
}

/// The pinned Java runtime, found: the directory it was unpacked into, whose
/// `bin/java` TLC and Apalache are run by. It is not admitted by digest here,
/// as Verus and Apalache are not: what was admitted is the archive it was
/// unpacked from, by `formal install`, and the release is in its place's name.
pub fn admitted_java(pin: &JavaPin) -> Result<PathBuf, FormalError> {
    let home = java_place(pin)?;
    let java = java_program(&home);
    if !java.is_file() {
        return Err(unavailable(
            JAVA,
            format!(
                "no Java {} at {}: sce-codegen formal install",
                pin.version,
                java.display()
            ),
        ));
    }
    Ok(home)
}

/// The pinned Storm, found: the program `bin/storm` under the unpacked
/// package. It is not admitted by digest here, as Apalache is not: what was
/// admitted is the package it was unpacked from, by `formal install`.
pub fn admitted_storm(pin: &StormPin) -> Result<PathBuf, FormalError> {
    let tool = storm_place(pin)?.join("bin").join("storm");
    if !tool.is_file() {
        return Err(unavailable(
            "Storm",
            format!("no Storm at {}: sce-codegen formal install", tool.display()),
        ));
    }
    Ok(tool)
}

/// The directory that holds the standard module `Apalache` of the pinned
/// release, found. TLC reads a model that extends it from there, so the module
/// both tools read is the one release's (docs/adr/0146, D3).
pub fn admitted_apalache_modules(pin: &ApalachePin) -> Result<PathBuf, FormalError> {
    let modules = apalache_place(pin)?.join(APALACHE_MODULES);
    if !modules.join("Apalache.tla").is_file() {
        return Err(unavailable(
            "Apalache's standard module",
            format!(
                "no Apalache.tla in {}, which a model that extends Apalache is read with: \
                 sce-codegen formal install",
                modules.display()
            ),
        ));
    }
    Ok(modules)
}

/// What one install did, for its report line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Installed {
    /// The tool was there and is the one pinned.
    Present(PathBuf),
    /// The tool was fetched, admitted and put there.
    Fetched(PathBuf),
}

/// The installer of the pinned tools, as the tree holds it beside the pins:
/// the one procedure that fetches a tool, admits what was fetched by its pin's
/// digest and unpacks it (docs/adr/0146, D3). A build machine's declaration
/// runs it from the tree (`bx.toml`, a table for each tool); it is compiled
/// into the binary with the pins, so `formal install` runs the same procedure
/// where there is no tree.
pub const INSTALLER: &str = include_str!("install_tool.sh");

/// Install the tool `pin` pins, whose name in the pins is `key` and in a
/// refusal `tool`, into `place` by the installer, run beside a
/// `toolchain.json` that holds that pin alone, as it is run beside the tree's
/// pins. `place` is absent or an empty directory: the installer refuses one
/// that holds anything, and puts the tool there whole or not at all.
fn run_installer<P: Serialize>(
    tool: &str,
    key: &str,
    pin: &P,
    place: &Path,
) -> Result<(), FormalError> {
    let pin = serde_json::to_string(pin)
        .map_err(|e| unavailable(tool, format!("cannot write its pin for the installer: {e}")))?;
    let pins = format!("{{\"{key}\": {pin}}}\n");
    let scratch = super::WorkDir::new("install")?;
    for (name, text) in [
        ("install_tool.sh", INSTALLER),
        ("toolchain.json", pins.as_str()),
    ] {
        let path = scratch.path().join(name);
        std::fs::write(&path, text)
            .map_err(|e| unavailable(tool, format!("cannot write {}: {e}", path.display())))?;
    }
    let script = scratch.path().join("install_tool.sh");
    let ran = Command::new("bash")
        .arg(&script)
        .arg(key)
        .arg(place)
        .output()
        .map_err(|e| unavailable(tool, format!("cannot run bash for its installer: {e}")))?;
    if !ran.status.success() {
        return Err(unavailable(
            tool,
            format!(
                "its installer did not put it at {}: {}",
                place.display(),
                String::from_utf8_lossy(&ran.stderr).trim()
            ),
        ));
    }
    Ok(())
}

/// Fetch the pinned TLC into `place` (`tlc_place`), unless the one pinned is
/// there.
pub fn install_tlc(pin: &TlcPin, place: &Path) -> Result<Installed, FormalError> {
    if place.is_file() {
        admit_tlc(pin, place)?;
        return Ok(Installed::Present(place.to_path_buf()));
    }
    run_installer("TLC", "tlc", pin, place)?;
    Ok(Installed::Fetched(place.to_path_buf()))
}

/// Fetch the pinned Verus into `place` (`verus_place`), unless it is there:
/// the release's `x86-linux` archive, admitted by its digest and unpacked, with
/// `bin/verus`, the program run on its Rust release with no rustup.
pub fn install_verus(pin: &VerusPin, place: &Path) -> Result<Installed, FormalError> {
    if place.join("bin").join("verus").is_file() {
        return Ok(Installed::Present(place.to_path_buf()));
    }
    run_installer("Verus", "verus", pin, place)?;
    Ok(Installed::Fetched(place.to_path_buf()))
}

/// Fetch the pinned Java runtime into `place` (`java_place`), unless it is
/// there: the archive, admitted by its digest, whose `jdk-<version>-jre/` is
/// the runtime.
pub fn install_java(pin: &JavaPin, place: &Path) -> Result<Installed, FormalError> {
    if java_program(place).is_file() {
        return Ok(Installed::Present(place.to_path_buf()));
    }
    run_installer(JAVA, "java", pin, place)?;
    Ok(Installed::Fetched(place.to_path_buf()))
}

/// Fetch the pinned Apalache into `place` (`apalache_place`), unless it is
/// there: the release's archive, admitted by its digest and unpacked, with the
/// standard module `Apalache` taken out of its jar into `modules/`, where TLC
/// reads it for a model that extends it, and `bin/apalache`, the release's own
/// launcher behind a line that answers `--version` with no JVM.
pub fn install_apalache(pin: &ApalachePin, place: &Path) -> Result<Installed, FormalError> {
    if place.join("bin").join("apalache").is_file() {
        return Ok(Installed::Present(place.to_path_buf()));
    }
    run_installer("Apalache", "apalache", pin, place)?;
    Ok(Installed::Fetched(place.to_path_buf()))
}

/// Fetch the pinned Storm into `place` (`storm_place`), unless it is there:
/// the Debian package, admitted by its digest and unpacked, never installed,
/// with `bin/storm`, the package's program run with the libraries the package
/// holds. What it asks of the system it does not get by being unpacked; the
/// image and the build machines name those libraries themselves.
pub fn install_storm(pin: &StormPin, place: &Path) -> Result<Installed, FormalError> {
    if place.join("bin").join("storm").is_file() {
        return Ok(Installed::Present(place.to_path_buf()));
    }
    run_installer("Storm", "storm", pin, place)?;
    Ok(Installed::Fetched(place.to_path_buf()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pins_are_whole_and_of_their_shape() {
        let pins = pinned();
        for (tool, sha256, url) in [
            ("tlc", &pins.tlc.sha256, &pins.tlc.url),
            ("verus", &pins.verus.sha256, &pins.verus.url),
            ("apalache", &pins.apalache.sha256, &pins.apalache.url),
            ("java", &pins.java.sha256, &pins.java.url),
            ("storm", &pins.storm.sha256, &pins.storm.url),
        ] {
            assert_eq!(sha256.len(), 64, "{tool}: a sha256 is 64 hex digits");
            assert!(
                sha256
                    .chars()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
                "{tool}: lowercase hex"
            );
            assert!(url.starts_with("https://"), "{tool}: fetched over https");
        }
        assert!(
            pins.tlc.url.contains(&format!("/v{}/", pins.tlc.version)),
            "the TLC fetched is the release the pin names"
        );
        assert!(
            pins.verus.url.contains(&pins.verus.version),
            "the Verus fetched is the release the pin names"
        );
        assert!(
            pins.apalache
                .url
                .ends_with(&format!("/v{0}/apalache-{0}.tgz", pins.apalache.version)),
            "the Apalache fetched is the archive of the release the pin names, \
             which the installer unpacks as apalache-<version>/"
        );
        assert!(
            pins.storm.url.ends_with(&format!(
                "/{0}/storm-{0}-debian12-amd64.deb",
                pins.storm.version
            )),
            "the Storm fetched is the Debian 12 package for x64 of the release the pin names, \
             whose program the installer finds as usr/bin/storm-<version>: {}",
            pins.storm.url
        );
    }

    #[test]
    fn the_java_pin_is_the_runtime_its_archive_unpacks_as() {
        let pins = pinned();
        let release = &pins.java.version;
        assert!(
            release.starts_with("21."),
            "Apalache {} is built for Java 21, and TLC runs on it too",
            pins.apalache.version
        );
        assert!(
            pins.java
                .url
                .contains(&format!("/jdk-{}/", release.replace('+', "%2B"))),
            "the runtime fetched is the release the pin names: {}",
            pins.java.url
        );
        assert!(
            pins.java.url.ends_with(&format!(
                "-jre_x64_linux_hotspot_{}.tar.gz",
                release.replace('+', "_")
            )),
            "the runtime fetched is the JRE for Linux on x64: {}",
            pins.java.url
        );
        assert_eq!(
            java_program(Path::new("/opt/jre")),
            Path::new("/opt/jre/bin/java")
        );
    }

    #[test]
    fn the_status_of_the_surface_is_the_schemas() {
        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../schemas/sce-formal-toolchain.v1.schema.json"
        ))
        .expect("the schema is JSON");
        assert_eq!(
            schema["x-sce-schema-status"],
            serde_json::Value::from(TOOLCHAIN_SCHEMA_STATUS)
        );
    }

    #[test]
    fn every_pin_is_one_line_of_strings_as_the_installer_and_bx_read_it() {
        let pins: serde_json::Value =
            serde_json::from_str(TOOLCHAIN_JSON).expect("the pins are JSON");
        let tools = pins.as_object().expect("the pins are an object of tools");
        assert_eq!(tools.len(), 5, "TLC, Verus, Apalache, Java and Storm");
        for (key, pin) in tools {
            let fields = pin.as_object().expect("a pin is an object");
            for (field, value) in fields {
                let value = value
                    .as_str()
                    .unwrap_or_else(|| panic!("{key}.{field}: the installer reads strings alone"));
                assert!(
                    !value.contains(['"', '\\', '{', '}', '\n']),
                    "{key}.{field}: the installer reads a value up to its closing quote, \
                     and a pin up to its closing brace: {value}"
                );
            }
            let version = fields["version"].as_str().expect("a pin names its version");
            let head = format!("  \"{key}\": {{\"version\": \"{version}\"");
            let lines: Vec<&str> = TOOLCHAIN_JSON
                .lines()
                .filter(|line| line.contains(&format!("\"{key}\":")))
                .collect();
            assert_eq!(
                lines.len(),
                1,
                "{key}: one line holds the pin, as bx.toml's version_pattern reads it"
            );
            assert!(
                lines[0].starts_with(&head),
                "{key}: the line starts with the version, as bx.toml's version_pattern reads it: {}",
                lines[0]
            );
        }
    }

    #[test]
    fn the_installer_writes_the_programs_the_judge_runs_and_bx_checks() {
        for program in [
            "verus_wrapper >\"$built/bin/verus\"",
            "apalache_wrapper >\"$built/bin/apalache\"",
            "storm_wrapper >\"$built/bin/storm\"",
            "built=\"$unpacked/jdk-$version-jre\"",
            "mkdir -p \"$built/modules\"",
        ] {
            assert!(
                INSTALLER.contains(program),
                "the installer writes what the judge finds: {program}"
            );
        }
        for answer in [
            "echo \"verus $version\"",
            "echo \"apalache $version\"",
            "echo \"storm $version\"",
        ] {
            assert!(
                INSTALLER.contains(answer),
                "a wrapper answers --version with no runtime, as bx.toml's check asks: {answer}"
            );
        }
        assert!(INSTALLER.contains("VERUS_USE_RUSTUP=0"));
        assert!(INSTALLER.contains("rust=\"\\$HOME/.local/rust/$toolchain\""));
        assert!(INSTALLER.contains(
            "LD_LIBRARY_PATH=\"\\$here/usr/lib\" exec \"\\$here/usr/bin/storm-$version\""
        ));
        assert!(INSTALLER.contains("/apalache-mc\" \"\\$@\""));
    }

    /// A file's URL, which curl fetches as it fetches a release's.
    #[cfg(target_os = "linux")]
    fn url_of(path: &Path) -> String {
        format!("file://{}", path.display())
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn a_tool_of_another_digest_is_refused_and_nothing_is_put_in_its_place() {
        let dir = super::super::WorkDir::new("test-admit").expect("a working directory");
        let fetched = dir.path().join("released.jar");
        std::fs::write(&fetched, b"not TLC").expect("written");
        let place = dir.path().join("cache").join("tla2tools.jar");
        let mut pin = TlcPin {
            version: "0.0.0".to_string(),
            url: url_of(&fetched),
            sha256: "0".repeat(64),
        };

        let refused = install_tlc(&pin, &place).expect_err("another digest");
        match refused {
            FormalError::ToolUnavailable { detail, .. } => {
                assert!(detail.contains("this judge pins"), "{detail}")
            }
            other => panic!("refused as unavailable, not {other:?}"),
        }
        assert!(!place.exists(), "nothing is put where the judge looks");
        let left: Vec<_> = std::fs::read_dir(dir.path().join("cache"))
            .expect("the place's directory")
            .collect();
        assert!(left.is_empty(), "the installer's scratch is gone: {left:?}");

        pin.sha256 = sha256_of(&fetched).expect("read");
        assert_eq!(
            install_tlc(&pin, &place).expect("the pinned bytes"),
            Installed::Fetched(place.clone())
        );
        assert_eq!(std::fs::read(&place).expect("installed"), b"not TLC");
        assert_eq!(
            install_tlc(&pin, &place).expect("there"),
            Installed::Present(place.clone())
        );

        std::fs::write(&place, b"another TLC").expect("replaced");
        assert!(
            matches!(
                install_tlc(&pin, &place),
                Err(FormalError::ToolUnavailable { .. })
            ),
            "a jar replaced under the same name is refused, never taken as present"
        );
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn the_runtime_is_put_whole_into_the_empty_place_a_tool_store_gives() {
        let dir = super::super::WorkDir::new("test-install").expect("a working directory");
        let version = "21.0.0+1";
        let root = format!("jdk-{version}-jre");
        let released = dir.path().join("released");
        std::fs::create_dir_all(released.join(&root).join("bin")).expect("made");
        std::fs::create_dir_all(released.join(&root).join("lib")).expect("made");
        std::fs::write(
            released.join(&root).join("bin").join("java"),
            b"#!/bin/sh\n",
        )
        .expect("written");
        std::fs::write(released.join(&root).join("lib").join("modules"), b"modules")
            .expect("written");
        let archive = dir.path().join("jre.tar.gz");
        let packed = Command::new("tar")
            .arg("-czf")
            .arg(&archive)
            .arg("-C")
            .arg(&released)
            .arg(&root)
            .status()
            .expect("tar runs");
        assert!(packed.success(), "the release's archive is made");
        let pin = JavaPin {
            version: version.to_string(),
            url: url_of(&archive),
            sha256: sha256_of(&archive).expect("read"),
        };

        let store = dir.path().join("store");
        let place = store.join("entry");
        std::fs::create_dir_all(&place).expect("the store's empty entry");
        assert_eq!(
            install_java(&pin, &place).expect("installed"),
            Installed::Fetched(place.clone())
        );
        assert!(java_program(&place).is_file());
        assert_eq!(
            std::fs::read(place.join("lib").join("modules")).expect("unpacked"),
            b"modules"
        );
        let beside: Vec<_> = std::fs::read_dir(&store)
            .expect("the store")
            .map(|entry| entry.expect("an entry").file_name())
            .collect();
        assert_eq!(
            beside,
            [std::ffi::OsString::from("entry")],
            "no scratch is left"
        );
        assert_eq!(
            install_java(&pin, &place).expect("there"),
            Installed::Present(place.clone())
        );

        let held = store.join("held");
        std::fs::create_dir_all(&held).expect("made");
        std::fs::write(held.join("kept"), b"kept").expect("written");
        assert!(
            install_java(&pin, &held).is_err(),
            "a place that holds anything is refused, never written into"
        );
        assert_eq!(std::fs::read(held.join("kept")).expect("kept"), b"kept");
        assert!(!java_program(&held).exists());
    }
}
