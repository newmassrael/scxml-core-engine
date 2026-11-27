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
//! A judge never fetches; `sce-codegen formal install` does, by `curl`, and
//! admits the bytes by their digest. The transport is not trusted, the digest is
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

use std::ffi::OsStr;
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

/// The program of the package, under the place it is unpacked in.
fn storm_program_in_package(pin: &StormPin) -> PathBuf {
    Path::new("usr")
        .join("bin")
        .join(format!("storm-{}", pin.version))
}

/// The program `bin/java` of the Java runtime whose home is `home`.
pub fn java_program(home: &Path) -> PathBuf {
    home.join("bin").join("java")
}

/// The directory the archive of the pinned Java runtime holds the runtime in.
fn java_archive_root(pin: &JavaPin) -> String {
    format!("jdk-{}-jre", pin.version)
}

/// What a refusal names as the tool when the Java runtime is missing.
const JAVA: &str = "the Java runtime TLC and Apalache run on";

/// The directory under an unpacked Apalache that holds the standard module
/// `Apalache`, taken out of the release's jar by `formal install`.
const APALACHE_MODULES: &str = "modules";

/// The one standard module of Apalache a model may extend, as its jar holds it.
const APALACHE_MODULE_IN_JAR: &str = "tla2sany/StandardModules/Apalache.tla";

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
    let actual = sha256_of(&jar)
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
    Ok(jar)
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

/// Fetch `url` to `to` by `curl`, which follows redirects and fails on an
/// HTTP error.
fn fetch(tool: &str, url: &str, to: &Path) -> Result<(), FormalError> {
    let ran = Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--retry",
            "3",
            "--output",
        ])
        .arg(to)
        .arg(url)
        .output()
        .map_err(|e| unavailable(tool, format!("cannot run curl to fetch {url}: {e}")))?;
    if !ran.status.success() {
        return Err(unavailable(
            tool,
            format!(
                "curl could not fetch {url}: {}",
                String::from_utf8_lossy(&ran.stderr).trim()
            ),
        ));
    }
    Ok(())
}

fn admit(tool: &str, path: &Path, url: &str, pinned: &str) -> Result<(), FormalError> {
    let actual = sha256_of(path)
        .map_err(|e| unavailable(tool, format!("cannot read {}: {e}", path.display())))?;
    if actual != pinned {
        return Err(unavailable(
            tool,
            format!("{url} is not the {tool} this judge pins (sha256 {actual}, pinned {pinned})"),
        ));
    }
    Ok(())
}

/// What one install did, for its report line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Installed {
    /// The tool was there and is the one pinned.
    Present(PathBuf),
    /// The tool was fetched, admitted and put there.
    Fetched(PathBuf),
}

/// Fetch the pinned TLC where the judges look, unless the one pinned is there.
pub fn install_tlc(pin: &TlcPin) -> Result<Installed, FormalError> {
    let jar = tlc_place(pin)?;
    if jar.is_file() {
        admit("TLC", &jar, &pin.url, &pin.sha256)?;
        return Ok(Installed::Present(jar));
    }
    let parent = jar.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)
        .map_err(|e| unavailable("TLC", format!("cannot create {}: {e}", parent.display())))?;
    let partial = jar.with_extension("partial");
    fetch("TLC", &pin.url, &partial)?;
    if let Err(refused) = admit("TLC", &partial, &pin.url, &pin.sha256) {
        let _ = std::fs::remove_file(&partial);
        return Err(refused);
    }
    std::fs::rename(&partial, &jar).map_err(|e| {
        unavailable(
            "TLC",
            format!(
                "cannot move {} to {}: {e}",
                partial.display(),
                jar.display()
            ),
        )
    })?;
    Ok(Installed::Fetched(jar))
}

/// The program `bin/verus` under the unpacked release: Verus run on its Rust
/// release with no rustup, which answers `--version` by itself so that a check
/// of the installation asks nothing of that release. Verus asks rustup for its
/// Rust release unless told not to; then it is the one whose libraries it finds.
pub fn verus_wrapper(pin: &VerusPin) -> String {
    format!(
        "#!/bin/sh\n\
         # Written by sce-codegen formal install: Verus {version} on Rust {toolchain}, with no rustup.\n\
         if [ \"$1\" = --version ]; then\n    echo \"verus {version}\"\n    exit 0\nfi\n\
         rust=\"$HOME/.local/rust/{toolchain}\"\n\
         VERUS_USE_RUSTUP=0 LD_LIBRARY_PATH=\"$rust/lib\" exec \"$(dirname \"$(readlink -f \"$0\")\")/../verus\" --sysroot \"$rust\" \"$@\"\n",
        version = pin.version,
        toolchain = pin.toolchain,
    )
}

