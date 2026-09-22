// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Which of a document's `<sce:import>` aliases its content names.
//!
//! A document may declare an import it never reads, and generated code must
//! not depend on it when it does. Go makes an unused import a compile error,
//! so every template that emitted one `import` line per declaration turned a
//! harmless leftover into a document that generates with exit 0 and does
//! not build (measured 2026-09-22: a transform importing an enum it never
//! read). The other backends only warn, which is why nothing else saw it.
//!
//! The answer is one set, asked once, before any backend renders: the
//! aliases the document names. The renderers then see only those imports,
//! so every backend agrees on what the document depends on — the verdict
//! cannot vary with the language asked for.
//!
//! # Why a name equal to an alias IS the alias
//!
//! [`crate::forge::namespace`] refuses a local declaration that reuses an
//! import's alias, so inside a document that passed it an identifier spelled
//! like an alias can only mean that import. That is what lets this module
//! read expressions for the names they mention without resolving them.
//!
//! # Where a document names an import
//!
//! Three kinds of position, and every one of them is walked below:
//!
//! * a type — `sce:type="enum:<alias>"`, on any field, parameter, local,
//!   constant or helper signature;
//! * an expression — `<alias>(…)` for a stateless kind, `<alias>.<member>`
//!   for a stateful one or an enum variant;
//! * a structural reference — a codec's embedded, repeated, chained or
//!   variant body, a link's framer and pools, a worker's link and outbox,
//!   an algorithm's `<sce:foreach in>` and `<sce:call target>`, a cycle's
//!   value space.
//!
//! A position added to the model and not to this walk drops the import it
//! names: the renderer then refuses the reference or emits code that does
//! not build, so the omission is loud — and the committed golden trees,
//! regenerated, are what prove the walk covers every form the corpus uses.

use std::collections::BTreeSet;

use crate::forge::error::ExprError;
use crate::forge::expr;
use crate::forge::model::{
    AlgorithmConstType, AlgorithmStmt, CodecVariant, Cycle, ForgeDocument, ForgeField, ForgeImport,
    SceType,
};

/// The aliases of `imports` that `document` or its `cycles` name.
///
/// `document` is the one about to be rendered — after any rewrite that
/// runs before rendering — so the set describes the code that is emitted.
pub(crate) fn named_aliases(
    document: &ForgeDocument,
    cycles: &[Cycle],
    imports: &[ForgeImport],
) -> Result<BTreeSet<String>, ExprError> {
    let mut names = Names::default();
    names.document(document)?;
    for cycle in cycles {
        names.name(&cycle.of);
        for step in &cycle.steps {
            names.opt_expr(step.when.as_deref())?;
        }
    }
    Ok(imports
        .iter()
        .filter(|imp| names.0.contains(imp.alias.as_str()))
        .map(|imp| imp.alias.clone())
        .collect())
}

/// Every name the walk met — more than the aliases, which
/// [`named_aliases`] keeps.
#[derive(Default)]
struct Names(BTreeSet<String>);

impl Names {
    fn name(&mut self, name: &str) {
        self.0.insert(name.to_string());
    }

    fn opt_name(&mut self, name: Option<&str>) {
        if let Some(name) = name {
            self.name(name);
        }
    }

    fn ty(&mut self, ty: &SceType) {
        if let SceType::Enum(r) = ty {
            self.name(&r.alias);
        }
    }

    /// Every name the expression reads as a value, callees included —
    /// `frame.len` reads `frame`, `crc(x)` reads `crc` and `x`.
    fn expr(&mut self, raw: &str) -> Result<(), ExprError> {
        for name in expr::read_identifiers(raw)? {
            self.0.insert(name);
        }
        Ok(())
    }

    fn opt_expr(&mut self, raw: Option<&str>) -> Result<(), ExprError> {
        match raw {
            Some(raw) => self.expr(raw),
            None => Ok(()),
        }
    }

    fn field(&mut self, field: &ForgeField) -> Result<(), ExprError> {
        self.ty(&field.sce_type);
        self.opt_expr(field.expr.as_deref())
    }

