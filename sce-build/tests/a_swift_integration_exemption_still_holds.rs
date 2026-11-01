// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What the Swift integration channel says it cannot do is still true.
//!
//! `common::integration_channels` lets a channel decline a stem with a reason, and says the entry is
//! deleted when the missing piece is built. Nothing deletes it but the author, and an exemption
//! that is true no longer is a stem whose coverage the repository reports as missing while it is
//! there: the first silent way for a debt register to outlive its debt. The Swift channel declines
//! stems for two reasons, and each is asked here.
//!
//! - A stem the generator refuses by name (`SWIFT_REFUSED`) is asked of `generate`, which must
//!   still answer exit status 7 (generate/unsupported-feature). A stem it now writes is an entry to
//!   delete, and the sweep (`scripts/regen_swift_integration.sh`) writes it by itself.
//! - A stem whose driver is not written yet (`SWIFT_DRIVER_PENDING`) must have no driver: the slice
//!   that writes one deletes the entry, and an entry beside a driver says the stem is unasserted
//!   while a test asserts it.

mod common;

use std::path::PathBuf;
use std::process::Command;

use tempfile::tempdir;

use common::integration_channels::{
    integration_stems, sites, Channel, Role, SWIFT_DRIVER_PENDING, SWIFT_REFUSED,
};
use common::repository::root as repo_root;

fn sce_codegen_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sce-codegen"))
}

#[test]
fn a_stem_swift_refuses_by_name_is_still_refused() {
    let root = repo_root();
    let mut written: Vec<String> = Vec::new();
    for (stem, _) in SWIFT_REFUSED {
        let document = root.join(format!("integration_resources/{stem}/{stem}.scxml"));
        // `generate` and not `check`: `check -l swift` does not see every refusal (the transition
        // function passes it), and it is the file a refusal withholds that the sweep goes by.
        let out = tempdir().expect("a directory to generate into");
        let status = Command::new(sce_codegen_bin())
            .arg("generate")
            .arg(&document)
            .args(["-l", "swift", "--error-format=json", "-o"])
            .arg(out.path())
            .current_dir(&root)
            .output()
            .expect("run sce-codegen generate")
            .status
            .code();
        if status != Some(7) {
            written.push(format!(
                "{stem}: `generate -l swift` answered {status:?}, not 7"
            ));
        }
    }
    assert!(
        written.is_empty(),
        "an exemption of the Swift integration channel is stale ({}):\n  {}\n\nThe generator no \
         longer refuses the stem by name, so the channel can host it: delete its entry from \
         `SWIFT_REFUSED`, and write its driver (`regen_swift_integration.sh` already writes the \
         machine).",
        written.len(),
        written.join("\n  "),
    );
}

#[test]
fn a_stem_whose_swift_driver_is_pending_has_none() {
    let root = repo_root();
    let driver_of = |stem: &str| -> String {
        sites(stem)
            .into_iter()
            .find(|site| site.role == Role::Driver(Channel::Swift))
            .expect("every stem has a Swift driver site")
            .path
    };

    let stems = integration_stems(&root);
    let mut stale: Vec<String> = Vec::new();
    for pending in SWIFT_DRIVER_PENDING {
        assert!(
            stems.iter().any(|stem| stem == pending),
            "`SWIFT_DRIVER_PENDING` names `{pending}`, which is not a stem under \
             integration_resources/"
        );
        let driver = driver_of(pending);
        if root.join(&driver).exists() {
            stale.push(format!("{pending}: {driver} exists"));
        }
    }
    assert!(
        stale.is_empty(),
        "a stem is listed as without a Swift driver and has one ({}):\n  {}\n\nDelete its entry \
         from `SWIFT_DRIVER_PENDING`.",
        stale.len(),
        stale.join("\n  "),
    );
}
