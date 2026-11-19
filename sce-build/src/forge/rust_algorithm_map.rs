// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! How a Rust algorithm holds a map or a set (docs/adr/0108, D5, D6 and D8):
//! in the runtime's `BoundedMap<K, V, N>` and `BoundedSet<K, N>`, named
//! through the leaf crate an algorithm names its other bounded types by
//! (`sce_portable_bytes`), which is the type a machine's variable of the same
//! parts is ([`crate::forge::static_lowering::StaticTarget::map_type`]): a
//! record and a value of an enum in the type its unit declares, every other
//! part in the type a value of its type is held in (D6 item 4).
//!
//! * A **parameter** states no bound of its own: its map is the caller's,
//!   whatever bound the caller's variable states, so the function is generic
//!   over it (`const N0: usize`) and takes the map lent, `&` to be read (D6
//!   item 1) and `&mut` where it is declared `inout` (D6 item 2). Nothing is
//!   copied into or out of it.
//! * The **local a body returns** is the caller's destination, as a returned
//!   list is (docs/adr/0082): a parameter of its own name after the declared
//!   ones, `&mut` the map of the local's bound, emptied where the local is
//!   declared, and the return says only that it was written. A map return
//!   declares `may-fail` (D5), so its `Result` carries the failure.
//! * Any **other local** is the body's own, built empty under a seed drawn,
//!   by its number among such locals, from the seed of a map the algorithm was
//!   lent or the destination it writes (the runtime's `derived_seed`): the
//!   caller's entropy, as each map of a machine draws its seed from the
//!   machine's (D8). An algorithm that is lent no map and writes none has no
//!   seed to draw from and is refused by name.
//!
//! `<sce:put>` and `<sce:remove>` are the map's own `put` and `remove`, whose
//! answer the statement writes into its `outcome` as the variant of the same
//! name of the standard enum; a put that names none fails `capacity-exceeded`
//! on a full map, which only a body that declares `may-fail` may write. A
//! `<sce:foreach>` walks the map's positions, over the map itself where
//! nothing in its body writes the map and over a copy taken where the loop
//! begins where something does (D6 item 3).
//!
//! A key and a value are numbers, truth values, values of an enum and
//! records; a text and a byte string, which an algorithm holds in types of
//! its own, are refused by name as a part of an algorithm's map.

use std::collections::HashMap;

use crate::forge::error::{ForgeError, GenerateError};
use crate::forge::model::{
    AlgorithmBinding, AlgorithmModel, AlgorithmStmt, AlgorithmValueType, SceType,
};

/// How a key or a value of a map is spelled as a Rust type, or `None` for a
/// part no Rust algorithm's map holds ([`RustAlgorithmMaps::of`]).
pub(crate) type PartSpelling<'a> =
    &'a dyn Fn(&AlgorithmValueType) -> Result<Option<String>, ForgeError>;

/// The leaf crate a Rust algorithm's map type and its outcomes are named in.
pub(crate) const RUST_ALGORITHM_MAP_CRATE: &str = "::sce_portable_bytes";

/// The module whose functions a Rust algorithm's reads of a map call
/// ([`crate::forge::types::TypeCtx::rust_map_reads`]).
pub(crate) const RUST_ALGORITHM_MAP_READS: &str = "::sce_portable_bytes::map";

/// How a map or a set the body holds came to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MapRole {
    /// A parameter, the caller's map lent: `&` it, or `&mut` it where it is
    /// written in place.
    Lent { inout: bool },
    /// The local the body returns, the caller's destination.
    Output,
    /// Any other local, built empty under the seed numbered `ordinal`.
    Working { ordinal: u32 },
}

/// One map or set the body holds.
#[derive(Debug, Clone)]
pub(crate) struct HeldMap {
    /// Its type, its bound in it.
    pub(crate) ty: String,
    /// Its key's declared type.
    pub(crate) key: AlgorithmValueType,
    /// Its value's declared type; `None` for a set.
    pub(crate) value: Option<AlgorithmValueType>,
    pub(crate) role: MapRole,
}

