// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Which files a document may make the generator open.
//!
//! A document names files: an import, an include, a template, a driver header, a script, a child
//! to invoke. The generator opens what it names and says what it found, and so a document handed
//! over by someone who does not own the machine is a way to ask the machine about its files: that
//! one is there, that it is a template whose root is `<state>`, a line of it quoted in the words a
//! refusal is written in.
//!
//! [`ROOT_ENV`] names the one folder a caller lets the generator open files in, and then every
//! file a document names has to be inside it. A file that is not is answered as one that is not
//! there, in the words a missing file is answered in, so that it cannot be told from one: a
//! refusal for being outside would itself say the file exists.
//!
//! It is the places that open a file that hold the rule, and not a reading of the document's
//! attributes before it is read: a list of the attributes that name a file is a list of the
//! places somebody thought of. Every place that opens a file a document names calls [`exists`] or
//! [`read_to_string`] (`tests/document_files_pass_through_confine.rs` holds the modules that read
//! documents to it), and a place that is added without them fails that test.
//!
//! No folder named is the generator as it always was: a person who runs it on their own
//! documents has no reason to be held to a folder.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::{fs, io};

/// The environment variable that names the folder files may be opened in.
pub const ROOT_ENV: &str = "SCE_FILE_ROOT";

/// Where a file a document names may be opened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Root {
    /// No folder is named: any file, as before.
    Anywhere,
    /// Only a file inside this folder (which is canonical).
    Within(PathBuf),
    /// A folder was asked for and cannot be used, so no file may be opened: a confinement that
    /// is lost when its folder is mistyped is no confinement.
    Nowhere,
}

impl Root {
    /// The root a value of [`ROOT_ENV`] names. Any value is a request to confine, an empty one
    /// too: leaving the variable out is the way to ask for none.
    pub fn named(value: Option<&OsStr>) -> Root {
        match value {
            None => Root::Anywhere,
            Some(folder) => match fs::canonicalize(folder) {
                Ok(real) if real.is_dir() => Root::Within(real),
                _ => Root::Nowhere,
            },
        }
    }

    /// Whether `path` may be opened. The path is read as the file system reads it, so that
    /// `..`, a link inside the folder that points out of it, and a path spelled another way all
    /// come to what they are; a path that is not there is not inside anything.
    pub fn permits(&self, path: &Path) -> bool {
        match self {
            Root::Anywhere => true,
            Root::Nowhere => false,
            Root::Within(root) => fs::canonicalize(path).is_ok_and(|real| real.starts_with(root)),
        }
    }
}

fn from_environment() -> &'static Root {
    static ROOT: OnceLock<Root> = OnceLock::new();
    ROOT.get_or_init(|| Root::named(std::env::var_os(ROOT_ENV).as_deref()))
}

#[cfg(test)]
thread_local! {
    static FOR_TEST: std::cell::RefCell<Option<Root>> = const { std::cell::RefCell::new(None) };
}

/// Run `body` with `root` in force on this thread, for a test of a place that opens a file:
/// the environment is read once for the process, and a test cannot change it.
#[cfg(test)]
pub(crate) fn within_for_test<T>(root: Root, body: impl FnOnce() -> T) -> T {
    struct Restore;
    impl Drop for Restore {
        fn drop(&mut self) {
            FOR_TEST.with(|slot| *slot.borrow_mut() = None);
        }
    }
    FOR_TEST.with(|slot| *slot.borrow_mut() = Some(root));
    let _restore = Restore;
    body()
}

/// Whether a file a document names may be opened here.
pub fn permits(path: &Path) -> bool {
    #[cfg(test)]
    if let Some(root) = FOR_TEST.with(|slot| slot.borrow().clone()) {
        return root.permits(path);
    }
    from_environment().permits(path)
}

/// Whether the file a document names is there, for a place that looks before it opens. A file
/// that may not be opened is not.
pub fn exists(path: &Path) -> bool {
    permits(path) && path.exists()
}

/// What a file that may not be opened is answered with: the error the file system gives for one
/// that is not there, in the same words. Error number 2 is that on Unix (`ENOENT`) and on Windows
/// (`ERROR_FILE_NOT_FOUND`), and a place that prints the error it got (`{e}`) then says the same
/// of a file it was refused as of one that is not there; `ErrorKind::NotFound` alone prints
/// another sentence.
fn not_there() -> io::Error {
    io::Error::from_raw_os_error(2)
}

/// The text of the file a document names. A file that may not be opened is answered as one that
/// is not there.
pub fn read_to_string(path: &Path) -> io::Result<String> {
    if permits(path) {
        fs::read_to_string(path)
    } else {
        Err(not_there())
    }
}

