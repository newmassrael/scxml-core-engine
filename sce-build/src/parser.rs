// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCXML Parser — ports scxml_parser.py using roxmltree.
// Parses W3C SCXML files into SCXMLModel for code generation.

use crate::model::*;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::LazyLock;

pub struct SCXMLParser {
    document_order_counter: u32,
    invoke_counter: u32,
    hybrid_invoke_counter: u32,
    inline_child_counter: u32,
    send_counter: u32,
}

impl SCXMLParser {
    pub fn new() -> Self {
        Self {
            document_order_counter: 0,
            invoke_counter: 0,
            hybrid_invoke_counter: 0,
            inline_child_counter: 0,
            send_counter: 0,
        }
    }

    /// Parse an SCXML file from disk.
    ///
    /// The error type is `Located<ForgeError>`: location is part of the
    /// error contract — every failure ties back to the file path so
    /// downstream diagnostics (CLI NDJSON, build scripts, agents) do
    /// not have to attach file context after the fact. I/O errors
    /// carry the path as `ForgeError::Io`; XML / validation errors
    /// use the file stem as the location label to match the naming
    /// W3C batch diagnostics expect.
    pub fn parse_file(
        &mut self,
        scxml_path: &str,
    ) -> Result<SCXMLModel, crate::forge::error::Located<crate::forge::error::ForgeError>> {
        use crate::forge::error::{ForgeError, Located};
        let content = std::fs::read_to_string(scxml_path).map_err(|e| {
            Located::new(
                ForgeError::Io {
                    path: Path::new(scxml_path).to_path_buf(),
                    source: e,
                },
                scxml_path,
                None,
                None,
            )
        })?;
        let name = Path::new(scxml_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        let base_dir = Path::new(scxml_path).parent().map(|p| p.to_path_buf());
        self.parse_impl(&content, &name, base_dir.as_deref())
    }

    /// Parse SCXML from a string (no filesystem access).
    /// Suitable for WASM and in-memory code generation.
    pub fn parse_string(
        &mut self,
        content: &str,
        name: &str,
    ) -> Result<SCXMLModel, crate::forge::error::Located<crate::forge::error::ForgeError>> {
        self.parse_impl(content, name, None)
    }

    fn parse_impl(
        &mut self,
        content: &str,
        name: &str,
        base_dir: Option<&Path>,
    ) -> Result<SCXMLModel, crate::forge::error::Located<crate::forge::error::ForgeError>> {
        use crate::forge::error::{ForgeError, Located, XmlError};

        // W3C SCXML + sce: namespace schema validation. Runs before any
        // structural parsing so malformed documents fail fast at the
        // system boundary with libxml2's line/column diagnostics. The
        // schema (`schemas/sce-forge.xsd`) is permissive for W3C SCXML
        // structural elements (xs:any lax) and strict for sce:* — pure
        // statechart documents pass through trivially while inline forge
        // kinds on <data> still get their sce: attributes validated.
        // Silently skipped if the schemas/ directory is unreachable
        // (downstream vendoring without the schemas) — see
        // `forge::xsd_validator::validate_or_skip`.
        crate::forge::xsd_validator::validate_or_skip(content, name).map_err(|errs| {
            Located::new(
                ForgeError::Xml(XmlError::SchemaValidation(errs)),
                name,
                None,
                None,
            )
        })?;

        let doc = roxmltree::Document::parse(content).map_err(|e| {
            // roxmltree reports row/col for every error via `pos()`;
            // passing it through preserves actionable location data
            // instead of fabricating `(1, 1)` at the parser boundary.
            let pos = e.pos();
            Located::new(
                ForgeError::Xml(XmlError::Parse(e.to_string())),
                name,
                Some(pos.row),
                Some(pos.col),
            )
        })?;
        let root = doc.root_element();

        // W3C SCXML 3.6: Get initial attribute
        let mut initial = root.attribute("initial").unwrap_or("").to_string();
        if initial.is_empty() {
            // Default to first child state in document order
            for child in root.children() {
                if child.is_element() && is_scxml_state_element(&child) {
                    if let Some(id) = child.attribute("id") {
                        initial = id.to_string();
                        break;
                    }
                }
            }
        }

        let mut model = SCXMLModel {
            name: name.to_string(),
            scxml_name: root.attribute("name").unwrap_or("").to_string(),
            initial,
            binding: root.attribute("binding").unwrap_or("early").to_string(),
            datamodel_type: root.attribute("datamodel").unwrap_or("ecmascript").to_string(),
            ..Default::default()
        };

        // Parse datamodel
        self.parse_datamodel(&root, &mut model, name)?;

        // Parse global scripts
        self.parse_global_scripts(&root, &mut model, base_dir);

        // Parse Named Context declarations (must be before states for transforms)
        self.parse_sce_contexts(&root, &mut model, name)?;

        // Parse states recursively
        self.parse_states(&root, None, &mut model, base_dir, name)?;

        // Feature detection
        self.detect_features(&mut model);

        // Named Context validation
        self.validate_context_usage(&model, name)?;

        // Resolve deep initial state
        self.resolve_deep_initial(&mut model);

        // W3C SCXML 3.13: Apply parallel initial overrides
        self.apply_parallel_initial_overrides(&mut model);

        // Resolve history targets
        self.resolve_history_targets(&mut model);

        // Compute history leaf targets (resolve default_target to leaf state)
        let history_ids: Vec<String> = model.history_states.keys().cloned().collect();
        for hid in &history_ids {
            if let Some(info) = model.history_states.get(hid) {
                let leaf = model.resolve_to_leaf(&info.default_target);
                let info_mut = model.history_states.get_mut(hid).unwrap();
                info_mut.leaf_target = leaf;
            }
        }

        // Compute initial_leaf
        if !model.initial.is_empty() {
            model.initial_leaf = model.resolve_to_leaf(&model.initial);
        }

        // Compute parallel regions
        self.compute_parallel_regions(&mut model);

        // Detect transition/entry/exit actions, hierarchy
        self.detect_transition_actions(&mut model);
        self.detect_entry_exit_actions(&mut model);
        self.detect_hierarchy(&mut model);

        // Add done.state events
        self.add_done_state_events(&mut model);

        // Set invoke event flags
        self.set_invoke_event_flags(&mut model);

        // Collect child→parent events
        self.collect_child_to_parent_events(&mut model, base_dir);

        // Parse initial_children
        self.parse_initial_children(&mut model);

        // Compute needs_nonstatic_method
        model.needs_nonstatic_method = model.needs_script_engine
            || model.has_scxml_invoke()
            || model.has_parent_communication
            || model.has_parallel_states
            || model.uses_in_predicate
            || !model.context_objects.is_empty();

        // Document rejection → redirect initial to "pass"
        if model.document_rejected {
            if let Some(state_id) = model
                .states
                .iter()
                .find(|(_, s)| s.is_final && s.id.to_lowercase() == "pass")
                .map(|(id, _)| id.clone())
            {
                model.initial = state_id.clone();
                model.initial_leaf = state_id;
            }
        }

        // Assign transition indices
        for state in model.states.values_mut() {
            for (i, trans) in state.transitions.iter_mut().enumerate() {
                trans.transition_index = i;
            }
        }

        // State-level `invokes` is authoritative; `SCXMLModel.invokes` is a
        // template-visible flat view. Build it once, here, after all
        // per-state mutations are finalised.
        model.refresh_invokes_view();

        Ok(model)
    }

    fn parse_datamodel(
        &mut self,
        root: &roxmltree::Node,
        model: &mut SCXMLModel,
        source_name: &str,
    ) -> Result<(), crate::forge::error::Located<crate::forge::error::ForgeError>> {
        use crate::forge::model::SCE_NAMESPACE;

        for child in root.children() {
            if !child.is_element() || local_name(&child) != "datamodel" {
                continue;
            }
            for data in child.children() {
                if !data.is_element() || local_name(&data) != "data" {
                    continue;
                }

                // SCE Forge: detect inline kind on <data sce:kind="..."> — classify
                // as InlineKind instead of Variable (single XML parse, no re-parsing).
                if let Some(kind_attr) = data.attribute((SCE_NAMESPACE, "kind")) {
                    match Self::try_parse_inline_kind(&data, kind_attr, source_name)? {
                        Some(inline) => {
                            model.inline_kinds.push(inline);
                            continue;
                        }
                        // Unknown/non-inline kind — fall through to variable.
                        None => {}
                    }
                }

                let var_id = data.attribute("id").unwrap_or("").to_string();
                let expr = data.attribute("expr").unwrap_or("").to_string();
                let src = data.attribute("src").unwrap_or("").to_string();

                let content = if src.is_empty() {
                    if data.children().any(|c| c.is_element()) {
                        // Serialize child elements as XML string
                        let mut xml = String::new();
                        for c in data.children().filter(|c| c.is_element()) {
                            xml.push_str(&serialize_node(&c));
                        }
                        xml
                    } else {
                        data.text().unwrap_or("").trim().to_string()
                    }
                } else {
                    String::new()
                };

                model.variables.push(Variable {
                    id: var_id,
                    expr,
                    src,
                    content,
                    var_type: String::new(),
                });
                model.needs_script_engine = true;
            }
        }
        Ok(())
    }

    /// SCE Forge: attempt to parse a <data sce:kind="..."> element as an inline kind.
    /// Returns `Ok(Some(kind))` on success, `Ok(None)` for unknown/non-inline kinds
    /// (fall through to variable), `Err` for recognized inline kinds with invalid content.
    ///
    /// Errors are fatal: a document that opted into `sce:kind=...` has
    /// asserted intent about the shape of this `<data>`, so silently
    /// demoting a malformed inline kind to an ECMAScript variable
    /// would mask the author's intent and hand the generator a type
    /// the rest of the pipeline was never told about (see the
    /// `feedback_silently_broken_hooks` memory). Errors flow through
    /// `Located<ForgeError>` with roxmltree-derived row/col so agents
    /// receive the same precision the codec-field parser already
    /// delivers; the already-located codec-field error is propagated
    /// unchanged (no `.map_err(|l| l.error)` unwrap).
    fn try_parse_inline_kind(
        data: &roxmltree::Node,
        kind_attr: &str,
        source_name: &str,
    ) -> Result<
        Option<crate::forge::model::InlineKind>,
        crate::forge::error::Located<crate::forge::error::ForgeError>,
    > {
        use crate::forge::error::{Located, ValidationError};
        use crate::forge::model::*;

        // Lift a leaf `ValidationError` into a `Located<ForgeError>`
        // anchored at the given node. `text_pos_at(range().start)`
        // recovers libxml2-style (row, col) for any node the
        // roxmltree document owns — the same mechanism the forge
        // codec-field parser uses, so inline-kind diagnostics reach
        // NDJSON with matching precision.
        let locate_at =
            |node: &roxmltree::Node,
             err: ValidationError|
             -> Located<crate::forge::error::ForgeError> {
                let pos = node.document().text_pos_at(node.range().start);
                Located::new(err.into(), source_name, Some(pos.row), Some(pos.col))
            };
        let locate = |err: ValidationError| locate_at(data, err);

        let kind = match ForgeKind::from_attr(kind_attr) {
            Some(k) => k,
            None => return Ok(None), // Unknown kind — treat as regular variable
        };
        if !kind.is_inline_eligible() {
            return Ok(None); // Stateful kind — cannot be inline, treat as variable
        }

        let id = data
            .attribute("id")
            .ok_or_else(|| {
                locate(ValidationError::MissingAttribute {
                    element: format!("inline <data sce:kind=\"{kind}\">"),
                    attr: "id".to_string(),
                })
            })?
            .to_string();

        let sce_attr = |local: &str| -> Option<String> {
            data.attribute((SCE_NAMESPACE, local)).map(|s| s.to_string())
        };

        let inline_data = match kind {
            ForgeKind::Lookup => {
                let input_id = sce_attr("input").unwrap_or_default();
                let default_value = sce_attr("default").unwrap_or_default();

                let mut entries = Vec::new();
                for child in data.children().filter(|n| n.is_element()) {
                    if child.tag_name().name() == "entry"
                        && child.tag_name().namespace() == Some(SCE_NAMESPACE)
                    {
                        let key = child
                            .attribute("key")
                            .ok_or_else(|| {
                                locate_at(&child, ValidationError::MissingAttribute {
                                    element: format!("<sce:entry> in inline lookup '{id}'"),
                                    attr: "key".to_string(),
                                })
                            })?
                            .to_string();
                        let value = child
                            .attribute("value")
                            .ok_or_else(|| {
                                locate_at(&child, ValidationError::MissingAttribute {
                                    element: format!("<sce:entry> in inline lookup '{id}'"),
                                    attr: "value".to_string(),
                                })
                            })?
                            .to_string();
                        entries.push(LookupEntry { key, value });
                    }
                }
                if entries.is_empty() {
                    return Err(locate(ValidationError::EmptyCollection {
                        kind: ForgeKind::Lookup,
                        what: format!("<sce:entry> (inline lookup '{id}')"),
                    }));
                }

                let final_default = if default_value.is_empty() {
                    entries[0].value.clone()
                } else {
                    default_value
                };

                InlineKindData::Lookup {
                    input_id,
                    entries,
                    default_value: final_default,
                }
            }
            ForgeKind::Condition => {
                let expr = data
                    .attribute("expr")
                    .ok_or_else(|| {
                        locate(ValidationError::MissingAttribute {
                            element: format!("inline condition '{id}' <data>"),
                            attr: "expr".to_string(),
                        })
                    })?
                    .to_string();
                InlineKindData::Condition { expr }
            }
            ForgeKind::Codec => {
                let default_endian = sce_attr("default-endian")
                    .and_then(|s| Endian::from_attr(&s))
                    .unwrap_or(Endian::Big);

                let mut fields = Vec::new();
                for child in data.children().filter(|n| n.is_element()) {
                    if child.tag_name().name() == "field"
                        && child.tag_name().namespace() == Some(SCE_NAMESPACE)
                    {
                        // `parse_codec_field_from_node` already returns
                        // `Located<ForgeError>`. Propagate it through
                        // `?` so the codec parser's row/col data
                        // reaches the wire contract; unwrapping it
                        // here would discard location information
                        // the leaf already computed.
                        fields.push(crate::forge::parser::parse_codec_field_from_node(
                            &child,
                            "<inline codec>",
                        )?);
                    }
                }
                if fields.is_empty() {
                    return Err(locate(ValidationError::EmptyCollection {
                        kind: ForgeKind::Codec,
                        what: format!("<sce:field> (inline codec '{id}')"),
                    }));
                }

                InlineKindData::Codec {
                    fields,
                    default_endian,
                }
            }
            ForgeKind::Transform => {
                let expr = data
                    .attribute("expr")
                    .ok_or_else(|| {
                        locate(ValidationError::MissingAttribute {
                            element: format!("inline transform '{id}' <data>"),
                            attr: "expr".to_string(),
                        })
                    })?
                    .to_string();
                let type_str = sce_attr("type").ok_or_else(|| {
                    locate(ValidationError::MissingAttribute {
                        element: format!("inline transform '{id}' <data>"),
                        attr: "sce:type".to_string(),
                    })
                })?;
                let output_type = SceType::from_attr(&type_str).ok_or_else(|| {
                    locate(ValidationError::InvalidAttribute {
                        element: format!("inline transform '{id}' <data>"),
                        attr: "sce:type".to_string(),
                        value: type_str.clone(),
                        expected: "uint8|uint16|uint32|uint64|int8|int16|int32|int64|float32|float64|bool|string|bytes"
                            .to_string(),
                    })
                })?;

                InlineKindData::Transform {
                    inputs: Vec::new(),
                    expr,
                    output_type,
                }
            }
            _ => return Ok(None),
        };

        Ok(Some(InlineKind {
            id,
            data: inline_data,
        }))
    }

    fn parse_global_scripts(
        &mut self,
        root: &roxmltree::Node,
        model: &mut SCXMLModel,
        base_dir: Option<&Path>,
    ) {
        for child in root.children() {
            if !child.is_element() || local_name(&child) != "script" {
                continue;
            }
            let src = child.attribute("src").unwrap_or("").to_string();
            let mut content = child.text().unwrap_or("").to_string();

            // W3C SCXML 5.8: Empty <script/> → document rejection
            if src.is_empty() && content.trim().is_empty() {
                model.document_rejected = true;
                continue;
            }

            if !src.is_empty() {
                if let Some(dir) = base_dir {
                    let normalized = src
                        .strip_prefix("file://")
                        .or_else(|| src.strip_prefix("file:"))
                        .unwrap_or(&src);
                    let script_path = dir.join(normalized);
                    match std::fs::read_to_string(&script_path) {
                        Ok(c) => content = c,
                        Err(_) => {
                            model.document_rejected = true;
                            continue;
                        }
                    }
                } else {
                    // No filesystem access (WASM) — skip external scripts
                    model.needs_script_engine = true;
                    continue;
                }
            }

            model.global_scripts.push(Action {
                action_type: "script".to_string(),
                content: content.trim().to_string(),
                ..Default::default()
            });
            model.needs_script_engine = true;
        }
    }

    /// Threaded `source_name` lets leaf validation errors
    /// (`parse_invoke` mesh-rpc reserved-param rules) construct
    /// `Located<ForgeError>` records with the same file label the
    /// top-level parse used, without reaching back to `model.name` at
    /// every call site.
    fn parse_states(
        &mut self,
        parent_elem: &roxmltree::Node,
        parent_id: Option<&str>,
        model: &mut SCXMLModel,
        base_dir: Option<&Path>,
        source_name: &str,
    ) -> Result<(), crate::forge::error::Located<crate::forge::error::ForgeError>> {
        // Parse <state> elements
        for child in scxml_children(parent_elem, "state") {
            let state_id = match child.attribute("id") {
                Some(id) => id.to_string(),
                None => continue,
            };

            let mut state = State {
                id: state_id.clone(),
                initial: child.attribute("initial").unwrap_or("").to_string(),
                parent: parent_id.map(|s| s.to_string()),
                document_order: self.document_order_counter,
                ..Default::default()
            };
            self.document_order_counter += 1;

            // Parse transitions
            for trans_elem in scxml_children(&child, "transition") {
                let transition = self.parse_transition(&trans_elem, model)?;
                // Collect event names
                if !transition.event.is_empty() {
                    let ev = &transition.event;
                    if ev != "*" && ev != ".*" && ev != "_*" {
                        for e in ev.split_whitespace() {
                            if e != "*" && e != ".*" && e != "_*" && !e.ends_with(".*") {
                                model.events.insert(e.to_string());
                            }
                        }
                    }
                }
                state.transitions.push(transition);
            }

            // Parse onentry blocks
            for entry_elem in scxml_children(&child, "onentry") {
                let block = self.parse_executable_content(&entry_elem, model)?;
                if !block.is_empty() {
                    state.on_entry_blocks.push(block);
                }
            }

            // Parse onexit blocks
            for exit_elem in scxml_children(&child, "onexit") {
                let block = self.parse_executable_content(&exit_elem, model)?;
                if !block.is_empty() {
                    state.on_exit_blocks.push(block);
                }
            }

            // Parse <initial> transition
            if let Some(initial_elem) = scxml_child(&child, "initial") {
                if let Some(initial_trans) = scxml_child(&initial_elem, "transition") {
                    state.initial_transition_actions =
                        self.parse_executable_content(&initial_trans, model)?;
                    if state.initial.is_empty() {
                        if let Some(target) = initial_trans.attribute("target") {
                            state.initial = target.to_string();
                        }
                    }
                }
            }

            // Parse state-level datamodel
            for dm_elem in scxml_children(&child, "datamodel") {
                for data in scxml_children(&dm_elem, "data") {
                    let content = data.text().unwrap_or("").trim().to_string();
                    state.datamodel.push(Variable {
                        id: data.attribute("id").unwrap_or("").to_string(),
                        expr: data.attribute("expr").unwrap_or("").to_string(),
                        src: data.attribute("src").unwrap_or("").to_string(),
                        content,
                        var_type: String::new(),
                    });
                }
            }

            // Parse invokes — parse_invoke returns the typed Invoke variant
            // directly. Unsupported classifications yield `Ok(None)` and
            // are skipped silently; reserved-param violations on
            // `<invoke type="sce:mesh-rpc">` short-circuit via `?`.
            for invoke_elem in scxml_children(&child, "invoke") {
                if let Some(invoke) =
                    self.parse_invoke(&invoke_elem, model, &state_id, base_dir, source_name)?
                {
                    state.invokes.push(invoke);
                }
            }

            model.states.insert(state_id.clone(), state);

            // Recurse into child states
            self.parse_states(&child, Some(&state_id), model, base_dir, source_name)?;

            // Set default initial (first child in document order)
            let state = model.states.get_mut(&state_id).unwrap();
            if state.initial.is_empty() {
                let first_child = model
                    .states
                    .iter()
                    .filter(|(_, s)| s.parent.as_deref() == Some(&state_id))
                    .filter(|(id, _)| !model.history_states.contains_key(id.as_str()))
                    .min_by_key(|(_, s)| s.document_order);
                if let Some((child_id, _)) = first_child {
                    let child_id = child_id.clone();
                    model.states.get_mut(&state_id).unwrap().initial = child_id;
                }
            }
        }

        // Parse <final> elements
        for child in scxml_children(parent_elem, "final") {
            let final_id = match child.attribute("id") {
                Some(id) => id.to_string(),
                None => continue,
            };

            let mut state = State {
                id: final_id.clone(),
                is_final: true,
                parent: parent_id.map(|s| s.to_string()),
                document_order: self.document_order_counter,
                ..Default::default()
            };
            self.document_order_counter += 1;

            for entry_elem in scxml_children(&child, "onentry") {
                let block = self.parse_executable_content(&entry_elem, model)?;
                if !block.is_empty() {
                    state.on_entry_blocks.push(block);
                }
            }
            for exit_elem in scxml_children(&child, "onexit") {
                let block = self.parse_executable_content(&exit_elem, model)?;
                if !block.is_empty() {
                    state.on_exit_blocks.push(block);
                }
            }

            // Parse donedata
            if let Some(dd_elem) = scxml_child(&child, "donedata") {
                state.donedata = Some(self.parse_donedata(&dd_elem, model));
            }

            model.states.insert(final_id, state);
        }

        // Parse <parallel> elements
        for child in scxml_children(parent_elem, "parallel") {
            let parallel_id = match child.attribute("id") {
                Some(id) => id.to_string(),
                None => continue,
            };

            let mut state = State {
                id: parallel_id.clone(),
                is_parallel: true,
                parent: parent_id.map(|s| s.to_string()),
                document_order: self.document_order_counter,
                ..Default::default()
            };
            self.document_order_counter += 1;

            for trans_elem in scxml_children(&child, "transition") {
                let transition = self.parse_transition(&trans_elem, model)?;
                if !transition.event.is_empty() {
                    let ev = &transition.event;
                    if ev != "*" && ev != ".*" && ev != "_*" {
                        for e in ev.split_whitespace() {
                            if e != "*" && e != ".*" && e != "_*" && !e.ends_with(".*") {
                                model.events.insert(e.to_string());
                            }
                        }
                    }
                }
                state.transitions.push(transition);
            }

            for entry_elem in scxml_children(&child, "onentry") {
                let block = self.parse_executable_content(&entry_elem, model)?;
                if !block.is_empty() {
                    state.on_entry_blocks.push(block);
                }
            }
            for exit_elem in scxml_children(&child, "onexit") {
                let block = self.parse_executable_content(&exit_elem, model)?;
                if !block.is_empty() {
                    state.on_exit_blocks.push(block);
                }
            }

            model.states.insert(parallel_id.clone(), state);
            model.has_parallel_states = true;
            self.parse_states(&child, Some(&parallel_id), model, base_dir, source_name)?;
        }

        // Parse <history> elements
        for child in scxml_children(parent_elem, "history") {
            let history_id = match child.attribute("id") {
                Some(id) => id.to_string(),
                None => continue,
            };
            let history_type = child.attribute("type").unwrap_or("shallow").to_string();

            let mut default_target = String::new();
            let mut default_actions = Vec::new();
            for trans_elem in scxml_children(&child, "transition") {
                if let Some(target) = trans_elem.attribute("target") {
                    default_target = target.to_string();
                    default_actions = self.parse_executable_content(&trans_elem, model)?;
                    break;
                }
            }

            if !default_target.is_empty() {
                model
                    .history_default_targets
                    .insert(history_id.clone(), default_target.clone());
                model.history_states.insert(
                    history_id,
                    HistoryInfo {
                        parent: parent_id.unwrap_or("").to_string(),
                        history_type,
                        leaf_target: String::new(), // resolved later
                        default_target,
                        default_actions,
                    },
                );
                model.has_history_states = true;
            }
        }
        Ok(())
    }

    fn parse_transition(
        &mut self,
        elem: &roxmltree::Node,
        model: &mut SCXMLModel,
    ) -> Result<Transition, crate::forge::error::Located<crate::forge::error::ForgeError>>
    {
        let cond = elem
            .attribute("cond")
            .or_else(|| elem.attribute("expr"))
            .unwrap_or("")
            .to_string();

        let mut is_cpp_condition = false;
        let mut is_kt_condition = false;
        let mut is_pure_in = false;
        let mut cond_cpp = String::new();
        let mut cond_cpp_transformed = String::new();
        let mut cond_kt = String::new();

        if let Some(stripped) = cond.strip_prefix("cpp:") {
            is_cpp_condition = true;
            cond_cpp = stripped.to_string();
            cond_cpp_transformed = if !model.context_object_ids.is_empty() {
                transform_cpp_code_with_named_contexts(&cond_cpp, &model.context_object_ids)
            } else {
                cond_cpp.clone()
            };
        } else if let Some(stripped) = cond.strip_prefix("kt:") {
            is_kt_condition = true;
            cond_kt = if !model.context_object_ids.is_empty() {
                transform_kt_code_with_named_contexts(&stripped.to_string(), &model.context_object_ids)
            } else {
                stripped.to_string()
            };
        } else if !cond.is_empty() && is_pure_in_predicate(&cond) {
            is_pure_in = true;
            cond_cpp = convert_in_to_cpp(&cond);
            cond_kt = convert_in_to_kotlin(&cond);
        }

        let mut transition = Transition {
            event: elem.attribute("event").unwrap_or("").to_string(),
            target: elem.attribute("target").unwrap_or("").to_string(),
            cond,
            cond_cpp,
            cond_cpp_transformed,
            is_pure_in_predicate: is_pure_in,
            is_cpp_condition,
            cond_kt,
            is_kt_condition,
            transition_type: elem.attribute("type").unwrap_or("external").to_string(),
            ..Default::default()
        };

        transition.actions = self.parse_executable_content(elem, model)?;

        // Detect guard conditions requiring script engine and In() predicate
        if !transition.cond.is_empty() && !transition.is_cpp_condition && !transition.is_kt_condition {
            let (needs_se, has_in) = check_expression_needs(&transition.cond);
            if needs_se {
                model.needs_script_engine = true;
            }
            if has_in {
                model.uses_in_predicate = true;
            }
        }

        Ok(transition)
    }

    fn parse_executable_content(
        &mut self,
        parent: &roxmltree::Node,
        model: &mut SCXMLModel,
    ) -> Result<Vec<Action>, crate::forge::error::Located<crate::forge::error::ForgeError>>
    {
        let mut actions = Vec::new();
        for child in parent.children() {
            if let Some(action) = self.parse_executable_content_single(&child, model)? {
                actions.push(action);
            }
        }
        Ok(actions)
    }

    fn parse_send_action(
        &mut self,
        elem: &roxmltree::Node,
        action: &mut Action,
        model: &mut SCXMLModel,
    ) -> Result<(), crate::forge::error::Located<crate::forge::error::ForgeError>> {
        action.event = elem.attribute("event").unwrap_or("").to_string();
        action.eventexpr = elem.attribute("eventexpr").unwrap_or("").to_string();
        action.target = elem.attribute("target").unwrap_or("").to_string();
        action.targetexpr = elem.attribute("targetexpr").unwrap_or("").to_string();
        action.send_type = elem.attribute("type").unwrap_or("").to_string();
        action.typeexpr = elem.attribute("typeexpr").unwrap_or("").to_string();
        action.delay = elem.attribute("delay").unwrap_or("").to_string();
        action.delayexpr = elem.attribute("delayexpr").unwrap_or("").to_string();
        action.delay_ms = parse_delay_to_ms(&action.delay);
        action.id = elem.attribute("id").unwrap_or("").to_string();
        action.idlocation = elem.attribute("idlocation").unwrap_or("").to_string();
        action.namelist = elem.attribute("namelist").unwrap_or("").to_string();

        if action.id.is_empty() {
            action.auto_send_id = format!("__send_{}", self.send_counter);
            self.send_counter += 1;
        } else {
            action.auto_send_id = action.id.clone();
        }

        if action.target == "#_parent" {
            model.has_parent_communication = true;
        } else if action.target == "#_child" {
            model.has_child_communication = true;
        }

        if !action.namelist.is_empty() {
            model.needs_script_engine = true;
        }

        // Parse <param> children
        for param_elem in scxml_children(elem, "param") {
            let param_expr = param_elem.attribute("expr").unwrap_or("").to_string();
            let is_static_literal = is_static_string_literal(&param_expr);
            let static_value = if is_static_literal {
                extract_static_string_literal(&param_expr)
            } else {
                String::new()
            };
            if !param_expr.is_empty() && !is_static_literal {
                model.needs_script_engine = true;
            }
            action.params.push(Param {
                name: param_elem.attribute("name").unwrap_or("").to_string(),
                expr: param_expr,
                location: param_elem.attribute("location").unwrap_or("").to_string(),
                is_static_literal,
                static_value,
            });
        }

        // Parse <content>
        if let Some(content_elem) = scxml_child(elem, "content") {
            action.contentexpr = content_elem.attribute("expr").unwrap_or("").to_string();
            if content_elem.children().any(|c| c.is_element()) {
                let mut xml = String::new();
                for c in content_elem.children().filter(|c| c.is_element()) {
                    xml.push_str(&serialize_node(&c));
                }
                action.content = xml.trim().to_string();
            } else {
                action.content = content_elem.text().unwrap_or("").trim().to_string();
            }
        }

        // Dynamic expressions
        if !action.eventexpr.is_empty()
            || !action.targetexpr.is_empty()
            || !action.delayexpr.is_empty()
            || !action.typeexpr.is_empty()
        {
            model.has_dynamic_expressions = true;
            model.needs_script_engine = true;
        }

        if !action.targetexpr.is_empty() {
            model.events.insert("error.communication".to_string());
        }

        if !action.event.is_empty() {
            model.events.insert(action.event.clone());
        } else if !action.content.is_empty() && action.eventexpr.is_empty() {
            // W3C SCXML C.2: content-only send (test 520) - empty event name
            model.events.insert(String::new());
        }

        // BasicHTTP send detection
        if action.send_type == "http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor"
            && (action.target.starts_with("http://") || action.target.starts_with("https://"))
        {
            model.needs_http_send = true;
        }

        // SCE_MESH.md §13 path B — SCXML purity: sce:qos / sce:pattern /
        // sce:reply-event / sce:reply-timeout / sce:deadline / sce:priority
        // attributes on <send> were removed. Stage 2 enforcement: presence
        // of any removed attribute is a hard build error
        // (`ValidationError::RemovedAttribute`). No migration tolerance
        // remains — third-party documents must migrate before building
        // against Session E1 or later.
        use crate::forge::error::{Located, ValidationError};
        use crate::forge::model::SCE_NAMESPACE;
        for removed_attr in [
            "qos",
            "pattern",
            "reply-event",
            "reply-timeout",
            "deadline",
            "priority",
        ] {
            if elem.attribute((SCE_NAMESPACE, removed_attr)).is_some() {
                let pos = elem.document().text_pos_at(elem.range().start);
                return Err(Located::new(
                    ValidationError::RemovedAttribute {
                        attribute: format!("sce:{removed_attr}"),
                        event: if action.event.is_empty() {
                            None
                        } else {
                            Some(action.event.clone())
                        },
                    }
                    .into(),
                    &model.name,
                    Some(pos.row),
                    Some(pos.col),
                ));
            }
        }
        Ok(())
    }

    fn parse_if_action(
        &mut self,
        elem: &roxmltree::Node,
        action: &mut Action,
        model: &mut SCXMLModel,
    ) -> Result<(), crate::forge::error::Located<crate::forge::error::ForgeError>> {
        let cond = elem.attribute("cond").unwrap_or("").to_string();
        let mut is_pure_in = false;
        let mut cond_cpp = String::new();
        let mut cond_kt = String::new();
        if !cond.is_empty() {
            let (needs_se, has_in) = check_expression_needs(&cond);
            if needs_se {
                model.needs_script_engine = true;
            }
            if has_in {
                model.uses_in_predicate = true;
            }
            if is_pure_in_predicate(&cond) {
                is_pure_in = true;
                cond_cpp = convert_in_to_cpp(&cond);
                cond_kt = convert_in_to_kotlin(&cond);
            }
        }

        action.cond = cond;
        action.cond_cpp = cond_cpp;
        action.cond_kt = cond_kt;
        action.is_pure_in_predicate = is_pure_in;

        let mut current_branch: usize = 0; // 0 = then, 1+ = elseif, usize::MAX = else
        for child in elem.children() {
            if !child.is_element() {
                continue;
            }
            let tag = local_name(&child);
            match tag.as_str() {
                "elseif" => {
                    let ei_cond = child.attribute("cond").unwrap_or("").to_string();
                    if !ei_cond.is_empty() {
                        let (needs_se, has_in) = check_expression_needs(&ei_cond);
                        if needs_se { model.needs_script_engine = true; }
                        if has_in { model.uses_in_predicate = true; }
                    }
                    let ei_pure_in = !ei_cond.is_empty() && is_pure_in_predicate(&ei_cond);
                    let ei_cpp = if ei_pure_in { convert_in_to_cpp(&ei_cond) } else { String::new() };
                    let ei_kt = if ei_pure_in { convert_in_to_kotlin(&ei_cond) } else { String::new() };
                    action.elseif_branches.push(ElseIfBranch {
                        cond: ei_cond,
                        cond_cpp: ei_cpp,
                        cond_kt: ei_kt,
                        is_pure_in_predicate: ei_pure_in,
                        actions: Vec::new(),
                    });
                    current_branch = action.elseif_branches.len(); // 1-indexed
                }
                "else" => {
                    current_branch = usize::MAX;
                }
                _ => {
                    // Parse the nested action
                    let nested_actions = self.parse_executable_content_single(&child, model)?;
                    if let Some(nested) = nested_actions {
                        match current_branch {
                            0 => action.then_actions.push(nested),
                            usize::MAX => action.else_actions.push(nested),
                            n => {
                                if let Some(branch) = action.elseif_branches.get_mut(n - 1) {
                                    branch.actions.push(nested);
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn parse_executable_content_single(
        &mut self,
        child: &roxmltree::Node,
        model: &mut SCXMLModel,
    ) -> Result<
        Option<Action>,
        crate::forge::error::Located<crate::forge::error::ForgeError>,
    > {
        if !child.is_element() {
            return Ok(None);
        }
        let tag = local_name(child);
        let mut action = Action {
            action_type: tag.clone(),
            ..Default::default()
        };
        match tag.as_str() {
            "raise" => {
                action.event = child.attribute("event").unwrap_or("").to_string();
                if !action.event.is_empty() {
                    model.events.insert(action.event.clone());
                }
            }
            "send" => self.parse_send_action(child, &mut action, model)?,
            "assign" => {
                action.location = child.attribute("location").unwrap_or("").to_string();
                action.expr = child.attribute("expr").unwrap_or("").to_string();
                if child.children().any(|c| c.is_element()) {
                    // W3C SCXML 5.4: Serialize with c14n (canonical XML)
                    let mut xml = String::new();
                    for c in child.children().filter(|c| c.is_element()) {
                        let part = serialize_node_c14n(&c);
                        let normalized: Vec<&str> = part.split_whitespace().collect();
                        xml.push_str(&normalized.join(" "));
                    }
                    action.content = xml;
                }
                model.needs_script_engine = true;
            }
            "log" => {
                action.label = child.attribute("label").unwrap_or("").to_string();
                action.expr = child.attribute("expr").unwrap_or("").to_string();
                if !action.expr.is_empty() {
                    model.needs_script_engine = true;
                }
            }
            "script" => {
                // Check for <cpp> or <kt> native code child elements (same as parse_executable_content)
                let mut found_native = false;
                for sc in child.children().filter(|n| n.is_element()) {
                    let sc_name = sc.tag_name().name();
                    if sc_name == "cpp" || sc.tag_name().namespace() == Some("urn:sce:cpp") {
                        let cpp_code = sc.text().unwrap_or("").to_string();
                        action.is_cpp_function = true;
                        action.content = cpp_code.clone();
                        action.content_transformed = if !model.context_object_ids.is_empty() {
                            transform_cpp_code_with_named_contexts(&cpp_code, &model.context_object_ids)
                        } else {
                            cpp_code
                        };
                        found_native = true;
                        break;
                    } else if sc_name == "kt" || sc.tag_name().namespace() == Some("urn:sce:kotlin") {
                        let kt_code = sc.text().unwrap_or("").to_string();
                        action.is_kt_function = true;
                        action.content = kt_code.clone();
                        action.content_kt = if !model.context_object_ids.is_empty() {
                            transform_kt_code_with_named_contexts(&kt_code, &model.context_object_ids)
                        } else {
                            kt_code
                        };
                        found_native = true;
                        break;
                    }
                }
                if !found_native {
                    action.content = child.text().unwrap_or("").to_string();
                    model.needs_script_engine = true;
                }
            }
            "cancel" => {
                action.sendid = child.attribute("sendid").unwrap_or("").to_string();
                action.sendidexpr = child.attribute("sendidexpr").unwrap_or("").to_string();
                if !action.sendidexpr.is_empty() {
                    model.needs_script_engine = true;
                }
            }
            "foreach" => {
                model.needs_script_engine = true;
                action.array = child.attribute("array").unwrap_or("").to_string();
                action.item = child.attribute("item").unwrap_or("").to_string();
                action.index = child.attribute("index").unwrap_or("").to_string();
                action.actions = self.parse_executable_content(child, model)?;
            }
            "if" => self.parse_if_action(child, &mut action, model)?,
            _ => return Ok(None),
        }
        Ok(Some(action))
    }

    /// W3C SCXML 6.4: Parse `<invoke>` into the typed [`Invoke`] sum.
    ///
    /// Returns `None` if the element neither resolves to a static SCXML
    /// session (`src` / inline `<content><scxml>`) nor to a hybrid session
    /// (`srcexpr` / `contentexpr`). The caller pushes the result directly
    /// onto `state.invokes`; there is no intermediate JSON representation.
    /// Parse a single `<invoke>` element into a typed [`Invoke`] variant.
    ///
    /// Returns `Ok(Some(_))` for a successful classification (scxml,
    /// hybrid, or sce:mesh-rpc), `Ok(None)` for unsupported invoke
    /// types that the parser skips silently, and `Err(_)` for SCE
    /// Mesh §9.5 reserved-param rule violations on
    /// `<invoke type="sce:mesh-rpc">`. The error surfaces the source
    /// label (`source_name`) and roxmltree row/col so downstream
    /// NDJSON diagnostics land with the same precision as every other
    /// parser-stage failure.
    fn parse_invoke(
        &mut self,
        elem: &roxmltree::Node,
        model: &mut SCXMLModel,
        state_id: &str,
        base_dir: Option<&Path>,
        source_name: &str,
    ) -> Result<
        Option<Invoke>,
        crate::forge::error::Located<crate::forge::error::ForgeError>,
    > {
        // W3C SCXML 6.4.1: Generate invoke ID if not provided. Auto-ids carry
        // a leading underscore by spec convention; templates building
        // identifiers (`child_<suffix>`) consume `field_suffix` instead so the
        // leading underscore does not double up.
        let mut invoke_id = elem.attribute("id").unwrap_or("").to_string();
        if invoke_id.is_empty() {
            invoke_id = format!("_invoke_{}", self.invoke_counter);
            self.invoke_counter += 1;
        }
        let field_suffix = invoke_id.trim_start_matches('_').to_string();

        let invoke_type = elem.attribute("type").unwrap_or("").to_string();
        let src = elem.attribute("src").unwrap_or("").to_string();
        let srcexpr = elem.attribute("srcexpr").unwrap_or("").to_string();
        let idlocation = elem.attribute("idlocation").unwrap_or("").to_string();
        let autoforward = elem.attribute("autoforward").unwrap_or("false") == "true";
        let namelist = elem.attribute("namelist").unwrap_or("").to_string();

        // SCE Mesh §9.5: <invoke type="sce:mesh-rpc"> — short-lived RPC
        // layered on W3C invoke lifecycle. Parsed through a dedicated
        // path because reserved `_mesh_*` `<param>` names are structural
        // metadata (strip from payload, populate envelope fields) rather
        // than author data, and the static/hybrid classifier below
        // would otherwise silently drop the invoke (scxml_type=false).
        if invoke_type == "sce:mesh-rpc" {
            return self
                .parse_mesh_rpc_invoke(elem, state_id, source_name, invoke_id, field_suffix, src, idlocation)
                .map(|info| Some(Invoke::MeshRpc(info)));
        }

        let mut contentexpr = String::new();
        let mut has_inline_scxml = false;
        let mut inline_scxml_text = String::new();

        // Parse inline <content>
        if let Some(content_elem) = scxml_child(elem, "content") {
            contentexpr = content_elem.attribute("expr").unwrap_or("").to_string();

            // Check for inline <scxml> child element (static content)
            if let Some(scxml_child_elem) = scxml_child(&content_elem, "scxml") {
                has_inline_scxml = true;
                // Extract inline SCXML text via roxmltree range
                let doc_text = scxml_child_elem.document().input_text();
                let range = scxml_child_elem.range();
                inline_scxml_text = doc_text[range].to_string();
            }
        }

        // Parse <param> children. Static invokes retain a static-literal
        // optimisation flag so codegen can inline literal values.
        let mut static_params: Vec<Param> = Vec::new();
        let mut hybrid_params: Vec<Param> = Vec::new();
        for param in scxml_children(elem, "param") {
            let name = param.attribute("name").unwrap_or("").to_string();
            let expr = param.attribute("expr").unwrap_or("").to_string();
            let location = param.attribute("location").unwrap_or("").to_string();
            let is_sl = is_static_string_literal(&expr);
            static_params.push(Param {
                name: name.clone(),
                expr: expr.clone(),
                location: location.clone(),
                is_static_literal: is_sl,
                static_value: if is_sl {
                    extract_static_string_literal(&expr)
                } else {
                    String::new()
                },
            });
            hybrid_params.push(Param {
                name,
                expr,
                location,
                ..Default::default()
            });
        }

        // Parse <finalize>
        let mut finalize_content = String::new();
        if let Some(finalize_elem) = scxml_child(elem, "finalize") {
            let finalize_actions = self.parse_executable_content(&finalize_elem, model)?;
            finalize_content = actions_to_javascript(&finalize_actions);
        }

        // W3C SCXML 6.4: Classify invoke type
        let has_static_child = !src.is_empty() || has_inline_scxml;
        let scxml_type = invoke_type.is_empty()
            || invoke_type == "scxml"
            || invoke_type == "http://www.w3.org/TR/scxml"
            || invoke_type == "http://www.w3.org/TR/scxml/";

        let is_static_invoke =
            scxml_type && srcexpr.is_empty() && contentexpr.is_empty() && has_static_child;
        let is_hybrid_invoke = scxml_type && (!srcexpr.is_empty() || !contentexpr.is_empty());

        if is_hybrid_invoke {
            model.needs_script_engine = true;
            let idx = self.hybrid_invoke_counter;
            self.hybrid_invoke_counter += 1;
            return Ok(Some(Invoke::Hybrid(HybridInvokeInfo {
                common: InvokeSessionCommon {
                    base: InvokeBase {
                        invoke_id,
                        field_suffix,
                        state_name: state_id.to_string(),
                        params: hybrid_params,
                        idlocation,
                    },
                    child_name: format!("{}_hybrid{idx}", model.name),
                    autoforward,
                    ..Default::default()
                },
                srcexpr,
                contentexpr,
            })));
        }

        if is_static_invoke {
            // W3C SCXML 6.4.1: Namelist validation requires script engine
            if !namelist.is_empty() {
                model.needs_script_engine = true;
            }

            // W3C SCXML 6.4: Inline `<content><scxml>` and external `src="..."`
            // resolve to a concrete child SCXML path eagerly, at parse time.
            // This way `ScxmlInvokeInfo` never holds "raw, awaiting
            // extraction" state; by the time it is pushed onto the state the
            // type already matches the invariant the codegen expects.
            //
            // `base_dir == None` means WASM-style parse with no filesystem
            // access; inline extraction is skipped and the invoke surfaces
            // with empty child_name, which downstream codegen rejects with a
            // clear diagnostic. Behaviourally identical to the pre-R9 path.
            let (resolved_src, resolved_child_name) =
                if has_inline_scxml && !inline_scxml_text.is_empty() {
                    if let Some(scxml_dir) = base_dir {
                        let child_name = extract_inline_child_name(
                            &inline_scxml_text,
                            &model.name,
                            &mut self.inline_child_counter,
                        );
                        let child_scxml_path =
                            scxml_dir.join(format!("{child_name}.scxml"));
                        let inline_with_ns = if !inline_scxml_text.contains("xmlns=") {
                            inline_scxml_text.replacen(
                                "<scxml",
                                "<scxml xmlns=\"http://www.w3.org/2005/07/scxml\"",
                                1,
                            )
                        } else {
                            inline_scxml_text.clone()
                        };
                        let xml_content =
                            format!("<?xml version=\"1.0\"?>\n\n{inline_with_ns}");
                        if let Err(e) = std::fs::write(&child_scxml_path, &xml_content) {
                            eprintln!(
                                "Warning: Cannot write inline SCXML {}: {e}",
                                child_scxml_path.display()
                            );
                        }
                        (format!("{child_name}.scxml"), child_name)
                    } else {
                        (src.clone(), String::new())
                    }
                } else if !src.is_empty() {
                    let stripped = src.replace("file:", "");
                    let child_name = Path::new(&stripped)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    (src.clone(), child_name)
                } else {
                    (src.clone(), String::new())
                };

            let mut scxml_info = ScxmlInvokeInfo {
                common: InvokeSessionCommon {
                    base: InvokeBase {
                        invoke_id,
                        field_suffix,
                        state_name: state_id.to_string(),
                        params: static_params,
                        idlocation,
                    },
                    child_name: resolved_child_name,
                    autoforward,
                    ..Default::default()
                },
                finalize_content,
                src: resolved_src,
                namelist,
            };

            // Populate child-side metadata (script-engine flag, datamodel
            // variable list) when the resolved child SCXML file exists.
            if let Some(scxml_dir) = base_dir {
                if !scxml_info.common.child_name.is_empty() {
                    let child_scxml_path = scxml_dir
                        .join(format!("{}.scxml", scxml_info.common.child_name));
                    parse_child_metadata(&child_scxml_path, &mut scxml_info.common);
                }
            }

            return Ok(Some(Invoke::Scxml(scxml_info)));
        }

        // Neither static, hybrid, nor sce:mesh-rpc — skip silently.
        // Unknown `type` URIs are documented in W3C §6.4.1 as producing
        // `error.execution` at runtime on foreign processors; the
        // parser declines to statically reject them so forward-compatible
        // documents still parse.
        Ok(None)
    }

    /// SCE Mesh §9.5: parse an `<invoke type="sce:mesh-rpc">` element.
    ///
    /// Enforces the reserved-`_mesh_*` `<param>` rules at parse time
    /// (missing required, duplicate, unknown prefix, invalid deadline)
    /// and strips the reserved names from the payload passed to the
    /// envelope. All four rule variants surface as the same
    /// [`ValidationError::MeshRpcReservedParam`] variant — the repair
    /// surface is uniform ("rename or retype your `<param>`"), and a
    /// single `DiagnosticCode` keeps the wire catalog tight.
    fn parse_mesh_rpc_invoke(
        &mut self,
        elem: &roxmltree::Node,
        state_id: &str,
        source_name: &str,
        invoke_id: String,
        field_suffix: String,
        src: String,
        idlocation: String,
    ) -> Result<
        MeshRpcInvokeInfo,
        crate::forge::error::Located<crate::forge::error::ForgeError>,
    > {
        use crate::forge::error::{Located, ValidationError};

        let locate = |err: ValidationError| -> Located<crate::forge::error::ForgeError> {
            let pos = elem.document().text_pos_at(elem.range().start);
            Located::new(err.into(), source_name, Some(pos.row), Some(pos.col))
        };

        // First pass: scan every `<param>` to detect reserved-name
        // violations and extract `_mesh_event` / `_mesh_deadline_ms`.
        // Only after all four rules pass do we construct the final
        // struct, so author errors report before the downstream
        // payload is assembled.
        let mut mesh_event: Option<String> = None;
        let mut mesh_event_count = 0usize;
        let mut deadline_ms: Option<u64> = None;
        let mut deadline_count = 0usize;
        let mut payload_params: Vec<Param> = Vec::new();

        for param in scxml_children(elem, "param") {
            let name = param.attribute("name").unwrap_or("").to_string();
            let expr = param.attribute("expr").unwrap_or("").to_string();
            let location = param.attribute("location").unwrap_or("").to_string();

            if name == "_mesh_event" {
                mesh_event_count += 1;
                // Rule 1: exactly-one. Even the first duplicate fires
                // eagerly so the diagnostic pinpoints the second
                // occurrence rather than waiting for the end of the
                // loop to count and lose the document order signal.
                if mesh_event_count > 1 {
                    return Err(locate(ValidationError::MeshRpcReservedParam {
                        param: "_mesh_event".into(),
                        detail: "<param name=\"_mesh_event\"> must appear exactly once"
                            .into(),
                    }));
                }
                mesh_event = Some(extract_static_string_literal(&expr));
            } else if name == "_mesh_deadline_ms" {
                deadline_count += 1;
                if deadline_count > 1 {
                    return Err(locate(ValidationError::MeshRpcReservedParam {
                        param: "_mesh_deadline_ms".into(),
                        detail:
                            "<param name=\"_mesh_deadline_ms\"> may appear at most once"
                                .into(),
                    }));
                }
                // §9.5: `_mesh_deadline_ms` is an integer in
                // milliseconds. The literal may be quoted (`expr="'50'"`)
                // or bare (`expr="50"`); both resolve to a non-negative
                // decimal integer string via the existing static-literal
                // extractor.
                let raw = if is_static_string_literal(&expr) {
                    extract_static_string_literal(&expr)
                } else {
                    expr.trim().to_string()
                };
                match raw.parse::<u64>() {
                    Ok(v) => deadline_ms = Some(v),
                    Err(_) => {
                        return Err(locate(ValidationError::MeshRpcReservedParam {
                            param: "_mesh_deadline_ms".into(),
                            detail: format!(
                                "value '{raw}' is not a non-negative integer (milliseconds)"
                            ),
                        }));
                    }
                }
            } else if name.starts_with("_mesh_") {
                return Err(locate(ValidationError::MeshRpcReservedParam {
                    param: name.clone(),
                    detail:
                        "unknown _mesh_* name is reserved for future envelope metadata"
                            .into(),
                }));
            } else {
                let is_sl = is_static_string_literal(&expr);
                payload_params.push(Param {
                    name,
                    expr: expr.clone(),
                    location,
                    is_static_literal: is_sl,
                    static_value: if is_sl {
                        extract_static_string_literal(&expr)
                    } else {
                        String::new()
                    },
                });
            }
        }

        let mesh_event = mesh_event.ok_or_else(|| {
            locate(ValidationError::MeshRpcReservedParam {
                param: "_mesh_event".into(),
                detail: "required <param name=\"_mesh_event\"> is missing".into(),
            })
        })?;

        Ok(MeshRpcInvokeInfo {
            base: InvokeBase {
                invoke_id,
                field_suffix,
                state_name: state_id.to_string(),
                params: payload_params,
                idlocation,
            },
            src,
            mesh_event,
            deadline_ms,
        })
    }

    fn parse_donedata(
        &mut self,
        elem: &roxmltree::Node,
        model: &mut SCXMLModel,
    ) -> DoneData {
        let mut dd = DoneData::default();

        // W3C SCXML 5.7: Parse <param> elements
        for child in scxml_children(elem, "param") {
            dd.params.push(DoneDataParam {
                name: child.attribute("name").unwrap_or("").to_string(),
                expr: child.attribute("expr").map(|s| s.to_string()),
                location: child.attribute("location").map(|s| s.to_string()),
            });
            // Donedata params require script engine
            model.needs_script_engine = true;
        }

        // W3C SCXML 5.5: Parse <content> element
        if let Some(content_elem) = scxml_child(elem, "content") {
            dd.contentexpr = content_elem.attribute("expr").unwrap_or("").to_string();
            if let Some(text) = content_elem.text() {
                dd.content = text.trim().to_string();
            }
            if !dd.contentexpr.is_empty() || !dd.content.is_empty() {
                model.needs_script_engine = true;
            }
        }

        dd
    }

    // ── Feature detection ────────────────────────────────

    fn detect_features(&mut self, model: &mut SCXMLModel) {
        for state in model.states.values() {
            if state.is_parallel {
                model.has_parallel_states = true;
            }
            if state.parent.is_some() {
                model.has_hierarchy = true;
            }
            // Note: needs_script_engine and uses_in_predicate are already set
            // during parse_transition via check_expression_needs. No redundant re-check.
            // W3C SCXML: detect _event.* usage in guard conditions (transitions + if/elseif)
            for trans in &state.transitions {
                if trans.cond.contains("_event.") {
                    model.has_event_metadata = true;
                }
                if actions_contain_event_metadata(&trans.actions) {
                    model.has_event_metadata = true;
                }
            }
            for block in state.on_entry_blocks.iter().chain(state.on_exit_blocks.iter()) {
                if actions_contain_event_metadata(block) {
                    model.has_event_metadata = true;
                }
            }
        }
    }

    fn resolve_deep_initial(&self, model: &mut SCXMLModel) {
        if model.initial.is_empty() {
            return;
        }
        // W3C SCXML 3.13: Check for space-separated parallel initial states
        let initial_states: Vec<&str> = model.initial.split_whitespace().collect();
        if initial_states.len() > 1 {
            // Multiple initial states (parallel entry) — verify all exist
            if initial_states.iter().all(|s| model.states.contains_key(*s)) {
                return; // Keep space-separated format
            }
            // Fallback: treat as single state
        }
        // Single initial state — resolve to leaf by following initial chain
        let mut current = initial_states[0].to_string();
        for _ in 0..20 {
            let state = match model.states.get(&current) {
                Some(s) => s,
                None => break,
            };
            if !state.initial.is_empty() && model.states.contains_key(&state.initial) {
                current = state.initial.clone();
            } else {
                break;
            }
        }
        // W3C SCXML 3.6: Update model.initial to the resolved leaf state
        model.initial = current;
    }

    fn apply_parallel_initial_overrides(&self, model: &mut SCXMLModel) {
        if model.initial.is_empty() {
            return;
        }
        let initial_states: Vec<String> =
            model.initial.split_whitespace().map(String::from).collect();
        if initial_states.len() <= 1 {
            return;
        }
        // W3C SCXML 3.6/3.13: Walk up from each target through ALL ancestors,
        // overriding compound states' initial to point to the child on the path.
        // Skip parallel states (they enter all children automatically).
        for state_id in &initial_states {
            if !model.states.contains_key(state_id) {
                continue;
            }
            let mut current = state_id.clone();
            loop {
                let parent_id = match model.states.get(&current).and_then(|s| s.parent.clone()) {
                    Some(p) if model.states.contains_key(&p) => p,
                    _ => break,
                };
                let is_parallel = model.states.get(&parent_id).map_or(false, |s| s.is_parallel);
                if !is_parallel {
                    if let Some(parent) = model.states.get_mut(&parent_id) {
                        parent.initial = current.clone();
                    }
                }
                current = parent_id;
            }
        }
        // After overrides, set model.initial to the first state
        model.initial = initial_states[0].clone();
    }

    fn resolve_history_targets(&self, model: &mut SCXMLModel) {
        // W3C SCXML 3.11: Resolve history default targets to leaf states
        let mut history_leaf_targets: BTreeMap<String, String> = BTreeMap::new();
        for (history_id, history_info) in &model.history_states {
            let default_target = &history_info.default_target;
            let leaf = model.resolve_to_leaf(default_target);
            history_leaf_targets.insert(history_id.clone(), leaf);
        }

        let history_defaults = model.history_default_targets.clone();
        for (_, state) in model.states.iter_mut() {
            // Resolve initial that points to history state
            if !state.initial.is_empty() {
                if let Some(default_target) = history_defaults.get(&state.initial) {
                    let history_id = state.initial.clone();
                    state.initial_history_id = history_id.clone();
                    state.initial_history_default_target = default_target.clone();
                    if let Some(info) = model.history_states.get(&history_id) {
                        state.initial_history_default_actions = info.default_actions.clone();
                    }
                    state.initial = default_target.clone();
                }
            }
            // Resolve transition targets that point to history states
            for trans in &mut state.transitions {
                if !trans.target.is_empty() {
                    if let Some(default_target) = history_defaults.get(&trans.target) {
                        trans.history_target = Some(trans.target.clone());
                        // W3C SCXML 3.11: Resolved leaf target for Kotlin Phase 1
                        trans.history_leaf_target = history_leaf_targets
                            .get(&trans.target)
                            .cloned();
                        trans.target = default_target.clone();
                    }
                }
            }
        }
    }

    fn compute_parallel_regions(&self, model: &mut SCXMLModel) {
        let parallel_ids: Vec<String> = model
            .states
            .iter()
            .filter(|(_, s)| s.is_parallel)
            .map(|(id, _)| id.clone())
            .collect();
        for pid in parallel_ids {
            let children: Vec<String> = model
                .states
                .iter()
                .filter(|(_, s)| s.parent.as_deref() == Some(&pid))
                .map(|(id, _)| id.clone())
                .collect();
            model.parallel_regions.insert(pid, children);
        }
    }

    fn detect_transition_actions(&self, model: &mut SCXMLModel) {
        for state in model.states.values() {
            for trans in &state.transitions {
                if !trans.actions.is_empty() {
                    model.has_transition_actions = true;
                    return;
                }
            }
        }
    }

    fn detect_entry_exit_actions(&self, model: &mut SCXMLModel) {
        for state in model.states.values() {
            if !model.has_entry_actions {
                if !state.on_entry_blocks.is_empty()
                    || state.has_scxml_invoke()
                    || state.has_hybrid_invoke()
                    || state.has_mesh_rpc_invoke()
                    || !state.datamodel.is_empty()
                    || !state.initial_transition_actions.is_empty()
                    || !state.initial_history_id.is_empty()
                    || (state.is_final && (state.donedata.is_some() || state.parent.is_some()))
                {
                    model.has_entry_actions = true;
                }
            }
            if !model.has_exit_actions {
                if !state.on_exit_blocks.is_empty()
                    || state.has_scxml_invoke()
                    || state.has_hybrid_invoke()
                    || state.has_mesh_rpc_invoke()
                {
                    model.has_exit_actions = true;
                }
            }
            if model.has_entry_actions && model.has_exit_actions {
                break;
            }
        }
    }

    fn detect_hierarchy(&self, model: &mut SCXMLModel) {
        model.has_hierarchy = model.states.values().any(|s| s.parent.is_some());
    }

    fn add_done_state_events(&self, model: &mut SCXMLModel) {
        let parent_ids_with_finals: Vec<String> = model
            .states
            .values()
            .filter(|s| s.is_final)
            .filter_map(|s| s.parent.clone())
            .collect();
        for parent_id in parent_ids_with_finals {
            let event_name = format!("done.state.{parent_id}");
            model.events.insert(event_name);
        }
    }

    /// W3C SCXML 6.4: Only add done.invoke.{id} events if transitions actually reference them.
    /// Matches Python _set_invoke_event_flags() behavior.
    fn set_invoke_event_flags(&self, model: &mut SCXMLModel) {
        // Build set of done.invoke.* events actually used in transitions
        let mut used_done_invoke_events = std::collections::BTreeSet::new();
        for state in model.states.values() {
            for trans in &state.transitions {
                if trans.event.starts_with("done.invoke.") {
                    used_done_invoke_events.insert(trans.event.clone());
                }
            }
        }

        // Set the use_specific_event flag on every Scxml/Hybrid invoke whose
        // id shows up in a `done.invoke.<id>` transition. The matching
        // specific event is also added to the model's event set so the
        // generated Event enum exposes it. `state.invokes` is the single
        // owner — no model-level mirror to keep in sync.
        let mut specific_events_to_add: Vec<String> = Vec::new();
        for state in model.states.values_mut() {
            for invoke in state.invokes.iter_mut() {
                let common: &mut InvokeSessionCommon = match invoke {
                    Invoke::Scxml(i) => &mut i.common,
                    Invoke::Hybrid(i) => &mut i.common,
                    Invoke::MeshRpc(_) => continue,
                };
                if common.invoke_id.is_empty() {
                    continue;
                }
                let specific = format!("done.invoke.{}", common.invoke_id);
                common.use_specific_event = used_done_invoke_events.contains(&specific);
                if common.use_specific_event {
                    specific_events_to_add.push(specific);
                }
            }
        }
        for ev in specific_events_to_add {
            model.events.insert(ev);
        }
    }

    /// W3C SCXML 6.2: Collect events from child state machines that send to parent (#_parent).
    /// Auto-adds child-to-parent events to parent Event enum for compile-time type safety.
    fn collect_child_to_parent_events(
        &self,
        model: &mut SCXMLModel,
        base_dir: Option<&Path>,
    ) {
        if !model.has_scxml_invoke() {
            return;
        }
        let scxml_dir = match base_dir {
            Some(dir) => dir.to_path_buf(),
            None => return, // No filesystem access (WASM)
        };
        let mut parsed_children: std::collections::HashSet<String> = std::collections::HashSet::new();

        let invokes: Vec<ScxmlInvokeInfo> = model
            .states
            .values()
            .flat_map(|s| s.iter_scxml_invokes().cloned())
            .collect();
        for si in &invokes {
            if si.child_name.is_empty() || parsed_children.contains(&si.child_name) {
                continue;
            }
            parsed_children.insert(si.child_name.clone());

            let child_scxml_path = scxml_dir.join(format!("{}.scxml", si.child_name));
            if !child_scxml_path.exists() {
                continue;
            }

            let child_model = match SCXMLParser::new().parse_file(&child_scxml_path.to_string_lossy()) {
                Ok(m) => m,
                Err(_) => continue,
            };

            // Scan child for <send target="#_parent" event="xxx"> actions
            let mut child_parent_events = std::collections::BTreeSet::new();
            for child_state in child_model.states.values() {
                // Check entry/exit actions
                for block in child_state.on_entry_blocks.iter().chain(child_state.on_exit_blocks.iter()) {
                    collect_parent_send_events(block, &mut child_parent_events);
                }
                // Check transition actions
                for trans in &child_state.transitions {
                    collect_parent_send_events(&trans.actions, &mut child_parent_events);
                }
                // Check initial transition actions
                collect_parent_send_events(&child_state.initial_transition_actions, &mut child_parent_events);
            }

            // Add collected events to parent's event set
            for event in child_parent_events {
                model.events.insert(event);
            }
        }
    }

    fn parse_initial_children(&self, model: &mut SCXMLModel) {
        // W3C SCXML 3.6: Parse initial attribute into list of children for ALL states
        for state in model.states.values_mut() {
            if !state.initial.is_empty() {
                state.initial_children =
                    state.initial.split_whitespace().map(String::from).collect();
            }
        }
    }

    // ── Named Context (sce:context) ─────────────────────────

    /// Parse <sce:context> elements for Named Context declarations.
    fn parse_sce_contexts(
        &self,
        root: &roxmltree::Node,
        model: &mut SCXMLModel,
        source_name: &str,
    ) -> Result<(), crate::forge::error::Located<crate::forge::error::ForgeError>> {
        use crate::forge::error::{Located, ValidationError};
        use crate::forge::model::SCE_NAMESPACE;
        for child in root.children().filter(|n| n.is_element()) {
            let is_sce_context =
                child.tag_name().namespace() == Some(SCE_NAMESPACE)
                    && child.tag_name().name() == "context";
            if !is_sce_context {
                continue;
            }
            let ctx_id = child
                .attribute("id")
                .ok_or_else(|| {
                    Located::new(
                        ValidationError::MissingAttribute {
                            element: "<sce:context>".to_string(),
                            attr: "id".to_string(),
                        }
                        .into(),
                        source_name,
                        None,
                        None,
                    )
                })?
                .to_string();
            if model.context_object_ids.contains(&ctx_id) {
                return Err(Located::new(
                    ValidationError::DuplicateContextObject { id: ctx_id }.into(),
                    source_name,
                    None,
                    None,
                ));
            }
            let cpp_type = child
                .attribute(("urn:sce:cpp", "type"))
                .unwrap_or("")
                .to_string();
            let cpp_include = child
                .attribute(("urn:sce:cpp", "include"))
                .unwrap_or("")
                .to_string();
            let kt_type = child
                .attribute(("urn:sce:kotlin", "type"))
                .unwrap_or("")
                .to_string();
            model.context_objects.push(ContextObject {
                id: ctx_id.clone(),
                cpp_type,
                cpp_include,
                kt_type,
            });
            model.context_object_ids.insert(ctx_id);
        }
        Ok(())
    }

    /// Validate that cpp:/kt: code referencing objects has <sce:context> declarations.
    fn validate_context_usage(
        &self,
        model: &SCXMLModel,
        source_name: &str,
    ) -> Result<(), crate::forge::error::Located<crate::forge::error::ForgeError>> {
        use crate::forge::error::{Located, ValidationError};
        let missing_context = |site: &str, detail: String| {
            Located::new(
                ValidationError::MissingContext {
                    site: site.to_string(),
                    detail,
                }
                .into(),
                source_name,
                None,
                None,
            )
        };
        if !model.context_object_ids.is_empty() {
            return Ok(());
        }
        static RE_OBJ: LazyLock<regex::Regex> = LazyLock::new(|| {
            regex::Regex::new(r"\b([a-zA-Z_][a-zA-Z0-9_]*)\s*\.").unwrap()
        });
        let re_obj = &*RE_OBJ;
        for state in model.states.values() {
            for trans in &state.transitions {
                if trans.is_cpp_condition && re_obj.is_match(&trans.cond_cpp) {
                    return Err(missing_context(
                        "cpp: condition",
                        format!(
                            "'{}' references objects but no <sce:context> declarations found",
                            trans.cond_cpp
                        ),
                    ));
                }
                if trans.is_kt_condition && re_obj.is_match(&trans.cond_kt) {
                    return Err(missing_context(
                        "kt: condition",
                        format!(
                            "'{}' references objects but no <sce:context> declarations found",
                            trans.cond_kt
                        ),
                    ));
                }
            }
            // Check entry/exit blocks AND transition actions for native code references
            let all_actions = state.on_entry_blocks.iter()
                .chain(state.on_exit_blocks.iter())
                .flat_map(|block| block.iter())
                .chain(state.transitions.iter().flat_map(|t| t.actions.iter()));
            for action in all_actions {
                if action.is_cpp_function && re_obj.is_match(&action.content) {
                    return Err(missing_context(
                        "<cpp>",
                        "action references objects but no <sce:context> declarations found"
                            .to_string(),
                    ));
                }
                if action.is_kt_function && re_obj.is_match(&action.content) {
                    return Err(missing_context(
                        "<kt>",
                        "action references objects but no <sce:context> declarations found"
                            .to_string(),
                    ));
                }
            }
        }
        Ok(())
    }
}

// ── Helper functions ────────────────────────────────

/// Recursively check if any if/elseif conditions within actions reference _event.*.
fn actions_contain_event_metadata(actions: &[Action]) -> bool {
    for action in actions {
        if action.action_type == "if" {
            if action.cond.contains("_event.") {
                return true;
            }
            if actions_contain_event_metadata(&action.then_actions) {
                return true;
            }
            for branch in &action.elseif_branches {
                if branch.cond.contains("_event.") {
                    return true;
                }
                if actions_contain_event_metadata(&branch.actions) {
                    return true;
                }
            }
            if actions_contain_event_metadata(&action.else_actions) {
                return true;
            }
        } else if action.action_type == "foreach" {
            if actions_contain_event_metadata(&action.actions) {
                return true;
            }
        }
    }
    false
}

// ── Named Context transforms ────────────────────────

/// Transform C++ code: replace declared context IDs with pointer dereference.
/// e.g., "hardware.powerOff()" → "this->hardware_->powerOff()"
fn transform_cpp_code_with_named_contexts(
    code: &str,
    declared_ids: &std::collections::BTreeSet<String>,
) -> String {
    let (code_with_placeholders, literals) = protect_context_strings(code);
    let mut result = code_with_placeholders;
    // Build single alternation regex for all context IDs (typically 2-3 IDs)
    let alternatives: Vec<String> = declared_ids.iter().map(|id| regex::escape(id)).collect();
    if !alternatives.is_empty() {
        let pattern = regex::Regex::new(&format!(r"\b({})\s*\.", alternatives.join("|"))).unwrap();
        result = pattern.replace_all(&result, |caps: &regex::Captures| {
            format!("this->{}_->", &caps[1])
        }).to_string();
    }
    restore_context_strings(&result, &literals)
}

/// Transform Kotlin code: replace declared context IDs with camelCase.
/// e.g., "my_hardware.powerOff()" → "myHardware.powerOff()"
fn transform_kt_code_with_named_contexts(
    code: &str,
    declared_ids: &std::collections::BTreeSet<String>,
) -> String {
    let (code_with_placeholders, literals) = protect_context_strings(code);
    let mut result = code_with_placeholders;
    // Build mapping of ids that need renaming, then single alternation regex
    let renames: Vec<(String, String)> = declared_ids
        .iter()
        .map(|id| (id.clone(), id_to_camel_case(id)))
        .filter(|(id, camel)| id != camel)
        .collect();
    if !renames.is_empty() {
        let alternatives: Vec<String> = renames.iter().map(|(id, _)| regex::escape(id)).collect();
        let pattern = regex::Regex::new(&format!(r"\b({})\b", alternatives.join("|"))).unwrap();
        result = pattern.replace_all(&result, |caps: &regex::Captures| {
            let matched = &caps[1];
            renames.iter()
                .find(|(id, _)| id == matched)
                .map(|(_, camel)| camel.clone())
                .unwrap_or_else(|| matched.to_string())
        }).to_string();
    }
    restore_context_strings(&result, &literals)
}

/// Convert snake_case/kebab-case identifier to camelCase (delegates to filters::to_camel_case).
fn id_to_camel_case(name: &str) -> String {
    crate::filters::to_camel_case(name.to_string())
}

/// Protect string literals in Named Context code transforms.
/// Distinct from lua_transformer::protect_string_literals which handles JS comments
/// and uses \x01-delimited placeholders for ECMAScript-to-Lua conversion.
fn protect_context_strings(code: &str) -> (String, Vec<String>) {
    static RE_STRING: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r#""(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*'"#).unwrap()
    });
    let mut literals = Vec::new();
    let result = RE_STRING.replace_all(code, |caps: &regex::Captures| {
        let idx = literals.len();
        literals.push(caps[0].to_string());
        format!("__STRING_PLACEHOLDER_{idx}__")
    });
    (result.to_string(), literals)
}

/// Restore string literals from Named Context placeholders.
fn restore_context_strings(code: &str, literals: &[String]) -> String {
    let mut result = code.to_string();
    for (i, literal) in literals.iter().enumerate() {
        let placeholder = format!("__STRING_PLACEHOLDER_{i}__");
        result = result.replace(&placeholder, literal);
    }
    result
}

/// W3C SCXML 6.5: Convert finalize actions to JavaScript code string.
/// Supports: assign, script, log, if/elseif/else.
fn actions_to_javascript(actions: &[Action]) -> String {
    if actions.is_empty() {
        return String::new();
    }
    let mut js_lines = Vec::new();
    for action in actions {
        match action.action_type.as_str() {
            "assign" => {
                if !action.location.is_empty() && !action.expr.is_empty() {
                    js_lines.push(format!("{} = {};", action.location, action.expr));
                }
            }
            "script" => {
                if !action.content.is_empty() {
                    js_lines.push(action.content.clone());
                }
            }
            "log" => {
                if !action.expr.is_empty() {
                    let log_msg = if !action.label.is_empty() {
                        format!("\"{}: \" + {}", action.label, action.expr)
                    } else {
                        action.expr.clone()
                    };
                    js_lines.push(format!("console.log({log_msg});"));
                }
            }
            "if" => {
                if !action.cond.is_empty() {
                    js_lines.push(format!("if ({}) {{", action.cond));
                    if !action.then_actions.is_empty() {
                        let then_js = actions_to_javascript(&action.then_actions);
                        if !then_js.is_empty() {
                            js_lines.push(format!("  {then_js}"));
                        }
                    }
                    js_lines.push("}".to_string());
                    for elseif in &action.elseif_branches {
                        if !elseif.cond.is_empty() {
                            js_lines.push(format!("else if ({}) {{", elseif.cond));
                            if !elseif.actions.is_empty() {
                                let elseif_js = actions_to_javascript(&elseif.actions);
                                if !elseif_js.is_empty() {
                                    js_lines.push(format!("  {elseif_js}"));
                                }
                            }
                            js_lines.push("}".to_string());
                        }
                    }
                    if !action.else_actions.is_empty() {
                        js_lines.push("else {".to_string());
                        let else_js = actions_to_javascript(&action.else_actions);
                        if !else_js.is_empty() {
                            js_lines.push(format!("  {else_js}"));
                        }
                        js_lines.push("}".to_string());
                    }
                }
            }
            _ => {}
        }
    }
    js_lines.join(" ")
}

fn local_name(node: &roxmltree::Node) -> String {
    node.tag_name().name().to_string()
}

fn is_scxml_state_element(node: &roxmltree::Node) -> bool {
    let name = node.tag_name().name();
    matches!(name, "state" | "parallel" | "final")
}

/// Find SCXML-namespaced children with a given local name
fn scxml_children<'a>(
    parent: &'a roxmltree::Node<'a, 'a>,
    tag: &'a str,
) -> impl Iterator<Item = roxmltree::Node<'a, 'a>> {
    parent
        .children()
        .filter(move |c| c.is_element() && c.tag_name().name() == tag)
}

/// Find first SCXML-namespaced child
fn scxml_child<'a>(parent: &'a roxmltree::Node<'a, 'a>, tag: &str) -> Option<roxmltree::Node<'a, 'a>> {
    parent
        .children()
        .find(|c| c.is_element() && c.tag_name().name() == tag)
}

/// XML node serialization matching Python lxml etree.tostring(method='xml').
/// Includes namespace declarations and uses self-closing for empty elements.
fn serialize_node(node: &roxmltree::Node) -> String {
    serialize_node_inner(node, None, false)
}

/// XML node serialization matching Python lxml etree.tostring(method='c14n').
/// Always uses explicit close tags (never self-closing).
fn serialize_node_c14n(node: &roxmltree::Node) -> String {
    serialize_node_inner(node, None, true)
}

fn serialize_node_inner(node: &roxmltree::Node, parent_ns: Option<&str>, c14n: bool) -> String {
    if !node.is_element() {
        return node.text().unwrap_or("").to_string();
    }
    let tag = node.tag_name().name();
    let mut result = format!("<{tag}");

    // Include namespace declaration only if it differs from parent
    let my_ns = node.tag_name().namespace();
    if my_ns != parent_ns {
        if let Some(ns) = my_ns {
            result.push_str(&format!(" xmlns=\"{ns}\""));
        } else if parent_ns.is_some() {
            result.push_str(" xmlns=\"\"");
        }
    }

    for attr in node.attributes() {
        result.push_str(&format!(" {}=\"{}\"", attr.name(), attr.value()));
    }

    // Check if element has any meaningful children (elements or non-empty text)
    let has_children = node.children().any(|c| {
        c.is_element() || (c.is_text() && !c.text().unwrap_or("").trim().is_empty())
    });

    if !has_children && !c14n {
        // Self-closing tag for empty elements (matches lxml method='xml')
        result.push_str("/>");
    } else {
        result.push('>');
        for child in node.children() {
            if child.is_element() {
                result.push_str(&serialize_node_inner(&child, my_ns, c14n));
            } else if child.is_text() {
                result.push_str(child.text().unwrap_or(""));
            }
        }
        result.push_str(&format!("</{tag}>"));
    }
    result
}

/// W3C SCXML 5.9.2: Check if expression is pure In() predicate
fn is_pure_in_predicate(cond: &str) -> bool {
    static RE_IN_CALL: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r#"In\(['"][^'"]+['"]\)"#).unwrap()
    });

    let trimmed = cond.trim();
    if trimmed.is_empty() {
        return false;
    }
    if !trimmed.contains("In(") {
        return false;
    }
    // Check if expression consists only of In() calls, &&, ||, !, (, ), whitespace
    let cleaned = RE_IN_CALL.replace_all(trimmed, "TRUE");
    let cleaned = cleaned
        .replace("&&", " ")
        .replace("||", " ")
        .replace('!', " ")
        .replace('(', " ")
        .replace(')', " ");
    cleaned.split_whitespace().all(|w| w == "TRUE")
}

/// Shared regex for In() predicate with capture group
static RE_IN_PREDICATE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r#"In\(['"]([^'"]+)['"]\)"#).unwrap()
});

/// Convert In() predicate to C++ isStateActive calls
fn convert_in_to_cpp(cond: &str) -> String {
    RE_IN_PREDICATE
        .replace_all(cond, r#"this->isStateActive("$1")"#)
        .to_string()
}

/// Convert In() predicate to Kotlin isStateActive calls
fn convert_in_to_kotlin(cond: &str) -> String {
    RE_IN_PREDICATE
        .replace_all(cond, r#"isStateActive("$1")"#)
        .to_string()
}

/// Check if expression requires script engine evaluation (ports Python _requires_script_engine).
/// Also returns whether the expression uses In() predicate.
fn check_expression_needs(cond: &str) -> (bool, bool) {
    if cond.is_empty() {
        return (false, false);
    }
    if cond.starts_with("cpp:") || cond.starts_with("kt:") {
        return (false, false);
    }
    let has_in = cond.contains("In(");
    if is_pure_in_predicate(cond) {
        return (false, has_in);
    }
    // Mixed In() with ECMAScript needs script engine
    if has_in {
        return (true, true);
    }
    // ECMAScript-specific features
    let js_features = ["typeof", "_event.", "function", "var ", "let ", "const "];
    for f in &js_features {
        if cond.contains(f) {
            return (true, false);
        }
    }
    // W3C SCXML 5.9: System-reserved identifiers starting with underscore
    static RE_UNDERSCORE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"\b_[a-zA-Z]\w*\b").unwrap()
    });
    if RE_UNDERSCORE.is_match(cond) {
        return (true, false);
    }
    // ECMAScript comparison and logical operators
    let operators = ["===", "!==", "==", "!=", "&&", "||", "<=", ">=", "<", ">"];
    for op in &operators {
        if cond.contains(op) {
            return (true, false);
        }
    }
    // String/number literals
    if cond.contains('\'') || cond.contains('"') {
        return (true, false);
    }
    // Note: _event.* fields are already caught by "_event." in js_features above.
    // C++/Rust reserved keywords that would be invalid as conditions
    let reserved = ["return", "break", "continue", "goto", "switch", "case", "default",
                    "if", "else", "while", "do", "for", "class", "struct", "typedef",
                    "using", "namespace", "template", "typename", "static", "extern",
                    "inline", "virtual", "operator", "new", "delete", "this", "throw",
                    "try", "catch", "public", "private", "protected"];
    let stripped = cond.trim();
    for kw in &reserved {
        if stripped == *kw
            || (stripped.starts_with(kw)
                && stripped.len() > kw.len()
                && !stripped.as_bytes()[kw.len()].is_ascii_alphanumeric()
                && stripped.as_bytes()[kw.len()] != b'_')
        {
            return (true, false);
        }
    }
    (false, false)
}

