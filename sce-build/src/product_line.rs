// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A statechart written against a feature model (ADR 0057): the condition each
//! element carries, the variant of the document each configuration denotes, and
//! the check that no variant keeps a reference to an element it removed.
//!
//! # Reading
//!
//! [`ProductLine::read`] walks the document once, in document order. An element
//! takes its condition from `sce:requires` when it is SCXML's and from
//! `requires` when it is SCE's; its *effective* condition is its own and its
//! ancestors' together, so a condition removes a whole subtree. While it walks it
//! refuses a condition on an element that takes none, one that does not parse, one
//! that names a feature the imported model does not declare, and one that removes
//! nothing beyond what an enclosing condition removes.
//!
//! # References
//!
//! An element that names another by id — a transition its targets, a state or the
//! document its `initial` children — must be present wherever the element it
//! names is not removed: `pc*(referrer) ⇒ pc*(referent)` over every
//! configuration. [`crate::presence_condition::PresenceCondition::implies`]
//! decides that exactly, and its failure is the least configuration in which a
//! variant would keep the reference and lose its referent (ADR 0057 §5, class 2).
//!
//! # Deriving
//!
//! [`ProductLine::derive`] writes the variant of one configuration: the text of
//! the document with every removed element, every condition attribute and the
//! feature-model import blanked out. Blanked, not cut: each character becomes a
//! space and each line break stays, so every line and column of a variant is the
//! line and column of the same text in the source, and a diagnostic raised against
//! a variant points into the document the author wrote.

use std::collections::BTreeMap;
use std::ops::Range;

use crate::feature_model::{Configuration, FeatureModel};
use crate::feature_selector::Selector;
use crate::forge::error::{
    ForgeError, Located, RelatedRole, RelatedSite, SourceLocation, ValidationError,
};
use crate::forge::model::SCE_NAMESPACE;
use crate::model::SCXML_NAMESPACE;
use crate::presence_condition::{self, PresenceCondition, PresenceExpr};

/// The name of the attribute a condition is written in: `sce:requires` on an
/// SCXML element, `requires` on an SCE element.
pub const CONDITION_ATTRIBUTE: &str = "requires";

/// An element that carries a condition of its own.
#[derive(Debug, Clone)]
pub struct Gate {
    /// The element, as a byte range of the source.
    pub element: Range<usize>,
    /// The condition attribute, as a byte range of the source.
    pub attribute: Range<usize>,
    /// The element as written: `transition`, `sce:action`.
    pub tag: String,
    /// The row of the element's start.
    pub line: u32,
    /// The condition written on the element.
    pub own: PresenceCondition,
    /// The condition under which the element is present: its own and those of
    /// the elements around it.
    pub effective: PresenceCondition,
}

/// A statechart written against a feature model.
#[derive(Debug, Clone)]
pub struct ProductLine {
    model: FeatureModel,
    /// The feature model's document name, for the refusals that quote it.
    model_name: String,
    /// The `<sce:import kind="feature-model">`, as a byte range of the source.
    import: Range<usize>,
    /// The elements that carry a condition, in document order.
    gates: Vec<Gate>,
    /// Every state, parallel, final and history by id: its effective condition
    /// and its row.
    states: BTreeMap<String, (PresenceCondition, u32)>,
    /// The elements that name states by id.
    references: Vec<Reference>,
}

/// The `<sce:import kind="feature-model">` of `root`, if it has one; two are
/// refused, at the second, since a statechart is written against one model.
pub fn feature_model_import<'a, 'input>(
    root: &roxmltree::Node<'a, 'input>,
    source_name: &str,
) -> Result<Option<roxmltree::Node<'a, 'input>>, Located<ForgeError>> {
    let mut imports = root.children().filter(|child| {
        child.is_element()
            && child.tag_name().namespace() == Some(SCE_NAMESPACE)
            && child.tag_name().name() == "import"
            && child.attribute("kind") == Some("feature-model")
    });
    let first = imports.next();
    if let (Some(first), Some(second)) = (first, imports.next()) {
        let first_line = row(&first);
        return Err(at(
            &second,
            source_name,
            ValidationError::FeatureModelImportedTwice {
                src: second.attribute("src").unwrap_or_default().to_string(),
            },
        )
        .related_to(RelatedSite {
            role: RelatedRole::AlsoRefused,
            location: location(source_name, first_line, None),
            actual: first.attribute("src").map(str::to_string),
        }));
    }
    Ok(first)
}

