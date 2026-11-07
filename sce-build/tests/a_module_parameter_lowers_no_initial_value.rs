// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A test component's module parameter (docs/SCE_TEST_PROFILE.md §11.5,
//! §11.8) has no `expr`: its value is the one the suite declares, which the
//! component reads into its variable before the machine starts. The static
//! lowering, written once for every backend, so lowers no initial value for it
//! on any backend: the field is declared with its type alone and marked as a
//! module parameter, and every other variable keeps the value its `expr`
//! lowers to. A module parameter of a type no suite declares one of is refused
//! by name.

use std::path::Path;

use sce_build::forge::error::GenerateError;
use sce_build::forge::static_lowering::{lower_cpp, lower_kotlin, StaticLowering};
use sce_build::model::SCXMLModel;
use sce_build::parser::SCXMLParser;

/// A test component whose eventless transition reads the module parameter
/// `limit`, beside the variable `seen`, which starts at its `expr`.
const COMPONENT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" sce:kind="test-component" name="parameterized"
       datamodel="sce-static" initial="checking">
  <datamodel>
    <data id="limit" sce:type="uint32" sce:module-parameter="true"/>
    <data id="seen" sce:type="uint32" expr="3"/>
  </datamodel>
  <state id="checking">
    <transition cond="limit === 7" target="done"/>
  </state>
  <final id="done"/>
</scxml>
"#;

/// `text`, written to `<dir>/<name>.scxml` and read as the generator reads a
/// document, from its file.
fn parsed(dir: &Path, name: &str, text: &str) -> SCXMLModel {
    let path = dir.join(format!("{name}.scxml"));
    std::fs::write(&path, text).expect("the component is written");
    SCXMLParser::new()
        .parse_file(path.to_str().expect("a UTF-8 path"))
        .unwrap_or_else(|e| panic!("{name} parses: {e:?}"))
}

type Lower = fn(&mut SCXMLModel, &str) -> Result<StaticLowering, GenerateError>;

/// Kotlin and C++, the backends whose static lowering every route that
/// sweeps the tree's `sce-static` documents runs.
const BACKENDS: [(&str, Lower); 2] = [("Kotlin", lower_kotlin), ("C++", lower_cpp)];

/// §11.5: the module parameter's field has no initial value lowered and is
/// marked as one; the other variable starts at its `expr`.
#[test]
fn a_module_parameter_is_declared_with_no_initial_value_on_every_backend() {
    let dir = tempfile::tempdir().expect("a directory");
    for (backend, lower) in BACKENDS {
        let mut model = parsed(dir.path(), "parameterized", COMPONENT);
        let lowering = lower(&mut model, "Parameterized")
            .unwrap_or_else(|e| panic!("{backend} lowers a module parameter: {e:?}"));
        let limit = lowering
            .fields
            .iter()
            .find(|field| field.id == "limit")
            .unwrap_or_else(|| panic!("{backend} declares the module parameter's field"));
        assert!(limit.module_parameter, "{backend}");
        assert_eq!(
            limit.init, "",
            "{backend}: no initial value is lowered for it"
        );
        assert!(
            !limit.seeded,
            "{backend}: a parent's <param> never seeds it"
        );
        let seen = lowering
            .fields
            .iter()
            .find(|field| field.id == "seen")
            .unwrap_or_else(|| panic!("{backend} declares `seen`"));
        assert!(!seen.module_parameter, "{backend}");
        assert!(seen.init.contains('3'), "{backend}: {}", seen.init);
    }
}

/// §11.8: a module parameter of a type no suite declares one of is refused by
/// name, on every backend alike.
#[test]
fn a_module_parameter_of_a_list_is_refused_by_name() {
    let dir = tempfile::tempdir().expect("a directory");
    let component = COMPONENT.replace(
        r#"<data id="limit" sce:type="uint32" sce:module-parameter="true"/>"#,
        r#"<data id="limit" sce:type="uint32" sce:module-parameter="true"/>
    <data id="limits" sce:type="list&lt;uint32&gt;[2]" sce:module-parameter="true"/>"#,
    );
    for (backend, lower) in BACKENDS {
        let mut model = parsed(dir.path(), "listed", &component);
        let Err(GenerateError::UnsupportedFeature { detail, .. }) = lower(&mut model, "Listed")
        else {
            panic!("{backend} lowered a module parameter of a list");
        };
        assert!(
            detail.contains("<data id=\"limits\" sce:module-parameter=\"true\">"),
            "{backend}: {detail}"
        );
        assert!(
            detail.contains("a module parameter is of a scalar, string[N] or bytes[N] type"),
            "{backend}: {detail}"
        );
    }
}