fn parse_delay_to_ms(delay: &str) -> i64 {
    let trimmed = delay.trim();
    if trimmed.is_empty() {
        return 0;
    }
    if let Some(s) = trimmed.strip_suffix("ms") {
        s.trim().parse().unwrap_or(0)
    } else if let Some(s) = trimmed.strip_suffix('s') {
        s.trim().parse::<f64>().map(|v| (v * 1000.0) as i64).unwrap_or(0)
    } else {
        // Bare number: default to seconds (common in W3C test suite)
        trimmed.parse::<f64>().map(|v| (v * 1000.0) as i64).unwrap_or(0)
    }
}

fn is_static_string_literal(expr: &str) -> bool {
    let trimmed = expr.trim();
    trimmed.len() >= 2
        && ((trimmed.starts_with('\'') && trimmed.ends_with('\''))
            || (trimmed.starts_with('"') && trimmed.ends_with('"')))
}

fn extract_static_string_literal(expr: &str) -> String {
    let trimmed = expr.trim();
    if trimmed.len() >= 2 {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        String::new()
    }
}

/// Scan actions for <send target="#_parent" event="xxx"> and collect event names.
fn collect_parent_send_events(actions: &[Action], events: &mut std::collections::BTreeSet<String>) {
    for action in actions {
        if action.action_type == "send" && action.target == "#_parent" && !action.event.is_empty() {
            events.insert(action.event.clone());
        }
        // Recurse into nested actions (if/then/else, foreach)
        collect_parent_send_events(&action.then_actions, events);
        collect_parent_send_events(&action.else_actions, events);
        collect_parent_send_events(&action.actions, events);
        for branch in &action.elseif_branches {
            collect_parent_send_events(&branch.actions, events);
        }
    }
}

