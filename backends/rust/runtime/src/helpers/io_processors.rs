// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

#![cfg(feature = "std")]

use super::scxml_constants::{BASIC_HTTP_EVENT_PROCESSOR_TYPE, SCXML_EVENT_PROCESSOR_TYPE};
use super::url_encoding::{url_decode, url_encode};
use crate::event::EventOrigin;

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

/// What every SCXML Event I/O Processor location begins with, before the
/// percent-encoded session id ([`scxml_location`]). Alone, it names no session.
pub const SCXML_LOCATION_PREFIX: &str = "sce://scxml/";

/// Address that reaches this session over the SCXML Event I/O Processor.
///
/// §scxml-C-1 leaves the transport platform-specific, so the address is an
/// SCE-scheme URI naming the session. The session id is percent-encoded
/// because it arrives from `<invoke>` and from embedders, neither of which is
/// constrained to URI-safe characters.
pub fn scxml_location(session_id: &str) -> String {
    let mut location = String::from(SCXML_LOCATION_PREFIX);
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
    match uri.strip_prefix(SCXML_LOCATION_PREFIX) {
        Some(rest) if !rest.is_empty() => url_decode(rest),
        _ => String::new(),
    }
}

/// The origin the SCXML Event I/O Processor writes for an event the session
/// `sender` sends (docs/adr/0090, section 2).
///
/// The SCXML processor's rule has the origin of an event raised in the
/// receiving session match the location the sending session publishes, so the
/// pair is the
/// processor's canonical type and [`scxml_location`] of the sender — the one
/// function that also writes the session's `_ioprocessors` entry. The sender's
/// id travels beside the pair, as the event's `sender`, for the lookups that
/// find a session by it.
pub fn scxml_origin(sender: &str) -> EventOrigin {
    EventOrigin::new(SCXML_EVENT_PROCESSOR_TYPE, scxml_location(sender))
}

/// [`scxml_origin`] of a sender that may be absent: a machine that holds no
/// session id has no address to publish, so an event it sends carries no
/// origin rather than the location of an empty id.
pub fn scxml_origin_of(sender: &str) -> Option<EventOrigin> {
    if sender.is_empty() {
        None
    } else {
        Some(scxml_origin(sender))
    }
}

/// The `origin` text a format-1 or format-2 saved state stores for an event's
/// origin and sender: the sending session's id for an event a session sent, the
/// address otherwise, and empty for none. The `origintype` beside it is the
/// pair's processor. A format-3 build writes it only for a state read from a
/// format-1 text and written back as one, which has no place for the pair
/// (docs/adr/0090, section 7); [`read_stored_origin`] reads it back.
pub fn stored_origin_text(origin: Option<&EventOrigin>, sender: &str) -> String {
    if sender.is_empty() {
        origin.map_or_else(String::new, |origin| origin.address.clone())
    } else {
        sender.to_string()
    }
}

/// An event's origin and sender, as a stored `origin` and `origintype` spell
/// them back.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StoredOrigin {
    /// The pair the event carries, `None` for none.
    pub origin: Option<EventOrigin>,
    /// The sending session's id, empty unless a session sent the event.
    pub sender: String,
}

/// The origin and sender a format-1 or format-2 `origin` and `origintype` are
/// read as (docs/adr/0090, section 7).
///
/// An empty `origin` is none whatever type was stored beside it: an empty
/// address answers nothing, and format 2 wrote the SCXML type beside it for
/// events no processor stamped. An empty `origintype` beside an address is the
/// SCXML processor's, which is what format 2 meant by it. A value without a
/// scheme read with the SCXML type is a session's id, read as that session's
/// pair with the id as its sender; an SCXML location (which one backend stored
/// in place of the id) is read the same way by the id it names. Anything else
/// is the pair as stored, with no sender. Never half a pair.
pub fn read_stored_origin(origin: &str, origin_type: &str) -> StoredOrigin {
    if origin.is_empty() {
        return StoredOrigin::default();
    }
    let located = session_id_from_scxml_location(origin);
    if !located.is_empty() {
        return StoredOrigin {
            origin: Some(scxml_origin(&located)),
            sender: located,
        };
    }
    let processor = if origin_type.is_empty() {
        SCXML_EVENT_PROCESSOR_TYPE
    } else {
        origin_type
    };
    if !origin.contains("://") && processor == SCXML_EVENT_PROCESSOR_TYPE {
        return StoredOrigin {
            origin: Some(scxml_origin(origin)),
            sender: origin.to_string(),
        };
    }
    StoredOrigin {
        origin: Some(EventOrigin::new(processor, origin)),
        sender: String::new(),
    }
}

