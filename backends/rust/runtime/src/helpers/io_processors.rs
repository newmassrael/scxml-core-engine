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
use super::url_encoding::{url_decode, url_encode};

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

/// Session id an SCXML Event I/O Processor location names, if any.
///
/// The inverse of [`scxml_location`], kept beside it so the two spellings of
/// one address cannot drift apart. §scxml-C-1 requires the location a session
/// publishes to be usable as a `<send>` target, which only holds if something
/// can read a session back out of it.
///
/// Returns an empty string when the input is not an SCXML processor location
/// or names no session.
pub fn session_id_from_scxml_location(uri: &str) -> String {
    match uri.strip_prefix("sce://scxml/") {
        Some(rest) if !rest.is_empty() => url_decode(rest),
        _ => String::new(),
    }
}

/// The `_event.origin` a receiver should see for an event sent by
/// `origin_session_id`.
///
/// §scxml-C-1 requires the origin of a delivered event to match the 'location'
/// the sending session published, which is what makes it an address the
/// receiver can answer. The engine carries the sender's BARE session id
/// internally — `EventMetadata::origin` — because its session-keyed lookups
/// (`<finalize>` dispatch, cancelled-invoke filtering) match on the id.
/// Converting where the event is raised would make one value serve two
/// consumers that need different spellings. So the conversion belongs at the
/// boundary where the value becomes visible to the document, and this is that
/// conversion — the same rule, and the same shape, as the C++
/// `IOProcessorHelper::publishedOrigin` both engines already share.
///
/// A remote invoke is the case that makes this more than a rename: its child
/// session is stamped with a URI rather than an id, and wrapping a URI in
/// [`scxml_location`] would produce an address naming nothing. An argument that
/// already carries a scheme is therefore passed through — it is already an
/// address.
pub fn published_origin(origin_session_id: &str) -> String {
    // §scxml-C-1: the 'origin' of the event raised in the receiving session
    // must match the 'location' the sending session published in its
    // `_ioprocessors` entry. This is where the id the engine carries
    // internally becomes that location.
    if origin_session_id.is_empty() {
        return String::new();
    }
    if origin_session_id.contains("://") {
        return origin_session_id.to_string();
    }
    scxml_location(origin_session_id)
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
    fn a_published_location_reads_back_as_the_session_it_names() {
        // The round trip is the clause: an origin that cannot be decoded back
        // to a session is not an address a peer can answer.
        assert_eq!(
            session_id_from_scxml_location(&scxml_location("a b/c#d")),
            "a b/c#d"
        );
        assert_eq!(session_id_from_scxml_location("sce://scxml/"), "");
        assert_eq!(session_id_from_scxml_location("session-1"), "");
        assert_eq!(session_id_from_scxml_location("http://host/x"), "");
    }

    #[test]
    fn published_origin_is_the_location_the_sender_publishes() {
        let descriptors = build("session-1", "");

        assert_eq!(
            Some(published_origin("session-1")),
            location_of(&descriptors, SCXML_ALIAS)
        );
    }

    #[test]
    fn published_origin_passes_an_address_through_and_keeps_empty_empty() {
        // A remote child is stamped with a URI, not an id; wrapping it again
        // would produce an address naming nothing.
        assert_eq!(published_origin("sce://mesh/peer-7"), "sce://mesh/peer-7");
        assert_eq!(published_origin(""), "");
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
