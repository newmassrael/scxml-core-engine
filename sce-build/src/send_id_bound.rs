// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! How long the id of a `<send>` of a document can be.
//!
//! A delayed send waits in the scheduler under an id so that a `<cancel>` can find
//! it, and the id is text. The Rust runtime stores it in a string whose capacity
//! the generator writes on the policy (`StatePolicy::ScheduledSendId`), so that a
//! document that names its sends `d_delayed` does not keep a buffer of the runtime's
//! string bound for each, in every slot of the scheduler and in each temporary a
//! copy of it passes through.
//!
//! The longest id is known when the document is built. A `<send>` is sent under
//! one of two things, and each is bounded:
//!
//! - the id the document writes: the `id` the author gave, or, where there is none,
//!   the one the parser names it by (`__send_N`), both read from `auto_send_id`;
//! - the id the machine generates for a `<send idlocation>`, `_auto_send_` and a
//!   `u64` count, which is [`AUTO_SEND_ID_MAX_BYTES`] at most.

use crate::forge::static_datamodel::AUTO_SEND_ID_MAX_BYTES;
use crate::host_processor_analyzer::walk_model_actions;
use crate::model::SCXMLModel;

/// The most bytes any `<send>` of `model` is sent under, at least one: a buffer of
/// no bytes has no use to anything that holds one.
pub fn send_id_max_bytes(model: &SCXMLModel) -> usize {
    let mut longest = 0;
    walk_model_actions(model, &mut |_state, action| {
        if action.action_type != "send" {
            return;
        }
        longest = longest.max(action.auto_send_id.len()).max(action.id.len());
        if !action.idlocation.is_empty() {
            longest = longest.max(AUTO_SEND_ID_MAX_BYTES as usize);
        }
    });
    longest.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SCXMLParser;

    fn bound(onentry: &str) -> usize {
        let document = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="a">
  <datamodel><data id="where"/></datamodel>
  <state id="a"><onentry>{onentry}</onentry></state>
</scxml>"#
        );
        let model = SCXMLParser::new()
            .parse_string(&document, "send_id_bound")
            .unwrap_or_else(|e| panic!("parse failed: {:?}", e.error));
        send_id_max_bytes(&model)
    }

    #[test]
    fn a_document_that_sends_nothing_still_has_a_cancel_key_of_one_byte() {
        assert_eq!(bound(""), 1);
    }

    #[test]
    fn a_send_is_held_to_the_id_the_author_gave_it() {
        assert_eq!(
            bound(r#"<send event="e" delay="5ms" id="d_delayed"/>"#),
            "d_delayed".len()
        );
        // The longest of several.
        assert_eq!(
            bound(r#"<send event="e" id="ab"/><send event="e" id="abcdef"/>"#),
            6
        );
    }

    #[test]
    fn a_send_without_an_id_is_held_to_the_name_the_parser_gives_it() {
        assert_eq!(bound(r#"<send event="e"/>"#), "__send_0".len());
    }

    #[test]
    fn a_send_that_stores_its_id_is_held_to_the_longest_the_machine_generates() {
        assert_eq!(
            bound(r#"<send event="e" idlocation="where"/>"#),
            AUTO_SEND_ID_MAX_BYTES as usize
        );
    }

    #[test]
    fn a_send_inside_an_if_counts_as_one_outside_it_does() {
        assert_eq!(
            bound(r#"<if cond="true"><send event="e" id="nested_id"/></if>"#),
            "nested_id".len()
        );
    }
}
