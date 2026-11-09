// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The messages of the Remote Execution API (REAPI v2,
//! `build.bazel.remote.execution.v2`) that an action key and an action
//! result are made of, with the field numbers of `remote_execution.proto`.
//!
//! An action key is the SHA-256 digest of an encoded [`Action`], and an
//! action result is an [`ActionResult`] whose files live in a
//! content-addressed store, so that the keys this generator computes are the
//! keys any REAPI action cache holds (ADR 0097, answer 3): the local cache and
//! the fleet's are one layout, and a standard server can replace either side.
//!
//! Only the fields an action of this generator sets are declared. An encoder
//! writes the fields it knows in field-number order and omits a field at its
//! default value, which is protobuf's canonical form for these messages; a
//! field left out here is a field this generator never sets, so the bytes, and
//! with them the digest, are the ones a full implementation of the schema
//! writes for the same values.

use prost::Message;
use sha2::{Digest as _, Sha256};

/// The digest function of every digest here: SHA-256, the function the fleet's
/// content store uses (bx RFC 0049, section 2.3).
pub const DIGEST_FUNCTION: &str = "sha256";

/// `Digest`: the lowercase hex SHA-256 of a blob and its length.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Message)]
pub struct Digest {
    #[prost(string, tag = "1")]
    pub hash: String,
    #[prost(int64, tag = "2")]
    pub size_bytes: i64,
}

impl Digest {
    /// The digest of `bytes`.
    pub fn of(bytes: &[u8]) -> Digest {
        let hash: [u8; 32] = Sha256::digest(bytes).into();
        Digest {
            hash: crate::generator_witness::hex_encode(&hash),
            size_bytes: i64::try_from(bytes.len()).expect("a blob is shorter than i64::MAX"),
        }
    }

    /// The digest of the canonical encoding of `message`, and that encoding.
    pub fn of_message<M: Message>(message: &M) -> (Digest, Vec<u8>) {
        let bytes = message.encode_to_vec();
        (Digest::of(&bytes), bytes)
    }

    /// Whether `hash` is a well-formed SHA-256 hex digest, the only form a
    /// store accepts as a file name.
    pub fn is_well_formed(&self) -> bool {
        self.hash.len() == 64
            && self
                .hash
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            && self.size_bytes >= 0
    }
}

impl std::fmt::Display for Digest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.hash, self.size_bytes)
    }
}

/// `Platform.Property`.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct Property {
    #[prost(string, tag = "1")]
    pub name: String,
    #[prost(string, tag = "2")]
    pub value: String,
}

/// `Platform`: properties sorted by name, then by value, as REAPI requires.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct Platform {
    #[prost(message, repeated, tag = "1")]
    pub properties: Vec<Property>,
}

/// `Command.EnvironmentVariable`.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct EnvironmentVariable {
    #[prost(string, tag = "1")]
    pub name: String,
    #[prost(string, tag = "2")]
    pub value: String,
}

/// `Command`: what runs, in what environment, and which paths it produces.
/// Environment variables are sorted by name and output paths sorted, as
/// REAPI requires, so that one command has one encoding.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct Command {
    #[prost(string, repeated, tag = "1")]
    pub arguments: Vec<String>,
    #[prost(message, repeated, tag = "2")]
    pub environment_variables: Vec<EnvironmentVariable>,
    #[prost(message, optional, tag = "5")]
    pub platform: Option<Platform>,
    #[prost(string, tag = "6")]
    pub working_directory: String,
    #[prost(string, repeated, tag = "7")]
    pub output_paths: Vec<String>,
}

/// `Action`: a command and the Merkle root of its inputs. Its digest is the
/// action key.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct Action {
    #[prost(message, optional, tag = "1")]
    pub command_digest: Option<Digest>,
    #[prost(message, optional, tag = "2")]
    pub input_root_digest: Option<Digest>,
    #[prost(bool, tag = "7")]
    pub do_not_cache: bool,
    #[prost(bytes = "vec", tag = "9")]
    pub salt: Vec<u8>,
    #[prost(message, optional, tag = "10")]
    pub platform: Option<Platform>,
}

/// `FileNode`.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct FileNode {
    #[prost(string, tag = "1")]
    pub name: String,
    #[prost(message, optional, tag = "2")]
    pub digest: Option<Digest>,
    #[prost(bool, tag = "4")]
    pub is_executable: bool,
}

/// `DirectoryNode`.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct DirectoryNode {
    #[prost(string, tag = "1")]
    pub name: String,
    #[prost(message, optional, tag = "2")]
    pub digest: Option<Digest>,
}