/// The sender a format-1 or format-2 waiting send's `origin` names
/// (docs/adr/0090, section 7): a waiting send is always a session's, so a bare
/// id is that id and an SCXML location names it. A send to the internal queue
/// is the exception, whose stored `origin` is not read.
pub fn read_stored_sender(origin: &str) -> String {
    let located = session_id_from_scxml_location(origin);
    if located.is_empty() {
        origin.to_string()
    } else {
        located
    }
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
    fn a_reserved_character_survives_the_round_trip() {
        // `%` is the escape itself, and `:` and `?` would read as URI
        // structure: a session id holding them must still be the one its
        // location names.
        for id in ["50%off", "a:b?c=d", "x/y#z", "planner-1.worker"] {
            assert_eq!(session_id_from_scxml_location(&scxml_location(id)), id);
        }
    }

    #[test]
    fn the_scxml_origin_is_the_processor_and_the_location_the_sender_publishes() {
        let descriptors = build("a b", "");

        let origin = scxml_origin("a b");
        assert_eq!(origin.processor, SCXML_EVENT_PROCESSOR_TYPE);
        assert_eq!(Some(origin.address), location_of(&descriptors, SCXML_ALIAS));
        assert_eq!(scxml_origin_of(""), None);
        assert_eq!(scxml_origin_of("a b"), Some(scxml_origin("a b")));
    }

    #[test]
    fn a_stored_origin_is_the_sender_for_a_session_and_the_address_otherwise() {
        let host = EventOrigin::new("x-sce-host", "motor");

        assert_eq!(
            stored_origin_text(Some(&scxml_origin("s1")), "s1"),
            "s1",
            "formats 1 and 2 store the sending session's id, not its location"
        );
        assert_eq!(stored_origin_text(Some(&host), ""), "motor");
        assert_eq!(stored_origin_text(None, ""), "");
    }

    #[test]
    fn a_stored_origin_is_read_by_the_table_of_section_7() {
        assert_eq!(read_stored_origin("", ""), StoredOrigin::default());
        // An empty address is none, whatever type format 2 saved beside it.
        assert_eq!(
            read_stored_origin("", SCXML_EVENT_PROCESSOR_TYPE),
            StoredOrigin::default()
        );
        // An address saved with no type is the SCXML processor's.
        assert_eq!(
            read_stored_origin("s1", ""),
            StoredOrigin {
                origin: Some(scxml_origin("s1")),
                sender: "s1".to_string(),
            }
        );
        assert_eq!(
            read_stored_origin("mesh://ecu", ""),
            StoredOrigin {
                origin: Some(EventOrigin::new(SCXML_EVENT_PROCESSOR_TYPE, "mesh://ecu")),
                sender: String::new(),
            }
        );
        assert_eq!(
            read_stored_origin("s1", SCXML_EVENT_PROCESSOR_TYPE),
            StoredOrigin {
                origin: Some(scxml_origin("s1")),
                sender: "s1".to_string(),
            }
        );
        // C11 stored the location in place of the id, with any type.
        assert_eq!(
            read_stored_origin("sce://scxml/a%20b", ""),
            StoredOrigin {
                origin: Some(scxml_origin("a b")),
                sender: "a b".to_string(),
            }
        );
        // A host's pair has no sender, whether or not its address has a scheme.
        assert_eq!(
            read_stored_origin("motor", "x-sce-host"),
            StoredOrigin {
                origin: Some(EventOrigin::new("x-sce-host", "motor")),
                sender: String::new(),
            }
        );
        assert_eq!(
            read_stored_origin("mesh://ecu", SCXML_EVENT_PROCESSOR_TYPE),
            StoredOrigin {
                origin: Some(EventOrigin::new(SCXML_EVENT_PROCESSOR_TYPE, "mesh://ecu")),
                sender: String::new(),
            }
        );
    }

    #[test]
    fn a_stored_waiting_send_names_its_sender_by_id_or_location() {
        assert_eq!(read_stored_sender("s1"), "s1");
        assert_eq!(read_stored_sender("sce://scxml/a%20b"), "a b");
        assert_eq!(read_stored_sender(""), "");
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