/// Every map and set a Rust algorithm holds, by the name it is declared
/// under, and what its signature gains for them.
#[derive(Debug, Clone, Default)]
pub(crate) struct RustAlgorithmMaps {
    held: HashMap<String, HeldMap>,
    /// The bound each parameter takes from its caller, `const N<i>: usize`
    /// for the parameter at position `i`.
    bounds: Vec<String>,
    /// The local the body returns, by its declared name.
    output: Option<String>,
    /// What a working local's seed is drawn from: the first map parameter, or
    /// the destination, as the body spells it.
    seed_source: Option<String>,
    /// Each local of an enum, by name, with its alias: what an `outcome` names.
    enum_locals: HashMap<String, String>,
}

impl RustAlgorithmMaps {
    /// The maps `m` holds. `part` spells a key or a value of a map as a Rust
    /// type, or answers `None` for a part no Rust algorithm's map holds;
    /// `local` spells a name as the body does. Refused by name: a part
    /// `part` does not spell, a map return that is not one local returned by
    /// its name, and a working local of an algorithm with no map to draw its
    /// seed from.
    pub(crate) fn of(
        m: &AlgorithmModel,
        part: PartSpelling<'_>,
        local: &dyn Fn(&str) -> String,
    ) -> Result<Self, ForgeError> {
        let mut maps = Self::default();
        let refused = |detail: String| -> ForgeError {
            GenerateError::unsupported(format!(
                "algorithm '{}' {detail}, which Rust does not lower in an algorithm ({})",
                m.name,
                crate::forge::value_type::MAP_LANDING
            ))
            .into()
        };
        let spelled = |held: &str, ty: &AlgorithmValueType, bound: &str| {
            map_type(ty, bound, part).and_then(|spelled| {
                spelled.ok_or_else(|| {
                    refused(format!(
                        "holds {held} of {}, a map whose key or value is a text or a byte string",
                        ty.as_attr()
                    ))
                })
            })
        };
        for (index, param) in m.signature.params.iter().enumerate() {
            if param.sce_type.map_key().is_none() {
                continue;
            }
            let bound = match param.sce_type.stated_outer_bound() {
                Some(bound) => bound.max.to_string(),
                None => {
                    let bound = format!("N{index}");
                    maps.bounds.push(format!("const {bound}: usize"));
                    bound
                }
            };
            let held = format!("`{}`", param.name);
            let ty = spelled(&held, &param.sce_type, &bound)?;
            maps.insert(
                &param.name,
                &param.sce_type,
                ty,
                MapRole::Lent {
                    inout: param.written_in_place(),
                },
            );
            if maps.seed_source.is_none() {
                maps.seed_source = Some(local(&param.name));
            }
        }
        let locals: Vec<(&str, &AlgorithmValueType)> = m
            .body_bindings()
            .into_iter()
            .filter_map(|binding| match binding {
                AlgorithmBinding::Local { name, sce_type } => Some((name, sce_type)),
                AlgorithmBinding::ForeachItem { .. }
                | AlgorithmBinding::ForeachKey { .. }
                | AlgorithmBinding::RecordLocal { .. }
                | AlgorithmBinding::UnionLocal { .. } => None,
            })
            .collect();
        for &(name, ty) in &locals {
            if let Some(SceType::Enum(reference)) = ty.scalar() {
                maps.enum_locals
                    .insert(name.to_string(), reference.alias.clone());
            }
        }
        let map_locals: Vec<(&str, &AlgorithmValueType)> = locals
            .into_iter()
            .filter(|(_, ty)| ty.map_key().is_some())
            .collect();
        // The local a map return returns, by its name at every return.
        if m.signature
            .return_type
            .as_ref()
            .is_some_and(|ty| ty.map_key().is_some())
        {
            let mut returned = Vec::new();
            returned_names(&m.body, &mut returned);
            let output = match returned.as_slice() {
                [first, rest @ ..]
                    if rest.iter().all(|name| name == first)
                        && map_locals.iter().any(|(name, _)| *name == first.as_str()) =>
                {
                    first.clone()
                }
                _ => {
                    return Err(refused(
                        "returns a map that is not one map local, named at every \
                         <sce:return>"
                            .to_string(),
                    ))
                }
            };
            maps.output = Some(output);
        }
        let mut ordinal = 0u32;
        for &(name, ty) in &map_locals {
            let bound = ty
                .stated_outer_bound()
                .map(|bound| bound.max)
                .ok_or_else(|| {
                    GenerateError::InvalidConfig(format!(
                        "algorithm '{}' local '{name}' of {} states no bound — the parser \
                         requires one on every map local",
                        m.name,
                        ty.as_attr()
                    ))
                })?;
            let held = format!("`{name}`");
            let local_ty = spelled(&held, ty, &bound.to_string())?;
            let role = if maps.output.as_deref() == Some(name) {
                MapRole::Output
            } else {
                let role = MapRole::Working { ordinal };
                ordinal += 1;
                role
            };
            maps.insert(name, ty, local_ty, role);
        }
        if maps.seed_source.is_none() {
            maps.seed_source = maps.output.as_deref().map(local);
        }
        if ordinal > 0 && maps.seed_source.is_none() {
            return Err(refused(
                "builds a map local and is lent no map and returns none, so there is no \
                 seed to draw the local's from (D8)"
                    .to_string(),
            ));
        }
        Ok(maps)
    }

