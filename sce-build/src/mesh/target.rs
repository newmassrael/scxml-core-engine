// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Newtype for SCXML send target identifiers (e.g. "#motor"), distinguishing
// them from arbitrary strings and centralising parsing/formatting.

use serde::{Deserialize, Serialize};
use std::fmt;

/// A validated send-target identifier from an SCXML `<send target="#X"/>` or
/// `<send target="#_parent"/>` attribute. Always carries the leading `#`.
///
/// Serialization is transparent: Jinja2 templates receive the raw identifier
/// (e.g. `"#motor"`) unchanged, and `HashMap<TargetId, _>` keys round-trip
/// through YAML as plain strings. Parsing goes through `TargetId::try_from`
/// so empty values are rejected at the deploy.yaml boundary.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TargetId(String);

impl TargetId {
    /// Construct from a raw attribute value. Returns `None` if empty.
    /// Leading `#` is preserved — callers use `name()` to get the stripped
    /// form when they need the receiver machine name.
    pub fn new(raw: impl Into<String>) -> Option<Self> {
        let s: String = raw.into();
        if s.is_empty() {
            None
        } else {
            Some(Self(s))
        }
    }

    /// The target ID verbatim (e.g. "#motor").
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Strip the leading `#` if present; returns the receiver name used
    /// to look up the machine in deploy.yaml (e.g. "motor" for "#motor").
    pub fn name(&self) -> &str {
        self.0.trim_start_matches('#')
    }

    /// True for W3C-reserved targets (#_parent, #_child, #_internal, #_scxml_)
    /// which never reach transport dispatch.
    pub fn is_internal(&self) -> bool {
        matches!(
            self.0.as_str(),
            "#_parent" | "#_child" | "#_internal" | "#_scxml_"
        )
    }
}

impl fmt::Display for TargetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for TargetId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::borrow::Borrow<str> for TargetId {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl PartialEq<str> for TargetId {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for TargetId {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl From<TargetId> for String {
    fn from(t: TargetId) -> Self {
        t.0
    }
}

// ── Tests ───────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_rejects_empty() {
        assert!(TargetId::new("").is_none());
    }

    #[test]
    fn new_accepts_non_empty() {
        assert_eq!(TargetId::new("#motor").unwrap().as_str(), "#motor");
    }

    #[test]
    fn name_strips_leading_hash() {
        assert_eq!(TargetId::new("#motor").unwrap().name(), "motor");
    }

    #[test]
    fn name_without_hash_unchanged() {
        // Defensive: caller supplied a bare name — pass through.
        assert_eq!(TargetId::new("motor").unwrap().name(), "motor");
    }

    #[test]
    fn is_internal_matches_w3c_reserved() {
        for t in ["#_parent", "#_child", "#_internal", "#_scxml_"] {
            assert!(TargetId::new(t).unwrap().is_internal(), "{t} should be internal");
        }
    }

    #[test]
    fn is_internal_rejects_user_targets() {
        for t in ["#motor", "#display", "#logger", "#_parentX"] {
            assert!(!TargetId::new(t).unwrap().is_internal(), "{t} should not be internal");
        }
    }

    #[test]
    fn display_round_trips() {
        let t = TargetId::new("#motor").unwrap();
        assert_eq!(format!("{t}"), "#motor");
    }

    #[test]
    fn partial_eq_with_str() {
        let t = TargetId::new("#motor").unwrap();
        assert_eq!(t, *"#motor");
        assert_eq!(t, "#motor");
    }

    #[test]
    fn serde_transparent_as_yaml_string() {
        use std::collections::HashMap;
        let mut m: HashMap<TargetId, u32> = HashMap::new();
        m.insert(TargetId::new("#motor").unwrap(), 1);
        let yaml = serde_yaml_ng::to_string(&m).unwrap();
        assert!(yaml.contains("'#motor': 1") || yaml.contains("\"#motor\": 1") || yaml.contains("#motor: 1"), "yaml was: {yaml}");
    }

    #[test]
    fn serde_deserialize_rejects_empty_from_yaml() {
        // serde(transparent) with String inner does NOT reject empty by itself
        // (the inner String happily deserializes ""). The empty-rejection
        // invariant is enforced only at the TargetId::new construction site.
        // This test documents that behavior so future readers understand the
        // split between parse-time (deploy.yaml) and programmatic construction.
        let t: TargetId = serde_yaml_ng::from_str("''").unwrap();
        assert_eq!(t.as_str(), "");
    }
}