/// Fetch the pinned Verus, admit the archive by its digest and unpack it into
/// `place` (`verus_place` unless a build machine's declaration names another),
/// unless it is there. The archive is the release's `x86-linux` one, unpacked
/// by `unzip`, which keeps each file's mode.
pub fn install_verus(pin: &VerusPin, place: &Path) -> Result<Installed, FormalError> {
    let wrapper = place.join("bin").join("verus");
    if wrapper.is_file() {
        return Ok(Installed::Present(place.to_path_buf()));
    }
    let beside = place.parent().unwrap_or(place);
    std::fs::create_dir_all(beside)
        .map_err(|e| unavailable("Verus", format!("cannot create {}: {e}", beside.display())))?;
    let scratch = super::WorkDir::new_in(beside, "verus")?;
    let archive = scratch.path().join("verus.zip");
    fetch("Verus", &pin.url, &archive)?;
    admit("Verus", &archive, &pin.url, &pin.sha256)?;
    let unpacked = scratch.path().join("unpacked");
    let ran = Command::new("unzip")
        .arg("-q")
        .arg(&archive)
        .arg("-d")
        .arg(&unpacked)
        .output()
        .map_err(|e| unavailable("Verus", format!("cannot run unzip: {e}")))?;
    if !ran.status.success() {
        return Err(unavailable(
            "Verus",
            format!(
                "unzip could not unpack {}: {}",
                pin.url,
                String::from_utf8_lossy(&ran.stderr).trim()
            ),
        ));
    }
    let tree = unpacked.join("verus-x86-linux");
    if !tree.join("verus").is_file() {
        return Err(unavailable(
            "Verus",
            format!("{} holds no verus-x86-linux/verus", pin.url),
        ));
    }
    std::fs::create_dir_all(place)
        .map_err(|e| unavailable("Verus", format!("cannot create {}: {e}", place.display())))?;
    let entries = std::fs::read_dir(&tree)
        .map_err(|e| unavailable("Verus", format!("cannot read {}: {e}", tree.display())))?;
    for entry in entries {
        let entry = entry
            .map_err(|e| unavailable("Verus", format!("cannot read {}: {e}", tree.display())))?;
        let to = place.join(entry.file_name());
        std::fs::rename(entry.path(), &to).map_err(|e| {
            unavailable(
                "Verus",
                format!(
                    "cannot move {} to {}: {e}",
                    entry.path().display(),
                    to.display()
                ),
            )
        })?;
    }
    std::fs::create_dir_all(wrapper.parent().unwrap_or(place)).map_err(|e| {
        unavailable(
            "Verus",
            format!("cannot create {}: {e}", place.join("bin").display()),
        )
    })?;
    std::fs::write(&wrapper, verus_wrapper(pin))
        .map_err(|e| unavailable("Verus", format!("cannot write {}: {e}", wrapper.display())))?;
    make_executable("Verus", &wrapper)?;
    Ok(Installed::Fetched(place.to_path_buf()))
}

