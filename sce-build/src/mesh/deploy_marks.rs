// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Where a deployment wrote each key (docs/adr/0106, item #76).
//!
//! A refusal of a deployment names the key it is about by its path
//! ([`DeployKeyPath`], from [`crate::mesh::error::DeployError::key_path`]), and
//! a key path is a name, not a coordinate a consumer can open
//! (SCE_ERROR_CONTRACT.md section 2.2). `serde_yaml_ng`, which reads the
//! deployment, keeps no marks, so the same text is read a second time by a YAML
//! event parser that reports where each event begins, and the path is looked up
//! in the index that reading builds ([`DeployMarks`]). The second reading runs
//! on the failure path only: a deployment that is accepted is never read twice.
//!
//! A path that names a key the author did not write (a refusal of a missing
//! field) stands on the deepest key of it that was written: the entry the field
//! is missing from. A refusal that carries `actual` stands where `actual` is
//! written, when the path's key or value, or one of its parents', is it
//! (SCE_ERROR_CONTRACT.md section 2.2: `col` is where `actual` begins).

use yaml_rust2::parser::{Event, Parser};
use yaml_rust2::scanner::{Marker, TScalarStyle};

/// One step of a [`DeployKeyPath`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeySeg {
    /// The mapping key of this name.
    Key(String),
    /// Any mapping key: the device a machine is declared under, which a
    /// refusal that names the machine does not name.
    AnyKey,
    /// The item of a sequence of mappings whose `field` is `value` (a
    /// queryable by its `name`).
    Item { field: &'static str, value: String },
    /// The item of a sequence of scalars that is `value` (a machine in a
    /// partition's `machines:` list).
    Value(String),
}

/// The key of a deployment a refusal is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeployKeyPath {
    /// A path of steps from the document's root.
    Keys(Vec<KeySeg>),
    /// A path a refusal already spells with dots (`topology.d.machines.m.
    /// bindings.t.retires`), matched against the written keys joined the same
    /// way, so a name that holds a dot is still found.
    Dotted(String),
}

impl DeployKeyPath {
    /// The top-level key `key`.
    pub fn root(key: &str) -> Self {
        Self::Keys(vec![KeySeg::Key(key.to_string())])
    }

    /// `topology.<device>.machines.<machine>`, whatever the device.
    pub fn machine(machine: &str) -> Self {
        Self::Keys(vec![
            KeySeg::Key("topology".to_string()),
            KeySeg::AnyKey,
            KeySeg::Key("machines".to_string()),
            KeySeg::Key(machine.to_string()),
        ])
    }

    /// `topology.<device>`.
    pub fn device(device: &str) -> Self {
        Self::root("topology").key(device)
    }

    /// `partitions.<partition>`.
    pub fn partition(partition: &str) -> Self {
        Self::root("partitions").key(partition)
    }

    /// `topology.<device>.machines.<machine>.links.<link>`.
    pub fn link(machine: &str, link: &str) -> Self {
        Self::machine(machine).key("links").key(link)
    }

    /// This path, then the key `key`.
    pub fn key(self, key: &str) -> Self {
        self.then(KeySeg::Key(key.to_string()))
    }

    /// This path, then the item of its sequence whose `field` is `value`.
    pub fn item(self, field: &'static str, value: &str) -> Self {
        self.then(KeySeg::Item {
            field,
            value: value.to_string(),
        })
    }

    /// This path, then the item of its sequence that is `value`.
    pub fn value(self, value: &str) -> Self {
        self.then(KeySeg::Value(value.to_string()))
    }

    fn then(self, seg: KeySeg) -> Self {
        match self {
            Self::Keys(mut segs) => {
                segs.push(seg);
                Self::Keys(segs)
            }
            // A dotted path is whole as the refusal spelled it.
            Self::Dotted(dotted) => Self::Dotted(dotted),
        }
    }
}

/// A place in the deployment's text: 1-based row, 1-based column counted in
/// characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mark {
    pub line: u32,
    pub col: u32,
}

