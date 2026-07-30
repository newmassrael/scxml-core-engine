// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! `_ioprocessors` entry set (§scxml-C-1-1, §scxml-C-2-3).
//!
//! Port of the C++ `IOProcessorHelper` (`sce/include/common/IOProcessorHelper.h`).
//! Deciding the entries here rather than inside each script engine is what
//! keeps a machine reading the same entry names and the same addresses
//! whichever backend runs it.
//!
//! Gated to `!no_std` for the same reason [`super::url_encoding`] is: the
//! entries are published into a script engine's globals, and the codegen-time
//! validator rejects the datamodel that would need one before any no_std
//! machine could reach this.

#![cfg(not(feature = "no_std"))]

use super::scxml_constants::{BASIC_HTTP_EVENT_PROCESSOR_TYPE, SCXML_EVENT_PROCESSOR_TYPE};
use super::url_encoding::url_encode;

/// Alias the SCXML Event I/O Processor is indexed under by SCXML documents.
pub const SCXML_ALIAS: &str = "scxml";

/// Alias the Basic HTTP Event I/O Processor is indexed under by SCXML documents.
pub const BASIC_HTTP_ALIAS: &str = "basichttp";

/// One entry of `_ioprocessors`: the key it is filed under, and the address
/// external entities use to reach this session through that processor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IoProcessorDescriptor {
    /// Key the entry is filed under in `_ioprocessors`.
    pub name: String,
    /// Value of the entry's 'location' field.
    pub location: String,
}

/// Address that reaches this session over the SCXML Event I/O Processor.
///
/// §scxml-C-1 leaves the transport platform-specific, so the address is an
/// SCE-scheme URI naming the session. The session id is percent-encoded
/// because it arrives from `<invoke>` and from embedders, neither of which is
/// constrained to URI-safe characters.
pub fn scxml_location(session_id: &str) -> String {
    let mut location = String::from("sce://scxml/");
    location.push_str(&url_encode(session_id));
    location
}

/// Entry set for a session.
///
/// Every processor is filed twice: under the specification's entry name and
/// under the short alias SCXML documents index with. Both keys carry the same
/// location, so the choice of spelling never changes where an event goes.
///
/// §scxml-C-2-3's entry appears only when `basic_http_access_uri` is non-empty.
/// Support for that processor is optional and per-deployment, so a session with
/// no inbound endpoint advertises no address rather than one nothing answers on.
pub fn build(session_id: &str, basic_http_access_uri: &str) -> Vec<IoProcessorDescriptor> {
    let mut descriptors = Vec::new();

    let scxml_uri = scxml_location(session_id);
    descriptors.push(IoProcessorDescriptor {
        name: SCXML_EVENT_PROCESSOR_TYPE.to_string(),
        location: scxml_uri.clone(),
    });
    descriptors.push(IoProcessorDescriptor {
        name: SCXML_ALIAS.to_string(),
        location: scxml_uri,
    });

    if !basic_http_access_uri.is_empty() {
        descriptors.push(IoProcessorDescriptor {
            name: BASIC_HTTP_EVENT_PROCESSOR_TYPE.to_string(),
            location: basic_http_access_uri.to_string(),
        });
        descriptors.push(IoProcessorDescriptor {
            name: BASIC_HTTP_ALIAS.to_string(),
            location: basic_http_access_uri.to_string(),
        });
    }

    descriptors
}

#[cfg(test)]
mod tests {
    use super::*;

    fn location_of(descriptors: &[IoProcessorDescriptor], name: &str) -> Option<String> {
        descriptors
            .iter()
            .find(|d| d.name == name)
            .map(|d| d.location.clone())
    }

    #[test]
    fn scxml_processor_is_published_under_both_spellings() {
        let descriptors = build("session-1", "");

        assert_eq!(
            location_of(&descriptors, SCXML_EVENT_PROCESSOR_TYPE),
            location_of(&descriptors, SCXML_ALIAS)
        );
        assert_eq!(
            location_of(&descriptors, SCXML_ALIAS).as_deref(),
            Some("sce://scxml/session-1")
        );
    }

    #[test]
    fn session_id_is_percent_encoded_into_the_location() {
        let descriptors = build("a b/c#d", "");

        assert_eq!(
            location_of(&descriptors, SCXML_ALIAS).as_deref(),
            Some("sce://scxml/a%20b%2Fc%23d")
        );
    }

    #[test]
    fn no_http_entry_when_no_endpoint_is_deployed() {
        let descriptors = build("session-1", "");

        assert!(location_of(&descriptors, BASIC_HTTP_EVENT_PROCESSOR_TYPE).is_none());
        assert!(location_of(&descriptors, BASIC_HTTP_ALIAS).is_none());
    }

    #[test]
    fn http_entry_carries_the_deployed_access_uri() {
        let descriptors = build("session-1", "http://localhost:8080/test");

        assert_eq!(
            location_of(&descriptors, BASIC_HTTP_EVENT_PROCESSOR_TYPE).as_deref(),
            Some("http://localhost:8080/test")
        );
        assert_eq!(
            location_of(&descriptors, BASIC_HTTP_ALIAS).as_deref(),
            Some("http://localhost:8080/test")
        );
    }
}
