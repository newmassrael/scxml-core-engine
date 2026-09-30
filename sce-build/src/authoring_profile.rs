// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The authoring profile: what a specification owner configures about how a
//! design is authored for them, stated in a file instead of said once in a
//! conversation.
//!
//! # Why a file, and why not a flag
//!
//! The product judges a document by grammar and by what SCXML means, and it
//! cannot know what its author was asked for. A statechart with no schema and
//! no `sce:interface` is a W3C conformance document, a legacy machine, or a
//! design an owner is about to be shown; nothing in it says which. Guessing
//! from a proxy — a recorded kind basis, an import — reads intent into a
//! feature that means something else, and a second guess for the next
//! expectation is a second way to say the same thing. The owner's
//! expectation belongs where the owner can state it, change it, and have it
//! remembered: a profile, beside the specification.
//!
//! So intent is explicit. A run without a profile judges nothing new, and a
//! run under one is held to exactly what the file says, by the same parse the
//! build compiles.
//!
//! # Three classes, fixed by the schema
//!
//! Every setting belongs to one class, and the class is not the file's to
//! choose ([`SETTINGS`]):
//!
//! ```text
//!   enforced   a draft that breaks it is refused
//!   reported   every departure is listed, and the owner decides
//!   guidance   handed to the author, checked by nothing, and said so
//! ```
//!
//! Version 1 holds one setting, `interface`, and it is enforced: an interface
//! is an attribute a statechart declares or does not, which a machine can
//! check. The other classes have no setting yet, and a profile naming a
//! setting this build does not know is REFUSED whole
//! ([`ProfileUnusable`]): a profile written for a newer tool that was half
//! applied would say "checked under this profile" of a document it never
//! held to it.
//!
//! # Part of the attempt key
//!
//! A design accepted under one profile is not the answer for another. The
//! acceptance record pins the file's sha256 as one of the things the design
//! was authored from ([`crate::acceptance_record::SourceRole::Profile`]), and
//! a run under a profile publishes the same digest in the manifest, so the
//! digest that judged a design and the digest an acceptance was taken under
//! can be compared.
//!
//! # What a profile may not configure
//!
//! The obligation to mark a guess, W3C SCXML semantics, the kind catalog and
//! `<sce:kind-basis>`, and acceptance by a person. Nothing here can turn
//! those off, and no setting is named so that it could.

use std::path::Path;

use serde::Deserialize;

use crate::forge::error::{ForgeError, Located};
use crate::generator_witness::{hex_encode, sha256_bytes};
use crate::model::SCXMLModel;

/// The value a profile's `record` field holds. A JSON file naming any other
/// kind is refused before its settings are read.
pub const RECORD_KIND: &str = "sce-authoring-profile";

/// The format version. Moves only when a setting changes meaning; a setting
/// is added without it.
pub const PROFILE_VERSION: u32 = 1;

/// Stability status of the profile wire surface. Pinned to the
/// `x-sce-schema-status` header of
/// `schemas/sce-authoring-profile.v1.schema.json` by
/// [`tests::schema_file_declares_status`].
pub const PROFILE_SCHEMA_STATUS: &str = "pre-release";

/// What the product does with a setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingClass {
    /// A draft that breaks it is refused.
    Enforced,
    /// Every departure is listed and the owner decides.
    Reported,
    /// Handed to the author; nothing checks it.
    Guidance,
}

/// Every setting the schema knows, with the class the schema fixes. One
/// table, read by the schema-drift guard, so a setting cannot be added to the
/// schema or to the reader alone.
pub const SETTINGS: &[(&str, SettingClass)] = &[("interface", SettingClass::Enforced)];

/// What `interface` asks of a statechart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InterfaceRule {
    /// Every statechart declares `sce:interface="closed"`.
    Closed,
}

/// What follows the `record` and `v` header. `deny_unknown_fields` is the
/// refusal of a setting this build does not know.
///
/// The header is read and checked first ([`AuthoringProfile::from_text`]),
/// so a profile from a newer tool is refused for its version and not for the
/// first setting it holds that this build has never heard of.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Settings {
    name: Option<String>,
    interface: Option<InterfaceRule>,
}

/// Why a profile cannot be used. The wire `kind` of `cli/profile-unusable`:
/// SCE determines it, unlike the parser's own sentence, so it can key a
/// record without a platform's spelling of an error in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileFault {
    /// The file could not be read.
    Unreadable,
    /// The text is not JSON.
    NotJson,
    /// JSON, and not the shape a profile has: a setting this build does not
    /// know, a value a setting does not take, or a missing field.
    InvalidShape,
    /// JSON of another kind of file.
    WrongRecord,
    /// A version this build does not read.
    UnsupportedVersion,
    /// A `name` with nothing in it.
    EmptyName,
}

