// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The originals of a spec, and what was read out of them (ADR 0032, 0026).
//!
//! An original is a bundle: a manifest of members named by the hash of their bytes, and the original is
//! named by the hash of its manifest. What is held here is what makes that a record a caller can rely
//! on: the same bundle is the same original whatever order it was given in; a path cannot climb out of
//! the bundle; a manifest names only bytes that are there, and a store whose bytes were lost or changed
//! says so and does not answer; and an extraction is kept as it was given, named by what it holds, for
//! a member that exists. Each refusal is paired with the nearest input that must not be refused.

mod common;

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use sce_specs::{
    call, ManualClock, MemberDraft, OriginalDraft, Revision, Role, SpecId, SpecStore, StoreError,
};
use serde_json::{json, Value};

use common::{scratch, FakeRenderer};

const T0: u64 = 1_791_190_800;

struct Fixture {
    root: PathBuf,
    store: SpecStore<Arc<ManualClock>>,
    spec: SpecId,
}

fn fixture(label: &str) -> Fixture {
    let root = scratch(label);
    let store = SpecStore::with_clock(&root, Arc::new(ManualClock::at(T0)));
    let spec = store.create_spec("Door lock").unwrap().id;
    Fixture { root, store, spec }
}

impl Fixture {
    fn dir(&self) -> PathBuf {
        self.root.join(self.spec.as_str())
    }

    fn add(&self, draft: OriginalDraft) -> sce_specs::OriginalAdded {
        self.store.add_original(&self.spec, draft).unwrap()
    }
}

fn one(path: &str, bytes: &[u8]) -> OriginalDraft {
    OriginalDraft {
        label: "spec".to_string(),
        members: vec![MemberDraft::new(path, Role::Document, bytes.to_vec())],
        missing: vec![],
    }
}