/// Fetch the pinned Java runtime, admit the archive by its digest and unpack it
/// into `place` (`java_place`), unless it is there. The archive is unpacked by
/// `tar`, which keeps each file's mode, beside `place`, and the runtime is moved
/// into it by one rename, so a runtime at `place` is a whole one.
pub fn install_java(pin: &JavaPin, place: &Path) -> Result<Installed, FormalError> {
    if java_program(place).is_file() {
        return Ok(Installed::Present(place.to_path_buf()));
    }
    let beside = place.parent().unwrap_or(place);
    std::fs::create_dir_all(beside)
        .map_err(|e| unavailable(JAVA, format!("cannot create {}: {e}", beside.display())))?;
    let scratch = super::WorkDir::new_in(beside, "jre")?;
    let archive = scratch.path().join("jre.tgz");
    fetch(JAVA, &pin.url, &archive)?;
    admit(JAVA, &archive, &pin.url, &pin.sha256)?;
    let unpacked = scratch.path().join("unpacked");
    std::fs::create_dir_all(&unpacked)
        .map_err(|e| unavailable(JAVA, format!("cannot create {}: {e}", unpacked.display())))?;
    run_for(
        JAVA,
        "tar",
        &[
            OsStr::new("-xzf"),
            archive.as_os_str(),
            OsStr::new("-C"),
            unpacked.as_os_str(),
        ],
    )?;
    let root = java_archive_root(pin);
    let tree = unpacked.join(&root);
    if !java_program(&tree).is_file() {
        return Err(unavailable(
            JAVA,
            format!("{} holds no {root}/bin/java", pin.url),
        ));
    }
    std::fs::rename(&tree, place).map_err(|e| {
        unavailable(
            JAVA,
            format!("cannot move {} to {}: {e}", tree.display(), place.display()),
        )
    })?;
    Ok(Installed::Fetched(place.to_path_buf()))
}

/// The program `bin/apalache` under the unpacked release: the release's own
/// launcher, `bin/apalache-mc`, which sets the JVM's heap and collector as its
/// authors run it, behind a line that answers `--version` by itself, so that a
/// check of the installation starts no JVM.
pub fn apalache_wrapper(pin: &ApalachePin) -> String {
    format!(
        "#!/bin/sh\n\
         # Written by sce-codegen formal install: Apalache {version}, run by its own launcher.\n\
         if [ \"$1\" = --version ]; then\n    echo \"apalache {version}\"\n    exit 0\nfi\n\
         exec \"$(dirname \"$(readlink -f \"$0\")\")/apalache-mc\" \"$@\"\n",
        version = pin.version,
    )
}

/// Run `program` with `args` for `tool`'s installation, and refuse it as
/// unavailable when it cannot run or fails.
fn run_for(tool: &str, program: &str, args: &[&OsStr]) -> Result<(), FormalError> {
    let ran = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| unavailable(tool, format!("cannot run {program}: {e}")))?;
    if !ran.status.success() {
        return Err(unavailable(
            tool,
            format!(
                "{program} failed: {}",
                String::from_utf8_lossy(&ran.stderr).trim()
            ),
        ));
    }
    Ok(())
}

/// Fetch the pinned Apalache, admit the archive by its digest and unpack it into
/// `place` (`apalache_place` unless a build machine's declaration names
/// another), unless it is there. The archive is the release's `.tgz`, unpacked
/// by `tar`, which keeps each file's mode. The standard module `Apalache` is
/// taken out of the release's jar by `unzip` into `modules/`, where TLC reads
/// it for a model that extends it. The wrapper is written last, so its presence
/// is the mark of a whole installation.
pub fn install_apalache(pin: &ApalachePin, place: &Path) -> Result<Installed, FormalError> {
    let wrapper = place.join("bin").join("apalache");
    if wrapper.is_file() {
        return Ok(Installed::Present(place.to_path_buf()));
    }
    let beside = place.parent().unwrap_or(place);
    std::fs::create_dir_all(beside).map_err(|e| {
        unavailable(
            "Apalache",
            format!("cannot create {}: {e}", beside.display()),
        )
    })?;
    let scratch = super::WorkDir::new_in(beside, "apalache")?;
    let archive = scratch.path().join("apalache.tgz");
    fetch("Apalache", &pin.url, &archive)?;
    admit("Apalache", &archive, &pin.url, &pin.sha256)?;
    let unpacked = scratch.path().join("unpacked");
    std::fs::create_dir_all(&unpacked).map_err(|e| {
        unavailable(
            "Apalache",
            format!("cannot create {}: {e}", unpacked.display()),
        )
    })?;
    run_for(
        "Apalache",
        "tar",
        &[
            OsStr::new("-xzf"),
            archive.as_os_str(),
            OsStr::new("-C"),
            unpacked.as_os_str(),
        ],
    )?;
    let tree = unpacked.join(format!("apalache-{}", pin.version));
    let jar = Path::new("lib").join("apalache.jar");
    for needed in [Path::new("bin").join("apalache-mc"), jar.clone()] {
        if !tree.join(&needed).is_file() {
            return Err(unavailable(
                "Apalache",
                format!(
                    "{} holds no apalache-{}/{}",
                    pin.url,
                    pin.version,
                    needed.display()
                ),
            ));
        }
    }
    std::fs::create_dir_all(place).map_err(|e| {
        unavailable(
            "Apalache",
            format!("cannot create {}: {e}", place.display()),
        )
    })?;
    let entries = std::fs::read_dir(&tree)
        .map_err(|e| unavailable("Apalache", format!("cannot read {}: {e}", tree.display())))?;
    for entry in entries {
        let entry = entry
            .map_err(|e| unavailable("Apalache", format!("cannot read {}: {e}", tree.display())))?;
        let to = place.join(entry.file_name());
        std::fs::rename(entry.path(), &to).map_err(|e| {
            unavailable(
                "Apalache",
                format!(
                    "cannot move {} to {}: {e}",
                    entry.path().display(),
                    to.display()
                ),
            )
        })?;
    }
    let modules = place.join(APALACHE_MODULES);
    std::fs::create_dir_all(&modules).map_err(|e| {
        unavailable(
            "Apalache",
            format!("cannot create {}: {e}", modules.display()),
        )
    })?;
    let jar = place.join(jar);
    run_for(
        "Apalache",
        "unzip",
        &[
            OsStr::new("-q"),
            OsStr::new("-j"),
            OsStr::new("-o"),
            jar.as_os_str(),
            OsStr::new(APALACHE_MODULE_IN_JAR),
            OsStr::new("-d"),
            modules.as_os_str(),
        ],
    )?;
    if !modules.join("Apalache.tla").is_file() {
        return Err(unavailable(
            "Apalache",
            format!("{} holds no {APALACHE_MODULE_IN_JAR}", jar.display()),
        ));
    }
    std::fs::write(&wrapper, apalache_wrapper(pin)).map_err(|e| {
        unavailable(
            "Apalache",
            format!("cannot write {}: {e}", wrapper.display()),
        )
    })?;
    make_executable("Apalache", &wrapper)?;
    Ok(Installed::Fetched(place.to_path_buf()))
}