impl ProductLine {
    /// Read the conditions of `document`, written against `model` (named
    /// `model_name`), which the document imports through `import`. `None` for
    /// `model` reads a document that imports no feature model, which is a
    /// product line of one variant if it carries no condition and is refused at
    /// its first condition if it carries one.
    pub fn read(
        document: &roxmltree::Document,
        imported: Option<(&FeatureModel, &str, roxmltree::Node)>,
        source_name: &str,
    ) -> Result<Option<Self>, Located<ForgeError>> {
        let root = document.root_element();
        let static_datamodel = root.attribute("datamodel") == Some("sce-static");
        let mut walk = Walk {
            model: imported.as_ref().map(|(model, name, _)| (*model, *name)),
            static_datamodel,
            source_name,
            gates: Vec::new(),
            states: BTreeMap::new(),
            references: Vec::new(),
        };
        walk.element(&root, &PresenceCondition::always())?;
        let Some((model, model_name, import)) = imported else {
            return Ok(None);
        };
        Ok(Some(Self {
            model: model.clone(),
            model_name: model_name.to_string(),
            import: import.range(),
            gates: walk.gates,
            states: walk.states,
            references: walk.references,
        }))
    }

    /// Hold every reference to the rule that its referents are present wherever
    /// it is: the first that is not is refused, with the least configuration
    /// that shows it. A family check: the parse of the document with every
    /// feature on runs it, and the parse of one variant does not need to, since
    /// that variant's own references are held by the reference checker.
    pub fn check_references(&self, source_name: &str) -> Result<(), Located<ForgeError>> {
        for reference in &self.references {
            for id in &reference.ids {
                // An id that names no state is the reference checker's to refuse
                // (`crate::scxml_references`), in every variant alike.
                let Some((referent, referent_line)) = self.states.get(id) else {
                    continue;
                };
                let Err(witness) = reference.condition.implies(referent) else {
                    continue;
                };
                return Err(Located::new(
                    ValidationError::FeatureGateEscapes {
                        element: reference.element.clone(),
                        attribute: reference.attribute.to_string(),
                        referent: id.clone(),
                        witness: self.model.names_of(&witness).map(str::to_string).collect(),
                    }
                    .into(),
                    source_name,
                    Some(reference.line),
                    Some(reference.col),
                )
                .related_to(RelatedSite {
                    role: RelatedRole::Referent,
                    location: location(source_name, *referent_line, None),
                    actual: Some(id.clone()),
                }));
            }
        }
        Ok(())
    }

    pub fn model(&self) -> &FeatureModel {
        &self.model
    }

    pub fn model_name(&self) -> &str {
        &self.model_name
    }

    /// The elements that carry a condition, in document order.
    pub fn gates(&self) -> &[Gate] {
        &self.gates
    }

    /// The effective condition of the element at `range` of the source: that of
    /// the innermost gated element enclosing it, `None` when no gate encloses it
    /// (it is present in every variant). A gate's range holds the ranges of the
    /// elements inside it, so containment is nesting.
    pub fn condition_at(&self, range: &Range<usize>) -> Option<&PresenceCondition> {
        self.gates
            .iter()
            .filter(|gate| gate.element.start <= range.start && range.end <= gate.element.end)
            .min_by_key(|gate| gate.element.end - gate.element.start)
            .map(|gate| &gate.effective)
    }

    /// The selector of the element at `range`, [`Selector::True`] where no gate
    /// encloses it.
    pub fn selector_at(&self, range: &Range<usize>) -> Selector {
        self.condition_at(range)
            .map_or(Selector::True, |condition| {
                Selector::of_condition(condition, &self.model)
            })
    }

