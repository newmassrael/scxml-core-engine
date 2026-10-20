// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! The channels an integration fixture runs in, the sites each one needs,
//! and the channels a fixture is exempt from, with the reason.
//!
//! A stem under `integration_resources/` fans out to seven channels — C++
//! Interpreter, C++ AOT, Rust, Go, Python, Kotlin, C11 — and each channel is
//! a set of sites: the ones that GENERATE its code and the ones that ASSERT
//! it. This module is the one statement of that set. The suite that holds the
//! tree to it, `integration_stem_registration`, reads it here, and so does any
//! check that asks which stems a backend's channel drives: a second copy of
//! the table would be a second answer to the same question, and the two
//! would drift the first time a stem or an exemption was added to one.
//!
//! The channel is a type rather than a label, so that "which backend does
//! this channel build" is a decision made once, by an exhaustive match, and
//! a site's channel cannot be misspelt into a channel nobody checks.

#![allow(dead_code)]

use std::path::Path;

use sce_build::generator::Language;

/// One channel an integration fixture is generated into or asserted in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Channel {
    /// The C++ interpreter, which reads the document at run time.
    CppInterpreter,
    /// The C++ code the generator emits.
    CppAot,
    Rust,
    Go,
    Python,
    Kotlin,
    C11,
}

impl Channel {
    /// Every channel, in the order the sites below list their drivers.
    pub const ALL: &'static [Channel] = &[
        Channel::CppInterpreter,
        Channel::CppAot,
        Channel::Rust,
        Channel::Go,
        Channel::Python,
        Channel::Kotlin,
        Channel::C11,
    ];

    /// The generated backend this channel builds and runs, or `None` for the
    /// channel that runs no generated code.
    ///
    /// Exhaustive on purpose: a channel added here has to say which backend
    /// it is the channel of before anything compiles.
    pub fn language(self) -> Option<Language> {
        match self {
            Channel::CppInterpreter => None,
            Channel::CppAot => Some(Language::Cpp),
            Channel::Rust => Some(Language::Rust),
            Channel::Go => Some(Language::Go),
            Channel::Python => Some(Language::Python),
            Channel::Kotlin => Some(Language::Kotlin),
            Channel::C11 => Some(Language::C11),
        }
    }

    /// The channel's name, as a reader of a failure message knows it.
    pub fn name(self) -> &'static str {
        match self {
            Channel::CppInterpreter => "C++ Interpreter",
            Channel::CppAot => "C++ AOT",
            Channel::Rust => "Rust",
            Channel::Go => "Go",
            Channel::Python => "Python",
            Channel::Kotlin => "Kotlin",
            Channel::C11 => "C11",
        }
    }
}

/// What a site does for the stem it is required for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// The canonical document every channel compiles.
    Fixture,
    /// A site that generates a channel's code for the stem. Loud when
    /// missing — a regen script, a module declaration, a CMake codegen call.
    Generation(Channel),
    /// The test file that runs the fixture in a channel. Silent when
    /// missing: generated code nobody runs still compiles.
    Driver(Channel),
    /// The build registration that compiles a channel's driver. A driver
    /// the build does not compile is a file, not a test.
    DriverCompiled(Channel),
    /// The layout doc's entry recording which axis the stem owns.
    LayoutDoc,
}

impl Role {
    /// The channel this site belongs to, or `None` for a site the stem
    /// needs whichever channels run it.
    pub fn channel(self) -> Option<Channel> {
        match self {
            Role::Fixture | Role::LayoutDoc => None,
            Role::Generation(channel) | Role::Driver(channel) | Role::DriverCompiled(channel) => {
                Some(channel)
            }
        }
    }
}

/// One required site, as the author would have to satisfy it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Site {
    pub role: Role,
    /// What the site is for, in the failure message.
    pub label: &'static str,
    /// Path that must exist, or the file a marker must appear in.
    pub path: String,
    /// When set, the file must additionally contain this text.
    pub marker: Option<String>,
}

/// How much of a channel an exemption suspends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Suspends {
    /// The channel generates and compiles the fixture; what it cannot do yet
    /// is run it, so only the driver and its registration are suspended.
    Assertion,
    /// The channel neither generates nor runs the fixture.
    GenerationAndAssertion,
}

/// A channel that cannot host a stem, and why.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Exemption {
    pub scope: Suspends,
    pub reason: &'static str,
}

impl Exemption {
    /// Whether a site with `role`, in the exempted channel, is suspended.
    pub fn suspends(&self, role: Role) -> bool {
        match role {
            Role::Fixture | Role::LayoutDoc => false,
            Role::Generation(_) => self.scope == Suspends::GenerationAndAssertion,
            Role::Driver(_) | Role::DriverCompiled(_) => true,
        }
    }
}

