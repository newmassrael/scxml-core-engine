// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The variants of a product line as the generation of one output for every
//! build reads them (ADR 0057 §9).
//!
//! Each distinct variant is parsed through its selection and prepared exactly
//! as the caller prepared the document, so that it is the model a generation
//! of that selection renders, and the backend renders each as it renders any
//! document. The output for every build is those renders merged
//! ([`crate::variant_merge`]); what this module adds is what the merge and its
//! check need of the product line:
//!
//! - [`Variants::selector_of`]: the predicate over a build's features that
//!   holds in the builds of a set of variants, spelled as small as the builds
//!   the product line tells apart allow;
//! - [`Variants::first_mismatch`]: the check that the merged output, resolved
//!   for each build, is what that build's variant generates alone.

use std::collections::{BTreeMap, BTreeSet};

use crate::feature_selector::Selector;
use crate::lift_validation::{first_difference, Lifted, Reference};
use crate::model::SCXMLModel;
use crate::parser::SCXMLParser;
use crate::product_line::ProductLine;
use crate::CompileError;

/// One distinct variant: the selector that picks it, the least configuration
/// that denotes it, which of the product line's distinct conditions hold in
/// it, and its model, prepared as the document was.
#[derive(Debug, Clone)]
pub struct Variant {
    pub selector: Selector,
    pub features: Vec<String>,
    /// For each of [`ProductLine::distinct_conditions`], whether it holds.
    pub valuation: Vec<bool>,
    pub model: SCXMLModel,
}

/// Every distinct variant of a product line.
#[derive(Debug, Clone)]
pub struct Variants {
    line: ProductLine,
    /// The selector of each distinct condition, in the order of a valuation.
    atoms: Vec<Selector>,
    variants: Vec<Variant>,
}

/// A build for which the output for every build is not what its variant
/// generates alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiftingMismatch {
    /// The features the build turns on, of those the conditions mention.
    pub features: Vec<String>,
    /// Where the two differ ([`first_difference`]).
    pub difference: String,
}

impl Variants {
    /// The distinct variants of `model`, the document at `scxml_path` as
    /// `parser` parsed it, each parsed through its selection by a parser
    /// `parser` makes and handed to `prepare`, which is to do to it what the
    /// caller did to the document — the analyzer, the host's declarations, the
    /// deploy — so that each is the model a generation of that selection
    /// renders. `None` for a document that carries no condition, which has
    /// one variant: itself.
    pub fn derive<E: From<CompileError>>(
        model: &SCXMLModel,
        scxml_path: &str,
        parser: &dyn Fn() -> SCXMLParser,
        prepare: &mut dyn FnMut(&mut SCXMLModel) -> Result<(), E>,
    ) -> Result<Option<Variants>, E> {
        let Some(line) = model.product_line.as_ref().filter(|line| line.is_gated()) else {
            return Ok(None);
        };
        let conditions = line.distinct_conditions();
        let mut variants = Vec::new();
        for configuration in line.variants() {
            let features: Vec<String> = line
                .model()
                .names_of(&configuration)
                .map(str::to_string)
                .collect();
            let mut variant = parser()
                .with_features(features.clone())
                .parse_file(scxml_path)?;
            prepare(&mut variant)?;
            variants.push(Variant {
                selector: line.variant_selector(&configuration),
                features,
                valuation: conditions
                    .iter()
                    .map(|condition| condition.holds_in(&configuration))
                    .collect(),
                model: variant,
            });
        }
        Ok(Some(Variants {
            atoms: conditions
                .iter()
                .map(|condition| Selector::of_condition(condition, line.model()))
                .collect(),
            line: line.clone(),
            variants,
        }))
    }

    pub fn variants(&self) -> &[Variant] {
        &self.variants
    }

    /// The product line the variants are of.
    pub fn product_line(&self) -> &ProductLine {
        &self.line
    }