    /// The distinct effective conditions of the document's gates, which every
    /// variant answers and which its selector is written over.
    pub fn distinct_conditions(&self) -> Vec<PresenceCondition> {
        let mut conditions: Vec<PresenceCondition> = self
            .gates
            .iter()
            .map(|gate| gate.effective.clone())
            .collect();
        conditions.sort();
        conditions.dedup();
        conditions
    }

    /// The selector of the variant `configuration` denotes (ADR 0057 §9).
    pub fn variant_selector(&self, configuration: &Configuration) -> Selector {
        Selector::of_variant(configuration, &self.distinct_conditions(), &self.model)
    }

    /// Whether any element carries a condition: a document that imports a model
    /// and gates nothing has one variant.
    pub fn is_gated(&self) -> bool {
        !self.gates.is_empty()
    }

    /// The text of the variant `configuration` denotes, from `source`, the text
    /// this product line was read from (see the module documentation).
    pub fn derive(&self, source: &str, configuration: &Configuration) -> String {
        let mut blanked: Vec<Range<usize>> = vec![self.import.clone()];
        for gate in &self.gates {
            if gate.effective.holds_in(configuration) {
                blanked.push(gate.attribute.clone());
            } else {
                blanked.push(gate.element.clone());
            }
        }
        blank(source, blanked)
    }

    /// The features some condition of the document mentions, with everything
    /// they require.
    fn mentioned(&self) -> Configuration {
        let mut mentioned = Configuration::empty();
        for gate in &self.gates {
            for least in gate.effective.least_configurations() {
                mentioned = mentioned.union(least);
            }
        }
        mentioned
    }

    /// Every configuration of the features the conditions mention, which hold
    /// what they require: what a build of the product line is as far as any
    /// variant can tell, each once. Two builds alike on these features build
    /// one variant.
    pub fn configurations(&self) -> Vec<Configuration> {
        self.model.configurations_over(&self.mentioned())
    }

    /// One configuration for each distinct variant, each the least
    /// configuration that denotes it, in the order of their size and then the
    /// model's order.
    ///
    /// Two configurations denote the same variant exactly when every gate is
    /// present in both or absent in both, and a condition mentions only some of
    /// the model's features, so the walk is over the configurations of those
    /// features alone ([`FeatureModel::configurations_over`]); each is mapped
    /// to the variant's signature, and the least configuration of each
    /// signature is kept.
    pub fn variants(&self) -> Vec<Configuration> {
        let mut least: BTreeMap<Vec<bool>, Configuration> = BTreeMap::new();
        for configuration in self.configurations() {
            let signature: Vec<bool> = self
                .gates
                .iter()
                .map(|gate| gate.effective.holds_in(&configuration))
                .collect();
            let kept = least
                .entry(signature)
                .or_insert_with(|| configuration.clone());
            if (configuration.len(), &configuration) < (kept.len(), &*kept) {
                *kept = configuration;
            }
        }
        let mut variants: Vec<Configuration> = least.into_values().collect();
        variants.sort_by(|x, y| x.len().cmp(&y.len()).then_with(|| x.cmp(y)));
        variants
    }
}

/// `source` with each character of `ranges` written as a space, line breaks
/// kept. A range inside another is the other's.
fn blank(source: &str, mut ranges: Vec<Range<usize>>) -> String {
    ranges.sort_by_key(|range| range.start);
    let mut out = String::with_capacity(source.len());
    let mut at = 0;
    for range in ranges {
        if range.start < at {
            // Inside a range already blanked: an attribute of a removed element,
            // or an element inside one.
            continue;
        }
        out.push_str(&source[at..range.start]);
        for c in source[range.clone()].chars() {
            out.push(if c == '\n' || c == '\r' { c } else { ' ' });
        }
        at = range.end;
    }
    out.push_str(&source[at..]);
    out
}

/// What [`ProductLine::read`] gathers while it walks.
struct Walk<'m, 's> {
    model: Option<(&'m FeatureModel, &'m str)>,
    static_datamodel: bool,
    source_name: &'s str,
    gates: Vec<Gate>,
    /// Every state, parallel, final and history by id: its effective condition
    /// and its row.
    states: BTreeMap<String, (PresenceCondition, u32)>,
    references: Vec<Reference>,
}