    fn insert(&mut self, name: &str, ty: &AlgorithmValueType, spelled: String, role: MapRole) {
        let (key, value) = match ty {
            AlgorithmValueType::Map { key, value, .. } => {
                ((**key).clone(), Some((**value).clone()))
            }
            AlgorithmValueType::Set { key, .. } => ((**key).clone(), None),
            AlgorithmValueType::Scalar(_)
            | AlgorithmValueType::Text { .. }
            | AlgorithmValueType::Bytes { .. }
            | AlgorithmValueType::Record { .. }
            | AlgorithmValueType::Union { .. }
            | AlgorithmValueType::List { .. }
            | AlgorithmValueType::Optional { .. } => return,
        };
        self.held.insert(
            name.to_string(),
            HeldMap {
                ty: spelled,
                key,
                value,
                role,
            },
        );
    }

    /// The map or set declared as `name`, if the body holds one so.
    pub(crate) fn held(&self, name: &str) -> Option<&HeldMap> {
        self.held.get(name)
    }

    /// Whether the body holds a map or a set declared as `name`.
    pub(crate) fn holds(&self, name: &str) -> bool {
        self.held.contains_key(name)
    }

    /// The bounds the function takes from its callers, `<const N0: usize>`, or
    /// nothing for a function lent no map that states none.
    pub(crate) fn generics(&self) -> String {
        if self.bounds.is_empty() {
            String::new()
        } else {
            format!("<{}>", self.bounds.join(", "))
        }
    }

    /// The type of the parameter `name`, a map lent: `&` the map, or `&mut`
    /// it where the algorithm writes it in place. `None` for any other
    /// parameter.
    pub(crate) fn param_type(&self, name: &str) -> Option<String> {
        match self.held.get(name)? {
            HeldMap {
                ty,
                role: MapRole::Lent { inout: true },
                ..
            } => Some(format!("&mut {ty}")),
            HeldMap {
                ty,
                role: MapRole::Lent { inout: false },
                ..
            } => Some(format!("&{ty}")),
            HeldMap {
                role: MapRole::Output | MapRole::Working { .. },
                ..
            } => None,
        }
    }

    /// The parameter the caller's destination is, after the declared ones, as
    /// `local` spells the local the body returns; `None` for an algorithm
    /// that returns no map.
    pub(crate) fn destination(&self, local: &dyn Fn(&str) -> String) -> Option<String> {
        let name = self.output.as_deref()?;
        let held = self.held.get(name)?;
        Some(format!("{}: &mut {}", local(name), held.ty))
    }

    /// The local the body returns, by its declared name.
    pub(crate) fn output(&self) -> Option<&str> {
        self.output.as_deref()
    }

    /// The alias of the enum the local `name` holds a value of: what a
    /// statement's `outcome` names.
    pub(crate) fn outcome_alias(&self, name: &str) -> Option<&str> {
        self.enum_locals.get(name).map(String::as_str)
    }

