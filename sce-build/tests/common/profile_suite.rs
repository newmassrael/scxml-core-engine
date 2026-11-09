// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! A test suite of the test profile a test writes into a directory of its own
//! (docs/SCE_TEST_PROFILE.md §11): one test case whose main test component
//! receives, on a port mapped to the test system, a message of one record,
//! so a test can vary the record and what it reaches and read what the TTCN-3
//! export makes of it.
//!
//! The documents a test writes to be refused are written here, at run time,
//! and never committed: a committed fixture holding what the export refuses
//! would be a document every other sweep of the fixtures reads too.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

/// The root element's namespaces and version, which every document writes.
const ROOT: &str =
    "xmlns=\"http://www.w3.org/2005/07/scxml\" xmlns:sce=\"http://sce.dev/ext\" version=\"1.0\"";

/// A document: `<scxml>` with `attributes` beside the namespaces, holding
/// `body`.
pub fn document(attributes: &str, body: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<scxml {ROOT} {attributes}>\n{body}</scxml>\n"
    )
}

/// Write `text` to `<stem>.scxml` in `dir`, and answer its path.
pub fn write(dir: &Path, stem: &str, text: &str) -> PathBuf {
    let path = dir.join(format!("{stem}.scxml"));
    std::fs::write(&path, text).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
    path
}

/// An event schema `<stem>.scxml` of the event `event`, its `imports` and the
/// `<data>` lines of `fields`, written to `dir`.
pub fn write_event_schema(dir: &Path, stem: &str, event: &str, imports: &str, fields: &str) {
    write(
        dir,
        stem,
        &document(
            &format!("sce:kind=\"event-schema\" name=\"{stem}\" sce:event-name=\"{event}\""),
            &format!("{imports}  <datamodel>\n{fields}  </datamodel>\n"),
        ),
    );
}

/// Write a suite whose message holds a value of the enum `outcome.scxml`,
/// whose first variant is named `variant`, and answer the suite's path.
pub fn write_suite_of_enum(dir: &Path, variant: &str) -> PathBuf {
    write(
        dir,
        "outcome",
        &document(
            "sce:kind=\"enum\" name=\"outcome\" sce:underlying-type=\"uint8\"",
            &format!(
                "  <datamodel>\n    <data id=\"variants\">\n      <sce:variant name=\"{variant}\" \
                 value=\"0\"/>\n      <sce:variant name=\"stays\" value=\"1\"/>\n    </data>\n  \
                 </datamodel>\n"
            ),
        ),
    );
    write_event_schema(
        dir,
        "report",
        "probe.report",
        "  <sce:import kind=\"enum\" src=\"outcome.scxml\" as=\"Outcome\"/>\n",
        "    <data id=\"outcome\" sce:type=\"enum:Outcome\" sce:direction=\"in\"/>\n",
    );
    write_suite(dir, "report", false)
}

/// Write the suite `probe_suite.scxml` to `dir`, with its test case, its
/// configuration, its main test component and its port type, and answer the
/// suite's path. The port type carries the record of `<message>.scxml`, which
/// the caller writes, into the component; the component receives it with the
/// template `probe_match.scxml`, which the caller writes too, when `matched`,
/// and receives any message otherwise.
pub fn write_suite(dir: &Path, message: &str, matched: bool) -> PathBuf {
    write(
        dir,
        "probe_port",
        &document(
            "sce:kind=\"test-port-type\" name=\"probe_port\"",
            &format!(
                "  <sce:import kind=\"event-schema\" src=\"{message}.scxml\" as=\"Message\"/>\n  \
                 <sce:message type=\"Message\" direction=\"in\"/>\n"
            ),
        ),
    );
    let (template_import, template_use) = if matched {
        (
            "  <sce:import kind=\"test-template\" src=\"probe_match.scxml\" as=\"Match\"/>\n",
            " sce:test-template=\"Match\"",
        )
    } else {
        ("", "")
    };
    write(
        dir,
        "probe_tester",
        &document(
            "sce:kind=\"test-component\" name=\"probe_tester\" datamodel=\"sce-static\" \
             initial=\"waiting\"",
            &format!(
                "  <sce:import kind=\"test-port-type\" src=\"probe_port.scxml\" as=\"ProbePort\"/>\n\
                 {template_import}  <sce:test-port id=\"sut\" type=\"ProbePort\"/>\n  \
                 <state id=\"waiting\">\n    <transition sce:receive=\"sut\"{template_use} \
                 target=\"done\">\n      <sce:setverdict value=\"pass\"/>\n    </transition>\n  \
                 </state>\n  <final id=\"done\"/>\n"
            ),
        ),
    );
    write(
        dir,
        "probe_configuration",
        &document(
            "sce:kind=\"test-configuration\" name=\"probe_configuration\"",
            "  <sce:import kind=\"test-component\" src=\"probe_tester.scxml\" as=\"Tester\"/>\n  \
             <sce:import kind=\"test-port-type\" src=\"probe_port.scxml\" as=\"ProbePort\"/>\n  \
             <sce:mtc type=\"Tester\"/>\n  <sce:system-port id=\"sut\" type=\"ProbePort\"/>\n  \
             <sce:map component=\"mtc\" port=\"sut\" system-port=\"sut\"/>\n",
        ),
    );
    write(
        dir,
        "probe_case",
        &document(
            "sce:kind=\"test-case\" name=\"probe_case\" sce:configuration=\"Configuration\"",
            "  <sce:import kind=\"test-configuration\" src=\"probe_configuration.scxml\" \
             as=\"Configuration\"/>\n",
        ),
    );
    write(
        dir,
        "probe_suite",
        &document(
            "sce:kind=\"test-suite\" name=\"probe_suite\"",
            "  <sce:import kind=\"test-case\" src=\"probe_case.scxml\" as=\"ProbeCase\"/>\n  \
             <sce:execute test-case=\"ProbeCase\"/>\n",
        ),
    )
}
