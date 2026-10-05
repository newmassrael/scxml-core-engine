// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A bundle: what one generation request made, published as one.
//!
//! A work's model and its requirement list are two chains, each moved by its own pointer. A
//! reader that takes the model's pointer and then the list's, with a writer between, is
//! handed a model of one generation and a list of another: a pair nobody wrote. A bundle is
//! the answer that has one pointer. It names the model, the requirement list, the text and
//! the answers they were made from, and what was checked, in one immutable file; the work's
//! current model and current list are the ones the current bundle names, and moving one
//! pointer (`bundles.head`) moves both.
//!
//! The model and the list stay where every revision of them is kept, named by what they
//! hold. A bundle does not copy them, it says which. So a bundle is small, and a revision
//! that two bundles name is stored once.
//!
//! This module is the file's shape and nothing about where it lives or when it is written.

use serde::{Deserialize, Serialize};

use crate::revision::Revision;

pub const BUNDLE_FORMAT: &str = "sce-bundle";
pub const BUNDLE_VERSION: u32 = 1;

/// Who ran a check. The two are not the same word: the core ran the checks it can run
/// itself (SCE's check of the model), and a client reports the rest (the authoring
/// package's check of the decision record). A report is not a measurement, and the file
/// says which each is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CheckedBy {
    Core,
    Client,
}

/// One check of the candidate, and how it came out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleCheck {
    pub by: CheckedBy,
    /// What was checked (`model`, `decisions`).
    pub name: String,
    /// `accepted` or `refused`, in the product's words.
    pub verdict: String,
    /// The generator that ran it, when it said.
    #[serde(default)]
    pub generator: Option<String>,
    /// The digest of the record the check wrote, so that what was checked is pinned and not
    /// only what it said.
    #[serde(default)]
    pub digest: Option<String>,
    /// The revision of the text the check ran on. A check says what it came to for one text,
    /// and a candidate that was written again after it was run is another text: publishing
    /// refuses a check whose subject is not the candidate it would publish.
    #[serde(default)]
    pub subject: Option<Revision>,
}

impl BundleCheck {
    pub fn is_accepted(&self) -> bool {
        self.verdict == "accepted"
    }
}

/// What the work's model and list were before the first bundle took over from them, so that
/// a history that began as two chains is not cut where it began to be bundles.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Previous {
    #[serde(default)]
    pub model: Option<Revision>,
    #[serde(default)]
    pub requirements: Option<Revision>,
}

/// One published bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bundle {
    pub format: String,
    pub v: u32,
    /// The request that made it, and the attempt that finished it.
    pub request: String,
    pub attempt: u32,
    /// The executor that made it.
    pub executor: String,
    /// The text and the answers it was made from: what the request was asked about.
    pub source: Revision,
    #[serde(default)]
    pub answers: Option<Revision>,
    pub model: Revision,
    pub requirements: Revision,
    /// Set on a work's first bundle, when it had a model or a list before.
    #[serde(default)]
    pub previous: Option<Previous>,
    /// The bundle this one replaced as the work's model and list; none for a work's first.
    #[serde(default)]
    pub replaces: Option<Revision>,
    /// The version of the working instructions its executor was given, in the executor's own
    /// words; none when it did not say.
    #[serde(default)]
    pub instructions: Option<String>,
    pub checks: Vec<BundleCheck>,
    pub published_at: String,
}

/// A file that is not a bundle this build reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleError(pub String);

impl std::fmt::Display for BundleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl Bundle {
    /// The file's text. The same bundle is the same bytes, so a bundle published twice is
    /// one revision.
    pub fn stored_text(&self) -> String {
        let mut text = serde_json::to_string_pretty(self).expect("a bundle is JSON");
        text.push('\n');
        text
    }

    /// Read a bundle back. A file of another format or a later version is refused and not
    /// guessed at: the model and the list it names would be read as something they are not.
    pub fn parse(text: &str) -> Result<Bundle, BundleError> {
        let bundle: Bundle =
            serde_json::from_str(text).map_err(|e| BundleError(format!("not a bundle: {e}")))?;
        if bundle.format != BUNDLE_FORMAT {
            return Err(BundleError(format!(
                "its format is `{}`, not `{BUNDLE_FORMAT}`",
                bundle.format
            )));
        }
        if bundle.v != BUNDLE_VERSION {
            return Err(BundleError(format!(
                "it is version {}, and this build reads version {BUNDLE_VERSION}",
                bundle.v
            )));
        }
        Ok(bundle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn revision(text: &str) -> Revision {
        Revision::of(text.as_bytes())
    }

    fn bundle() -> Bundle {
        Bundle {
            format: BUNDLE_FORMAT.to_string(),
            v: BUNDLE_VERSION,
            request: "req-0123456789ab".to_string(),
            attempt: 1,
            executor: "adapter-a".to_string(),
            source: revision("text"),
            answers: None,
            model: revision("model"),
            requirements: revision("list"),
            previous: None,
            replaces: None,
            instructions: None,
            checks: vec![BundleCheck {
                by: CheckedBy::Core,
                name: "model".to_string(),
                verdict: "accepted".to_string(),
                generator: Some("sce-codegen 0".to_string()),
                digest: Some(revision("record").to_string()),
                subject: Some(revision("model")),
            }],
            published_at: "2026-10-05T09:00:00Z".to_string(),
        }
    }

    #[test]
    fn a_bundle_is_read_back_from_its_text_and_the_same_bundle_is_the_same_bytes() {
        let published = bundle();

        let text = published.stored_text();

        assert_eq!(Bundle::parse(&text).unwrap(), published);
        assert_eq!(bundle().stored_text(), text);
        assert!(text.ends_with('\n'));
    }

    #[test]
    fn a_bundle_that_differs_in_one_revision_is_other_bytes() {
        let mut other = bundle();
        other.model = revision("another model");

        assert_ne!(
            Revision::of(other.stored_text().as_bytes()),
            Revision::of(bundle().stored_text().as_bytes())
        );
    }

    #[test]
    fn a_file_of_another_format_or_a_later_version_is_refused_and_not_guessed_at() {
        let mut other = bundle();
        other.format = "something-else".to_string();
        assert!(Bundle::parse(&other.stored_text())
            .unwrap_err()
            .0
            .contains("something-else"));

        let mut later = bundle();
        later.v = BUNDLE_VERSION + 1;
        assert!(Bundle::parse(&later.stored_text())
            .unwrap_err()
            .0
            .contains("version"));
        assert!(Bundle::parse("{ not json")
            .unwrap_err()
            .0
            .contains("not a bundle"));
    }

    #[test]
    fn a_check_says_who_ran_it_and_whether_it_was_accepted() {
        let mut check = bundle().checks.remove(0);
        assert!(check.is_accepted());
        check.verdict = "refused".to_string();
        assert!(!check.is_accepted());
        assert_eq!(
            serde_json::to_string(&CheckedBy::Client).unwrap(),
            "\"client\""
        );
    }

    #[test]
    fn the_first_bundle_of_a_work_that_had_a_model_before_says_what_it_took_over_from() {
        let mut first = bundle();
        first.previous = Some(Previous {
            model: Some(revision("old model")),
            requirements: None,
        });

        let read = Bundle::parse(&first.stored_text()).unwrap();

        assert_eq!(read.previous.unwrap().model, Some(revision("old model")));
    }
}