/// Extract child name from inline SCXML text. Uses `name` attribute if present,
/// otherwise generates `{parent}_child{N}`.
fn extract_inline_child_name(
    inline_text: &str,
    parent_name: &str,
    counter: &mut u32,
) -> String {
    // Try to parse the name attribute from the inline SCXML
    if let Ok(doc) = roxmltree::Document::parse(inline_text) {
        let root = doc.root_element();
        if let Some(name) = root.attribute("name") {
            if !name.is_empty() {
                return format!("{parent_name}_{name}");
            }
        }
    }
    let name = format!("{parent_name}_child{counter}");
    *counter += 1;
    name
}

/// Parse a child SCXML file to extract metadata (needs_script_engine, datamodel vars).
///
/// The fields written (`child_needs_script_engine`, `child_datamodel_vars`)
/// are session-only — they live on [`InvokeSessionCommon`]. Both
/// `ScxmlInvokeInfo` and `HybridInvokeInfo` expose this via `&mut si.common`.
fn parse_child_metadata(child_path: &Path, common: &mut InvokeSessionCommon) {
    if !child_path.exists() {
        common.child_needs_script_engine = true;
        common.child_datamodel_vars = Some(Vec::new());
        return;
    }
    match SCXMLParser::new().parse_file(&child_path.to_string_lossy()) {
        Ok(child_model) => {
            common.child_needs_script_engine = child_model.needs_script_engine;
            common.child_datamodel_vars = Some(
                child_model.variables.iter().map(|v| v.id.clone()).collect(),
            );
        }
        Err(_) => {
            common.child_needs_script_engine = true;
            common.child_datamodel_vars = Some(Vec::new());
        }
    }
}

