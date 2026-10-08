// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a document an AI wrote may name: the other documents it was handed, and no file.
//!
//! The checker opens what an import names, and says so in its diagnostics: a file that is there
//! reads differently from one that is not, and what the checker finds in it can be quoted in the
//! words it refuses with. A refusal goes back to the AI when a draft is asked for again, so a
//! document that names a file of the machine is a way for a specification that says to do so to
//! read it. The authoring server refuses such a document when the AI hands it over to be checked
//! (`tools/authoring/sce_author/mcp.py`); this is the same rule for the answer the AI ends with,
//! which the application hands to the checker without that server.
//!
//! The two are written twice, in two languages, and kept one rule by the cases both are held to
//! (`tools/authoring/tests/fixtures/file_references.json`).

use roxmltree::{Document, ParsingOptions};

use crate::model_set::check_name;

/// The namespace of `xml:base`, which changes what a name resolves against.
const XML_NAMESPACE: &str = "http://www.w3.org/XML/1998/namespace";

/// An attribute that makes the checker open a file, on whatever element it is: `sce:import src`,
/// `sce:driver href`, `script src`, `data src`, `invoke src`, an XInclude's `href`, `sce:use
/// template`. Taken by the attribute's name and not by a list of elements, so that an element
/// nobody listed is not a way round it.
///
/// An early refusal in words the model can act on, and a list of the places somebody thought of:
/// `template` was not on it when a document made the checker read a file by it. What holds is the
/// checker, which opens a file only inside the folder the application runs it in
/// (`figures::run_bounded`, `SCE_FILE_ROOT`), whatever names the file.
const FILE_ATTRIBUTES: [&str; 3] = ["src", "href", "template"];

/// The namespace of SCE's own attributes (`sce:candidates`).
const SCE_NAMESPACE: &str = "http://sce.dev/ext";

/// An attribute of SCE's namespace whose value lists documents, separated by white space, each of
/// which the checker may open: `sce:candidates`, the documents an `<invoke srcexpr>` may choose
/// among, which the generator copies into what it writes. Only in the namespace: the `candidates`
/// of an `<sce:unresolved>` has no namespace and lists kinds, and `sce:unresolved-candidates` is
/// another name.
const FILE_LISTS: [&str; 1] = ["candidates"];

/// Refuse `text`, the document `name` of a draft, when it would send the checker to a file that is
/// not one of the documents of the draft: a name that is not one plain name, a base to resolve
/// names against, a document type, or text that cannot be read as XML here. The last is refused
/// and not passed on, because a document this reader refuses and the checker's reader accepts
/// would be a way past it.
pub(crate) fn refuse_file_access(name: &str, text: &str) -> Result<(), String> {
    let options = ParsingOptions {
        allow_dtd: false,
        ..ParsingOptions::default()
    };
    let document = Document::parse_with_options(text, options)
        .map_err(|e| format!("`{name}` cannot be read as XML without a document type: {e}"))?;
    for element in document.descendants().filter(|node| node.is_element()) {
        for attribute in element.attributes() {
            if attribute.namespace() == Some(XML_NAMESPACE) && attribute.name() == "base" {
                return Err(format!(
                    "`{name}` changes the base its references resolve against (`xml:base`)"
                ));
            }
            let lists_files = attribute.namespace() == Some(SCE_NAMESPACE)
                && FILE_LISTS.contains(&attribute.name());
            let names_a_file_wrongly = if lists_files {
                attribute
                    .value()
                    .split_whitespace()
                    .any(|listed| check_name(listed).is_err())
            } else {
                FILE_ATTRIBUTES.contains(&attribute.name())
                    && check_name(attribute.value()).is_err()
            };
            if names_a_file_wrongly {
                return Err(format!(
                    "`{name}` has a `{}` that is not the plain name of another document of the \
                     draft: a reference may name a companion and no file",
                    attribute.name()
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The cases the authoring server's reader is held to as well: one rule in two languages.
    const CASES: &str = include_str!("../../tools/authoring/tests/fixtures/file_references.json");

    #[test]
    fn the_cases_both_readers_are_held_to_are_refused_and_accepted_as_they_say() {
        let cases: Vec<serde_json::Value> = serde_json::from_str(CASES).unwrap();
        assert!(cases.len() >= 20, "the shared cases were lost");
        for case in &cases {
            let name = case["name"].as_str().unwrap();
            let refused = case["refused"].as_bool().unwrap();
            let said = refuse_file_access("main.scxml", case["text"].as_str().unwrap());
            assert_eq!(said.is_err(), refused, "{name}: {said:?}");
        }
        // Both sides of the line are in them, so that neither a reader that refuses everything
        // nor one that refuses nothing passes.
        assert!(cases.iter().any(|c| c["refused"] == true));
        assert!(cases.iter().any(|c| c["refused"] == false));
    }

    #[test]
    fn a_refusal_says_which_document_and_what_it_names_without_repeating_the_path() {
        let said = refuse_file_access(
            "levels.scxml",
            r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"><x src="/private/secret.scxml"/></scxml>"#,
        )
        .unwrap_err();

        assert!(said.contains("levels.scxml"), "{said}");
        assert!(said.contains("`src`"), "{said}");
        // The answer goes back to the AI when a draft is asked for again: it is not a way to
        // echo what the document asked for.
        assert!(!said.contains("/private"), "{said}");
    }
}