    fn fields<'a>(
        &mut self,
        fields: impl IntoIterator<Item = &'a ForgeField>,
    ) -> Result<(), ExprError> {
        fields.into_iter().try_for_each(|f| self.field(f))
    }

    fn codec_variant(&mut self, variant: &CodecVariant) {
        for arm in variant.arms.iter().chain(&variant.default_arm) {
            self.name(&arm.body_alias);
        }
    }

    fn stmts(&mut self, stmts: &[AlgorithmStmt]) -> Result<(), ExprError> {
        for stmt in stmts {
            match stmt {
                AlgorithmStmt::Var { sce_type, init, .. } => {
                    self.ty(sce_type);
                    self.opt_expr(init.as_deref())?;
                }
                // A target is an lvalue in expression syntax — `x`,
                // `buf[i]` — so it is read the way an expression is.
                AlgorithmStmt::Assign { target, expr } | AlgorithmStmt::Append { target, expr } => {
                    self.expr(target)?;
                    self.expr(expr)?;
                }
                AlgorithmStmt::If {
                    cond,
                    then_body,
                    else_body,
                } => {
                    self.expr(cond)?;
                    self.stmts(then_body)?;
                    if let Some(else_body) = else_body {
                        self.stmts(else_body)?;
                    }
                }
                AlgorithmStmt::While { cond, body, .. } => {
                    self.expr(cond)?;
                    self.stmts(body)?;
                }
                AlgorithmStmt::Foreach { source, body, .. } => {
                    self.name(source);
                    self.stmts(body)?;
                }
                AlgorithmStmt::Return { expr } => self.opt_expr(expr.as_deref())?,
                AlgorithmStmt::Call { target, args, .. } => {
                    self.expr(target)?;
                    for arg in args {
                        self.expr(arg)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn document(&mut self, document: &ForgeDocument) -> Result<(), ExprError> {
        match document {
            ForgeDocument::Transform(m) => self.fields(m.inputs.iter().chain(&m.outputs)),
            ForgeDocument::Lookup(m) => self.fields([&m.input, &m.output]),
            ForgeDocument::Condition(m) => {
                self.fields(&m.inputs)?;
                self.expr(&m.expr)
            }
            ForgeDocument::Validator(m) => {
                self.fields(&m.inputs)?;
                // Range bounds and rate limits are numeric literals; the
                // plausibility rule is the one expression a validator has.
                self.opt_expr(m.rules.plausibility.as_deref())
            }
            ForgeDocument::Procedure(m) => {
                self.fields(m.inputs.iter().chain(&m.internals))?;
                for helper in &m.helpers {
                    helper.args.iter().for_each(|t| self.ty(t));
                    self.ty(&helper.returns);
                }
                for state in &m.states {
                    for transition in &state.transitions {
                        self.opt_expr(transition.cond.as_deref())?;
                        for assign in &transition.assigns {
                            self.expr(&assign.location)?;
                            self.expr(&assign.expr)?;
                        }
                    }
                    for send in &state.on_entry_sends {
                        self.opt_expr(send.addr.as_deref())?;
                        self.opt_expr(send.payload.as_deref())?;
                    }
                    for param in &state.done_params {
                        self.expr(&param.expr)?;
                    }
                }
                Ok(())
            }
            ForgeDocument::Codec(m) => {
                for field in &m.fields {
                    self.ty(&field.sce_type);
                    self.opt_name(field.repeat_body_alias.as_deref());
                    self.opt_name(field.tlv_chain_body_alias.as_deref());
                    self.opt_name(field.embed_body_alias.as_deref());
                }
                if let Some(variant) = &m.variant {
                    self.codec_variant(variant);
                }
                Ok(())
            }
            ForgeDocument::Filter(m) => self.fields([&m.input, &m.output]),
            ForgeDocument::Interpolation(m) => {
                self.fields(m.inputs.iter().chain(std::iter::once(&m.output)))
            }
            ForgeDocument::Observer(m) => {
                self.fields(&m.inputs)?;
                for monitor in &m.monitors {
                    self.expr(&monitor.enter_expr)?;
                    self.opt_expr(monitor.leave_expr.as_deref())?;
                }
                Ok(())
            }
            ForgeDocument::Algorithm(m) => {
                m.signature.params.iter().for_each(|p| self.ty(&p.sce_type));
                if let Some(ret) = &m.signature.return_type {
                    self.ty(ret);
                }
                for c in &m.consts {
                    match &c.sce_type {
                        AlgorithmConstType::Scalar(t) => self.ty(t),
                        AlgorithmConstType::Array { elem, .. } => self.ty(elem),
                    }
                    self.opt_expr(c.init.as_deref())?;
                    if let Some(fold) = &c.fold {
                        self.ty(&fold.elem_type);
                        self.stmts(&fold.body)?;
                        self.expr(&fold.yield_expr)?;
                    }
                }
                self.stmts(&m.body)
            }
            ForgeDocument::Link(m) => {
                self.name(&m.framer);
                self.opt_name(m.rx_pool.as_deref());
                self.opt_name(m.tx_pool.as_deref());
                self.opt_name(m.stage_pool.as_deref());
                for inbound in &m.inbound {
                    self.opt_expr(inbound.when.as_deref())?;
                }
                for outbound in &m.outbound {
                    self.name(&outbound.encode);
                }
                Ok(())
            }
            ForgeDocument::Worker(m) => {
                self.name(&m.link_rx);
                self.opt_name(m.outbox.as_deref());
                Ok(())
            }
            ForgeDocument::BoundedCollection(m) => {
                self.name(&m.element_type);
                Ok(())
            }
            ForgeDocument::EventSchema(m) => self.fields(&m.fields),
            // A timer names events, a buffer pool sizes storage and an enum
            // lists values: none of them reads another document.
            ForgeDocument::Timer(_) | ForgeDocument::BufferPool(_) | ForgeDocument::Enum(_) => {
                Ok(())
            }
            // The statechart pipeline owns a statechart's imports; forge
            // rendering never receives one (see `classify_document`).
            ForgeDocument::Statechart(_) => unreachable!(
                "ForgeDocument::Statechart never reaches forge rendering — \
                 the SCXML pipeline owns a statechart's imports"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge::model::ParsedForge;

    fn parse(src: &str) -> ParsedForge {
        crate::forge::parser::parse_forge_with_imports(
            src,
            crate::DocumentLabel::symmetric("t.scxml"),
        )
        .expect("parses")
        .expect("is a forge document")
    }

    fn named(src: &str) -> Vec<String> {
        let parsed = parse(src);
        named_aliases(&parsed.document, &parsed.cycles, &parsed.imports)
            .expect("every expression reads")
            .into_iter()
            .collect()
    }

    const HEAD: &str = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext" version="1.0""#;

    /// The measured defect: an import nothing reads is not one the
    /// document depends on, so no backend may be handed it.
    #[test]
    fn an_import_nothing_reads_is_not_named() {
        let names = named(&format!(
            r#"{HEAD} sce:kind="transform" name="t">
                 <sce:import src="e.scxml" kind="enum" as="session"/>
                 <datamodel>
                   <data id="x" sce:type="float64" sce:direction="in"/>
                   <data id="y" sce:type="float64" sce:direction="out" expr="x * 2"/>
                 </datamodel></scxml>"#
        ));
        assert!(names.is_empty(), "{names:?}");
    }

    #[test]
    fn an_enum_type_names_its_import() {
        let names = named(&format!(
            r#"{HEAD} sce:kind="transform" name="t">
                 <sce:import src="e.scxml" kind="enum" as="session"/>
                 <datamodel>
                   <data id="x" sce:type="enum:session" sce:direction="in"/>
                   <data id="y" sce:type="bool" sce:direction="out" expr="true"/>
                 </datamodel></scxml>"#
        ));
        assert_eq!(names, ["session"]);
    }

    /// A call names its callee and a member read names its object — the
    /// two ways an expression reaches a stateless and a stateful import.
    #[test]
    fn an_expression_names_the_imports_it_calls_and_reads() {
        let names = named(&format!(
            r#"{HEAD} sce:kind="validator">
                 <sce:import src="c.scxml" kind="condition" as="tempWarn"/>
                 <sce:import src="f.scxml" kind="codec" as="frame"/>
                 <sce:import src="u.scxml" kind="lookup" as="unused"/>
                 <datamodel>
                   <data id="t" sce:type="float64" sce:direction="in"/>
                   <data id="ok" sce:type="bool" sce:direction="out"
                         sce:plausibility="!tempWarn(t) &amp;&amp; frame.msgId === 1"/>
                 </datamodel></scxml>"#
        ));
        assert_eq!(names, ["frame", "tempWarn"]);
    }

    #[test]
    fn a_codec_names_the_body_it_embeds() {
        let names = named(&format!(
            r#"{HEAD} sce:kind="codec" name="c">
                 <sce:import src="l.scxml" kind="codec" as="locator"/>
                 <sce:import src="n.scxml" kind="codec" as="neverEmbedded"/>
                 <datamodel>
                   <sce:field id="tag" sce:type="uint8" sce:byte="0" sce:bit-size="8"/>
                   <sce:embed id="loc" type="locator" sce:byte="1"/>
                 </datamodel></scxml>"#
        ));
        assert_eq!(names, ["locator"]);
    }

    #[test]
    fn an_algorithm_names_the_collection_it_walks() {
        let names = named(&format!(
            r#"{HEAD} sce:kind="algorithm" name="a">
                 <sce:import kind="bounded-collection" src="s.scxml" as="subs"/>
                 <sce:signature>
                   <sce:param name="target" type="uint32"/>
                   <sce:return type="uint16"/>
                 </sce:signature>
                 <sce:body>
                   <sce:foreach item="entry" in="subs">
                     <sce:return expr="0"/>
                   </sce:foreach>
                   <sce:return expr="0xFFFF"/>
                 </sce:body></scxml>"#
        ));
        assert_eq!(names, ["subs"]);
    }

    /// A bytes buffer carries no initializer, and its absence is not an
    /// expression to read — measured on `algorithm_cobs_encode`, which the
    /// first version of this walk refused on every backend.
    #[test]
    fn a_bytes_buffer_has_no_initializer_to_read() {
        let names = named(&format!(
            r#"{HEAD} sce:kind="algorithm" name="a">
                 <sce:signature>
                   <sce:param name="data" type="bytes"/>
                   <sce:return type="bytes" returns-max-size="4"/>
                 </sce:signature>
                 <sce:body>
                   <sce:var name="out" type="bytes" capacity="4"/>
                   <sce:return expr="out"/>
                 </sce:body></scxml>"#
        ));
        assert!(names.is_empty(), "{names:?}");
    }
}