fn refusal(error: StoreError) -> (&'static str, String) {
    match error {
        StoreError::Refused { kind, message, .. } => (kind, message),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn an_original_is_added_named_and_read_back_member_by_member() {
    let f = fixture("orig-roundtrip");
    let draft = OriginalDraft {
        label: "Door lock, from the customer".to_string(),
        members: vec![
            MemberDraft::new("spec.md", Role::Document, b"The lock opens.".to_vec()),
            MemberDraft::new(
                "images/lock.png",
                Role::Picture,
                vec![0x89, b'P', b'N', b'G'],
            ),
        ],
        missing: vec!["images/gone.png".to_string()],
    };

    let added = f.add(draft);
    let manifest = f.store.read_original(&f.spec, &added.digest).unwrap();

    assert!(added.created);
    assert_eq!((added.members, added.bytes), (2, 19));
    assert_eq!(manifest.label, "Door lock, from the customer");
    assert_eq!(manifest.revision(), added.digest);
    assert_eq!(manifest.missing, vec!["images/gone.png"]);
    assert_eq!(
        f.store
            .read_original_member(&f.spec, &added.digest, "spec.md")
            .unwrap(),
        b"The lock opens."
    );
    let picture = manifest.member("images/lock.png").unwrap();
    assert_eq!(picture.role, Role::Picture);
    assert_eq!(picture.hash, Revision::of(&[0x89, b'P', b'N', b'G']));
    assert_eq!(picture.size, 4);
}

#[test]
fn the_same_bundle_is_the_same_original_in_any_order_and_is_added_once() {
    let f = fixture("orig-same");
    let members = |reversed: bool| {
        let mut list = vec![
            MemberDraft::new("a.md", Role::Document, b"a".to_vec()),
            MemberDraft::new("b.png", Role::Picture, b"b".to_vec()),
        ];
        if reversed {
            list.reverse();
        }
        OriginalDraft {
            label: "x".to_string(),
            members: list,
            missing: vec!["z.png".to_string(), "y.png".to_string()],
        }
    };

    let first = f.add(members(false));
    let second = f.add(members(true));

    assert_eq!(first.digest, second.digest);
    assert!(first.created && !second.created);
    assert_eq!(f.store.list_originals(&f.spec).unwrap().len(), 1);
}

#[test]
fn another_label_or_another_role_or_other_bytes_is_another_original() {
    let f = fixture("orig-other");
    let base = f.add(one("spec.md", b"one"));

    let relabelled = f.add(OriginalDraft {
        label: "other".to_string(),
        ..one("spec.md", b"one")
    });
    let other_bytes = f.add(one("spec.md", b"two"));
    let other_role = f.add(OriginalDraft {
        members: vec![MemberDraft::new(
            "spec.md",
            Role::Attachment,
            b"one".to_vec(),
        )],
        ..one("spec.md", b"one")
    });

    let digests = [
        &base.digest,
        &relabelled.digest,
        &other_bytes.digest,
        &other_role.digest,
    ];
    for (i, a) in digests.iter().enumerate() {
        for b in &digests[i + 1..] {
            assert_ne!(a, b);
        }
    }
}

#[test]
fn two_originals_that_hold_one_picture_keep_one_copy_of_it() {
    let f = fixture("orig-shared");
    let picture = vec![7u8; 1000];
    f.add(OriginalDraft {
        members: vec![
            MemberDraft::new("a.md", Role::Document, b"a".to_vec()),
            MemberDraft::new("p.png", Role::Picture, picture.clone()),
        ],
        ..one("a.md", b"a")
    });
    f.add(OriginalDraft {
        members: vec![
            MemberDraft::new("b.md", Role::Document, b"b".to_vec()),
            MemberDraft::new("images/q.png", Role::Picture, picture),
        ],
        ..one("b.md", b"b")
    });

    let held = fs::read_dir(f.dir().join("members")).unwrap().count();

    // a, b, and the picture once.
    assert_eq!(held, 3);
}

#[test]
fn two_members_with_one_content_are_one_blob_and_two_members() {
    let f = fixture("orig-twin");
    let added = f.add(OriginalDraft {
        members: vec![
            MemberDraft::new("a.txt", Role::Document, b"same".to_vec()),
            MemberDraft::new("b.txt", Role::Attachment, b"same".to_vec()),
        ],
        ..one("a.txt", b"")
    });

    let manifest = f.store.read_original(&f.spec, &added.digest).unwrap();

    assert_eq!(manifest.members.len(), 2);
    assert_eq!(fs::read_dir(f.dir().join("members")).unwrap().count(), 1);
}

#[test]
fn a_path_that_could_leave_the_bundle_is_refused_and_a_nested_one_is_carried() {
    let f = fixture("orig-paths");
    for bad in [
        "",
        "/etc/passwd",
        "../up.md",
        "a/../b.md",
        "./a.md",
        "a//b.md",
        "a\\b.md",
        "a/",
        ".",
        "..",
        "tab\tname.md",
    ] {
        let error = f
            .store
            .add_original(&f.spec, one(bad, b"x"))
            .expect_err(&format!("{bad:?} was accepted"));
        assert_eq!(refusal(error).0, "bad-original", "{bad:?}");
    }
    let long = "a".repeat(256);
    assert_eq!(
        refusal(f.store.add_original(&f.spec, one(&long, b"x")).unwrap_err()).0,
        "bad-original"
    );

    for good in [
        "spec.md",
        "images/a.png",
        "a/b/c/d.txt",
        "no suffix",
        "dots.in.name.v2.md",
        "..hidden-ish.md",
    ] {
        f.store
            .add_original(&f.spec, one(good, b"x"))
            .unwrap_or_else(|e| panic!("{good:?} was refused: {e}"));
    }
}

#[test]
fn two_paths_that_differ_only_in_case_are_refused_because_a_bundle_is_carried_between_systems() {
    let f = fixture("orig-case");
    let draft = OriginalDraft {
        members: vec![
            MemberDraft::new("Img.png", Role::Picture, b"1".to_vec()),
            MemberDraft::new("img.png", Role::Picture, b"2".to_vec()),
        ],
        ..one("x", b"")
    };

    let (kind, message) = refusal(f.store.add_original(&f.spec, draft).unwrap_err());

    assert_eq!(kind, "bad-original");
    assert!(message.contains("differing only in case"), "{message}");
}

#[test]
fn the_same_path_twice_and_a_missing_reference_that_is_a_member_are_refused() {
    let f = fixture("orig-dup");
    let twice = OriginalDraft {
        members: vec![
            MemberDraft::new("a.md", Role::Document, b"1".to_vec()),
            MemberDraft::new("a.md", Role::Document, b"2".to_vec()),
        ],
        ..one("x", b"")
    };
    let reference_is_member = OriginalDraft {
        missing: vec!["spec.md".to_string()],
        ..one("spec.md", b"x")
    };

    assert_eq!(
        refusal(f.store.add_original(&f.spec, twice).unwrap_err()).0,
        "bad-original"
    );
    assert_eq!(
        refusal(
            f.store
                .add_original(&f.spec, reference_is_member)
                .unwrap_err()
        )
        .0,
        "bad-original"
    );
}

#[test]
fn a_reference_with_no_member_is_kept_and_a_bad_one_is_refused() {
    let f = fixture("orig-missing");
    let added = f.add(OriginalDraft {
        missing: vec!["images/not-brought.png".to_string()],
        ..one("spec.md", b"x")
    });

    let manifest = f.store.read_original(&f.spec, &added.digest).unwrap();
    assert_eq!(manifest.missing, vec!["images/not-brought.png"]);

    let climbing = OriginalDraft {
        missing: vec!["../elsewhere.png".to_string()],
        ..one("spec.md", b"x")
    };
    assert_eq!(
        refusal(f.store.add_original(&f.spec, climbing).unwrap_err()).0,
        "bad-original"
    );
}

#[test]
fn a_rendition_says_what_it_was_made_from_and_what_drew_it() {
    let f = fixture("orig-rendition");
    let added = f.add(OriginalDraft {
        members: vec![
            MemberDraft::new("scan.tiff", Role::Picture, b"tiff".to_vec()),
            MemberDraft::rendition(
                "renditions/scan.png",
                b"png".to_vec(),
                "scan.tiff",
                "pdfium 7.0",
            ),
        ],
        ..one("x", b"")
    });

    let manifest = f.store.read_original(&f.spec, &added.digest).unwrap();

    let rendition = manifest.member("renditions/scan.png").unwrap();
    assert_eq!(rendition.role, Role::Rendition);
    assert_eq!(rendition.of, Some(Revision::of(b"tiff")));
    assert_eq!(rendition.renderer.as_deref(), Some("pdfium 7.0"));
    assert!(manifest.member("scan.tiff").unwrap().of.is_none());
}

#[test]
fn a_rendition_that_hides_what_the_original_holds_is_refused() {
    let f = fixture("orig-rendition-bad");
    let source = MemberDraft::new("scan.tiff", Role::Picture, b"tiff".to_vec());
    let cases: Vec<(&str, Vec<MemberDraft>)> = vec![
        (
            "no source",
            vec![
                source.clone(),
                MemberDraft {
                    of: None,
                    ..MemberDraft::rendition("r.png", b"p".to_vec(), "scan.tiff", "x")
                },
            ],
        ),
        (
            "a source that is not a member",
            vec![
                source.clone(),
                MemberDraft::rendition("r.png", b"p".to_vec(), "nope.tiff", "x"),
            ],
        ),
        (
            "a rendition of a rendition",
            vec![
                source.clone(),
                MemberDraft::rendition("r1.png", b"p1".to_vec(), "scan.tiff", "x"),
                MemberDraft::rendition("r2.png", b"p2".to_vec(), "r1.png", "x"),
            ],
        ),
        (
            "no renderer",
            vec![
                source.clone(),
                MemberDraft::rendition("r.png", b"p".to_vec(), "scan.tiff", ""),
            ],
        ),
        (
            "an original that says it was made from another member",
            vec![MemberDraft {
                of: Some("scan.tiff".to_string()),
                ..MemberDraft::new("other.png", Role::Picture, b"o".to_vec())
            }],
        ),
    ];
    for (what, members) in cases {
        let draft = OriginalDraft {
            members,
            ..one("x", b"")
        };
        assert_eq!(
            refusal(f.store.add_original(&f.spec, draft).expect_err(what)).0,
            "bad-original",
            "{what}"
        );
    }
}

#[test]
fn nothing_and_too_much_are_refused() {
    let f = fixture("orig-limits");
    let nothing = OriginalDraft {
        members: vec![],
        ..one("x", b"")
    };
    assert_eq!(
        refusal(f.store.add_original(&f.spec, nothing).unwrap_err()).0,
        "bad-original"
    );

    let many = OriginalDraft {
        members: (0..sce_specs::original::MAX_MEMBERS + 1)
            .map(|i| MemberDraft::new(format!("m{i}.txt"), Role::Attachment, vec![]))
            .collect(),
        ..one("x", b"")
    };
    assert_eq!(
        refusal(f.store.add_original(&f.spec, many).unwrap_err()).0,
        "bad-original"
    );

    let long_label = OriginalDraft {
        label: "x".repeat(201),
        ..one("a.md", b"x")
    };
    assert_eq!(
        refusal(f.store.add_original(&f.spec, long_label).unwrap_err()).0,
        "bad-original"
    );

    // The most that is allowed is allowed.
    let most = OriginalDraft {
        members: (0..sce_specs::original::MAX_MEMBERS)
            .map(|i| MemberDraft::new(format!("m{i}.txt"), Role::Attachment, vec![]))
            .collect(),
        ..one("x", b"")
    };
    f.store.add_original(&f.spec, most).unwrap();
    f.store
        .add_original(
            &f.spec,
            OriginalDraft {
                label: "x".repeat(200),
                ..one("a.md", b"x")
            },
        )
        .unwrap();
}

#[test]
fn a_member_past_the_limit_is_refused_and_not_cut() {
    let f = fixture("orig-big");
    let big = vec![0u8; sce_specs::original::MAX_MEMBER_BYTES + 1];

    let error = f
        .store
        .add_original(&f.spec, one("big.pdf", &big))
        .unwrap_err();

    assert!(matches!(error, StoreError::TooLarge { .. }), "{error:?}");
    // Nothing was kept of it.
    assert!(!f.dir().join("members").exists());
    assert!(f.store.list_originals(&f.spec).unwrap().is_empty());
}

#[test]
fn a_spec_that_has_no_original_lists_none_and_an_unknown_one_is_not_found() {
    let f = fixture("orig-none");
    assert!(f.store.list_originals(&f.spec).unwrap().is_empty());
    let digest = Revision::of(b"nothing");

    for error in [
        f.store.read_original(&f.spec, &digest).unwrap_err(),
        f.store
            .read_original_member(&f.spec, &digest, "a.md")
            .unwrap_err(),
    ] {
        assert!(matches!(error, StoreError::NotFound { .. }), "{error:?}");
    }
    let other = SpecId::parse("no-such-spec").unwrap();
    assert!(matches!(
        f.store.add_original(&other, one("a.md", b"x")).unwrap_err(),
        StoreError::NotFound { .. }
    ));
}

#[test]
fn a_member_that_is_not_in_the_original_is_not_found() {
    let f = fixture("orig-no-member");
    let added = f.add(one("spec.md", b"x"));

    let error = f
        .store
        .read_original_member(&f.spec, &added.digest, "other.md")
        .unwrap_err();

    assert!(matches!(error, StoreError::NotFound { .. }), "{error:?}");
}

#[test]
fn the_list_says_when_and_how_big_and_the_newest_is_first() {
    let f = fixture("orig-list");
    let first = f.add(OriginalDraft {
        label: "first".to_string(),
        ..one("a.md", b"12345")
    });
    let second = f.add(OriginalDraft {
        label: "second".to_string(),
        ..one("b.md", b"1")
    });

    let listed = f.store.list_originals(&f.spec).unwrap();

    assert_eq!(
        listed.iter().map(|e| e.digest.clone()).collect::<Vec<_>>(),
        vec![second.digest, first.digest]
    );
    assert_eq!(listed[1].label, "first");
    assert_eq!((listed[1].members, listed[1].bytes), (1, 5));
    assert_eq!(listed[0].at, sce_specs::clock::utc_timestamp(T0));
}

// ---------------------------------------------------------------- a store whose bytes are not there

#[test]
fn an_original_whose_bytes_were_lost_is_corrupt_and_is_not_read_as_a_bundle_with_a_hole() {
    let f = fixture("orig-lost");
    let added = f.add(one("spec.md", b"The lock opens."));
    let blob = f
        .dir()
        .join("members")
        .join(format!("{}.bin", Revision::of(b"The lock opens.").as_str()));
    fs::remove_file(&blob).unwrap();

    for error in [
        f.store
            .read_original(&f.spec, &added.digest)
            .map(|_| ())
            .unwrap_err(),
        f.store
            .read_original_member(&f.spec, &added.digest, "spec.md")
            .map(|_| ())
            .unwrap_err(),
    ] {
        assert!(matches!(error, StoreError::Corrupt { .. }), "{error:?}");
    }
}

#[test]
fn a_member_whose_bytes_changed_is_corrupt_even_at_the_same_size() {
    let f = fixture("orig-changed");
    let added = f.add(one("spec.md", b"abcd"));
    let blob = f
        .dir()
        .join("members")
        .join(format!("{}.bin", Revision::of(b"abcd").as_str()));
    fs::write(&blob, b"abce").unwrap();

    let error = f
        .store
        .read_original_member(&f.spec, &added.digest, "spec.md")
        .unwrap_err();

    assert!(matches!(error, StoreError::Corrupt { .. }), "{error:?}");
}

#[test]
fn a_member_cut_short_is_corrupt_on_the_read_of_the_original() {
    let f = fixture("orig-short");
    let added = f.add(one("spec.md", b"abcdef"));
    let blob = f
        .dir()
        .join("members")
        .join(format!("{}.bin", Revision::of(b"abcdef").as_str()));
    fs::write(&blob, b"abc").unwrap();

    let error = f.store.read_original(&f.spec, &added.digest).unwrap_err();

    assert!(matches!(error, StoreError::Corrupt { .. }), "{error:?}");
}

#[test]
fn a_manifest_that_was_edited_is_corrupt_because_its_name_is_the_hash_of_its_text() {
    let f = fixture("orig-edited");
    let added = f.add(one("spec.md", b"x"));
    let path = f
        .dir()
        .join("originals")
        .join(format!("{}.json", added.digest.as_str()));
    let text = fs::read_to_string(&path).unwrap();
    fs::write(&path, text.replace("\"spec\"", "\"edited\"")).unwrap();

    let error = f.store.read_original(&f.spec, &added.digest).unwrap_err();

    assert!(matches!(error, StoreError::Corrupt { .. }), "{error:?}");
}

#[test]
fn adding_bytes_over_a_blob_that_holds_other_bytes_is_refused_and_not_trusted_for_its_name() {
    let f = fixture("orig-poisoned");
    f.add(one("spec.md", b"first"));
    let blob = f
        .dir()
        .join("members")
        .join(format!("{}.bin", Revision::of(b"first").as_str()));
    fs::write(&blob, b"forged").unwrap();

    let error = f
        .store
        .add_original(&f.spec, one("other.md", b"first"))
        .unwrap_err();

    assert!(matches!(error, StoreError::Corrupt { .. }), "{error:?}");
}

#[test]
fn a_torn_end_of_the_log_is_not_an_entry_and_the_next_add_is_not_glued_to_it() {
    let f = fixture("orig-torn");
    let first = f.add(one("a.md", b"1"));
    let log = f.dir().join("originals.log");
    let mut text = fs::read_to_string(&log).unwrap();
    text.push_str("{\"digest\":\"half a li");
    fs::write(&log, text).unwrap();

    assert_eq!(f.store.list_originals(&f.spec).unwrap().len(), 1);
    let second = f.add(one("b.md", b"2"));

    let listed = f.store.list_originals(&f.spec).unwrap();
    assert_eq!(
        listed.iter().map(|e| e.digest.clone()).collect::<Vec<_>>(),
        vec![second.digest, first.digest]
    );
}

#[test]
fn a_removed_spec_takes_no_original_and_gives_none() {
    let f = fixture("orig-removed");
    let added = f.add(one("spec.md", b"x"));
    f.store.remove_spec(&f.spec).unwrap();

    for error in [
        f.store
            .add_original(&f.spec, one("b.md", b"y"))
            .map(|_| ())
            .unwrap_err(),
        f.store
            .read_original(&f.spec, &added.digest)
            .map(|_| ())
            .unwrap_err(),
        f.store.list_originals(&f.spec).map(|_| ()).unwrap_err(),
    ] {
        assert!(matches!(error, StoreError::NotFound { .. }), "{error:?}");
    }
}

// ---------------------------------------------------------------------------- what was read out

fn extraction_text(extractor: &str, marker: &str) -> String {
    json!({
        "v": 1, "extractor": extractor, "extractor_version": "1",
        "member": "spec.md", "blocks": [], "marker": marker,
    })
    .to_string()
}

#[test]
fn an_extraction_is_kept_as_given_named_by_what_it_holds_and_listed_for_its_original() {
    let f = fixture("ext-roundtrip");
    let original = f.add(one("spec.md", b"The lock opens.")).digest;
    let text = extraction_text("plain", "a");

    let saved = f
        .store
        .save_extraction(&f.spec, &original, "spec.md", &text)
        .unwrap();
    let read = f.store.read_extraction(&f.spec, &saved.digest).unwrap();
    let listed = f.store.list_extractions(&f.spec, Some(&original)).unwrap();

    assert!(saved.created);
    assert_eq!(saved.digest, Revision::of(text.as_bytes()));
    assert_eq!(read, text);
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].member, "spec.md");
    assert_eq!(
        (
            listed[0].extractor.as_str(),
            listed[0].extractor_version.as_str()
        ),
        ("plain", "1")
    );
    assert_eq!(listed[0].original, original);
}