// ══════════════════════════════════════════════════════════════
// ── Unit tests ───────────────────────────────────────────────
// ══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    // ── parse_delay_to_ms ────────────────────────────────────

    #[test]
    fn delay_empty_string() {
        assert_eq!(parse_delay_to_ms(""), 0);
    }

    #[test]
    fn delay_whitespace_only() {
        assert_eq!(parse_delay_to_ms("   "), 0);
    }

    #[test]
    fn delay_milliseconds() {
        assert_eq!(parse_delay_to_ms("500ms"), 500);
    }

    #[test]
    fn delay_milliseconds_with_whitespace() {
        assert_eq!(parse_delay_to_ms("  100 ms"), 100);
    }

    #[test]
    fn delay_zero_ms() {
        assert_eq!(parse_delay_to_ms("0ms"), 0);
    }

    #[test]
    fn delay_seconds_integer() {
        assert_eq!(parse_delay_to_ms("2s"), 2000);
    }

    #[test]
    fn delay_seconds_fractional() {
        assert_eq!(parse_delay_to_ms("1.5s"), 1500);
    }

    #[test]
    fn delay_seconds_with_whitespace() {
        assert_eq!(parse_delay_to_ms("  3 s"), 3000);
    }

    #[test]
    fn delay_bare_number_treated_as_seconds() {
        assert_eq!(parse_delay_to_ms("5"), 5000);
    }

    #[test]
    fn delay_bare_fractional_as_seconds() {
        assert_eq!(parse_delay_to_ms("0.5"), 500);
    }

    #[test]
    fn delay_invalid_numeric_returns_zero() {
        assert_eq!(parse_delay_to_ms("abc"), 0);
    }

    #[test]
    fn delay_invalid_ms_suffix_returns_zero() {
        assert_eq!(parse_delay_to_ms("xyzms"), 0);
    }

    #[test]
    fn delay_negative_ms() {
        assert_eq!(parse_delay_to_ms("-100ms"), -100);
    }

    // ── is_pure_in_predicate ─────────────────────────────────

    #[test]
    fn pure_in_single() {
        assert!(is_pure_in_predicate("In('s1')"));
    }

    #[test]
    fn pure_in_double_quotes() {
        assert!(is_pure_in_predicate("In(\"s1\")"));
    }

    #[test]
    fn pure_in_conjunction() {
        assert!(is_pure_in_predicate("In('s1') && In('s2')"));
    }

    #[test]
    fn pure_in_disjunction() {
        assert!(is_pure_in_predicate("In('s1') || In('s2')"));
    }

    #[test]
    fn pure_in_negation() {
        assert!(is_pure_in_predicate("!In('s1')"));
    }

    #[test]
    fn pure_in_complex_boolean() {
        assert!(is_pure_in_predicate("(In('s1') && !In('s2')) || In('s3')"));
    }

    #[test]
    fn not_pure_in_empty() {
        assert!(!is_pure_in_predicate(""));
    }

    #[test]
    fn not_pure_in_no_in_call() {
        assert!(!is_pure_in_predicate("x > 5"));
    }

    #[test]
    fn not_pure_in_mixed_with_ecmascript() {
        assert!(!is_pure_in_predicate("In('s1') && x > 5"));
    }

    #[test]
    fn not_pure_in_bare_variable() {
        assert!(!is_pure_in_predicate("In('s1') && someVar"));
    }

    // ── convert_in_to_cpp / convert_in_to_kotlin ─────────────

    #[test]
    fn convert_in_cpp_single() {
        assert_eq!(
            convert_in_to_cpp("In('active')"),
            "this->isStateActive(\"active\")"
        );
    }

    #[test]
    fn convert_in_cpp_conjunction() {
        assert_eq!(
            convert_in_to_cpp("In('s1') && In('s2')"),
            "this->isStateActive(\"s1\") && this->isStateActive(\"s2\")"
        );
    }

    #[test]
    fn convert_in_cpp_double_quotes() {
        assert_eq!(
            convert_in_to_cpp("In(\"running\")"),
            "this->isStateActive(\"running\")"
        );
    }

    #[test]
    fn convert_in_kotlin_single() {
        assert_eq!(
            convert_in_to_kotlin("In('active')"),
            "isStateActive(\"active\")"
        );
    }

    #[test]
    fn convert_in_kotlin_negation() {
        assert_eq!(
            convert_in_to_kotlin("!In('idle')"),
            "!isStateActive(\"idle\")"
        );
    }

    // ── check_expression_needs ───────────────────────────────

    #[test]
    fn expr_needs_empty() {
        assert_eq!(check_expression_needs(""), (false, false));
    }

    #[test]
    fn expr_needs_cpp_prefix_no_engine() {
        assert_eq!(check_expression_needs("cpp:someExpr"), (false, false));
    }

    #[test]
    fn expr_needs_kt_prefix_no_engine() {
        assert_eq!(check_expression_needs("kt:someExpr"), (false, false));
    }

    #[test]
    fn expr_needs_pure_in_no_engine() {
        let (needs_engine, has_in) = check_expression_needs("In('s1')");
        assert!(!needs_engine);
        assert!(has_in);
    }

    #[test]
    fn expr_needs_mixed_in_requires_engine() {
        let (needs_engine, has_in) = check_expression_needs("In('s1') && x > 5");
        assert!(needs_engine);
        assert!(has_in);
    }

    #[test]
    fn expr_needs_typeof_requires_engine() {
        let (needs_engine, _) = check_expression_needs("typeof x === 'number'");
        assert!(needs_engine);
    }

    #[test]
    fn expr_needs_event_dot_requires_engine() {
        let (needs_engine, _) = check_expression_needs("_event.data");
        assert!(needs_engine);
    }

    #[test]
    fn expr_needs_underscore_system_var() {
        let (needs_engine, _) = check_expression_needs("_sessionid");
        assert!(needs_engine);
    }

    #[test]
    fn expr_needs_comparison_operators() {
        assert!(check_expression_needs("x == 5").0);
        assert!(check_expression_needs("x != 5").0);
        assert!(check_expression_needs("x === 5").0);
        assert!(check_expression_needs("x !== 5").0);
        assert!(check_expression_needs("x < 5").0);
        assert!(check_expression_needs("x > 5").0);
        assert!(check_expression_needs("x <= 5").0);
        assert!(check_expression_needs("x >= 5").0);
    }

    #[test]
    fn expr_needs_logical_operators() {
        assert!(check_expression_needs("a && b").0);
        assert!(check_expression_needs("a || b").0);
    }

    #[test]
    fn expr_needs_string_literal() {
        assert!(check_expression_needs("'hello'").0);
        assert!(check_expression_needs("\"hello\"").0);
    }

    #[test]
    fn expr_needs_reserved_keyword() {
        assert!(check_expression_needs("return").0);
        assert!(check_expression_needs("if(true)").0);
    }

    #[test]
    fn expr_needs_reserved_keyword_boundary() {
        // "ifelse" should NOT match "if" because next char is alphanumeric
        assert!(!check_expression_needs("ifelse").0);
        // "if_something" should NOT match because next char is underscore
        assert!(!check_expression_needs("if_something").0);
    }

    #[test]
    fn expr_needs_simple_identifier_no_engine() {
        // A bare identifier without operators/keywords should not need engine
        assert_eq!(check_expression_needs("myVariable"), (false, false));
    }

    // ── is_static_string_literal / extract_static_string_literal ─

    #[test]
    fn static_string_single_quotes() {
        assert!(is_static_string_literal("'hello'"));
        assert_eq!(extract_static_string_literal("'hello'"), "hello");
    }

    #[test]
    fn static_string_double_quotes() {
        assert!(is_static_string_literal("\"hello\""));
        assert_eq!(extract_static_string_literal("\"hello\""), "hello");
    }

    #[test]
    fn static_string_with_whitespace() {
        assert!(is_static_string_literal("  'hello'  "));
        assert_eq!(extract_static_string_literal("  'hello'  "), "hello");
    }

    #[test]
    fn static_string_empty_quoted() {
        assert!(is_static_string_literal("''"));
        assert_eq!(extract_static_string_literal("''"), "");
    }

    #[test]
    fn static_string_mismatched_quotes() {
        assert!(!is_static_string_literal("'hello\""));
        assert!(!is_static_string_literal("\"hello'"));
    }

    #[test]
    fn static_string_too_short() {
        assert!(!is_static_string_literal("x"));
        assert!(!is_static_string_literal(""));
    }

    #[test]
    fn static_string_no_quotes() {
        assert!(!is_static_string_literal("hello"));
    }

    // ── protect_context_strings / restore_context_strings ─────

    #[test]
    fn protect_restore_roundtrip() {
        let code = r#"hardware.powerOff("reason") && sensor.read('temp')"#;
        let (protected, literals) = protect_context_strings(code);
        assert!(!protected.contains("\"reason\""));
        assert!(!protected.contains("'temp'"));
        assert!(protected.contains("__STRING_PLACEHOLDER_0__"));
        assert!(protected.contains("__STRING_PLACEHOLDER_1__"));
        let restored = restore_context_strings(&protected, &literals);
        assert_eq!(restored, code);
    }

    #[test]
    fn protect_no_strings() {
        let code = "hardware.powerOff()";
        let (protected, literals) = protect_context_strings(code);
        assert_eq!(protected, code);
        assert!(literals.is_empty());
    }

    #[test]
    fn protect_escaped_quotes() {
        let code = r#"x.call("escaped \" quote")"#;
        let (protected, literals) = protect_context_strings(code);
        assert_eq!(literals.len(), 1);
        let restored = restore_context_strings(&protected, &literals);
        assert_eq!(restored, code);
    }

    // ── transform_cpp_code_with_named_contexts ───────────────

    #[test]
    fn cpp_transform_simple_context() {
        let mut ids = BTreeSet::new();
        ids.insert("hardware".to_string());
        let result = transform_cpp_code_with_named_contexts("hardware.powerOff()", &ids);
        assert_eq!(result, "this->hardware_->powerOff()");
    }

    #[test]
    fn cpp_transform_multiple_contexts() {
        let mut ids = BTreeSet::new();
        ids.insert("hw".to_string());
        ids.insert("sensor".to_string());
        let result = transform_cpp_code_with_named_contexts(
            "hw.reset() && sensor.read()",
            &ids,
        );
        assert_eq!(result, "this->hw_->reset() && this->sensor_->read()");
    }

    #[test]
    fn cpp_transform_preserves_string_literals() {
        let mut ids = BTreeSet::new();
        ids.insert("hw".to_string());
        let result = transform_cpp_code_with_named_contexts(
            r#"hw.log("hw.error")"#,
            &ids,
        );
        // "hw.error" inside quotes must NOT be transformed
        assert_eq!(result, r#"this->hw_->log("hw.error")"#);
    }

    #[test]
    fn cpp_transform_empty_ids() {
        let ids = BTreeSet::new();
        let result = transform_cpp_code_with_named_contexts("hardware.powerOff()", &ids);
        assert_eq!(result, "hardware.powerOff()");
    }

    // ── transform_kt_code_with_named_contexts ────────────────

    #[test]
    fn kt_transform_snake_to_camel() {
        let mut ids = BTreeSet::new();
        ids.insert("my_hardware".to_string());
        let result = transform_kt_code_with_named_contexts("my_hardware.powerOff()", &ids);
        assert_eq!(result, "myHardware.powerOff()");
    }

    #[test]
    fn kt_transform_already_camel_no_change() {
        let mut ids = BTreeSet::new();
        ids.insert("hardware".to_string());
        // "hardware" → camelCase is still "hardware", no rename needed
        let result = transform_kt_code_with_named_contexts("hardware.powerOff()", &ids);
        assert_eq!(result, "hardware.powerOff()");
    }

    #[test]
    fn kt_transform_preserves_string_literals() {
        let mut ids = BTreeSet::new();
        ids.insert("my_obj".to_string());
        let result = transform_kt_code_with_named_contexts(
            r#"my_obj.call("my_obj")"#,
            &ids,
        );
        // Inside quotes should not be transformed
        assert_eq!(result, r#"myObj.call("my_obj")"#);
    }

    // ── actions_to_javascript ────────────────────────────────

    #[test]
    fn actions_to_js_empty() {
        assert_eq!(actions_to_javascript(&[]), "");
    }

    #[test]
    fn actions_to_js_assign() {
        let action = Action {
            action_type: "assign".to_string(),
            location: "x".to_string(),
            expr: "5".to_string(),
            ..Default::default()
        };
        assert_eq!(actions_to_javascript(&[action]), "x = 5;");
    }

    #[test]
    fn actions_to_js_script() {
        let action = Action {
            action_type: "script".to_string(),
            content: "doSomething();".to_string(),
            ..Default::default()
        };
        assert_eq!(actions_to_javascript(&[action]), "doSomething();");
    }

    #[test]
    fn actions_to_js_log_with_label() {
        let action = Action {
            action_type: "log".to_string(),
            label: "debug".to_string(),
            expr: "x".to_string(),
            ..Default::default()
        };
        assert_eq!(
            actions_to_javascript(&[action]),
            "console.log(\"debug: \" + x);"
        );
    }

    #[test]
    fn actions_to_js_log_without_label() {
        let action = Action {
            action_type: "log".to_string(),
            expr: "x".to_string(),
            ..Default::default()
        };
        assert_eq!(actions_to_javascript(&[action]), "console.log(x);");
    }

    #[test]
    fn actions_to_js_if_then_else() {
        let action = Action {
            action_type: "if".to_string(),
            cond: "x > 0".to_string(),
            then_actions: vec![Action {
                action_type: "assign".to_string(),
                location: "y".to_string(),
                expr: "1".to_string(),
                ..Default::default()
            }],
            else_actions: vec![Action {
                action_type: "assign".to_string(),
                location: "y".to_string(),
                expr: "0".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let result = actions_to_javascript(&[action]);
        assert!(result.contains("if (x > 0) {"));
        assert!(result.contains("y = 1;"));
        assert!(result.contains("else {"));
        assert!(result.contains("y = 0;"));
    }

    #[test]
    fn actions_to_js_multiple_actions() {
        let actions = vec![
            Action {
                action_type: "assign".to_string(),
                location: "x".to_string(),
                expr: "1".to_string(),
                ..Default::default()
            },
            Action {
                action_type: "assign".to_string(),
                location: "y".to_string(),
                expr: "2".to_string(),
                ..Default::default()
            },
        ];
        assert_eq!(actions_to_javascript(&actions), "x = 1; y = 2;");
    }

    #[test]
    fn actions_to_js_skips_empty_assign() {
        let action = Action {
            action_type: "assign".to_string(),
            location: "".to_string(),
            expr: "5".to_string(),
            ..Default::default()
        };
        assert_eq!(actions_to_javascript(&[action]), "");
    }

    // ── extract_inline_child_name ────────────────────────────

    #[test]
    fn inline_child_name_from_attribute() {
        let scxml = r#"<scxml name="child1" xmlns="http://www.w3.org/2005/07/scxml" version="1.0"><state id="s"/></scxml>"#;
        let mut counter = 0u32;
        let name = extract_inline_child_name(scxml, "parent", &mut counter);
        assert_eq!(name, "parent_child1");
        assert_eq!(counter, 0); // counter not incremented when name found
    }

    #[test]
    fn inline_child_name_auto_generated() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"><state id="s"/></scxml>"#;
        let mut counter = 1u32;
        let name = extract_inline_child_name(scxml, "parent", &mut counter);
        assert_eq!(name, "parent_child1");
        assert_eq!(counter, 2);
    }

    #[test]
    fn inline_child_name_empty_name_attr() {
        let scxml = r#"<scxml name="" xmlns="http://www.w3.org/2005/07/scxml" version="1.0"><state id="s"/></scxml>"#;
        let mut counter = 0u32;
        let name = extract_inline_child_name(scxml, "parent", &mut counter);
        assert_eq!(name, "parent_child0");
        assert_eq!(counter, 1);
    }

    #[test]
    fn inline_child_name_invalid_xml() {
        let mut counter = 0u32;
        let name = extract_inline_child_name("<not valid xml", "parent", &mut counter);
        assert_eq!(name, "parent_child0");
        assert_eq!(counter, 1);
    }

    // ── actions_contain_event_metadata ────────────────────────

    #[test]
    fn event_metadata_empty_actions() {
        assert!(!actions_contain_event_metadata(&[]));
    }

    #[test]
    fn event_metadata_in_if_cond() {
        let action = Action {
            action_type: "if".to_string(),
            cond: "_event.data > 0".to_string(),
            ..Default::default()
        };
        assert!(actions_contain_event_metadata(&[action]));
    }

    #[test]
    fn event_metadata_in_nested_then() {
        let inner = Action {
            action_type: "if".to_string(),
            cond: "_event.type == 'error'".to_string(),
            ..Default::default()
        };
        let outer = Action {
            action_type: "if".to_string(),
            cond: "x > 0".to_string(),
            then_actions: vec![inner],
            ..Default::default()
        };
        assert!(actions_contain_event_metadata(&[outer]));
    }

    #[test]
    fn event_metadata_in_elseif_branch() {
        let action = Action {
            action_type: "if".to_string(),
            cond: "x > 0".to_string(),
            elseif_branches: vec![ElseIfBranch {
                cond: "_event.name".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        };
        assert!(actions_contain_event_metadata(&[action]));
    }

    #[test]
    fn event_metadata_in_foreach() {
        let inner = Action {
            action_type: "if".to_string(),
            cond: "_event.data".to_string(),
            ..Default::default()
        };
        let foreach = Action {
            action_type: "foreach".to_string(),
            actions: vec![inner],
            ..Default::default()
        };
        assert!(actions_contain_event_metadata(&[foreach]));
    }

    #[test]
    fn event_metadata_no_match() {
        let action = Action {
            action_type: "if".to_string(),
            cond: "x > 0".to_string(),
            then_actions: vec![Action {
                action_type: "assign".to_string(),
                location: "y".to_string(),
                expr: "1".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        };
        assert!(!actions_contain_event_metadata(&[action]));
    }

    // ── collect_parent_send_events ───────────────────────────

    #[test]
    fn collect_parent_events_basic() {
        let action = Action {
            action_type: "send".to_string(),
            target: "#_parent".to_string(),
            event: "done".to_string(),
            ..Default::default()
        };
        let mut events = BTreeSet::new();
        collect_parent_send_events(&[action], &mut events);
        assert_eq!(events.len(), 1);
        assert!(events.contains("done"));
    }

    #[test]
    fn collect_parent_events_ignores_non_parent() {
        let action = Action {
            action_type: "send".to_string(),
            target: "#other".to_string(),
            event: "done".to_string(),
            ..Default::default()
        };
        let mut events = BTreeSet::new();
        collect_parent_send_events(&[action], &mut events);
        assert!(events.is_empty());
    }

    #[test]
    fn collect_parent_events_ignores_empty_event() {
        let action = Action {
            action_type: "send".to_string(),
            target: "#_parent".to_string(),
            event: "".to_string(),
            ..Default::default()
        };
        let mut events = BTreeSet::new();
        collect_parent_send_events(&[action], &mut events);
        assert!(events.is_empty());
    }

    #[test]
    fn collect_parent_events_nested_in_if() {
        let send = Action {
            action_type: "send".to_string(),
            target: "#_parent".to_string(),
            event: "child.done".to_string(),
            ..Default::default()
        };
        let if_action = Action {
            action_type: "if".to_string(),
            cond: "true".to_string(),
            then_actions: vec![send],
            ..Default::default()
        };
        let mut events = BTreeSet::new();
        collect_parent_send_events(&[if_action], &mut events);
        assert!(events.contains("child.done"));
    }

    #[test]
    fn collect_parent_events_deduplicates() {
        let a1 = Action {
            action_type: "send".to_string(),
            target: "#_parent".to_string(),
            event: "same".to_string(),
            ..Default::default()
        };
        let a2 = a1.clone();
        let mut events = BTreeSet::new();
        collect_parent_send_events(&[a1, a2], &mut events);
        assert_eq!(events.len(), 1);
    }

    // ── parse_string (integration-level) ─────────────────────

    #[test]
    fn parse_minimal_scxml() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1"/>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        assert_eq!(model.initial, "s1");
        assert!(model.states.contains_key("s1"));
    }

    #[test]
    fn parse_with_datamodel() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1" datamodel="ecmascript">
            <datamodel>
                <data id="x" expr="0"/>
                <data id="y" expr="'hello'"/>
            </datamodel>
            <state id="s1"/>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        assert_eq!(model.variables.len(), 2);
        assert_eq!(model.variables[0].id, "x");
        assert_eq!(model.variables[1].id, "y");
    }

    #[test]
    fn parse_transitions() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1">
                <transition event="go" target="s2"/>
            </state>
            <final id="s2"/>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        let s1 = &model.states["s1"];
        assert_eq!(s1.transitions.len(), 1);
        assert_eq!(s1.transitions[0].event, "go");
        assert_eq!(s1.transitions[0].target, "s2");
    }

    #[test]
    fn parse_parallel_states() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="p">
            <parallel id="p">
                <state id="r1"><state id="r1a"/></state>
                <state id="r2"><state id="r2a"/></state>
            </parallel>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        assert!(model.states["p"].is_parallel);
        assert!(model.has_parallel_states);
    }

    #[test]
    fn parse_entry_exit_actions() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1">
                <onentry><log expr="'entering'"/></onentry>
                <onexit><log expr="'exiting'"/></onexit>
            </state>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        let s1 = &model.states["s1"];
        assert!(!s1.on_entry_blocks.is_empty());
        assert!(!s1.on_exit_blocks.is_empty());
    }

    #[test]
    fn parse_history_state() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1">
                <history id="h1" type="deep">
                    <transition target="s1a"/>
                </history>
                <state id="s1a"/>
                <state id="s1b"/>
            </state>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        assert!(model.has_history_states);
        assert!(model.history_states.contains_key("h1"));
        assert_eq!(model.history_states["h1"].history_type, "deep");
    }

    #[test]
    fn parse_send_action_attributes() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1">
                <onentry>
                    <send event="timer" delay="500ms"/>
                </onentry>
            </state>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        let entry = &model.states["s1"].on_entry_blocks[0];
        assert_eq!(entry[0].event, "timer");
        assert_eq!(entry[0].delay_ms, 500);
    }

    #[test]
    fn parse_if_elseif_else() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1" datamodel="ecmascript">
            <state id="s1">
                <onentry>
                    <if cond="x > 0">
                        <log expr="'positive'"/>
                    <elseif cond="x == 0"/>
                        <log expr="'zero'"/>
                    <else/>
                        <log expr="'negative'"/>
                    </if>
                </onentry>
            </state>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        let entry = &model.states["s1"].on_entry_blocks[0];
        let if_action = &entry[0];
        assert_eq!(if_action.action_type, "if");
        assert_eq!(if_action.elseif_branches.len(), 1);
        assert!(!if_action.else_actions.is_empty());
    }

    // ── Error path tests ─────────────────────────────────────

    #[test]
    fn error_invalid_xml() {
        let mut parser = SCXMLParser::new();
        let result = parser.parse_string("<not valid xml", "test");
        assert!(result.is_err());
        // May fail at XSD validation or XML parse — either error path is valid
    }

    #[test]
    fn error_empty_input() {
        let mut parser = SCXMLParser::new();
        let result = parser.parse_string("", "test");
        assert!(result.is_err());
    }

    #[test]
    fn error_non_scxml_root() {
        let mut parser = SCXMLParser::new();
        let result = parser.parse_string("<html><body/></html>", "test");
        // XSD validation rejects non-SCXML root — should not panic
        assert!(result.is_err());
    }

    #[test]
    fn graceful_state_without_id() {
        // States without id are skipped (no panic)
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1"/>
            <state/>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        // Only s1 should be in the model, the id-less state is skipped
        assert!(model.states.contains_key("s1"));
        assert_eq!(
            model.states.len(),
            1,
            "id-less state should be skipped, got: {:?}",
            model.states.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn graceful_transition_empty_target() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1">
                <transition event="go"/>
            </state>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        let s1 = &model.states["s1"];
        assert_eq!(s1.transitions.len(), 1);
        assert_eq!(s1.transitions[0].target, "");
    }

    #[test]
    fn graceful_transition_nonexistent_target() {
        // Target references a state that doesn't exist
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1">
                <transition event="go" target="nonexistent"/>
            </state>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        // Should not panic — just stores the target string
        let model = parser.parse_string(scxml, "test").unwrap();
        assert_eq!(model.states["s1"].transitions[0].target, "nonexistent");
    }

    #[test]
    fn graceful_empty_datamodel() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <datamodel/>
            <state id="s1"/>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        assert!(model.variables.is_empty());
    }

    #[test]
    fn graceful_data_without_id() {
        // <data> without id — should be skipped gracefully
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <datamodel>
                <data expr="5"/>
                <data id="x" expr="10"/>
            </datamodel>
            <state id="s1"/>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        // Only the data element with id="x" should be parsed
        assert!(
            model.variables.iter().any(|v| v.id == "x"),
            "variable x should exist"
        );
    }

    #[test]
    fn graceful_deeply_nested_states() {
        // 5 levels of state nesting — should parse correctly
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="a">
            <state id="a">
                <state id="b">
                    <state id="c">
                        <state id="d">
                            <state id="e"/>
                        </state>
                    </state>
                </state>
            </state>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        assert_eq!(model.states.len(), 5);
        assert_eq!(model.states["e"].parent, Some("d".to_string()));
        assert_eq!(model.states["d"].parent, Some("c".to_string()));
        assert!(model.has_hierarchy);
    }

    #[test]
    fn graceful_send_with_missing_event() {
        // <send> without event attribute — should not panic
        let scxml = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1">
                <onentry>
                    <send target="#_parent"/>
                </onentry>
            </state>
        </scxml>"##;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        let entry = &model.states["s1"].on_entry_blocks[0];
        assert_eq!(entry[0].action_type, "send");
        assert_eq!(entry[0].event, "");
    }

    #[test]
    fn graceful_foreach_action() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1" datamodel="ecmascript">
            <state id="s1">
                <onentry>
                    <foreach array="items" item="x" index="i">
                        <log expr="x"/>
                    </foreach>
                </onentry>
            </state>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        let entry = &model.states["s1"].on_entry_blocks[0];
        assert_eq!(entry[0].action_type, "foreach");
        assert_eq!(entry[0].array, "items");
        assert_eq!(entry[0].item, "x");
        assert_eq!(entry[0].index, "i");
    }

    #[test]
    fn graceful_cancel_action() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1">
                <onentry>
                    <cancel sendid="timer1"/>
                </onentry>
            </state>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        let entry = &model.states["s1"].on_entry_blocks[0];
        assert_eq!(entry[0].action_type, "cancel");
        assert_eq!(entry[0].sendid, "timer1");
    }

    #[test]
    fn graceful_raise_action() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1">
                <onentry>
                    <raise event="internal.event"/>
                </onentry>
            </state>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        let entry = &model.states["s1"].on_entry_blocks[0];
        assert_eq!(entry[0].action_type, "raise");
        assert_eq!(entry[0].event, "internal.event");
    }

    #[test]
    fn graceful_multiple_transitions() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1">
                <transition event="a" target="s2"/>
                <transition event="b" target="s3"/>
                <transition event="c" target="s1"/>
            </state>
            <state id="s2"/>
            <state id="s3"/>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        assert_eq!(model.states["s1"].transitions.len(), 3);
        assert_eq!(model.states["s1"].transitions[0].event, "a");
        assert_eq!(model.states["s1"].transitions[1].event, "b");
        assert_eq!(model.states["s1"].transitions[2].event, "c");
    }

    #[test]
    fn graceful_final_state_with_donedata() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s1">
            <state id="s1">
                <transition event="done" target="final"/>
            </state>
            <final id="final">
                <donedata>
                    <param name="result" expr="42"/>
                </donedata>
            </final>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        assert!(model.states["final"].is_final);
    }

    #[test]
    fn error_duplicate_context_id() {
        use crate::forge::error::{ForgeError, ValidationError};
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                        xmlns:sce="http://sce.dev/ext"
                        version="1.0" initial="s1">
            <sce:context id="hw"/>
            <sce:context id="hw"/>
            <state id="s1"/>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let err = parser
            .parse_string(scxml, "test")
            .expect_err("duplicate <sce:context id=\"hw\"> must fail validation");
        assert!(
            matches!(
                err.error,
                ForgeError::Validation(
                    ValidationError::DuplicateContextObject { ref id },
                ) if id == "hw",
            ),
            "expected ValidationError::DuplicateContextObject(id=\"hw\"), got: {:?}",
            err.error,
        );
    }

    #[test]
    fn error_inline_kind_missing_id_is_fatal() {
        // Malformed inline kind (`sce:kind="lookup"` without `id`) must
        // fail the whole parse. Silently demoting it to a JS variable
        // would mask the author's declared intent and hand the
        // generator a data type it was never told about
        // (feedback_silently_broken_hooks).
        use crate::forge::error::{ForgeError, ValidationError};
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                        xmlns:sce="http://sce.dev/ext"
                        version="1.0" initial="s1">
            <datamodel>
                <data sce:kind="lookup">
                    <sce:entry key="1" value="one"/>
                </data>
            </datamodel>
            <state id="s1"/>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let err = parser
            .parse_string(scxml, "test")
            .expect_err("malformed inline lookup must fail hard");
        assert!(
            matches!(
                err.error,
                ForgeError::Validation(ValidationError::MissingAttribute { ref attr, .. })
                    if attr == "id",
            ),
            "expected ValidationError::MissingAttribute(attr=\"id\"), got: {:?}",
            err.error,
        );
        assert_eq!(err.location.file, "test", "located error must carry source name");
        assert!(
            err.location.line.is_some() && err.location.col.is_some(),
            "inline-kind leaf errors must carry roxmltree row/col, got: {:?}",
            err.location,
        );
    }

    #[test]
    fn error_missing_context_on_cpp_condition() {
        // cpp: condition references `hw.ready` but no <sce:context id="hw">
        // is declared — parse must fail with MissingContext, not the
        // semantically wrong IncompatibleAttributes.
        use crate::forge::error::{ForgeError, ValidationError};
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                        xmlns:sce="http://sce.dev/ext"
                        version="1.0" initial="s1">
            <state id="s1">
                <transition cond="cpp:hw.ready()" target="s2"/>
            </state>
            <state id="s2"/>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let err = parser.parse_string(scxml, "test").expect_err(
            "cpp: condition without <sce:context> must fail validation",
        );
        assert!(
            matches!(
                err.error,
                ForgeError::Validation(ValidationError::MissingContext { ref site, .. })
                    if site == "cpp: condition"
            ),
            "expected ValidationError::MissingContext(site=\"cpp: condition\"), got: {:?}",
            err.error,
        );
    }

    #[test]
    fn graceful_default_initial_from_first_state() {
        // No initial attribute — default to first child state
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0">
            <state id="first"/>
            <state id="second"/>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        assert_eq!(model.initial, "first");
    }

    #[test]
    fn graceful_whitespace_in_initial() {
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="  s1  ">
            <state id="s1"/>
        </scxml>"#;
        let mut parser = SCXMLParser::new();
        // Parser may trim or not — should not panic
        let _model = parser.parse_string(scxml, "test").unwrap();
    }

    // ── InvokeBase.field_suffix derivation ───────────────────────────

    /// Helper: parse an SCXML fragment with a single `<invoke>` and return its
    /// (invoke_id, field_suffix) pair.
    fn first_invoke_ids(scxml: &str) -> (String, String) {
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        let invoke = model
            .states
            .values()
            .find_map(|s| s.invokes.first())
            .expect("expected at least one invoke");
        let base = match invoke {
            Invoke::Scxml(info) => &info.common.base,
            Invoke::Hybrid(info) => &info.common.base,
            Invoke::MeshRpc(info) => &info.base,
        };
        (base.invoke_id.clone(), base.field_suffix.clone())
    }

    #[test]
    fn invoke_field_suffix_strips_auto_id_leading_underscore() {
        // No `id` attribute → parser auto-generates `_invoke_0` per W3C SCXML 6.4.1.
        // field_suffix drops the leading underscore so `child_` + suffix is
        // `child_invoke_0`, not `child__invoke_0`.
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s">
                <invoke type="scxml" src="child.scxml"/>
            </state>
        </scxml>"#;
        let (invoke_id, field_suffix) = first_invoke_ids(scxml);
        assert_eq!(invoke_id, "_invoke_0");
        assert_eq!(field_suffix, "invoke_0");
    }

    #[test]
    fn invoke_field_suffix_preserves_user_supplied_id() {
        // User-supplied id has no leading underscore — round-trips unchanged.
        let scxml = r#"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s">
                <invoke id="invokedChild" type="scxml" src="child.scxml"/>
            </state>
        </scxml>"#;
        let (invoke_id, field_suffix) = first_invoke_ids(scxml);
        assert_eq!(invoke_id, "invokedChild");
        assert_eq!(field_suffix, "invokedChild");
    }

    // ── <invoke type="sce:mesh-rpc"> reserved-param rules (§9.5) ────

    /// Helper: parse a fragment and pull the first [`Invoke::MeshRpc`].
    fn first_mesh_rpc_invoke(scxml: &str) -> MeshRpcInvokeInfo {
        let mut parser = SCXMLParser::new();
        let model = parser.parse_string(scxml, "test").unwrap();
        let invoke = model
            .states
            .values()
            .find_map(|s| s.invokes.first())
            .expect("expected at least one invoke");
        match invoke {
            Invoke::MeshRpc(info) => info.clone(),
            other => panic!("expected MeshRpc invoke, got {other:?}"),
        }
    }

    /// Helper: parse a fragment, expect `MeshRpcReservedParam`, return the
    /// `(param, detail)` pair. Any other outcome is a test failure.
    fn first_mesh_rpc_violation(scxml: &str) -> (String, String) {
        use crate::forge::error::{ForgeError, ValidationError};
        let mut parser = SCXMLParser::new();
        let err = parser.parse_string(scxml, "test").unwrap_err();
        match err.error {
            ForgeError::Validation(ValidationError::MeshRpcReservedParam {
                param,
                detail,
            }) => (param, detail),
            other => panic!("expected MeshRpcReservedParam, got {other:?}"),
        }
    }

    #[test]
    fn mesh_rpc_invoke_happy_path_populates_envelope_fields() {
        // Extra `#` on the raw-string delimiter because `src="#motor"`
        // literally contains a `"#` byte sequence that would otherwise
        // terminate the standard `r#"..."#` form early.
        let scxml = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s">
                <invoke type="sce:mesh-rpc" src="#motor">
                    <param name="_mesh_event" expr="'service.request.compute_force'"/>
                    <param name="_mesh_deadline_ms" expr="'250'"/>
                    <param name="torque" expr="'42'"/>
                </invoke>
            </state>
        </scxml>"##;
        let info = first_mesh_rpc_invoke(scxml);
        assert_eq!(info.src, "#motor");
        assert_eq!(info.mesh_event, "service.request.compute_force");
        assert_eq!(info.deadline_ms, Some(250));
        // Reserved names are stripped from the payload; author params
        // pass through unchanged.
        assert_eq!(info.base.params.len(), 1);
        assert_eq!(info.base.params[0].name, "torque");
    }

    #[test]
    fn mesh_rpc_invoke_rejects_missing_mesh_event() {
        let scxml = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s">
                <invoke type="sce:mesh-rpc" src="#motor">
                    <param name="torque" expr="'42'"/>
                </invoke>
            </state>
        </scxml>"##;
        let (param, detail) = first_mesh_rpc_violation(scxml);
        assert_eq!(param, "_mesh_event");
        assert!(
            detail.contains("required") && detail.contains("_mesh_event"),
            "expected 'required _mesh_event' phrasing, got: {detail}",
        );
    }

    #[test]
    fn mesh_rpc_invoke_rejects_duplicate_mesh_event() {
        let scxml = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s">
                <invoke type="sce:mesh-rpc" src="#motor">
                    <param name="_mesh_event" expr="'x'"/>
                    <param name="_mesh_event" expr="'y'"/>
                </invoke>
            </state>
        </scxml>"##;
        let (param, detail) = first_mesh_rpc_violation(scxml);
        assert_eq!(param, "_mesh_event");
        assert!(
            detail.contains("exactly once"),
            "expected 'exactly once' phrasing, got: {detail}",
        );
    }

    #[test]
    fn mesh_rpc_invoke_rejects_unknown_reserved_prefix() {
        let scxml = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s">
                <invoke type="sce:mesh-rpc" src="#motor">
                    <param name="_mesh_event" expr="'x'"/>
                    <param name="_mesh_oracle" expr="'42'"/>
                </invoke>
            </state>
        </scxml>"##;
        let (param, detail) = first_mesh_rpc_violation(scxml);
        assert_eq!(param, "_mesh_oracle");
        assert!(
            detail.contains("reserved"),
            "expected 'reserved' phrasing, got: {detail}",
        );
    }

    #[test]
    fn mesh_rpc_invoke_rejects_non_integer_deadline() {
        let scxml = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s">
                <invoke type="sce:mesh-rpc" src="#motor">
                    <param name="_mesh_event" expr="'x'"/>
                    <param name="_mesh_deadline_ms" expr="'abc'"/>
                </invoke>
            </state>
        </scxml>"##;
        let (param, detail) = first_mesh_rpc_violation(scxml);
        assert_eq!(param, "_mesh_deadline_ms");
        assert!(
            detail.contains("non-negative integer"),
            "expected 'non-negative integer' phrasing, got: {detail}",
        );
    }

    #[test]
    fn mesh_rpc_invoke_accepts_bare_deadline_literal() {
        // Bare numeric literal (`expr="50"` not `expr="'50'"`) is the
        // idiomatic shape for integer-typed params and should parse
        // through the same path as the quoted form.
        let scxml = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" initial="s">
            <state id="s">
                <invoke type="sce:mesh-rpc" src="#motor">
                    <param name="_mesh_event" expr="'x'"/>
                    <param name="_mesh_deadline_ms" expr="50"/>
                </invoke>
            </state>
        </scxml>"##;
        let info = first_mesh_rpc_invoke(scxml);
        assert_eq!(info.deadline_ms, Some(50));
    }

    // ── Session C/D attribute deprecation → Stage 2 hard error ─────

    /// Helper: parse a document and return the first RemovedAttribute
    /// diagnostic payload. Panics with an explanatory message if the
    /// parser succeeded (meaning Stage 2 enforcement is broken).
    fn first_removed_attribute(scxml: &str) -> (String, Option<String>) {
        use crate::forge::error::{ForgeError, ValidationError};
        let mut p = SCXMLParser::new();
        let res = p.parse_string(scxml, "test");
        let err = match res {
            Ok(_) => panic!(
                "parser should reject deprecated sce:* attributes \
                 as hard errors (got Ok)"
            ),
            Err(e) => e,
        };
        match err.error {
            ForgeError::Validation(ValidationError::RemovedAttribute {
                attribute,
                event,
            }) => (attribute, event),
            other => panic!("expected RemovedAttribute, got: {other:?}"),
        }
    }

    #[test]
    fn send_with_sce_qos_is_hard_error() {
        let scxml = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                              xmlns:sce="http://sce.dev/ext"
                              version="1.0" initial="s">
            <state id="s">
                <onentry>
                    <send event="brake.activate" target="#motor" sce:qos="reliable"/>
                </onentry>
            </state>
        </scxml>"##;
        let (attr, event) = first_removed_attribute(scxml);
        assert_eq!(attr, "sce:qos");
        assert_eq!(event.as_deref(), Some("brake.activate"));
    }

    #[test]
    fn send_with_sce_pattern_is_hard_error() {
        let scxml = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                              xmlns:sce="http://sce.dev/ext"
                              version="1.0" initial="s">
            <state id="s">
                <onentry>
                    <send event="svc.call" target="#motor" sce:pattern="request"/>
                </onentry>
            </state>
        </scxml>"##;
        let (attr, _) = first_removed_attribute(scxml);
        assert_eq!(attr, "sce:pattern");
    }

    #[test]
    fn send_with_sce_reply_timeout_is_hard_error() {
        // reply-timeout was missing from the Stage 1 watch list — Stage 2
        // picks it up alongside qos/pattern/reply-event/deadline/priority.
        let scxml = r##"<scxml xmlns="http://www.w3.org/2005/07/scxml"
                              xmlns:sce="http://sce.dev/ext"
                              version="1.0" initial="s">
            <state id="s">
                <onentry>
                    <send event="svc.call" target="#motor" sce:reply-timeout="500"/>
                </onentry>
            </state>
        </scxml>"##;
        let (attr, _) = first_removed_attribute(scxml);
        assert_eq!(attr, "sce:reply-timeout");
    }
}