/// `Directory`: one level of a Merkle tree, files and directories each sorted
/// by name.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct Directory {
    #[prost(message, repeated, tag = "1")]
    pub files: Vec<FileNode>,
    #[prost(message, repeated, tag = "2")]
    pub directories: Vec<DirectoryNode>,
}

/// `OutputFile`: a file an action produced, by its path relative to the
/// working directory.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct OutputFile {
    #[prost(string, tag = "1")]
    pub path: String,
    #[prost(message, optional, tag = "2")]
    pub digest: Option<Digest>,
    #[prost(bool, tag = "4")]
    pub is_executable: bool,
}

/// `ActionResult`: the files an action produced and how it ended.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct ActionResult {
    #[prost(message, repeated, tag = "2")]
    pub output_files: Vec<OutputFile>,
    #[prost(int32, tag = "4")]
    pub exit_code: i32,
    #[prost(message, optional, tag = "6")]
    pub stdout_digest: Option<Digest>,
    #[prost(message, optional, tag = "8")]
    pub stderr_digest: Option<Digest>,
}

/// `properties` sorted the way REAPI requires a platform's to be.
pub fn platform(mut properties: Vec<Property>) -> Platform {
    properties.sort_by(|a, b| (&a.name, &a.value).cmp(&(&b.name, &b.value)));
    Platform { properties }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The empty blob's digest is the one every REAPI implementation knows by
    /// heart; a hex or length mistake shows here first.
    #[test]
    fn the_empty_blob_has_the_well_known_digest() {
        let d = Digest::of(b"");
        assert_eq!(
            d.hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(d.size_bytes, 0);
        assert!(d.is_well_formed());
    }

    /// The bytes of a `Digest` message, written out by hand from the
    /// protobuf wire format: field 1 (length-delimited, tag byte 0x0a), the
    /// 64 hex characters; field 2 (varint, tag byte 0x10), the size. A
    /// derive that numbered or typed a field differently from
    /// `remote_execution.proto` would encode other bytes, and every action
    /// key would differ from the one a REAPI server computes.
    #[test]
    fn a_digest_encodes_as_the_proto_schema_says() {
        let d = Digest {
            hash: "a".repeat(64),
            size_bytes: 300,
        };
        let mut expected = vec![0x0a, 64];
        expected.extend(std::iter::repeat_n(b'a', 64));
        expected.extend([0x10, 0xac, 0x02]);
        assert_eq!(d.encode_to_vec(), expected);
    }

    /// The tags of `Action` that carry a key: 1, 2, 9 and 10. Encoded by hand
    /// for a minimal action and compared, for the reason above.
    #[test]
    fn an_action_encodes_its_fields_under_their_proto_tags() {
        let digest = Digest {
            hash: "0".repeat(64),
            size_bytes: 1,
        };
        let action = Action {
            command_digest: Some(digest.clone()),
            input_root_digest: Some(digest.clone()),
            do_not_cache: false,
            salt: b"s".to_vec(),
            platform: Some(platform(vec![Property {
                name: "n".into(),
                value: "v".into(),
            }])),
        };
        let inner = digest.encode_to_vec();
        let mut expected = Vec::new();
        for tag in [0x0a_u8, 0x12] {
            expected.push(tag);
            expected.push(u8::try_from(inner.len()).unwrap());
            expected.extend(&inner);
        }
        expected.extend([0x4a, 1, b's']);
        // Platform { properties: [Property { name: "n", value: "v" }] }
        expected.extend([0x52, 8, 0x0a, 6, 0x0a, 1, b'n', 0x12, 1, b'v']);
        assert_eq!(action.encode_to_vec(), expected);
    }

    #[test]
    fn a_platform_is_sorted_by_name_then_value() {
        let p = platform(vec![
            Property {
                name: "b".into(),
                value: "1".into(),
            },
            Property {
                name: "a".into(),
                value: "2".into(),
            },
            Property {
                name: "a".into(),
                value: "1".into(),
            },
        ]);
        let order: Vec<(&str, &str)> = p
            .properties
            .iter()
            .map(|q| (q.name.as_str(), q.value.as_str()))
            .collect();
        assert_eq!(order, vec![("a", "1"), ("a", "2"), ("b", "1")]);
    }

    #[test]
    fn a_digest_with_a_foreign_character_is_not_well_formed() {
        let d = Digest {
            hash: format!("{}G", "0".repeat(63)),
            size_bytes: 0,
        };
        assert!(!d.is_well_formed());
        let d = Digest {
            hash: "../".repeat(21) + "x",
            size_bytes: 0,
        };
        assert!(!d.is_well_formed());
    }
}