/// The program `bin/storm` under the unpacked package: the package's program
/// run with the libraries the package holds, which it does not find by itself
/// (its run path names `/usr/local/lib`, where it is not unpacked), behind a
/// line that answers `--version` by itself, as the wrappers of Verus and
/// Apalache do. The libraries it asks of the system are found where the system
/// keeps them.
pub fn storm_wrapper(pin: &StormPin) -> String {
    format!(
        "#!/bin/sh\n\
         # Written by sce-codegen formal install: Storm {version}, run with the libraries of its package.\n\
         if [ \"$1\" = --version ]; then\n    echo \"storm {version}\"\n    exit 0\nfi\n\
         here=\"$(dirname \"$(readlink -f \"$0\")\")/..\"\n\
         LD_LIBRARY_PATH=\"$here/usr/lib\" exec \"$here/{program}\" \"$@\"\n",
        version = pin.version,
        program = storm_program_in_package(pin).display(),
    )
}

/// Fetch the pinned Storm, admit the package by its digest and unpack it into
/// `place` (`storm_place` unless a build machine's declaration names
/// another), unless it is there. The package is unpacked by `dpkg-deb -x`,
/// which keeps each file's mode and link, and is never installed: what it asks
/// of the system it does not get by being unpacked, and the image and the build
/// machines name those libraries themselves. The wrapper is written last, so
/// its presence is the mark of a whole installation.
pub fn install_storm(pin: &StormPin, place: &Path) -> Result<Installed, FormalError> {
    let wrapper = place.join("bin").join("storm");
    if wrapper.is_file() {
        return Ok(Installed::Present(place.to_path_buf()));
    }
    let beside = place.parent().unwrap_or(place);
    std::fs::create_dir_all(beside)
        .map_err(|e| unavailable("Storm", format!("cannot create {}: {e}", beside.display())))?;
    let scratch = super::WorkDir::new_in(beside, "storm")?;
    let package = scratch.path().join("storm.deb");
    fetch("Storm", &pin.url, &package)?;
    admit("Storm", &package, &pin.url, &pin.sha256)?;
    let unpacked = scratch.path().join("unpacked");
    std::fs::create_dir_all(&unpacked).map_err(|e| {
        unavailable(
            "Storm",
            format!("cannot create {}: {e}", unpacked.display()),
        )
    })?;
    run_for(
        "Storm",
        "dpkg-deb",
        &[OsStr::new("-x"), package.as_os_str(), unpacked.as_os_str()],
    )?;
    let program = storm_program_in_package(pin);
    if !unpacked.join(&program).is_file() {
        return Err(unavailable(
            "Storm",
            format!("{} holds no {}", pin.url, program.display()),
        ));
    }
    std::fs::create_dir_all(place.join("bin")).map_err(|e| {
        unavailable(
            "Storm",
            format!("cannot create {}: {e}", place.join("bin").display()),
        )
    })?;
    let tree = unpacked.join("usr");
    let to = place.join("usr");
    std::fs::rename(&tree, &to).map_err(|e| {
        unavailable(
            "Storm",
            format!("cannot move {} to {}: {e}", tree.display(), to.display()),
        )
    })?;
    std::fs::write(&wrapper, storm_wrapper(pin))
        .map_err(|e| unavailable("Storm", format!("cannot write {}: {e}", wrapper.display())))?;
    make_executable("Storm", &wrapper)?;
    Ok(Installed::Fetched(place.to_path_buf()))
}