    /// Each name an expression reads a map the body is lent through, to its
    /// spelling there: the map behind the reference the parameter or the
    /// destination is, so a read borrows the map itself (`&(*m)`), as it does
    /// a local.
    pub(crate) fn renames(&self, local: &dyn Fn(&str) -> String) -> Vec<(String, String)> {
        self.held
            .iter()
            .filter(|(_, held)| matches!(held.role, MapRole::Lent { .. } | MapRole::Output))
            .map(|(name, _)| (name.clone(), format!("*{}", local(name))))
            .collect()
    }

    /// The declaration of the local `name`, spelled `local`, at `pad`: the
    /// caller's destination emptied, or a working local built empty under its
    /// seed, `mutable` where a statement writes it. `None` for a name that is
    /// no map local.
    pub(crate) fn declare(
        &self,
        pad: &str,
        name: &str,
        local: &str,
        mutable: bool,
    ) -> Option<String> {
        let held = self.held.get(name)?;
        match held.role {
            MapRole::Output => Some(format!("{pad}{local}.clear();\n")),
            MapRole::Working { ordinal } => {
                let source = self.seed_source.as_deref()?;
                let binding = if mutable { "let mut" } else { "let" };
                Some(format!(
                    "{pad}{binding} {local}: {} = {RUST_ALGORITHM_MAP_CRATE}::FromSeed::from_seed(\
                     {source}.derived_seed({ordinal}));\n",
                    held.ty
                ))
            }
            MapRole::Lent { .. } => None,
        }
    }
}

/// The type of the map or set `ty` of the bound `bound`, each part spelled by
/// `part`; `Ok(None)` for a part `part` does not spell.
fn map_type(
    ty: &AlgorithmValueType,
    bound: &str,
    part: PartSpelling<'_>,
) -> Result<Option<String>, ForgeError> {
    Ok(match ty {
        AlgorithmValueType::Map { key, value, .. } => match (part(key)?, part(value)?) {
            (Some(key), Some(value)) => Some(format!(
                "{RUST_ALGORITHM_MAP_CRATE}::BoundedMap<{key}, {value}, {bound}>"
            )),
            _ => None,
        },
        AlgorithmValueType::Set { key, .. } => {
            part(key)?.map(|key| format!("{RUST_ALGORITHM_MAP_CRATE}::BoundedSet<{key}, {bound}>"))
        }
        AlgorithmValueType::Scalar(_)
        | AlgorithmValueType::Text { .. }
        | AlgorithmValueType::Bytes { .. }
        | AlgorithmValueType::Record { .. }
        | AlgorithmValueType::Union { .. }
        | AlgorithmValueType::List { .. }
        | AlgorithmValueType::Optional { .. } => None,
    })
}

/// Every name a `<sce:return>` of `stmts` returns, nested bodies included, in
/// document order.
fn returned_names(stmts: &[AlgorithmStmt], into: &mut Vec<String>) {
    for stmt in stmts {
        match stmt {
            AlgorithmStmt::Return {
                expr: Some(expr), ..
            } => into.push(expr.trim().to_string()),
            AlgorithmStmt::If {
                then_body,
                else_body,
                ..
            } => {
                returned_names(then_body, into);
                if let Some(else_body) = else_body {
                    returned_names(else_body, into);
                }
            }
            AlgorithmStmt::While { body, .. } | AlgorithmStmt::Foreach { body, .. } => {
                returned_names(body, into);
            }
            AlgorithmStmt::Return { expr: None, .. }
            | AlgorithmStmt::Var { .. }
            | AlgorithmStmt::RecordVar { .. }
            | AlgorithmStmt::RecordFromCall { .. }
            | AlgorithmStmt::UnionVar { .. }
            | AlgorithmStmt::UnionCopied { .. }
            | AlgorithmStmt::Assign { .. }
            | AlgorithmStmt::Append { .. }
            | AlgorithmStmt::Put { .. }
            | AlgorithmStmt::Remove { .. }
            | AlgorithmStmt::Clear { .. }
            | AlgorithmStmt::Call { .. }
            | AlgorithmStmt::Require { .. } => {}
        }
    }
}

/// What a statement of a map writes into its `outcome`: the place, and the
/// document enum's spelling of each variant of the runtime's answer, in the
/// runtime's order (`static_lowering::RUST_PUT_OUTCOMES`,
/// `RUST_REMOVE_OUTCOMES`).
pub(crate) struct Outcome<'a> {
    pub(crate) place: &'a str,
    pub(crate) variants: &'a [String],
}