#[test]
fn the_same_extraction_again_is_the_same_and_another_extractor_is_another() {
    let f = fixture("ext-twice");
    let original = f.add(one("spec.md", b"x")).digest;
    let plain = extraction_text("plain", "a");
    let rich = extraction_text("rich", "b");

    let first = f
        .store
        .save_extraction(&f.spec, &original, "spec.md", &plain)
        .unwrap();
    let again = f
        .store
        .save_extraction(&f.spec, &original, "spec.md", &plain)
        .unwrap();
    let other = f
        .store
        .save_extraction(&f.spec, &original, "spec.md", &rich)
        .unwrap();

    assert_eq!(first.digest, again.digest);
    assert!(first.created && !again.created && other.created);
    assert_ne!(first.digest, other.digest);
    assert_eq!(f.store.list_extractions(&f.spec, None).unwrap().len(), 2);
}

#[test]
fn the_list_of_extractions_is_of_one_original_when_it_is_asked_for_one() {
    let f = fixture("ext-filter");
    let a = f.add(one("a.md", b"a")).digest;
    let b = f.add(one("b.md", b"b")).digest;
    f.store
        .save_extraction(&f.spec, &a, "a.md", &extraction_text("plain", "1"))
        .unwrap();
    f.store
        .save_extraction(&f.spec, &b, "b.md", &extraction_text("plain", "2"))
        .unwrap();

    assert_eq!(
        f.store.list_extractions(&f.spec, Some(&a)).unwrap().len(),
        1
    );
    assert_eq!(
        f.store.list_extractions(&f.spec, Some(&b)).unwrap().len(),
        1
    );
    assert_eq!(f.store.list_extractions(&f.spec, None).unwrap().len(), 2);
    assert!(f
        .store
        .list_extractions(&f.spec, Some(&Revision::of(b"none")))
        .unwrap()
        .is_empty());
}