#[cfg(unix)]
fn make_executable(tool: &str, path: &Path) -> Result<(), FormalError> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).map_err(|e| {
        unavailable(
            tool,
            format!("cannot make {} executable: {e}", path.display()),
        )
    })
}

#[cfg(not(unix))]
fn make_executable(tool: &str, path: &Path) -> Result<(), FormalError> {
    Err(unavailable(
        tool,
        format!(
            "the pinned release is installed on Linux, so {} is not installed here",
            path.display()
        ),
    ))
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
             which install_apalache unpacks as apalache-<version>/"
        );
        assert!(
            pins.storm.url.ends_with(&format!(
                "/{0}/storm-{0}-debian12-amd64.deb",
                pins.storm.version
            )),
            "the Storm fetched is the Debian 12 package for x64 of the release the pin names, \
             whose program install_storm finds as usr/bin/storm-<version>: {}",
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
        assert_eq!(java_archive_root(&pins.java), format!("jdk-{release}-jre"));
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
    fn a_tool_of_another_digest_is_refused() {
        let dir = super::super::WorkDir::new("test-admit").expect("a working directory");
        let jar = dir.path().join("tla2tools.jar");
        std::fs::write(&jar, b"not TLC").expect("written");
        let digest = sha256_of(&jar).expect("read");
        assert!(admit("TLC", &jar, "u", &digest).is_ok());
        let refused = admit("TLC", &jar, "u", &"0".repeat(64)).expect_err("another digest");
        assert!(matches!(refused, FormalError::ToolUnavailable { .. }));
    }

    #[test]
    fn the_wrapper_runs_verus_on_its_release_with_no_rustup() {
        let pins = pinned();
        let text = verus_wrapper(&pins.verus);
        assert!(text.starts_with("#!/bin/sh\n"));
        assert!(text.contains(&format!("echo \"verus {}\"", pins.verus.version)));
        assert!(text.contains(&format!(
            "rust=\"$HOME/.local/rust/{}\"",
            pins.verus.toolchain
        )));
        assert!(text.contains("VERUS_USE_RUSTUP=0"));
    }

    #[test]
    fn the_storm_wrapper_runs_the_package_with_its_own_libraries() {
        let pins = pinned();
        let text = storm_wrapper(&pins.storm);
        assert!(text.starts_with("#!/bin/sh\n"));
        assert!(text.contains(&format!("echo \"storm {}\"", pins.storm.version)));
        assert!(text.contains("LD_LIBRARY_PATH=\"$here/usr/lib\""), "{text}");
        assert!(
            text.contains(&format!(
                "exec \"$here/usr/bin/storm-{}\" \"$@\"",
                pins.storm.version
            )),
            "{text}"
        );
    }

    #[test]
    fn the_apalache_wrapper_answers_its_release_and_runs_the_launcher() {
        let pins = pinned();
        let text = apalache_wrapper(&pins.apalache);
        assert!(text.starts_with("#!/bin/sh\n"));
        assert!(text.contains(&format!("echo \"apalache {}\"", pins.apalache.version)));
        assert!(text.contains("/apalache-mc\" \"$@\""));
    }
}
