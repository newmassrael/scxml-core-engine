// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Which Codex the application runs, and what it refuses to run beside.
//!
//! Codex has no switch that turns its built-in tools off (Claude Code has one: `--tools ""`), and
//! it gains tools from one version to the next. A specification is text a person may have pasted
//! from anywhere, and a client that has read it must not be one that can be talked into reading
//! the person's files or reaching the network. So a version is run only when it was verified, by
//! a person, against a specification written to attack it, and what was verified is recorded: the
//! operating system, the version, and the instructions the application gave it (which name the
//! arguments, the answer's form and the tools switched off, so a change to any of them is
//! another verification). A version that is not on the list is not run, whatever else is true
//! of it: nothing in the schema or the version number says how it treats a tool.
//!
//! A feature the version has switched on that nobody switched off or reviewed is not run beside
//! either, so that a tool an update adds is a refusal and not an opening.
//!
//! The list is data that ships with the application (`data/codex_support.json`) and is never
//! fetched: it is what permits a run, and a server that could change it would be a way to make
//! the application run anything. It is empty until somebody has done the verification.

use serde::Deserialize;

/// The list this build ships.
const SHIPPED: &str = include_str!("../data/codex_support.json");

/// A version, as it was verified.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Verified {
    pub os: String,
    pub version: String,
    /// The instructions the application gave it (`Generator::instructions`).
    pub instructions: String,
}

/// What is verified, what is switched off, and what is reviewed.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Support {
    verified: Vec<Verified>,
    /// Features the application switches off (`--disable`) on every run.
    #[serde(default)]
    disabled_features: Vec<String>,
    /// Features that are left on because a person looked at what they do and found it
    /// harmless to a run. Empty until somebody has.
    #[serde(default)]
    reviewed_features: Vec<String>,
}

/// Why a Codex is not run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unsupported {
    /// This operating system, version and set of instructions was not verified.
    Version {
        os: String,
        version: String,
        instructions: String,
    },
    /// These features are on, and nobody switched them off or reviewed them.
    Features(Vec<String>),
}

impl std::fmt::Display for Unsupported {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Unsupported::Version { os, version, .. } => write!(
                f,
                "Codex {version} on {os} has not been verified by this build of the application, \
                 so it is not run. Install a version this build supports, or choose another \
                 connection"
            ),
            Unsupported::Features(names) => write!(
                f,
                "this Codex has features on that this build has neither switched off nor \
                 reviewed ({}), so it is not run: they may be tools a specification could use. \
                 Install a version this build supports, or choose another connection",
                names.join(", ")
            ),
        }
    }
}

impl std::error::Error for Unsupported {}

impl Support {
    /// The list this build ships.
    pub fn shipped() -> Support {
        Support::from_json(SHIPPED).expect("the list the build ships is well formed")
    }

    /// A list from its text, for a test and for the build's own.
    pub fn from_json(text: &str) -> Result<Support, String> {
        serde_json::from_str(text).map_err(|e| format!("a list of verified Codex versions: {e}"))
    }

    pub fn verified(&self) -> &[Verified] {
        &self.verified
    }

    /// The features to switch off on every run.
    pub fn disabled_features(&self) -> &[String] {
        &self.disabled_features
    }

    /// The features that were reviewed and are left on.
    pub fn reviewed_features(&self) -> &[String] {
        &self.reviewed_features
    }

    /// Whether a Codex of `version` on `os`, given `instructions`, with `enabled` on, may run.
    pub fn check(
        &self,
        os: &str,
        version: &str,
        instructions: &str,
        enabled: &[String],
    ) -> Result<(), Unsupported> {
        let verified = self
            .verified
            .iter()
            .any(|v| v.os == os && v.version == version && v.instructions == instructions);
        if !verified {
            return Err(Unsupported::Version {
                os: os.to_string(),
                version: version.to_string(),
                instructions: instructions.to_string(),
            });
        }
        let unsaid: Vec<String> = enabled
            .iter()
            .filter(|feature| {
                !self.disabled_features.contains(feature)
                    && !self.reviewed_features.contains(feature)
            })
            .cloned()
            .collect();
        if unsaid.is_empty() {
            Ok(())
        } else {
            Err(Unsupported::Features(unsaid))
        }
    }
}

/// The features `codex features list` says are on, in the order it lists them. A line is a name,
/// a stage of one or two words, and whether it is on; anything else is not a feature.
pub fn enabled_features(listing: &str) -> Vec<String> {
    listing
        .lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            let name = words.next()?;
            let on = line.split_whitespace().last()?;
            // A name, at least one word of a stage, and a verdict: fewer is not a feature line.
            (line.split_whitespace().count() >= 3 && on == "true").then(|| name.to_string())
        })
        .collect()
}