/// Every stem under `integration_resources/`, sorted.
pub fn integration_stems(root: &Path) -> Vec<String> {
    let mut found: Vec<String> = std::fs::read_dir(root.join("integration_resources"))
        .expect("read integration_resources/")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    found.sort();
    found
}

/// `send_param_payload` -> `SendParamPayload`, the spelling the C++ and
/// Kotlin drivers are named with.
pub fn upper_camel(stem: &str) -> String {
    stem.split('_')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut cs = w.chars();
            match cs.next() {
                Some(c) => c.to_uppercase().chain(cs).collect::<String>(),
                None => String::new(),
            }
        })
        .collect()
}

/// Every site a stem needs in every channel, before any exemption.
pub fn sites(stem: &str) -> Vec<Site> {
    let camel = upper_camel(stem);
    let file = |role: Role, label: &'static str, path: String| Site {
        role,
        label,
        path,
        marker: None,
    };
    let marker = |role: Role, label: &'static str, path: &str, marker: String| Site {
        role,
        label,
        path: path.to_string(),
        marker: Some(marker),
    };

    vec![
        // The fixture itself: the canonical document every channel compiles.
        file(
            Role::Fixture,
            "canonical fixture",
            format!("integration_resources/{stem}/{stem}.scxml"),
        ),
        // Generation. Loud when missing, but listed so one report covers the
        // whole contract instead of the author meeting it in instalments.
        file(
            Role::Generation(Channel::Rust),
            "regen (rust)",
            format!("scripts/regen_{stem}.sh"),
        ),
        file(
            Role::Generation(Channel::Go),
            "regen (go)",
            format!("scripts/regen_{stem}_go.sh"),
        ),
        file(
            Role::Generation(Channel::Kotlin),
            "regen (kotlin)",
            format!("scripts/regen_{stem}_kotlin.sh"),
        ),
        file(
            Role::Generation(Channel::Python),
            "regen (python)",
            format!("scripts/regen_{stem}_python.sh"),
        ),
        marker(
            Role::Generation(Channel::Rust),
            "rust module tree",
            "backends/rust/tests/src/integration/mod.rs",
            format!("pub mod {stem};"),
        ),
        marker(
            Role::Generation(Channel::CppAot),
            "C++ AOT codegen registration",
            "tests/CMakeLists.txt",
            format!("sce_generate_static_integration_test({stem}"),
        ),
        marker(
            Role::Generation(Channel::C11),
            "C11 codegen registration",
            "backends/c/tests/CMakeLists.txt",
            format!("sce_generate_static_integration_c_test({stem}"),
        ),
        // Assertion. These are the silent ones.
        file(
            Role::Driver(Channel::Rust),
            "Rust AOT driver",
            format!("backends/rust/tests/tests/{stem}.rs"),
        ),
        file(
            Role::Driver(Channel::Go),
            "Go AOT driver",
            format!("backends/go/tests/integration/{stem}/{stem}_test.go"),
        ),
        file(
            Role::Driver(Channel::Python),
            "Python AOT driver",
            format!("backends/python/tests/integration/{stem}/test_{stem}_aot.py"),
        ),
        file(
            Role::Driver(Channel::Kotlin),
            "Kotlin AOT driver",
            format!("backends/kotlin/tests/src/test/kotlin/com/sce/integration/{camel}Test.kt"),
        ),
        file(
            Role::Driver(Channel::C11),
            "C11 AOT driver",
            format!("backends/c/tests/integration/test_{stem}.c"),
        ),
        file(
            Role::Driver(Channel::CppInterpreter),
            "C++ Interpreter driver",
            format!("tests/integration/{camel}Test.cpp"),
        ),
        file(
            Role::Driver(Channel::CppAot),
            "C++ AOT driver",
            format!("tests/integration/{camel}AotTest.cpp"),
        ),
        // A driver CMake does not compile is a file, not a test.
        marker(
            Role::DriverCompiled(Channel::CppInterpreter),
            "C++ driver compiled",
            "tests/CMakeLists.txt",
            format!("integration/{camel}Test.cpp"),
        ),
        marker(
            Role::DriverCompiled(Channel::CppAot),
            "C++ AOT driver compiled",
            "tests/CMakeLists.txt",
            format!("integration/{camel}AotTest.cpp"),
        ),
        marker(
            Role::DriverCompiled(Channel::C11),
            "C11 driver compiled",
            "backends/c/tests/CMakeLists.txt",
            format!("integration/test_{stem}.c"),
        ),
        // The layout doc records what axis each fixture owns, and "fixtures
        // stay on one axis" is the rule it states. A stem with no entry there
        // is an axis nobody chose — and the enumeration this replaced was
        // wrong within one round of a stem being added, which is why the
        // requirement is a per-stem mention rather than a list to maintain.
        marker(
            Role::LayoutDoc,
            "layout doc entry",
            "docs/SCE_INTEGRATION_FIXTURE_LAYOUT.md",
            stem.to_string(),
        ),
    ]
}