/// An element that names states by id.
#[derive(Debug, Clone)]
struct Reference {
    /// `<transition target>`, `<state initial>`.
    element: String,
    attribute: &'static str,
    ids: Vec<String>,
    condition: PresenceCondition,
    line: u32,
    col: u32,
}

impl Walk<'_, '_> {
    fn element(
        &mut self,
        node: &roxmltree::Node,
        enclosing: &PresenceCondition,
    ) -> Result<(), Located<ForgeError>> {
        let mut effective = enclosing.clone();
        if let Some(attribute) = condition_attribute(node) {
            effective = self.gate(node, &attribute, enclosing)?;
        }
        if is_scxml(node) {
            let tag = node.tag_name().name();
            if matches!(tag, "state" | "parallel" | "final" | "history") {
                if let Some(id) = node.attribute("id") {
                    self.states
                        .insert(id.to_string(), (effective.clone(), row(node)));
                }
            }
            let named = match tag {
                "transition" => Some("target"),
                "scxml" | "state" => Some("initial"),
                _ => None,
            };
            if let Some(attribute) = named {
                if let Some(written) = node.attribute_node(attribute) {
                    let ids = written.value();
                    // Placed on the value, where the id the record quotes is.
                    let pos = node.document().text_pos_at(written.range_value().start);
                    self.references.push(Reference {
                        element: tag.to_string(),
                        attribute,
                        ids: ids.split_ascii_whitespace().map(str::to_string).collect(),
                        condition: effective.clone(),
                        line: pos.row,
                        col: pos.col,
                    });
                }
            }
        }
        if is_opaque(node) {
            return Ok(());
        }
        for child in node.children().filter(roxmltree::Node::is_element) {
            self.element(&child, &effective)?;
        }
        Ok(())
    }

    /// Read the condition `attribute` of `node`, enclosed by `enclosing`, and
    /// answer the element's effective condition.
    fn gate(
        &mut self,
        node: &roxmltree::Node,
        attribute: &roxmltree::Attribute,
        enclosing: &PresenceCondition,
    ) -> Result<PresenceCondition, Located<ForgeError>> {
        let tag = written_tag(node);
        if let Err(reason) = takes_condition(node, self.static_datamodel) {
            return Err(at(
                node,
                self.source_name,
                ValidationError::FeatureGateUnsupported {
                    element: tag,
                    reason: reason.to_string(),
                },
            ));
        }
        let text = attribute.value();
        let value_at = attribute.range_value().start;
        let expr = presence_condition::parse(text).map_err(|error| {
            at_offset(
                node,
                value_at + error.offset.min(text.len()),
                self.source_name,
                ValidationError::FeatureConditionMalformed {
                    condition: text.to_string(),
                    reason: error.message,
                },
            )
        })?;
        let Some((model, model_name)) = self.model else {
            let (feature, offset) = first_feature(&expr);
            return Err(at_offset(
                node,
                value_at + offset,
                self.source_name,
                ValidationError::FeatureUndeclared {
                    feature: feature.to_string(),
                    model: None,
                },
            ));
        };
        let own = PresenceCondition::resolve(&expr, model).map_err(|undeclared| {
            at_offset(
                node,
                value_at + undeclared.offset,
                self.source_name,
                ValidationError::FeatureUndeclared {
                    feature: undeclared.name,
                    model: Some(model_name.to_string()),
                },
            )
        })?;
        if !enclosing.is_always() && enclosing.implies(&own).is_ok() {
            return Err(at(
                node,
                self.source_name,
                ValidationError::FeatureConditionRedundant {
                    element: tag,
                    condition: text.to_string(),
                    enclosing: spelled(enclosing, model),
                },
            ));
        }
        let effective = enclosing.and(&own);
        self.gates.push(Gate {
            element: node.range(),
            attribute: attribute.range(),
            tag,
            line: row(node),
            own,
            effective: effective.clone(),
        });
        Ok(effective)
    }
}

