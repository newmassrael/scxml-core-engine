// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Reading an author's JSON the way the author meant it.
//!
//! JSON lets an object write one key twice, and `serde_json` answers by
//! keeping the LAST value and saying nothing. A scenario set that wrote
//! `"origin": "ai-proposed"` and then `"origin": "owner-written"` was read as
//! the owner's own examples with no problem reported (reproduced against
//! `sce-codegen scenarios`); an authoring profile, an acceptance record, a
//! requirement manifest and an observation trace read the same way. Each is a
//! file that loads cleanly and means something its author did not write.
//!
//! [`from_str`] is `serde_json::from_str` with that one refusal added, at any
//! depth, naming the key and where the second one is. It keeps the error type,
//! so a reader that already answers a parse error answers this one in the
//! same place and the same words. [`check`] is the refusal alone, for a reader
//! that parses into something it does not own.
//!
//! Two spellings of one key (`"a"` and `"a"`) are one key once decoded,
//! and are refused as one: the reader's own account of a key is the decoded
//! one.

use std::collections::HashSet;
use std::fmt;

use serde::de::{self, DeserializeOwned, DeserializeSeed, MapAccess, SeqAccess, Visitor};

/// Parse `text` as `T`, refusing an object that writes a key twice.
pub fn from_str<T: DeserializeOwned>(text: &str) -> serde_json::Result<T> {
    check(text)?;
    serde_json::from_str(text)
}

/// Refuse `text` when an object in it writes a key twice. Malformed JSON is
/// refused too, as `serde_json` would refuse it.
pub fn check(text: &str) -> serde_json::Result<()> {
    let mut reader = serde_json::Deserializer::from_str(text);
    Unrepeated.deserialize(&mut reader)?;
    reader.end()
}

/// Walks a document and holds nothing of it: each object's keys are kept only
/// while that object is being read.
struct Unrepeated;

impl<'de> DeserializeSeed<'de> for Unrepeated {
    type Value = ();

    fn deserialize<D: de::Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Unrepeated {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    fn visit_bool<E>(self, _: bool) -> Result<(), E> {
        Ok(())
    }

    fn visit_i64<E>(self, _: i64) -> Result<(), E> {
        Ok(())
    }

    fn visit_u64<E>(self, _: u64) -> Result<(), E> {
        Ok(())
    }

    fn visit_f64<E>(self, _: f64) -> Result<(), E> {
        Ok(())
    }

    fn visit_str<E>(self, _: &str) -> Result<(), E> {
        Ok(())
    }

    fn visit_unit<E>(self) -> Result<(), E> {
        Ok(())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        while seq.next_element_seed(Unrepeated)?.is_some() {}
        Ok(())
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        let mut seen: HashSet<String> = HashSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if seen.contains(&key) {
                return Err(de::Error::custom(format!(
                    "the key {key:?} is written twice in one object; a repeated key keeps \
                     only the last value and says nothing, so say which one is meant"
                )));
            }
            map.next_value_seed(Unrepeated)?;
            seen.insert(key);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn refused(text: &str) -> String {
        check(text)
            .expect_err("a repeated key is refused")
            .to_string()
    }

    #[test]
    fn a_key_written_twice_is_refused_naming_the_key() {
        let message = refused(r#"{"origin": "ai-proposed", "origin": "owner-written"}"#);
        assert!(
            message.contains(r#"the key "origin" is written twice"#),
            "{message}"
        );
    }

    #[test]
    fn it_is_found_at_any_depth_and_inside_an_array() {
        for text in [
            r#"{"a": {"b": {"c": 1, "c": 2}}}"#,
            r#"{"a": [1, {"c": 1, "c": 2}]}"#,
            r#"[{"x": 1}, {"y": 1, "y": 1}]"#,
        ] {
            assert!(refused(text).contains("is written twice"), "{text}");
        }
    }

    #[test]
    fn the_refusal_says_where_the_second_one_is() {
        let message = refused("{\n  \"a\": 1,\n  \"a\": 2\n}");
        assert!(message.contains("line 3"), "{message}");
    }

    #[test]
    fn two_spellings_of_one_key_are_one_key() {
        assert!(refused(r#"{"a": 1, "a": 2}"#).contains("is written twice"));
    }

    #[test]
    fn the_same_key_in_different_objects_is_not_a_repeat() {
        assert!(check(r#"{"a": {"x": 1}, "b": {"x": 2}, "c": [{"x": 3}, {"x": 4}]}"#).is_ok());
    }

    #[test]
    fn every_kind_of_value_is_walked() {
        assert!(check(
            r#"{"t": true, "f": false, "n": null, "i": -3, "u": 3, "d": 1.5, "s": "s"}"#
        )
        .is_ok());
        assert!(check("[]").is_ok());
        assert!(check("7").is_ok());
    }

    #[test]
    fn malformed_json_is_refused_as_serde_json_refuses_it() {
        let ours = check(r#"{"a": "#).expect_err("truncated").to_string();
        let theirs = serde_json::from_str::<Value>(r#"{"a": "#)
            .expect_err("truncated")
            .to_string();
        assert_eq!(theirs, ours);
        assert!(check(r#"{"a": 1} trailing"#).is_err());
    }

    #[test]
    fn what_is_read_is_what_serde_json_reads() {
        let text = r#"{"a": [1, 2], "b": {"c": null}}"#;
        let ours: Value = from_str(text).expect("well formed");
        let theirs: Value = serde_json::from_str(text).expect("well formed");
        assert_eq!(theirs, ours);
    }

    #[test]
    fn the_refusal_classifies_as_data_so_a_reader_that_tells_shape_from_syntax_calls_it_shape() {
        let error = check(r#"{"a": 1, "a": 2}"#).expect_err("repeated");
        assert_eq!(serde_json::error::Category::Data, error.classify());
    }
}
