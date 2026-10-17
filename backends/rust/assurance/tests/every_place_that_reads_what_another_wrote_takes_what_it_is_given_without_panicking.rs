// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
//
// Every place the trust, relay and group key code reads what someone else wrote (the targets of
// `sce_rust_assurance::targets`) is given what it reads when it is true, and what a seeded mutator makes
// of it (a bit, a cut, a hole, a repeat, a number that does not fit, a value of another type). A place
// may refuse. It may not panic, and what it accepts it writes back as it read it. The same targets are
// fed by the coverage-guided fuzzer under `fuzz/`.

mod common;

use std::path::Path;

use common::fuzz;
use sce_rust_assurance::targets::{self, NAMES};

fn seeded(name: &str) {
    let mut target = targets::build(name);
    let corpus = target.corpus.clone();
    fuzz(name, &corpus, target.rounds, |input| target.feed(input));
}

macro_rules! each {
    ($($test:ident => $name:literal,)*) => {
        $(
            #[test]
            fn $test() {
                seeded($name);
            }
        )*

        /// The names the tests above are for, to be compared with what the registry says.
        const TESTED: &[&str] = &[$($name),*];
    };
}

each! {
    the_entries_of_the_membership_log_are_read_without_a_panic_and_written_back_as_read => "entries",
    checkpoints_are_read_without_a_panic_and_written_back_as_read => "checkpoints",
    settlements_are_read_without_a_panic_and_written_back_as_read => "settlements",
    the_encodings_under_them_read_what_they_wrote_and_nothing_that_is_not => "encoding",
    a_hub_is_given_operations_that_were_changed_and_never_panics => "operations-hub",
    a_member_is_given_operations_that_were_changed_and_never_panics => "operations-member",
    a_floor_that_was_changed_never_panics_a_host => "floors",
    a_store_text_that_was_changed_never_panics_a_host => "store",
    a_grant_of_a_key_that_was_changed_never_panics_a_host => "grant",
    a_copy_of_a_devices_secrets_is_read_or_refused_and_never_panics => "backup",
    the_record_of_an_audit_is_read_or_refused_and_never_panics => "audit",
    requests_are_read_without_a_panic_and_written_back_as_read => "requests",
    responses_are_read_without_a_panic_and_written_back_as_read => "responses",
    a_relay_is_asked_for_whatever_the_mutator_makes_of_every_request_and_is_still_a_relay => "relay",
    a_store_that_was_damaged_is_read_or_refused_and_never_panics_a_relay => "store-file",
    a_device_state_that_was_changed_is_refused_and_never_panics => "device-state",
    a_welcome_that_was_changed_makes_the_device_a_member_of_nothing => "welcome",
    a_commit_that_joins_and_was_changed_is_refused => "join-commit",
    a_commit_that_follows_and_was_changed_is_refused_and_moves_nothing => "apply-commit",
    a_key_package_that_was_changed_is_refused_by_the_administrators_device => "key-package",
    the_c_abi_reads_mutated_bytes_with_a_right_or_a_wrong_length_and_never_panics => "capi",
}

#[test]
fn every_target_has_a_test_and_a_file_for_the_coverage_guided_fuzzer() {
    let mut tested: Vec<&str> = TESTED.to_vec();
    let mut named: Vec<&str> = NAMES.to_vec();
    tested.sort_unstable();
    named.sort_unstable();
    assert_eq!(
        tested, named,
        "a target with no seeded test, or a test for none"
    );

    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("fuzz/fuzz_targets");
    for name in NAMES {
        let file = dir.join(format!("{}.rs", name.replace('-', "_")));
        let source = std::fs::read_to_string(&file)
            .unwrap_or_else(|error| panic!("{}: no file for the fuzzer: {error}", file.display()));
        assert!(
            source.contains(&format!("targets::feed(\"{name}\"")),
            "{} does not feed the target {name}",
            file.display()
        );
    }
    let files = std::fs::read_dir(&dir).unwrap().count();
    assert_eq!(
        files,
        NAMES.len(),
        "a file for the fuzzer that is no target"
    );
}