#[test]
fn what_is_not_an_extraction_is_refused_and_a_minimal_one_is_not() {
    let f = fixture("ext-bad");
    let original = f.add(one("spec.md", b"x")).digest;
    for bad in [
        "not json",
        "[]",
        "7",
        "null",
        r#"{"extractor":"e","extractor_version":"1"}"#,
        r#"{"v":"1","extractor":"e","extractor_version":"1"}"#,
        r#"{"v":1,"extractor_version":"1"}"#,
        r#"{"v":1,"extractor":"","extractor_version":"1"}"#,
        r#"{"v":1,"extractor":"e"}"#,
        r#"{"v":1,"extractor":5,"extractor_version":"1"}"#,
    ] {
        let error = f
            .store
            .save_extraction(&f.spec, &original, "spec.md", bad)
            .expect_err(bad);
        assert_eq!(refusal(error).0, "bad-original", "{bad}");
    }
    f.store
        .save_extraction(
            &f.spec,
            &original,
            "spec.md",
            r#"{"v":1,"extractor":"e","extractor_version":"1"}"#,
        )
        .unwrap();
}

#[test]
fn an_extraction_is_of_an_original_and_a_member_that_are_there() {
    let f = fixture("ext-of");
    let original = f.add(one("spec.md", b"x")).digest;
    let text = extraction_text("plain", "a");

    let no_member = f
        .store
        .save_extraction(&f.spec, &original, "other.md", &text)
        .unwrap_err();
    let no_original = f
        .store
        .save_extraction(&f.spec, &Revision::of(b"none"), "spec.md", &text)
        .unwrap_err();

    assert_eq!(refusal(no_member).0, "bad-original");
    assert!(
        matches!(no_original, StoreError::NotFound { .. }),
        "{no_original:?}"
    );
}