/// The condition attribute `node` carries, if any.
fn condition_attribute<'a, 'input>(
    node: &roxmltree::Node<'a, 'input>,
) -> Option<roxmltree::Attribute<'a, 'input>> {
    if is_scxml(node) {
        crate::sce_attr::attribute(node, CONDITION_ATTRIBUTE)
    } else if node.tag_name().namespace() == Some(SCE_NAMESPACE) {
        node.attributes()
            .find(|a| a.namespace().is_none() && a.name() == CONDITION_ATTRIBUTE)
    } else {
        None
    }
}

/// Whether `node` takes a condition (ADR 0057 §3), and why not when it does not.
fn takes_condition(node: &roxmltree::Node, static_datamodel: bool) -> Result<(), &'static str> {
    let tag = node.tag_name().name();
    if is_scxml(node) {
        return match tag {
            "state" | "parallel" | "final" | "transition" | "onentry" | "onexit" | "invoke"
            | "raise" | "if" | "foreach" | "log" | "assign" | "script" | "send" | "cancel" => {
                Ok(())
            }
            "data" if static_datamodel => Ok(()),
            "data" => Err(
                "a <data> takes a condition only under datamodel=\"sce-static\", where every \
                 expression that reads it is known when the document is built",
            ),
            "scxml" => Err("the document itself is in every variant"),
            "initial" | "history" | "datamodel" | "donedata" | "finalize" => Err(
                "it is structure its parent requires; gate the parent, or the transitions it holds",
            ),
            "elseif" | "else" => Err(
                "it divides the content of its <if> and is not an element of its own; gate the \
                 content it guards, or the <if>",
            ),
            _ => Err("it is part of the element that holds it; gate that element"),
        };
    }
    match tag {
        "action" => Ok(()),
        "import" if node.attribute("kind") == Some("feature-model") => {
            Err("the feature model is what conditions are written against")
        }
        "import" => Ok(()),
        _ => Err("it is part of the element that holds it; gate that element"),
    }
}

/// Whether the children of `node` are its value rather than SCXML structure:
/// the walk does not enter them, and a condition there is not read.
fn is_opaque(node: &roxmltree::Node) -> bool {
    is_scxml(node) && matches!(node.tag_name().name(), "data" | "content" | "script")
}

fn is_scxml(node: &roxmltree::Node) -> bool {
    node.tag_name().namespace() == Some(SCXML_NAMESPACE)
}

/// The element as an author writes it: `transition`, `sce:action`.
fn written_tag(node: &roxmltree::Node) -> String {
    let name = node.tag_name().name();
    if node.tag_name().namespace() == Some(SCE_NAMESPACE) {
        format!("sce:{name}")
    } else {
        name.to_string()
    }
}

/// The first feature `expr` names, and its offset.
fn first_feature(expr: &PresenceExpr) -> (&str, usize) {
    match expr {
        PresenceExpr::Feature { name, offset } => (name, *offset),
        PresenceExpr::All(operands) | PresenceExpr::Any(operands) => first_feature(&operands[0]),
    }
}

/// A condition in the authoring grammar's canonical spelling.
fn spelled(condition: &PresenceCondition, model: &FeatureModel) -> String {
    condition
        .canonical_form(model)
        .map(|form| form.to_string())
        .unwrap_or_default()
}

fn row(node: &roxmltree::Node) -> u32 {
    node.document().text_pos_at(node.range().start).row
}

fn location(file: &str, line: u32, col: Option<u32>) -> SourceLocation {
    SourceLocation {
        file: file.to_string(),
        line: Some(line),
        col,
    }
}

fn at(node: &roxmltree::Node, source_name: &str, error: ValidationError) -> Located<ForgeError> {
    at_offset(node, node.range().start, source_name, error)
}

fn at_offset(
    node: &roxmltree::Node,
    offset: usize,
    source_name: &str,
    error: ValidationError,
) -> Located<ForgeError> {
    let pos = node.document().text_pos_at(offset);
    Located::new(error.into(), source_name, Some(pos.row), Some(pos.col))
}