impl ProfileFault {
    /// The spelling on the wire.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unreadable => "unreadable",
            Self::NotJson => "not-json",
            Self::InvalidShape => "invalid-shape",
            Self::WrongRecord => "wrong-record",
            Self::UnsupportedVersion => "unsupported-version",
            Self::EmptyName => "empty-name",
        }
    }
}

/// A profile file the product cannot use: unreadable, not JSON, another kind
/// of file, a version it does not read, or a setting it does not know.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileUnusable {
    /// Which refusal.
    pub kind: ProfileFault,
    /// What is wrong, in a sentence. Names no path: the caller knows which
    /// file it handed over.
    pub detail: String,
}

impl std::fmt::Display for ProfileUnusable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.detail)
    }
}

/// A finding the profile makes about a document: the document is valid, and
/// it is not what the profile asks for.
#[derive(Debug, thiserror::Error)]
pub enum ProfileError {
    /// The profile requires `sce:interface="closed"`, and the statechart does
    /// not declare it.
    #[error("{}", interface_not_closed_message(profile.as_deref(), imports))]
    InterfaceNotClosed {
        /// The profile's `name`, when it has one.
        profile: Option<String>,
        /// The statechart's model name (its file stem), so two statecharts
        /// that break one profile are two findings.
        machine: String,
        /// The event-schemas the statechart imports and so describes a
        /// boundary with, which nothing then holds it to.
        imports: Vec<String>,
    },
}

/// [`ProfileError::InterfaceNotClosed`]'s message.
fn interface_not_closed_message(profile: Option<&str>, imports: &[String]) -> String {
    let subject = match profile {
        Some(name) => format!("the authoring profile '{name}'"),
        None => "the authoring profile it was given".to_string(),
    };
    let described = if imports.is_empty() {
        String::new()
    } else {
        format!(
            "; it imports event-schema(s) ({}) that describe a boundary nothing holds it to",
            imports.join(", ")
        )
    };
    format!(
        "{subject} requires sce:interface=\"closed\" on a statechart, and this one does not \
         declare it{described} — declare it closed on the root <scxml> and give every event it \
         takes or sends an event-schema, or use a profile that does not require it: which \
         boundary a design is held to is the owner's decision"
    )
}

/// A profile, read and validated.
#[derive(Debug, Clone)]
pub struct AuthoringProfile {
    name: Option<String>,
    interface: Option<InterfaceRule>,
    sha256: String,
}

impl AuthoringProfile {
    /// Read a profile from its text. `text` is what the digest is taken over,
    /// so the same bytes are the same profile wherever they are read.
    pub fn from_text(text: &str) -> Result<Self, ProfileUnusable> {
        let unusable = |kind, detail: String| ProfileUnusable { kind, detail };
        let shape = |error: serde_json::Error| {
            let kind = match error.classify() {
                serde_json::error::Category::Data => ProfileFault::InvalidShape,
                _ => ProfileFault::NotJson,
            };
            unusable(kind, format!("not an authoring profile: {error}"))
        };
        let document: serde_json::Value = serde_json::from_str(text).map_err(shape)?;
        let serde_json::Value::Object(mut fields) = document else {
            return Err(unusable(
                ProfileFault::InvalidShape,
                "not an authoring profile: a profile is one JSON object".to_string(),
            ));
        };
        // The header first: `record` names the kind of file and `v` says
        // which settings it may hold, so both are judged before any setting
        // is read.
        let record = fields.remove("record");
        if record.as_ref().and_then(serde_json::Value::as_str) != Some(RECORD_KIND) {
            return Err(unusable(
                ProfileFault::WrongRecord,
                format!(
                    "the file's `record` is {}, and an authoring profile's is `{RECORD_KIND}`",
                    record.map_or_else(|| "missing".to_string(), |value| value.to_string())
                ),
            ));
        }
        let version = fields.remove("v");
        if version.as_ref().and_then(serde_json::Value::as_u64) != Some(u64::from(PROFILE_VERSION))
        {
            return Err(unusable(
                ProfileFault::UnsupportedVersion,
                format!(
                    "profile version {} — this build reads version {PROFILE_VERSION}, and half \
                     applying a profile it does not fully read would say a document was held to \
                     it when it was not",
                    version.map_or_else(|| "missing".to_string(), |value| value.to_string())
                ),
            ));
        }
        let settings: Settings =
            serde_json::from_value(serde_json::Value::Object(fields)).map_err(shape)?;
        if settings.name.as_deref() == Some("") {
            return Err(unusable(
                ProfileFault::EmptyName,
                "`name` is empty; leave it out or give it a label".to_string(),
            ));
        }
        Ok(Self {
            name: settings.name,
            interface: settings.interface,
            sha256: hex_encode(&sha256_bytes(text.as_bytes())),
        })
    }