#[test]
fn an_extraction_past_the_limit_is_refused_and_one_at_it_is_kept() {
    let f = fixture("ext-big");
    let original = f.add(one("spec.md", b"x")).digest;
    let head = r#"{"v":1,"extractor":"e","extractor_version":"1","pad":""#;
    let tail = r#""}"#;
    let limit = sce_specs::original::MAX_EXTRACTION_BYTES;
    let at = format!(
        "{head}{}{tail}",
        "a".repeat(limit - head.len() - tail.len())
    );
    let over = format!("{at}  ");

    assert_eq!(at.len(), limit);
    f.store
        .save_extraction(&f.spec, &original, "spec.md", &at)
        .unwrap();
    let error = f
        .store
        .save_extraction(&f.spec, &original, "spec.md", &over)
        .unwrap_err();
    assert!(matches!(error, StoreError::TooLarge { .. }), "{error:?}");
}

#[test]
fn an_extraction_another_spec_saved_is_not_this_specs_to_read() {
    let f = fixture("ext-other-spec");
    let other = f.store.create_spec("Window blind").unwrap().id;
    let original = f.add(one("spec.md", b"x")).digest;
    let text = extraction_text("plain", "a");
    let saved = f
        .store
        .save_extraction(&f.spec, &original, "spec.md", &text)
        .unwrap();

    let error = f.store.read_extraction(&other, &saved.digest).unwrap_err();

    assert!(matches!(error, StoreError::NotFound { .. }), "{error:?}");
}

