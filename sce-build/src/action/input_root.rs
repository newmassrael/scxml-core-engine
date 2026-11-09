// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The Merkle tree of an action's inputs, in REAPI's form: one [`Directory`]
//! message per directory, its files and subdirectories each sorted by name,
//! and a directory named by the digest of its encoding. The digest of the root
//! is the `input_root_digest` of an [`super::reapi::Action`], so two input sets
//! give one key exactly when they hold the same files at the same paths.

use std::collections::BTreeMap;

use super::reapi::{Digest, Directory, DirectoryNode, FileNode};

/// A file of an input root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputFile {
    pub digest: Digest,
    pub is_executable: bool,
}

/// An input root: its digest and the encoded directories it is made of,
/// root last, each with its digest, for a store that has to hold them.
#[derive(Clone, Debug)]
pub struct InputRoot {
    pub digest: Digest,
    pub directories: Vec<(Digest, Vec<u8>)>,
}

/// Why a set of paths is not a tree.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InputRootError {
    /// A path that is empty, absolute, or holds an empty, `.` or `..`
    /// component: it names no single place under the root.
    #[error("`{0}` is not a relative path of plain components")]
    NotRelative(String),
    /// One name is both a file and a directory.
    #[error("`{0}` is both a file and the directory of another input")]
    FileAndDirectory(String),
}

enum Node {
    File(InputFile),
    Dir(BTreeMap<String, Node>),
}

/// The input root holding `files`, keyed by their paths relative to the root
/// with `/` between components.
pub fn input_root(files: &BTreeMap<String, InputFile>) -> Result<InputRoot, InputRootError> {
    let mut root: BTreeMap<String, Node> = BTreeMap::new();
    for (path, file) in files {
        let components: Vec<&str> = path.split('/').collect();
        if path.is_empty()
            || components
                .iter()
                .any(|c| c.is_empty() || *c == "." || *c == "..")
        {
            return Err(InputRootError::NotRelative(path.clone()));
        }
        let (name, parents) = components.split_last().expect("a path has a component");
        let mut level = &mut root;
        for (depth, parent) in parents.iter().enumerate() {
            let node = level
                .entry((*parent).to_string())
                .or_insert_with(|| Node::Dir(BTreeMap::new()));
            level = match node {
                Node::Dir(children) => children,
                Node::File(_) => {
                    return Err(InputRootError::FileAndDirectory(
                        components[..=depth].join("/"),
                    ))
                }
            };
        }
        // A file of this name cannot be here already (the map lists a path
        // once); a directory of this name was made by an earlier, longer path.
        if level.contains_key(*name) {
            return Err(InputRootError::FileAndDirectory(path.clone()));
        }
        level.insert((*name).to_string(), Node::File(file.clone()));
    }
    let mut directories = Vec::new();
    let digest = encode(&root, &mut directories);
    Ok(InputRoot {
        digest,
        directories,
    })
}

fn encode(level: &BTreeMap<String, Node>, out: &mut Vec<(Digest, Vec<u8>)>) -> Digest {
    let mut directory = Directory::default();
    for (name, node) in level {
        match node {
            Node::File(file) => directory.files.push(FileNode {
                name: name.clone(),
                digest: Some(file.digest.clone()),
                is_executable: file.is_executable,
            }),
            Node::Dir(children) => {
                let digest = encode(children, out);
                directory.directories.push(DirectoryNode {
                    name: name.clone(),
                    digest: Some(digest),
                });
            }
        }
    }
    let (digest, bytes) = Digest::of_message(&directory);
    out.push((digest.clone(), bytes));
    digest
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(content: &[u8]) -> InputFile {
        InputFile {
            digest: Digest::of(content),
            is_executable: false,
        }
    }

    fn files(entries: &[(&str, &[u8])]) -> BTreeMap<String, InputFile> {
        entries
            .iter()
            .map(|(p, c)| ((*p).to_string(), file(c)))
            .collect()
    }

    #[test]
    fn the_root_is_a_function_of_paths_and_contents() {
        let a = input_root(&files(&[("x/a.scxml", b"1"), ("t/b.jinja2", b"2")])).unwrap();
        let b = input_root(&files(&[("t/b.jinja2", b"2"), ("x/a.scxml", b"1")])).unwrap();
        assert_eq!(a.digest, b.digest);

        let moved = input_root(&files(&[("y/a.scxml", b"1"), ("t/b.jinja2", b"2")])).unwrap();
        assert_ne!(
            a.digest, moved.digest,
            "a moved input left the key where it was"
        );
        let edited = input_root(&files(&[("x/a.scxml", b"1!"), ("t/b.jinja2", b"2")])).unwrap();
        assert_ne!(
            a.digest, edited.digest,
            "an edited input left the key where it was"
        );
    }

    /// The empty directory's digest is the encoding of an empty message, the
    /// empty blob: the value every REAPI implementation uses for "no inputs".
    #[test]
    fn no_inputs_is_the_empty_directory() {
        let empty = input_root(&BTreeMap::new()).unwrap();
        assert_eq!(empty.digest, Digest::of(b""));
    }

    #[test]
    fn every_directory_is_listed_with_the_root_last() {
        let root = input_root(&files(&[("a/b/c.txt", b"c"), ("a/d.txt", b"d")])).unwrap();
        assert_eq!(root.directories.len(), 3, "a/b, a and the root");
        assert_eq!(root.directories.last().unwrap().0, root.digest);
        for (digest, bytes) in &root.directories {
            assert_eq!(Digest::of(bytes), *digest);
        }
    }

    #[test]
    fn a_path_that_leaves_the_root_is_refused() {
        for bad in ["", "/abs", "a//b", "a/./b", "../up", "a/.."] {
            assert_eq!(
                input_root(&files(&[(bad, b"x")])).unwrap_err(),
                InputRootError::NotRelative(bad.to_string()),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn a_name_that_is_both_a_file_and_a_directory_is_refused() {
        let err = input_root(&files(&[("a", b"x"), ("a/b", b"y")])).unwrap_err();
        assert_eq!(err, InputRootError::FileAndDirectory("a".into()));
    }
}