/// The bytes of the file a document names, for a place that copies it and does not read it as
/// text. A file that may not be opened is answered as one that is not there.
pub fn read(path: &Path) -> io::Result<Vec<u8>> {
    if permits(path) {
        fs::read(path)
    } else {
        Err(not_there())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// A folder with a file in it, and a folder beside it with another.
    fn two_folders() -> (TempDir, TempDir) {
        let inside = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();
        fs::write(inside.path().join("here.xml"), "inside").unwrap();
        fs::write(outside.path().join("there.xml"), "outside").unwrap();
        (inside, outside)
    }

    fn within(folder: &Path) -> Root {
        Root::named(Some(folder.as_os_str()))
    }

    #[test]
    fn no_folder_named_is_any_file_as_it_always_was() {
        let (_, outside) = two_folders();

        assert_eq!(Root::named(None), Root::Anywhere);
        assert!(Root::Anywhere.permits(&outside.path().join("there.xml")));
    }

    #[test]
    fn a_folder_that_cannot_be_used_lets_nothing_through_and_is_not_no_folder() {
        let (inside, _) = two_folders();
        let a_file = inside.path().join("here.xml");

        for value in ["", "/no/such/folder"] {
            assert_eq!(
                Root::named(Some(OsStr::new(value))),
                Root::Nowhere,
                "{value:?}"
            );
        }
        // A file is not a folder to open files in.
        assert_eq!(Root::named(Some(a_file.as_os_str())), Root::Nowhere);
        assert!(!Root::Nowhere.permits(&a_file));
    }

    #[test]
    fn a_file_inside_the_folder_is_let_through_and_one_beside_it_is_not() {
        let (inside, outside) = two_folders();
        let root = within(inside.path());

        assert!(root.permits(&inside.path().join("here.xml")));
        assert!(!root.permits(&outside.path().join("there.xml")));
    }

    #[test]
    fn a_path_that_climbs_out_of_the_folder_is_the_file_it_comes_to() {
        let (inside, outside) = two_folders();
        let root = within(inside.path());
        let climbing = inside
            .path()
            .join("..")
            .join(outside.path().file_name().unwrap())
            .join("there.xml");

        assert!(climbing.exists(), "the test's own path is a real one");
        assert!(!root.permits(&climbing));
    }

    #[test]
    #[cfg(unix)]
    fn a_link_inside_the_folder_that_points_out_of_it_is_the_file_it_points_to() {
        let (inside, outside) = two_folders();
        let link = inside.path().join("link.xml");
        std::os::unix::fs::symlink(outside.path().join("there.xml"), &link).unwrap();

        assert!(link.exists());
        assert!(!within(inside.path()).permits(&link));
    }

    #[test]
    fn a_file_that_is_not_there_is_not_inside_anything() {
        let (inside, _) = two_folders();

        assert!(!within(inside.path()).permits(&inside.path().join("absent.xml")));
    }

    #[test]
    fn a_file_outside_is_answered_as_a_file_that_is_not_there() {
        let (inside, outside) = two_folders();
        let there = outside.path().join("there.xml");
        let absent = outside.path().join("absent.xml");

        within_for_test(within(inside.path()), || {
            assert!(!exists(&there));
            let refused = read_to_string(&there).unwrap_err().kind();
            let missing = read_to_string(&absent).unwrap_err().kind();
            // Neither says that the file is there.
            assert_eq!(refused, io::ErrorKind::NotFound);
            assert_eq!(refused, missing);
            // And one inside is read as it is.
            assert_eq!(
                read_to_string(&inside.path().join("here.xml")).unwrap(),
                "inside"
            );
        });
    }

    #[test]
    fn a_refusal_reads_in_words_as_the_file_system_says_a_file_is_not_there() {
        // A place that prints the error it got (`{e}`) would otherwise say "entity not found" for
        // a file it was refused and "No such file or directory" for one that is not there.
        let (inside, outside) = two_folders();

        within_for_test(within(inside.path()), || {
            let really_absent = inside.path().join("absent.xml");
            let missing_text = fs::read_to_string(&really_absent).unwrap_err().to_string();

            let refused_text = read_to_string(&outside.path().join("there.xml"))
                .unwrap_err()
                .to_string();
            let refused_bytes = read(&outside.path().join("there.xml"))
                .unwrap_err()
                .to_string();

            assert_eq!(refused_text, missing_text);
            assert_eq!(refused_bytes, missing_text);
        });
    }

    #[test]
    fn the_bytes_of_a_file_outside_are_answered_as_a_file_that_is_not_there() {
        let (inside, outside) = two_folders();
        let there = outside.path().join("there.xml");
        let absent = outside.path().join("absent.xml");

        within_for_test(within(inside.path()), || {
            let refused = read(&there).unwrap_err().kind();
            let missing = read(&absent).unwrap_err().kind();
            assert_eq!(refused, io::ErrorKind::NotFound);
            assert_eq!(refused, missing);
            assert_eq!(read(&inside.path().join("here.xml")).unwrap(), b"inside");
        });
    }

    #[test]
    fn the_root_a_test_names_is_gone_when_the_test_ends() {
        let (inside, outside) = two_folders();
        let there = outside.path().join("there.xml");

        within_for_test(within(inside.path()), || assert!(!permits(&there)));

        // Back to the environment's, which no test sets.
        assert!(permits(&there));
    }
}