impl Mark {
    fn of(marker: Marker, quoted: bool) -> Self {
        let to_u32 = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
        Self {
            line: to_u32(marker.line()),
            // The parser counts columns from 0, and a quoted scalar from its
            // quote; the text the author wrote begins one further.
            col: to_u32(marker.col() + 1 + usize::from(quoted)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PathSeg {
    Key(String),
    Index(usize),
}

/// One key (or sequence item) the deployment wrote.
#[derive(Debug, Clone)]
struct Entry {
    path: Vec<PathSeg>,
    /// Where the key (or the item) begins.
    at: Mark,
    /// The value, when it is a scalar, and where it begins.
    value: Option<(String, Mark)>,
}

/// Every key the deployment wrote, in document order, with where it and its
/// scalar value begin.
#[derive(Debug, Clone, Default)]
pub struct DeployMarks {
    entries: Vec<Entry>,
}

impl DeployMarks {
    /// Read `text`, or `None` when it is no YAML the event parser reads (a
    /// syntax refusal, which serde's own reading has already placed in its
    /// message).
    pub fn read(text: &str) -> Option<Self> {
        let mut parser = Parser::new_from_str(text);
        let mut events = Vec::new();
        loop {
            let (event, marker) = parser.next_token().ok()?;
            let end = event == Event::StreamEnd;
            events.push((event, marker));
            if end {
                break;
            }
        }
        let mut walk = Walk {
            events: &events,
            at: 0,
            entries: Vec::new(),
        };
        while matches!(
            walk.events.get(walk.at),
            Some((Event::StreamStart | Event::DocumentStart, _))
        ) {
            walk.at += 1;
        }
        // The first document is the deployment.
        walk.node(Some(&[]));
        Some(Self {
            entries: walk.entries,
        })
    }

    /// Where the refusal of `path` stands: the deepest key of it the
    /// deployment wrote, or the place of `actual` among that key and its
    /// parents. `None` when not even the path's first key was written.
    pub fn locate(&self, path: &DeployKeyPath, actual: Option<&str>) -> Option<Mark> {
        let deepest = match path {
            DeployKeyPath::Keys(segs) => (1..=segs.len()).rev().find_map(|len| {
                self.entries
                    .iter()
                    .find(|entry| entry.path.len() == len && self.matches(entry, &segs[..len]))
            })?,
            DeployKeyPath::Dotted(dotted) => self
                .entries
                .iter()
                .find(|entry| joined(&entry.path) == *dotted)?,
        };
        if let Some(actual) = actual {
            // The deepest entry first, then its parents, nearest first.
            for len in (1..=deepest.path.len()).rev() {
                let Some(entry) = self
                    .entries
                    .iter()
                    .find(|entry| entry.path[..] == deepest.path[..len])
                else {
                    continue;
                };
                if let Some((value, at)) = &entry.value {
                    if value == actual {
                        return Some(*at);
                    }
                }
                if matches!(entry.path.last(), Some(PathSeg::Key(key)) if key == actual) {
                    return Some(entry.at);
                }
            }
        }
        Some(deepest.at)
    }

    fn matches(&self, entry: &Entry, pattern: &[KeySeg]) -> bool {
        pattern
            .iter()
            .zip(&entry.path)
            .enumerate()
            .all(|(depth, (want, have))| match (want, have) {
                (KeySeg::Key(want), PathSeg::Key(have)) => want == have,
                (KeySeg::AnyKey, PathSeg::Key(_)) => true,
                (KeySeg::Item { field, value }, PathSeg::Index(_)) => {
                    let mut named = entry.path[..=depth].to_vec();
                    named.push(PathSeg::Key((*field).to_string()));
                    self.scalar_at(&named) == Some(value.as_str())
                }
                (KeySeg::Value(value), PathSeg::Index(_)) => {
                    self.scalar_at(&entry.path[..=depth]) == Some(value.as_str())
                }
                _ => false,
            })
    }

    fn scalar_at(&self, path: &[PathSeg]) -> Option<&str> {
        self.entries
            .iter()
            .find(|entry| entry.path[..] == *path)
            .and_then(|entry| entry.value.as_ref())
            .map(|(value, _)| value.as_str())
    }
}

/// The keys of `path` joined by dots, its sequence items by their index.
fn joined(path: &[PathSeg]) -> String {
    path.iter()
        .map(|seg| match seg {
            PathSeg::Key(key) => key.clone(),
            PathSeg::Index(index) => index.to_string(),
        })
        .collect::<Vec<_>>()
        .join(".")
}

/// One pass over the parser's events, recording every key under the root.
struct Walk<'e> {
    events: &'e [(Event, Marker)],
    at: usize,
    entries: Vec<Entry>,
}

impl Walk<'_> {
    /// Read one node. Its keys are recorded under `path` when there is one (a
    /// key that is itself a mapping or a sequence records nothing). Answers
    /// the node's value and where it begins when it is a scalar.
    fn node(&mut self, path: Option<&[PathSeg]>) -> Option<(String, Mark)> {
        let events = self.events;
        let (event, marker) = events.get(self.at)?;
        self.at += 1;
        match event {
            Event::Scalar(value, style, ..) => {
                let quoted = matches!(
                    style,
                    TScalarStyle::SingleQuoted | TScalarStyle::DoubleQuoted
                );
                Some((value.clone(), Mark::of(*marker, quoted)))
            }
            Event::MappingStart(..) => {
                while !matches!(events.get(self.at), None | Some((Event::MappingEnd, _))) {
                    let key = self.node(None);
                    match (path, key) {
                        (Some(parent), Some((key, at))) => {
                            let mut child = parent.to_vec();
                            child.push(PathSeg::Key(key));
                            self.record(child, at);
                        }
                        _ => {
                            self.node(None);
                        }
                    }
                }
                self.at += 1;
                None
            }
            Event::SequenceStart(..) => {
                let mut index = 0;
                while !matches!(events.get(self.at), None | Some((Event::SequenceEnd, _))) {
                    match (path, events.get(self.at)) {
                        (Some(parent), Some((_, item_marker))) => {
                            let mut child = parent.to_vec();
                            child.push(PathSeg::Index(index));
                            let at = Mark::of(*item_marker, false);
                            self.record(child, at);
                        }
                        _ => {
                            self.node(None);
                        }
                    }
                    index += 1;
                }
                self.at += 1;
                None
            }
            _ => None,
        }
    }

    /// Record the entry `path` at `at`, then read its value under it.
    fn record(&mut self, path: Vec<PathSeg>, at: Mark) {
        let index = self.entries.len();
        self.entries.push(Entry {
            path: path.clone(),
            at,
            value: None,
        });
        let value = self.node(Some(&path));
        self.entries[index].value = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "\
version: \"1.0\"
topology:
  edge:
    machines:
      door:
        source: door.scxml
        links:
          udp_scout:
            bind: \"224.0.0.224:7446\"
            driver: tokio_udpp
        queryables:
          - name: status
            key: door/status
partitions:
  p1:
    machines: [door, lamp]
";

    fn marks() -> DeployMarks {
        DeployMarks::read(TEXT).expect("the text is YAML")
    }

    #[test]
    fn a_key_path_finds_the_row_and_column_of_its_last_key() {
        let at = marks()
            .locate(
                &DeployKeyPath::link("door", "udp_scout").key("driver"),
                None,
            )
            .expect("the driver key is written");
        assert_eq!(at, Mark { line: 10, col: 13 });
    }

    #[test]
    fn an_actual_that_is_the_value_stands_where_the_value_begins() {
        let at = marks()
            .locate(
                &DeployKeyPath::link("door", "udp_scout").key("driver"),
                Some("tokio_udpp"),
            )
            .expect("the driver key is written");
        assert_eq!(at, Mark { line: 10, col: 21 });
    }

    #[test]
    fn a_quoted_value_begins_after_its_quote() {
        let at = marks()
            .locate(&DeployKeyPath::root("version"), Some("1.0"))
            .expect("the version is written");
        assert_eq!(at, Mark { line: 1, col: 11 });
    }

    #[test]
    fn a_key_not_written_stands_on_the_deepest_key_that_is() {
        let at = marks()
            .locate(
                &DeployKeyPath::link("door", "udp_scout").key("mtu_bytes"),
                None,
            )
            .expect("the link is written");
        assert_eq!(at, Mark { line: 8, col: 11 });
    }

    #[test]
    fn an_actual_that_names_a_parent_stands_on_that_parent() {
        let at = marks()
            .locate(
                &DeployKeyPath::machine("door").key("scheduler"),
                Some("door"),
            )
            .expect("the machine is written");
        assert_eq!(at, Mark { line: 5, col: 7 });
    }

    #[test]
    fn a_sequence_item_is_found_by_its_name_and_by_its_value() {
        let named = marks()
            .locate(
                &DeployKeyPath::machine("door")
                    .key("queryables")
                    .item("name", "status")
                    .key("key"),
                None,
            )
            .expect("the queryable's key is written");
        assert_eq!(named, Mark { line: 13, col: 13 });
        let listed = marks()
            .locate(
                &DeployKeyPath::partition("p1").key("machines").value("lamp"),
                Some("lamp"),
            )
            .expect("the partition lists the machine");
        assert_eq!(listed, Mark { line: 16, col: 22 });
    }

    #[test]
    fn a_dotted_path_is_matched_as_the_refusal_spells_it() {
        let at = marks()
            .locate(
                &DeployKeyPath::Dotted("topology.edge.machines.door.source".to_string()),
                None,
            )
            .expect("the source key is written");
        assert_eq!(at, Mark { line: 6, col: 9 });
    }

    #[test]
    fn a_path_whose_first_key_is_not_written_is_not_placed() {
        assert_eq!(
            marks().locate(&DeployKeyPath::root("discovery"), None),
            None
        );
    }
}