    /// The selector of the builds of the variants in `set`, as an output read
    /// only in the builds of the variants in `within` asks it: it holds in
    /// every build of `set`, in no build of the rest of `within`, and says
    /// whatever is shortest of the builds that are of no variant in `within`
    /// (Quine and McCluskey's don't-cares). [`Selector::True`] when `set` is
    /// all of `within`.
    ///
    /// A variant is one valuation of the product line's distinct conditions,
    /// so the selector is a sum of products over the conditions, found by
    /// Espresso's two moves (Brayton et al., 1984): each valuation of `set` is
    /// expanded into the largest product that meets no valuation of the rest,
    /// conditions left out in turn, those a variant does not satisfy first —
    /// so a product names what holds rather than what does not where it can —
    /// and a product whose valuations the others cover is dropped. An item
    /// written for one element of the document then reads as that element's
    /// own condition, which is the spelling an author reads.
    pub fn selector_of(&self, set: &BTreeSet<usize>, within: &BTreeSet<usize>) -> Selector {
        if set.is_superset(within) {
            return Selector::True;
        }
        let on: Vec<&[bool]> = set
            .iter()
            .map(|&i| self.variants[i].valuation.as_slice())
            .collect();
        let off: Vec<&[bool]> = within
            .difference(set)
            .map(|&i| self.variants[i].valuation.as_slice())
            .collect();
        let products = minimal_cover(&on, &off);
        Selector::any(products.into_iter().map(|product| {
            Selector::all(
                product
                    .into_iter()
                    .enumerate()
                    .filter_map(|(atom, literal)| {
                        literal.map(|holds| {
                            if holds {
                                self.atoms[atom].clone()
                            } else {
                                !self.atoms[atom].clone()
                            }
                        })
                    }),
            )
        }))
    }

    /// The first build for which the files `lifted`, each Rust file resolved
    /// for that build, are not the files its variant generates alone —
    /// `references[i]` being those of `self.variants()[i]` — or `None` when
    /// there is none (ADR 0057 §9). Files are `(name, text)`; a file that is
    /// not Rust source is compared as text, since nothing in it is selected.
    ///
    /// Every build the product line tells apart is judged
    /// ([`ProductLine::configurations`]), not one per variant: a predicate the
    /// lifted machine writes may tell apart builds that build one variant, and
    /// a check of one of them would say nothing of the other. The files are
    /// resolved once for each pair of a variant and a valuation of their
    /// predicates ([`Lifted::valuation`]), since two builds alike in both
    /// resolve them alike and are held to one reference. An error is a Rust
    /// file `syn` could not read.
    pub fn first_mismatch(
        &self,
        lifted: &[(String, String)],
        references: &[Vec<(String, String)>],
    ) -> Result<Option<LiftingMismatch>, String> {
        let model = self.line.model();
        let is_feature = |name: &str| model.lookup(name).is_some();
        let lifted: BTreeMap<&str, LiftedFile> = lifted
            .iter()
            .map(|(name, text)| Ok((name.as_str(), LiftedFile::read(name, text, &is_feature)?)))
            .collect::<Result<_, String>>()?;
        let references: Vec<BTreeMap<&str, ReferenceFile>> = references
            .iter()
            .map(|files| {
                files
                    .iter()
                    .map(|(name, text)| Ok((name.as_str(), ReferenceFile::read(name, text)?)))
                    .collect::<Result<_, String>>()
            })
            .collect::<Result<_, String>>()?;
        let mut judged: BTreeSet<(usize, Vec<bool>)> = BTreeSet::new();
        for configuration in self.line.configurations() {
            let on: BTreeSet<&str> = model.names_of(&configuration).collect();
            let answer = |name: &str| is_feature(name).then(|| on.contains(name));
            let variant = self
                .variants
                .iter()
                .position(|variant| variant.selector.holds_with(&|name| on.contains(name)))
                .expect("the selectors of the variants partition the builds");
            let valuation: Vec<bool> = lifted
                .values()
                .flat_map(|file| file.valuation(&answer))
                .collect();
            if !judged.insert((variant, valuation)) {
                continue;
            }
            let reference = &references[variant];
            let difference = if !lifted.keys().eq(reference.keys()) {
                Some(format!(
                    "the generation writes the files {:?} where the variant writes {:?}",
                    lifted.keys().collect::<Vec<_>>(),
                    reference.keys().collect::<Vec<_>>()
                ))
            } else {
                lifted.iter().find_map(|(name, file)| {
                    file.difference(&answer, &reference[name])
                        .map(|difference| format!("in `{name}`, {difference}"))
                })
            };
            if let Some(difference) = difference {
                return Ok(Some(LiftingMismatch {
                    features: on.iter().map(|name| name.to_string()).collect(),
                    difference,
                }));
            }
        }
        Ok(None)
    }
}

/// A product over the distinct conditions: for each, the value it must have,
/// or `None` where the product does not mention it.
type Product = Vec<Option<bool>>;