/// The sites a stem needs, with every site an exemption suspends left out.
pub fn required_sites(stem: &str) -> Vec<Site> {
    sites(stem)
        .into_iter()
        .filter(|site| !is_suspended(stem, site.role))
        .collect()
}

/// Whether a site with `role` is suspended for `stem` by its channel's
/// exemption.
pub fn is_suspended(stem: &str, role: Role) -> bool {
    role.channel()
        .and_then(|channel| channel_cannot_host(stem, channel))
        .is_some_and(|exemption| exemption.suspends(role))
}

/// Whether `channel` runs `stem`: its driver is a site the stem requires.
pub fn channel_drives(stem: &str, channel: Channel) -> bool {
    required_sites(stem)
        .iter()
        .any(|site| site.role == Role::Driver(channel))
}

/// The channels a given stem cannot be asserted in, and why.
///
/// A channel that cannot host a fixture is a decision with a reason, not an
/// absence. The reason belongs next to the requirement it suspends, so a
/// reader who meets the gap meets the measurement that explains it — and so
/// the entry has to be deleted, deliberately, when the missing piece is
/// built.
///
/// Every entry here names a MISSING PIECE OF HARNESS, never a defect in the
/// engine: a channel whose engine gets the clause wrong must be red, not
/// exempt.
pub fn channel_cannot_host(stem: &str, channel: Channel) -> Option<Exemption> {
    match (stem, channel) {
        // The fixture posts over BasicHTTP and reads the form back out of the
        // echoed event, which needs a listener that echoes. Rust, Go, Python
        // and C11 all host it: each already drives the echoing listener from
        // its own lane.
        ("send_namelist_over_http", Channel::CppInterpreter | Channel::CppAot) => Some(Exemption {
            scope: Suspends::GenerationAndAssertion,
            reason: "the C++ integration lane's own server, \
                     `tests/integration/SimpleMockHttpServer.cpp`, answers a fixed body \
                     (`\"data\": \"\"` with `\"form\": {}`), so a document cannot read back \
                     what it posted there. The echoing listener \
                     (`tests/w3c/standalone_http_server.js`) is wired into the W3C lane \
                     only, through `HttpAotTest`; pointing the integration lane at it is \
                     its own piece of work and is open debt, not a property of C++.",
        }),
        ("send_namelist_over_http", Channel::Kotlin) => Some(Exemption {
            scope: Suspends::Assertion,
            reason: "the Kotlin test tree has no BasicHTTP harness at all: neither an HTTP \
                     send callback nor an access-URI setter is bound anywhere under \
                     `backends/kotlin/tests/src` (measured 2026-08-21). The generated tree is \
                     still produced and compiled, which is what the drift gate checks; it is \
                     the assertion that is missing.",
        }),
        // SCE_MESH.md §mesh-19. Not an engine exemption: what C++ does
        // meanwhile is recorded as debt rather than asserted here. With no
        // hook installed, `StaticExecutionEngine::raiseExternal` delivers a
        // `#peer` send to this session's own queue (measured 2026-09-28),
        // where §mesh-19 would make an unserved Mesh send error.execution. A
        // C++ mesh lane that drives the `TransportRouter` is where that
        // belongs.
        (
            "a_peer_named_at_run_time_reaches_the_router",
            Channel::CppInterpreter | Channel::CppAot,
        ) => Some(Exemption {
            scope: Suspends::GenerationAndAssertion,
            reason: "the fixture asks whether a `targetexpr` naming a Mesh peer reaches the \
                     host's Mesh router, the `sce:mesh` host-served processor. That route is \
                     how every backend but C++ reaches a Mesh core; C++ reaches one through \
                     the generated `TransportRouter`'s mesh-send hook, which the build emits \
                     only from a `deploy.yaml`, and the integration lane generates without \
                     one, so there is no router for a C++ driver to register or observe.",
        }),
        // SCE_MESH.md §9.5, §mesh-19.
        ("a_mesh_request_reaches_the_router", Channel::CppInterpreter | Channel::CppAot) => {
            Some(Exemption {
                scope: Suspends::GenerationAndAssertion,
                reason: "the fixture asks whether an `<invoke type=\"sce:mesh-rpc\">` reaches \
                         the host's Mesh router through the runtime's mesh-rpc door. C++ is the \
                         one backend that has no such door to observe: its generated \
                         `TransportRouter` takes the request itself (the `generated` route), \
                         and the build emits that router only from a `deploy.yaml`, which the \
                         integration lane does not pass. The C++ route is exercised where its \
                         router is built: the `mesh_*` lanes over \
                         `tests/mesh/brake_invoke.scxml`.",
            })
        }
        _ => None,
    }
}