/// `<place> = match <call> { … };`: the runtime's answer `runtime`, of the
/// variants `declared`, written as the variant of the same name of the
/// document's enum.
fn written_outcome(
    pad: &str,
    call: &str,
    (runtime, declared): (&str, &[&str]),
    outcome: &Outcome<'_>,
) -> String {
    let arms: Vec<String> = declared
        .iter()
        .zip(outcome.variants)
        .map(|(variant, spelled)| {
            format!(
                "{RUST_ALGORITHM_MAP_CRATE}::{runtime}::{} => {spelled}",
                crate::filters::to_pascal_case((*variant).to_string())
            )
        })
        .collect();
    format!(
        "{pad}{} = match {call} {{ {} }};\n",
        outcome.place,
        arms.join(", ")
    )
}

/// `<sce:put>` of `key`, and `value` for a map, into the map spelled `map`:
/// the outcome written where the statement names one, and otherwise
/// `full` statement (the return of `capacity-exceeded`) run where the map is
/// full.
pub(crate) fn put(
    pad: &str,
    map: &str,
    key: &str,
    value: Option<&str>,
    outcome: Option<&Outcome<'_>>,
    full: &str,
) -> String {
    let call = match value {
        Some(value) => format!("{map}.put({key}, {value})"),
        None => format!("{map}.put({key})"),
    };
    match outcome {
        Some(outcome) => written_outcome(
            pad,
            &call,
            crate::forge::static_lowering::RUST_PUT_OUTCOMES,
            outcome,
        ),
        None => format!(
            "{pad}if {call} == {RUST_ALGORITHM_MAP_CRATE}::PutOutcome::Full {{\n\
             {pad}    {full}\n\
             {pad}}}\n"
        ),
    }
}

/// `<sce:remove>` of `key` from the map spelled `map`: the outcome written
/// where the statement names one; a key that is absent leaves the map as it
/// was.
pub(crate) fn remove(pad: &str, map: &str, key: &str, outcome: Option<&Outcome<'_>>) -> String {
    let call = format!("{map}.remove(&{key})");
    match outcome {
        Some(outcome) => written_outcome(
            pad,
            &call,
            crate::forge::static_lowering::RUST_REMOVE_OUTCOMES,
            outcome,
        ),
        None => format!("{pad}let _ = {call};\n"),
    }
}

/// `<sce:clear>` of the map spelled `map`.
pub(crate) fn clear(pad: &str, map: &str) -> String {
    format!("{pad}{map}.clear();\n")
}

/// The walk of a map or a set `held`, spelled `map`, at `pad`, a loop of its
/// own numbered `depth` (D6 item 3): what opens it, the body to be lowered
/// two levels in, and what closes it. Over a copy taken where it begins where
/// `copies`, its body writing the map, and over the map itself otherwise,
/// which nothing in the body changes. Each turn binds `item` (each value of a
/// map, each key of a set) and `key` (each key of a map): a number and a
/// truth value by copy, any other part by a clone of it, so no turn holds a
/// borrow of the map.
pub(crate) struct Walk<'a> {
    pub(crate) held: &'a HeldMap,
    pub(crate) map: &'a str,
    pub(crate) item: &'a str,
    pub(crate) key: Option<&'a str>,
    pub(crate) copies: bool,
    pub(crate) depth: usize,
}