fn product_holds(product: &Product, valuation: &[bool]) -> bool {
    product
        .iter()
        .zip(valuation)
        .all(|(literal, value)| literal.is_none_or(|wanted| wanted == *value))
}

/// A set of products that holds in every valuation of `on` and in none of
/// `off` (see [`Variants::selector_of`]).
fn minimal_cover(on: &[&[bool]], off: &[&[bool]]) -> Vec<Product> {
    let mut products: Vec<Product> = Vec::new();
    for valuation in on {
        if products
            .iter()
            .any(|product| product_holds(product, valuation))
        {
            continue;
        }
        let mut product: Product = valuation.iter().map(|value| Some(*value)).collect();
        let mut order: Vec<usize> = (0..valuation.len()).collect();
        order.sort_by_key(|&atom| valuation[atom]);
        for atom in order {
            let kept = product[atom].take();
            if off.iter().any(|excluded| product_holds(&product, excluded)) {
                product[atom] = kept;
            }
        }
        products.push(product);
    }
    let mut index = 0;
    while index < products.len() {
        let others_cover = on.iter().all(|valuation| {
            !product_holds(&products[index], valuation)
                || products
                    .iter()
                    .enumerate()
                    .any(|(other, product)| other != index && product_holds(product, valuation))
        });
        if others_cover {
            products.remove(index);
        } else {
            index += 1;
        }
    }
    products
}

/// A file of the lifted generation: Rust source, resolved for each build, or
/// any other text, which no build selects within.
enum LiftedFile {
    Rust(Lifted),
    Text(String),
}

impl LiftedFile {
    fn read(name: &str, text: &str, is_feature: &dyn Fn(&str) -> bool) -> Result<Self, String> {
        Ok(if is_rust(name) {
            LiftedFile::Rust(Lifted::parse(text, is_feature).map_err(|e| format!("{name}: {e}"))?)
        } else {
            LiftedFile::Text(text.to_string())
        })
    }

    fn valuation(&self, answer: &dyn Fn(&str) -> Option<bool>) -> Vec<bool> {
        match self {
            LiftedFile::Rust(lifted) => lifted.valuation(answer),
            LiftedFile::Text(_) => Vec::new(),
        }
    }

    fn difference(
        &self,
        answer: &dyn Fn(&str) -> Option<bool>,
        reference: &ReferenceFile,
    ) -> Option<String> {
        match (self, reference) {
            (LiftedFile::Rust(lifted), ReferenceFile::Rust(reference)) => {
                first_difference(&lifted.resolve(answer), reference)
            }
            (LiftedFile::Text(text), ReferenceFile::Text(reference)) => (text != reference)
                .then(|| "the generation writes a text the variant does not".to_string()),
            _ => Some("the file is of another kind".to_string()),
        }
    }
}

/// A file a variant generates alone.
enum ReferenceFile {
    Rust(Reference),
    Text(String),
}

impl ReferenceFile {
    fn read(name: &str, text: &str) -> Result<Self, String> {
        Ok(if is_rust(name) {
            ReferenceFile::Rust(Reference::parse(text).map_err(|e| format!("{name}: {e}"))?)
        } else {
            ReferenceFile::Text(text.to_string())
        })
    }
}

fn is_rust(name: &str) -> bool {
    name.ends_with(".rs")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A product that names a condition where one says enough, and the
    /// positive one where either would.
    #[test]
    fn a_cover_names_the_condition_that_decides() {
        // Two conditions; the item is in the variants where the first holds.
        let on: Vec<&[bool]> = vec![&[true, false], &[true, true]];
        let off: Vec<&[bool]> = vec![&[false, false]];
        assert_eq!(minimal_cover(&on, &off), vec![vec![Some(true), None]]);
    }

    #[test]
    fn a_cover_holds_in_every_valuation_it_must_and_in_none_it_must_not() {
        let on: Vec<&[bool]> = vec![&[true, false, false], &[false, true, true]];
        let off: Vec<&[bool]> = vec![&[false, false, false], &[true, true, true]];
        let cover = minimal_cover(&on, &off);
        for valuation in &on {
            assert!(cover
                .iter()
                .any(|product| product_holds(product, valuation)));
        }
        for valuation in &off {
            assert!(!cover
                .iter()
                .any(|product| product_holds(product, valuation)));
        }
    }
}