#[test]
fn an_extraction_whose_file_was_changed_is_corrupt() {
    let f = fixture("ext-changed");
    let original = f.add(one("spec.md", b"x")).digest;
    let saved = f
        .store
        .save_extraction(
            &f.spec,
            &original,
            "spec.md",
            &extraction_text("plain", "a"),
        )
        .unwrap();
    let path = f
        .dir()
        .join("extractions")
        .join(format!("{}.json", saved.digest.as_str()));
    fs::write(&path, extraction_text("plain", "changed")).unwrap();

    let error = f.store.read_extraction(&f.spec, &saved.digest).unwrap_err();

    assert!(matches!(error, StoreError::Corrupt { .. }), "{error:?}");
}

// ---------------------------------------------------------------------------------- the commands

fn run(f: &Fixture, name: &str, args: Value) -> Result<Value, sce_specs::CommandError> {
    call(&f.store, &FakeRenderer, name, args)
}

fn data(bytes: &[u8]) -> String {
    // Written out here so that the test does not lean on the code it is holding.
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        out.push(A[(n >> 18) as usize & 63] as char);
        out.push(A[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            A[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            A[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

#[test]
fn the_commands_add_read_and_list_an_original_and_hand_a_member_back_as_it_was() {
    let f = fixture("orig-commands");
    let bytes: Vec<u8> = (0..=255).collect();

    let added = run(
        &f,
        "add_original",
        json!({
            "id": f.spec.as_str(), "label": "from the customer",
            "members": [
                {"path": "spec.pdf", "role": "document", "data": data(&bytes)},
                {"path": "r/p1.png", "role": "rendition", "data": data(b"png"),
                 "of": "spec.pdf", "renderer": "pdfium 7.0"},
            ],
            "missing": ["figures/lost.png"],
        }),
    )
    .unwrap();
    let digest = added["digest"].as_str().unwrap().to_string();
    let read = run(
        &f,
        "read_original",
        json!({"id": f.spec.as_str(), "original": digest}),
    )
    .unwrap();
    let member = run(
        &f,
        "read_original_member",
        json!({"id": f.spec.as_str(), "original": digest, "path": "spec.pdf"}),
    )
    .unwrap();
    let listed = run(&f, "list_originals", json!({"id": f.spec.as_str()})).unwrap();

    assert_eq!(added["created"], true);
    assert_eq!(added["members"], 2);
    assert_eq!(read["label"], "from the customer");
    assert_eq!(read["missing"], json!(["figures/lost.png"]));
    assert_eq!(read["bytes"], 259);
    // The members come back in path order, whatever order they were given in.
    assert_eq!(read["members"][0]["path"], "r/p1.png");
    assert_eq!(read["members"][0]["role"], "rendition");
    assert_eq!(read["members"][0]["renderer"], "pdfium 7.0");
    assert_eq!(read["members"][1]["path"], "spec.pdf");
    assert_eq!(member["size"], 256);
    assert_eq!(member["data"], data(&bytes));
    assert_eq!(listed["originals"][0]["digest"], digest);
}

#[test]
fn a_member_that_is_not_base64_is_a_bad_request_naming_it_and_nothing_is_kept() {
    let f = fixture("orig-commands-b64");

    let error = run(
        &f,
        "add_original",
        json!({"id": f.spec.as_str(), "members": [{"path": "a.md", "role": "document", "data": "not base64!"}]}),
    )
    .unwrap_err();

    assert_eq!(error.kind, "bad-request");
    assert!(error.message.contains("`a.md`"), "{}", error.message);
    assert!(f.store.list_originals(&f.spec).unwrap().is_empty());
}

#[test]
fn an_argument_nothing_reads_and_a_role_nobody_defined_are_refused() {
    let f = fixture("orig-commands-args");
    let member = json!({"path": "a.md", "role": "document", "data": data(b"x")});

    let extra = run(
        &f,
        "add_original",
        json!({"id": f.spec.as_str(), "members": [member.clone()], "colour": "red"}),
    )
    .unwrap_err();
    let role = run(
        &f,
        "add_original",
        json!({"id": f.spec.as_str(), "members": [{"path": "a.md", "role": "folder", "data": ""}]}),
    )
    .unwrap_err();
    let in_member = run(
        &f,
        "add_original",
        json!({"id": f.spec.as_str(), "members": [{"path": "a.md", "role": "document", "data": "", "extra": 1}]}),
    )
    .unwrap_err();

    for error in [extra, role, in_member] {
        assert_eq!(error.kind, "bad-request");
    }
}

#[test]
fn the_refusals_of_the_store_reach_the_caller_by_their_words() {
    let f = fixture("orig-commands-refusals");
    let climbing = run(
        &f,
        "add_original",
        json!({"id": f.spec.as_str(), "members": [{"path": "../a.md", "role": "document", "data": data(b"x")}]}),
    )
    .unwrap_err();
    let unknown = run(
        &f,
        "read_original",
        json!({"id": f.spec.as_str(), "original": Revision::of(b"x").as_str()}),
    )
    .unwrap_err();
    let not_a_revision = run(
        &f,
        "read_original",
        json!({"id": f.spec.as_str(), "original": "abc"}),
    )
    .unwrap_err();

    assert_eq!(climbing.kind, "bad-original");
    assert_eq!(unknown.kind, "not-found");
    assert_eq!(not_a_revision.kind, "bad-request");
}

#[test]
fn the_commands_save_read_and_list_an_extraction() {
    let f = fixture("ext-commands");
    let digest = run(
        &f,
        "add_original",
        json!({"id": f.spec.as_str(), "members": [{"path": "spec.md", "role": "document", "data": data(b"x")}]}),
    )
    .unwrap()["digest"]
        .as_str()
        .unwrap()
        .to_string();
    let text = extraction_text("plain", "a");

    let saved = run(
        &f,
        "save_extraction",
        json!({"id": f.spec.as_str(), "original": digest, "member": "spec.md", "text": text}),
    )
    .unwrap();
    let read = run(
        &f,
        "read_extraction",
        json!({"id": f.spec.as_str(), "digest": saved["digest"]}),
    )
    .unwrap();
    let listed = run(
        &f,
        "list_extractions",
        json!({"id": f.spec.as_str(), "original": digest}),
    )
    .unwrap();

    assert_eq!(saved["created"], true);
    assert_eq!(read["text"], text);
    assert_eq!(listed["extractions"][0]["digest"], saved["digest"]);
    assert_eq!(listed["extractions"][0]["extractor"], "plain");
}

#[test]
fn the_new_commands_are_listed_and_the_version_moved_so_a_screen_can_tell_a_core_that_has_them() {
    let f = fixture("orig-commands-describe");

    let described = run(&f, "describe", json!({})).unwrap();

    assert_eq!(
        described["command_set_version"],
        sce_specs::COMMAND_SET_VERSION
    );
    let commands: Vec<&str> = described["commands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap())
        .collect();
    for name in [
        "add_original",
        "read_original",
        "list_originals",
        "read_original_member",
        "save_extraction",
        "read_extraction",
        "list_extractions",
    ] {
        assert!(commands.contains(&name), "{name}");
    }
}
