// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Unicode pin names the data the lock holds (docs/adr/0158, D6): every `icu_*` crate
//! `Cargo.lock` resolves is at the ICU4X version `unicode/pin.toml` names, and every `icu_*`
//! crate the workspace requires, it requires exactly at that version. So the Unicode a build
//! reads moves only with the pin, never with a refresh of the lock.

const PIN: &str = include_str!("../../unicode/pin.toml");
const LOCK: &str = include_str!("../../Cargo.lock");
const WORKSPACE: &str = include_str!("../../Cargo.toml");

fn table(text: &str, file: &str) -> toml::Table {
    text.parse()
        .unwrap_or_else(|error| panic!("{file} is TOML: {error}"))
}

#[test]
fn the_unicode_pin_names_the_data_the_lock_holds() {
    let pin = table(PIN, "unicode/pin.toml");
    let versions = pin["versions"]
        .as_table()
        .expect("unicode/pin.toml has a [versions] table");
    for name in ["icu4x", "unicode", "cldr"] {
        let version = versions
            .get(name)
            .and_then(toml::Value::as_str)
            .unwrap_or_else(|| panic!("unicode/pin.toml names the {name} version"));
        let parts: Vec<&str> = version.split('.').collect();
        assert!(
            parts.len() == 3 && parts.iter().all(|part| part.parse::<u32>().is_ok()),
            "the {name} version {version:?} is x.y.z"
        );
    }
    let icu4x = versions["icu4x"].as_str().expect("checked above");

    let lock = table(LOCK, "Cargo.lock");
    let packages = lock["package"]
        .as_array()
        .expect("Cargo.lock lists packages");
    let mut locked = Vec::new();
    for package in packages {
        let name = package["name"].as_str().expect("a package has a name");
        if name.starts_with("icu_") {
            let version = package["version"]
                .as_str()
                .expect("a package has a version");
            locked.push((name, version));
        }
    }
    assert!(
        locked.iter().any(|(name, _)| *name == "icu_properties"),
        "Cargo.lock holds icu_properties: {locked:?}"
    );
    for (name, version) in &locked {
        assert_eq!(
            *version, icu4x,
            "Cargo.lock holds {name} {version}, and unicode/pin.toml names ICU4X {icu4x}"
        );
    }

    let workspace = table(WORKSPACE, "Cargo.toml");
    let required = workspace["workspace"]["dependencies"]
        .as_table()
        .expect("the root Cargo.toml has [workspace.dependencies]");
    let exactly = format!("={icu4x}");
    let mut pinned = 0;
    for (name, requirement) in required {
        if name.starts_with("icu_") {
            let version = requirement
                .as_str()
                .or_else(|| requirement.get("version").and_then(toml::Value::as_str));
            assert_eq!(
                version,
                Some(exactly.as_str()),
                "[workspace.dependencies] requires {name} as {requirement}, not exactly at the pin"
            );
            pinned += 1;
        }
    }
    assert!(
        pinned > 0,
        "[workspace.dependencies] requires no icu_* crate"
    );
}