impl Walk<'_> {
    /// What opens the walk at `pad`, and what closes it.
    pub(crate) fn spelled(&self, pad: &str) -> (String, String) {
        let depth = self.depth;
        let (cursor, at) = (format!("sce_cursor_{depth}"), format!("sce_at_{depth}"));
        let (walked, copy) = if self.copies {
            let walked = format!("sce_walk_{depth}");
            let copy = format!("{pad}    let {walked} = {}.clone();\n", self.map);
            (walked, copy)
        } else {
            (self.map.to_string(), String::new())
        };
        let read = |part: &AlgorithmValueType, accessor: &str| {
            if copied(part) {
                format!("*{walked}.{accessor}({at})")
            } else {
                format!("{walked}.{accessor}({at}).clone()")
            }
        };
        let item = match &self.held.value {
            Some(value) => read(value, "value_at"),
            None => read(&self.held.key, "key_at"),
        };
        let mut bindings = vec![format!("let {} = {item};", self.item)];
        let mut bound = vec![format!("&{}", self.item)];
        if let Some(key) = self.key {
            bindings.push(format!("let {key} = {};", read(&self.held.key, "key_at")));
            bound.push(format!("&{key}"));
        }
        bindings.push(format!("let _ = ({},);", bound.join(", ")));
        let inner = format!("{pad}        ");
        let open = format!(
            "{pad}{{\n\
             {copy}\
             {pad}    let mut {cursor} = {walked}.first();\n\
             {pad}    while let Some({at}) = {cursor} {{\n\
             {inner}{cursor} = {walked}.next_after({at});\n\
             {inner}{}\n",
            bindings.join(&format!("\n{inner}"))
        );
        let close = format!("{pad}    }}\n{pad}}}\n");
        (open, close)
    }
}

/// Whether a key or a value of type `part` is bound by copy where a walk
/// reads it: a number and a truth value.
fn copied(part: &AlgorithmValueType) -> bool {
    matches!(
        part,
        AlgorithmValueType::Scalar(scalar)
            if !matches!(scalar, SceType::Enum(_) | SceType::String | SceType::Bytes)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn held(value: Option<AlgorithmValueType>) -> HeldMap {
        HeldMap {
            ty: "::sce_portable_bytes::BoundedMap<u32, u64, N0>".to_string(),
            key: AlgorithmValueType::Scalar(SceType::Uint32),
            value,
            role: MapRole::Lent { inout: true },
        }
    }

    /// docs/adr/0108, D6 item 3: a walk whose body does not write its map
    /// walks the map itself, and one whose body does walks a copy taken where
    /// it begins; a number is bound by copy.
    #[test]
    fn a_walk_copies_the_map_only_where_its_body_writes_it() {
        let map = held(Some(AlgorithmValueType::Scalar(SceType::Uint64)));
        let walk = Walk {
            held: &map,
            map: "resets",
            item: "v",
            key: Some("k"),
            copies: false,
            depth: 1,
        };
        let (open, close) = walk.spelled("    ");
        assert!(!open.contains("clone()"), "{open}");
        assert!(
            open.contains("let mut sce_cursor_1 = resets.first();"),
            "{open}"
        );
        assert!(
            open.contains("let v = *resets.value_at(sce_at_1);"),
            "{open}"
        );
        assert!(open.contains("let k = *resets.key_at(sce_at_1);"), "{open}");
        assert_eq!(close, "        }\n    }\n");
        let copied = Walk {
            copies: true,
            ..walk
        };
        let (open, _) = copied.spelled("    ");
        assert!(open.contains("let sce_walk_1 = resets.clone();"), "{open}");
        assert!(
            open.contains("let v = *sce_walk_1.value_at(sce_at_1);"),
            "{open}"
        );
    }

    /// docs/adr/0108, D3 and D4: a put and a remove write the runtime's
    /// answer into the outcome as the document enum's variant of the same
    /// name, and a put that names none returns the failure of a full map.
    #[test]
    fn a_put_writes_its_outcome_or_fails_on_a_full_map() {
        let variants = [
            "E::Inserted".to_string(),
            "E::Replaced".to_string(),
            "E::Full".to_string(),
        ];
        let outcome = Outcome {
            place: "stored",
            variants: &variants,
        };
        assert_eq!(
            put("", "m", "k", Some("v"), Some(&outcome), "return Err(e);"),
            "stored = match m.put(k, v) { ::sce_portable_bytes::PutOutcome::Inserted => \
             E::Inserted, ::sce_portable_bytes::PutOutcome::Replaced => E::Replaced, \
             ::sce_portable_bytes::PutOutcome::Full => E::Full };\n"
        );
        assert_eq!(
            put("", "m", "k", None, None, "return Err(e);"),
            "if m.put(k) == ::sce_portable_bytes::PutOutcome::Full {\n    return Err(e);\n}\n"
        );
        assert_eq!(remove("", "m", "k", None), "let _ = m.remove(&k);\n");
    }
}