    /// Read a profile from a file.
    pub fn load(path: &Path) -> Result<Self, ProfileUnusable> {
        let text = std::fs::read_to_string(path).map_err(|error| ProfileUnusable {
            kind: ProfileFault::Unreadable,
            detail: format!("cannot read it: {error}"),
        })?;
        Self::from_text(&text)
    }

    /// The label a report prints, when the profile has one.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// The sha256 of the file's bytes: what an acceptance record pins and the
    /// manifest publishes.
    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    /// Whether the profile asks anything of a statechart at all — false for a
    /// profile that holds only settings no statechart is judged by.
    pub fn judges_statecharts(&self) -> bool {
        self.interface.is_some()
    }

    /// Every way a statechart departs from the profile's ENFORCED settings,
    /// each located where it is written.
    ///
    /// The model is the one the build compiles — the caller parsed the file
    /// through the production parser — so what is judged is what would be
    /// built, not a second reading of the text.
    pub fn judge_statechart(
        &self,
        model: &SCXMLModel,
        diag_label: &str,
    ) -> Vec<Located<ForgeError>> {
        let mut findings = Vec::new();
        if self.interface == Some(InterfaceRule::Closed) && !model.interface_closed {
            let error = ForgeError::Profile(Box::new(ProfileError::InterfaceNotClosed {
                profile: self.name.clone(),
                machine: model.name.clone(),
                imports: crate::open_matters::interface_left_open(model),
            }));
            findings.push(model.locate(error, model.source_location.as_ref(), diag_label));
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLOSED: &str =
        r#"{"record":"sce-authoring-profile","v":1,"name":"owner-review","interface":"closed"}"#;

    fn schema() -> serde_json::Value {
        serde_json::from_str(include_str!(
            "../../schemas/sce-authoring-profile.v1.schema.json"
        ))
        .expect("authoring profile schema is JSON")
    }

    fn violations(instance: &serde_json::Value) -> Vec<String> {
        let schema = schema();
        let validator = jsonschema::JSONSchema::options()
            .with_draft(jsonschema::Draft::Draft7)
            .compile(&schema)
            .expect("authoring profile schema compiles");
        let outcome = validator.validate(instance);
        match outcome {
            Ok(()) => Vec::new(),
            Err(errors) => errors.map(|e| e.to_string()).collect(),
        }
    }

    fn statechart(root_attrs: &str) -> SCXMLModel {
        crate::parser::SCXMLParser::new()
            .parse_string(
                &format!(
                    r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                             xmlns:sce="http://sce.dev/ext" version="1.0" name="gate"
                             initial="a" {root_attrs}>
                         <sce:import src="s.scxml" kind="event-schema" as="Sig"/>
                         <state id="a"/>
                       </scxml>"#
                ),
                "authoring_profile",
            )
            .expect("parses")
    }

    #[test]
    fn schema_file_declares_status() {
        assert_eq!(
            schema()["x-sce-schema-status"].as_str(),
            Some(PROFILE_SCHEMA_STATUS),
            "schemas/sce-authoring-profile.v1.schema.json x-sce-schema-status disagrees with \
             PROFILE_SCHEMA_STATUS; SCE_WIRE_CONTRACTS.md requires one commit to move both",
        );
    }

    #[test]
    fn schema_version_and_kind_match_the_reader() {
        let schema = schema();
        assert_eq!(
            schema["properties"]["v"]["const"].as_u64(),
            Some(u64::from(PROFILE_VERSION))
        );
        assert_eq!(
            schema["properties"]["record"]["const"].as_str(),
            Some(RECORD_KIND)
        );
    }

    /// The schema's settings and the reader's are one list. A setting added
    /// to the schema and not to [`SETTINGS`] — or the reverse — has a class
    /// nobody fixed, or a property nothing reads.
    #[test]
    fn the_schema_and_the_settings_table_name_the_same_settings() {
        let schema = schema();
        let properties = schema["properties"].as_object().expect("properties");
        let mut in_schema: Vec<&str> = properties
            .keys()
            .map(String::as_str)
            .filter(|key| !matches!(*key, "record" | "v" | "name"))
            .collect();
        in_schema.sort_unstable();
        let mut in_table: Vec<&str> = SETTINGS.iter().map(|(key, _)| *key).collect();
        in_table.sort_unstable();
        assert_eq!(in_schema, in_table);
    }

    #[test]
    fn every_profile_the_product_reads_validates_against_the_wire_schema() {
        for text in [
            CLOSED,
            r#"{"record":"sce-authoring-profile","v":1}"#,
            r#"{"record":"sce-authoring-profile","v":1,"interface":"closed"}"#,
        ] {
            let instance: serde_json::Value = serde_json::from_str(text).expect("JSON");
            assert!(violations(&instance).is_empty(), "{text}");
            AuthoringProfile::from_text(text).expect("the reader takes what the schema does");
        }
    }

    /// Starts from a profile the schema accepts and changes ONE thing, so the
    /// refusal is about that thing.
    #[test]
    fn the_profile_schema_rejects_a_setting_the_product_does_not_know() {
        let valid: serde_json::Value = serde_json::from_str(CLOSED).expect("JSON");
        assert!(violations(&valid).is_empty(), "the control has to be valid");
        let mut unknown = valid.clone();
        unknown["naming"] = serde_json::json!("camel");
        assert!(!violations(&unknown).is_empty());
        let mut wrong_value = valid;
        wrong_value["interface"] = serde_json::json!("open");
        assert!(!violations(&wrong_value).is_empty());
    }

    /// The reader refuses what the schema refuses, and says why — a profile
    /// half applied would say a document was held to it when it was not.
    #[test]
    fn the_reader_refuses_the_whole_profile_it_cannot_fully_read() {
        for (text, kind, needle) in [
            (
                r#"{"record":"sce-authoring-profile","v":1,"naming":"camel"}"#,
                ProfileFault::InvalidShape,
                "naming",
            ),
            (
                r#"{"record":"sce-authoring-profile","v":1,"interface":"open"}"#,
                ProfileFault::InvalidShape,
                "open",
            ),
            (
                r#"{"record":"sce-authoring-profile","v":2}"#,
                ProfileFault::UnsupportedVersion,
                "version 2",
            ),
            // A newer profile is refused for its version, not for the first
            // setting this build has never heard of.
            (
                r#"{"record":"sce-authoring-profile","v":2,"naming":"camel"}"#,
                ProfileFault::UnsupportedVersion,
                "version 2",
            ),
            (
                r#"{"record":"sce-authoring-profile"}"#,
                ProfileFault::UnsupportedVersion,
                "version missing",
            ),
            (
                r#"{"record":"sce-decision-record","v":1}"#,
                ProfileFault::WrongRecord,
                "sce-decision-record",
            ),
            (r#"{"v":1}"#, ProfileFault::WrongRecord, "missing"),
            (
                r#"{"record":"sce-authoring-profile","v":1,"name":""}"#,
                ProfileFault::EmptyName,
                "`name` is empty",
            ),
            ("[1]", ProfileFault::InvalidShape, "one JSON object"),
            (
                "not json",
                ProfileFault::NotJson,
                "not an authoring profile",
            ),
        ] {
            let refused = AuthoringProfile::from_text(text).expect_err(text);
            assert_eq!(refused.kind, kind, "{text}: {refused}");
            assert!(
                refused.detail.contains(needle),
                "{text}: expected the refusal to mention {needle:?}, got {refused}"
            );
        }
    }

    #[test]
    fn a_profile_file_that_is_not_there_is_unreadable() {
        let refused = AuthoringProfile::load(Path::new("/nonexistent/profile.json"))
            .expect_err("nothing there");
        assert_eq!(refused.kind, ProfileFault::Unreadable);
        assert_eq!(refused.kind.as_str(), "unreadable");
    }

    #[test]
    fn the_digest_is_the_files_own() {
        let a = AuthoringProfile::from_text(CLOSED).expect("reads");
        let renamed = CLOSED.replace("owner-review", "house-review");
        let b = AuthoringProfile::from_text(&renamed).expect("reads");
        assert_eq!(a.sha256().len(), 64);
        assert_ne!(
            a.sha256(),
            b.sha256(),
            "a renamed profile is a new profile to an acceptance record"
        );
        assert_eq!(
            a.sha256(),
            AuthoringProfile::from_text(CLOSED).expect("reads").sha256()
        );
        assert_eq!(a.name(), Some("owner-review"));
    }

    #[test]
    fn a_statechart_that_is_not_closed_departs_from_a_profile_that_requires_it() {
        let profile = AuthoringProfile::from_text(CLOSED).expect("reads");
        let open = profile.judge_statechart(&statechart(""), "gate.scxml");
        assert_eq!(open.len(), 1, "{open:?}");
        let said = open[0].error.to_string();
        assert!(
            said.contains("'owner-review'") && said.contains("(Sig)"),
            "{said}"
        );

        let closed =
            profile.judge_statechart(&statechart("sce:interface=\"closed\""), "gate.scxml");
        assert!(closed.is_empty(), "{closed:?}");
    }

    #[test]
    fn a_profile_that_constrains_nothing_finds_nothing() {
        let profile = AuthoringProfile::from_text(r#"{"record":"sce-authoring-profile","v":1}"#)
            .expect("reads");
        assert!(!profile.judges_statecharts());
        assert!(profile
            .judge_statechart(&statechart(""), "gate.scxml")
            .is_empty());
    }
}
