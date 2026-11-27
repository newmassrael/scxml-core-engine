// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! How a Rust algorithm holds a map or a set (docs/adr/0108, D5, D6 and D8):
//! in the runtime's `BoundedMap<K, V, N>` and `BoundedSet<K, N>`, named
//! through the leaf crate an algorithm names its other bounded types by
//! (`sce_portable_bytes`), which is the type a machine's variable of the same
//! parts is ([`crate::forge::static_lowering::StaticTarget::map_type`]): a
//! record and a value of an enum in the type its unit declares, a text in
//! `SceString<N>` and a byte string in `SceBytes<N>`, and every other part in
//! the type a value of its type is held in (D6 item 4).
//!
//! * A **parameter** states no bound of its own: its map is the caller's,
//!   whatever bound the caller's variable states and in whatever storage the
//!   caller keeps it (`Inline`, `Heap`, `Linear`, or a run its host lends,
//!   `Lent` or `LentIndexed`: D4 "Storage", docs/adr/0154), so the function is
//!   generic over both (`const N0: usize`, `S0: MapStorage`) and takes the map
//!   lent, `&` to be read (D6 item 1) and `&mut` where it is declared `inout`
//!   (D6 item 2). A text or a byte string key or value is held to the bound the
//!   parameter's type states inside it, or, where it states none, to the
//!   caller's, a bound the function is generic over too (`K0` for the key's,
//!   `V0` for the value's). Nothing is copied into or out of the map.
//! * The **local a body returns** is the caller's destination, as a returned
//!   list is (docs/adr/0082): a parameter of its own name after the declared
//!   ones, `&mut` the map of the local's bound in the caller's storage (`SR`),
//!   emptied where the local is declared, and the return says only that it was
//!   written. A map return declares `may-fail` (D5), so its `Result` carries
//!   the failure.
//! * Any **other local** is the body's own, built empty under a seed drawn,
//!   by its number among such locals, from the seed of a map the algorithm was
//!   lent or the destination it writes (the runtime's `derived_seed`): the
//!   caller's entropy, as each map of a machine draws its seed from the
//!   machine's (D8). An algorithm that is lent no map and returns none, and
//!   builds a map or calls an algorithm that takes a seed, takes the seed as
//!   a parameter of its own after the declared ones, `sce_map_seed`
//!   ([`crate::forge::model::AlgorithmModel::takes_map_seed`]), which its
//!   caller hands it, and draws each local's from that. An algorithm that
//!   calls one hands it a seed drawn from its own, after its locals'. Such a
//!   local is kept in the owned storage of the map its seed is drawn from
//!   (`<S0 as MapStorage>::Owned`, docs/adr/0154, D3): the map's own storage
//!   where it holds its slots, so an algorithm lent an inline map builds its
//!   own maps inline and allocates nothing, and the storage that holds them in
//!   the value with the same lookup where its host lends them, a run being lent
//!   to one map. One that takes its caller's seed is lent no storage to follow,
//!   and its locals take the default (`DefaultMapStorage`).
//!
//! Each map an algorithm is lent or writes into is a variable of its caller,
//! kept in the storage the caller chose for it as it chose its bound, so the
//! function is generic over each map's storage apart, as it is over each
//! bound: a caller may lend an inline map beside one that grows.
//!
//! `<sce:put>` and `<sce:remove>` are the map's own `put` and `remove`, whose
//! answer the statement writes into its `outcome` as the variant of the same
//! name of the standard enum; a put that names none fails `capacity-exceeded`
//! on a full map, which only a body that declares `may-fail` may write. A
//! `<sce:foreach>` walks the map's positions, over the map itself where
//! nothing in its body writes the map and over a copy taken where the loop
//! begins where something does (D6 item 3), the copy in the map's owned
//! storage (`to_owned_storage`, docs/adr/0154, D3), which a map in storage its
//! host lends has, and `Clone` does not.

use std::collections::HashMap;

use crate::forge::error::{ForgeError, GenerateError};
use crate::forge::model::{
    AlgorithmBinding, AlgorithmModel, AlgorithmStmt, AlgorithmValueType, SceType,
};

/// How a key or a value of a map that is no text and no byte string is
/// spelled as a Rust type, or `None` for a part no Rust algorithm's map holds
/// ([`RustAlgorithmMaps::of`]).
pub(crate) type PartSpelling<'a> =
    &'a dyn Fn(&AlgorithmValueType) -> Result<Option<String>, ForgeError>;

/// The leaf crate a Rust algorithm's map type and its outcomes are named in.
pub(crate) const RUST_ALGORITHM_MAP_CRATE: &str = "::sce_portable_bytes";

/// The module whose functions a Rust algorithm's reads of a map call
/// ([`crate::forge::types::AlgorithmMapReads::Rust`]).
pub(crate) const RUST_ALGORITHM_MAP_READS: &str = "::sce_portable_bytes::map";

/// The storage the destination of a map return is kept in, which the function
/// is generic over ([`RustAlgorithmMaps::destination`]): the caller's.
pub(crate) const RUST_ALGORITHM_OUTPUT_STORAGE: &str = "SR";

/// The parameter an algorithm that takes its caller's seed takes it in
/// ([`RustAlgorithmMaps::seed_parameter`]). Under the generator's own prefix,
/// which no author's name spells (`rust_local_spelling`).
pub(crate) const RUST_ALGORITHM_SEED_PARAM: &str = "sce_map_seed";

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

/// Which bounded type a text or a byte string part of a map is held in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BoundedPart {
    /// `SceString<N>`.
    Text,
    /// `SceBytes<N>`.
    Bytes,
}

impl BoundedPart {
    /// The part `ty` is, where it is a text or a byte string: what a key or a
    /// value put into a map of it is built as.
    pub(crate) fn of(ty: &AlgorithmValueType) -> Option<Self> {
        match ty {
            AlgorithmValueType::Text { .. } => Some(Self::Text),
            AlgorithmValueType::Bytes { .. } => Some(Self::Bytes),
            AlgorithmValueType::Scalar(_)
            | AlgorithmValueType::Record { .. }
            | AlgorithmValueType::Union { .. }
            | AlgorithmValueType::List { .. }
            | AlgorithmValueType::Map { .. }
            | AlgorithmValueType::Set { .. }
            | AlgorithmValueType::Optional { .. } => None,
        }
    }

    /// The type it is held in, of the bound `bound`.
    fn ty(self, bound: &str) -> String {
        match self {
            Self::Text => format!("{RUST_ALGORITHM_MAP_CRATE}::SceString<{bound}>"),
            Self::Bytes => format!("{RUST_ALGORITHM_MAP_CRATE}::SceBytes<{bound}>"),
        }
    }

    /// A value of it built from `from`, held in the same type of a bound the
    /// build proves no greater (a walk's item or key, a record's text field):
    /// the widening is a build error where it does not fit.
    pub(crate) fn widened(self, bound: &str, from: &str) -> String {
        match self {
            Self::Text => {
                format!("{RUST_ALGORITHM_MAP_CRATE}::SceString::<{bound}>::widened(&{from})")
            }
            Self::Bytes => {
                format!("{RUST_ALGORITHM_MAP_CRATE}::SceBytes::<{bound}>::widened(&{from})")
            }
        }
    }

    /// A value of it built from `from`, a text or a byte string whose length
    /// the build does not know, in a body that declares `may-fail`: one past
    /// the bound fails `capacity-exceeded`, as an append past a buffer's does.
    pub(crate) fn checked(self, bound: &str, from: &str) -> String {
        let built = match self {
            Self::Text => {
                format!("{RUST_ALGORITHM_MAP_CRATE}::SceString::<{bound}>::from_text(&({from}))")
            }
            Self::Bytes => {
                format!("{RUST_ALGORITHM_MAP_CRATE}::SceBytes::<{bound}>::from_slice(&({from}))")
            }
        };
        format!(
            "{built}.map_err(|_| sce_forge_runtime::algorithm::AlgorithmError::CapacityExceeded)?"
        )
    }
}

/// A text built from the string literal whose text is `literal`, held in a
/// `SceString` of the bound `bound`: the literal's length is judged against
/// the bound at build time, so one that does not fit is a build error.
pub(crate) fn literal_text(bound: &str, literal: &str) -> String {
    let spelled = format!("{literal:?}");
    format!(
        "{RUST_ALGORITHM_MAP_CRATE}::SceString::<{bound}>::proven::<{{ {spelled}.len() }}>({spelled})"
    )
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
    /// The bound its key is held to, a number or the name of one the function
    /// takes, where the key is a text or a byte string.
    pub(crate) key_bound: Option<String>,
    /// ... and its value's.
    pub(crate) value_bound: Option<String>,
    pub(crate) role: MapRole,
}

/// Every map and set a Rust algorithm holds, by the name it is declared
/// under, and what its signature gains for them.
#[derive(Debug, Clone, Default)]
pub(crate) struct RustAlgorithmMaps {
    held: HashMap<String, HeldMap>,
    /// The bounds the function takes from its caller: `const N<i>: usize` for
    /// the map of the parameter at position `i`, `K<i>` and `V<i>` for its
    /// key's and its value's where they are texts or byte strings that state
    /// none, and the storage of each map it is lent, `S<i>: MapStorage`, and of
    /// the destination it writes, [`RUST_ALGORITHM_OUTPUT_STORAGE`].
    bounds: Vec<String>,
    /// The local the body returns, by its declared name.
    output: Option<String>,
    /// What a working local's seed is drawn from: the first map parameter, or
    /// the destination, as the body spells it.
    seed_source: Option<String>,
    /// The storage a working local is kept in: the owned storage of the map
    /// its seed is drawn from ([`owned_storage`] of `S<i>` of the first map
    /// parameter, or of the destination's), and the default where the seed is
    /// the caller's.
    working_storage: Option<String>,
    /// Whether the function takes its caller's seed ([`RUST_ALGORITHM_SEED_PARAM`]).
    seed_param: bool,
    /// Each imported algorithm the body calls that takes its caller's seed, by
    /// its alias, with the seed the body hands it.
    call_seeds: HashMap<String, String>,
    /// Each local of an enum, by name, with its alias: what an `outcome` names.
    enum_locals: HashMap<String, String>,
}

impl RustAlgorithmMaps {
    /// The maps `m` holds. `part` spells a key or a value of a map that is no
    /// text and no byte string as a Rust type, or answers `None` for a part no
    /// Rust algorithm's map holds; `local` spells a name as the body does;
    /// `takes_seed` is whether the algorithm takes its caller's seed
    /// ([`AlgorithmModel::takes_map_seed`]), and `seeded` are the aliases of
    /// the imported algorithms the body may call that take one. Refused by
    /// name: a part `part` does not spell, a map local whose text or byte
    /// string states no bound, and a map return that is not one local
    /// returned by its name.
    pub(crate) fn of(
        m: &AlgorithmModel,
        part: PartSpelling<'_>,
        local: &dyn Fn(&str) -> String,
        takes_seed: bool,
        seeded: &[&str],
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
            let storage = format!("S{index}");
            let spelled =
                MapSpelling::of(&param.sce_type, &bound, Some(&storage), part, Some(index))?
                    .ok_or_else(|| {
                        refused(format!(
                            "holds `{}` of {}, a map whose key or value no algorithm's map holds",
                            param.name,
                            param.sce_type.as_attr()
                        ))
                    })?;
            maps.bounds.extend(spelled.generics.iter().cloned());
            maps.bounds.push(storage_bound(&storage));
            maps.insert(
                &param.name,
                &param.sce_type,
                spelled,
                MapRole::Lent {
                    inout: param.written_in_place(),
                },
            );
            if maps.seed_source.is_none() {
                maps.seed_source = Some(local(&param.name));
                maps.working_storage = Some(owned_storage(&storage));
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
        // A working local follows the owned storage of the map its seed is
        // drawn from: the first map lent, or else the destination.
        if maps.working_storage.is_none() && maps.output.is_some() {
            maps.working_storage = Some(owned_storage(RUST_ALGORITHM_OUTPUT_STORAGE));
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
            let role = if maps.output.as_deref() == Some(name) {
                MapRole::Output
            } else {
                let role = MapRole::Working { ordinal };
                ordinal += 1;
                role
            };
            let storage = match role {
                MapRole::Output => {
                    maps.bounds
                        .push(storage_bound(RUST_ALGORITHM_OUTPUT_STORAGE));
                    Some(RUST_ALGORITHM_OUTPUT_STORAGE.to_string())
                }
                MapRole::Working { .. } => maps.working_storage.clone(),
                MapRole::Lent { .. } => None,
            };
            // A local holds its entries, so a text or a byte string in it states
            // its bound, which no caller gives it.
            let spelled = MapSpelling::of(ty, &bound.to_string(), storage.as_deref(), part, None)?
                .ok_or_else(|| {
                    refused(format!(
                        "holds the local `{name}` of {}, a map whose key or value no algorithm's \
                     map holds, or whose text or byte string states no bound",
                        ty.as_attr()
                    ))
                })?;
            maps.insert(name, ty, spelled, role);
        }
        if maps.seed_source.is_none() {
            maps.seed_source = maps.output.as_deref().map(local);
        }
        // D8: a body with no map to draw a seed from takes its caller's, by the
        // one rule every caller judges it by, so a statechart or an algorithm
        // that calls it hands it one; it hands each algorithm it calls that
        // takes one a seed drawn from its own after its locals'. The rule gives
        // every body that calls such an algorithm a seed of its own.
        maps.seed_param = takes_seed;
        for (offset, alias) in (0u32..).zip(seeded) {
            let seed = maps.seed(ordinal + offset).ok_or_else(|| -> ForgeError {
                GenerateError::InvalidConfig(format!(
                    "algorithm '{}' calls `{alias}`, which takes its caller's seed, and has none \
                     of its own to hand it, which the rule of D8 rules out",
                    m.name
                ))
                .into()
            })?;
            maps.call_seeds.insert((*alias).to_string(), seed);
        }
        Ok(maps)
    }

    fn insert(&mut self, name: &str, ty: &AlgorithmValueType, spelled: MapSpelling, role: MapRole) {
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
                ty: spelled.ty,
                key,
                value,
                key_bound: spelled.key_bound,
                value_bound: spelled.value_bound,
                role,
            },
        );
    }

    /// The seed numbered `ordinal` the body draws: from its caller's where it
    /// takes one, and otherwise from the map it is lent or writes; `None` for a
    /// body that has neither.
    fn seed(&self, ordinal: u32) -> Option<String> {
        if self.seed_param {
            return Some(format!("{RUST_ALGORITHM_SEED_PARAM}.derive({ordinal})"));
        }
        let source = self.seed_source.as_deref()?;
        Some(format!("{source}.derived_seed({ordinal})"))
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

    /// The parameter the caller's seed is handed in, after the declared ones
    /// and before any destination; `None` for an algorithm that takes none.
    pub(crate) fn seed_parameter(&self) -> Option<String> {
        self.seed_param
            .then(|| format!("{RUST_ALGORITHM_SEED_PARAM}: {RUST_ALGORITHM_MAP_CRATE}::MapSeed"))
    }

    /// The seed the body hands each call of an imported algorithm that takes
    /// its caller's, by the algorithm's alias.
    pub(crate) fn call_seeds(&self) -> &HashMap<String, String> {
        &self.call_seeds
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
                let seed = self.seed(ordinal)?;
                let binding = if mutable { "let mut" } else { "let" };
                Some(format!(
                    "{pad}{binding} {local}: {} = {RUST_ALGORITHM_MAP_CRATE}::FromSeed::from_seed(\
                     {seed});\n",
                    held.ty
                ))
            }
            MapRole::Lent { .. } => None,
        }
    }
}

/// The type of a map or a set, and the bounds of its text and byte string
/// parts.
struct MapSpelling {
    ty: String,
    key_bound: Option<String>,
    value_bound: Option<String>,
    /// The bounds the function takes for those parts, a parameter's that states
    /// none.
    generics: Vec<String>,
}

impl MapSpelling {
    /// The map or set `ty` of the bound `bound`, kept in the storage the
    /// function names `storage` or, where it is `None`, in the default one;
    /// each part spelled by `part`, a text or a byte string by the bound it
    /// states or, for the parameter at `param`, the caller's; `Ok(None)` for a
    /// part spelled by neither.
    fn of(
        ty: &AlgorithmValueType,
        bound: &str,
        storage: Option<&str>,
        part: PartSpelling<'_>,
        param: Option<usize>,
    ) -> Result<Option<Self>, ForgeError> {
        let stored = storage.map_or_else(String::new, |storage| format!(", {storage}"));
        let mut generics = Vec::new();
        let mut spell = |held: &AlgorithmValueType,
                         side: &str|
         -> Result<Option<(String, Option<String>)>, ForgeError> {
            let Some(bounded) = BoundedPart::of(held) else {
                return Ok(part(held)?.map(|spelled| (spelled, None)));
            };
            let stated = match held {
                AlgorithmValueType::Text { max } | AlgorithmValueType::Bytes { max } => {
                    max.as_ref().map(|max| max.max.to_string())
                }
                AlgorithmValueType::Scalar(_)
                | AlgorithmValueType::Record { .. }
                | AlgorithmValueType::Union { .. }
                | AlgorithmValueType::List { .. }
                | AlgorithmValueType::Map { .. }
                | AlgorithmValueType::Set { .. }
                | AlgorithmValueType::Optional { .. } => None,
            };
            let bound = match (stated, param) {
                (Some(stated), _) => stated,
                (None, Some(index)) => {
                    let name = format!("{side}{index}");
                    generics.push(format!("const {name}: usize"));
                    name
                }
                (None, None) => return Ok(None),
            };
            Ok(Some((bounded.ty(&bound), Some(bound))))
        };
        Ok(match ty {
            AlgorithmValueType::Map { key, value, .. } => {
                match (spell(key, "K")?, spell(value, "V")?) {
                    (Some((key, key_bound)), Some((value, value_bound))) => Some(Self {
                        ty: format!(
                            "{RUST_ALGORITHM_MAP_CRATE}::BoundedMap<{key}, {value}, {bound}{stored}>"
                        ),
                        key_bound,
                        value_bound,
                        generics,
                    }),
                    _ => None,
                }
            }
            AlgorithmValueType::Set { key, .. } => spell(key, "K")?.map(|(key, key_bound)| Self {
                ty: format!("{RUST_ALGORITHM_MAP_CRATE}::BoundedSet<{key}, {bound}{stored}>"),
                key_bound,
                value_bound: None,
                generics,
            }),
            AlgorithmValueType::Scalar(_)
            | AlgorithmValueType::Text { .. }
            | AlgorithmValueType::Bytes { .. }
            | AlgorithmValueType::Record { .. }
            | AlgorithmValueType::Union { .. }
            | AlgorithmValueType::List { .. }
            | AlgorithmValueType::Optional { .. } => None,
        })
    }
}

/// The bound a function states on the storage it is generic over, named
/// `storage`: one of the runtime's (D4 "Storage").
fn storage_bound(storage: &str) -> String {
    format!("{storage}: {RUST_ALGORITHM_MAP_CRATE}::MapStorage")
}

/// The storage a map the function builds beside one kept in `storage` is kept
/// in, and a copy of it: `storage`'s owned storage (docs/adr/0154, D3), itself
/// where the map holds its slots and, where its host lends them, the one that
/// holds them in the value, since a run is lent to one map.
fn owned_storage(storage: &str) -> String {
    format!("<{storage} as {RUST_ALGORITHM_MAP_CRATE}::MapStorage>::Owned")
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
/// runtime's order ([`crate::forge::static_lowering::MapOutcomes`]).
pub(crate) struct Outcome<'a> {
    pub(crate) place: &'a str,
    pub(crate) variants: &'a [String],
}

/// `<place> = match <call> { … };`: the runtime's answer, one of `outcomes`,
/// written as the variant of the same name of the document's enum.
fn written_outcome(
    pad: &str,
    call: &str,
    outcomes: &crate::forge::static_lowering::MapOutcomes,
    outcome: &Outcome<'_>,
) -> String {
    let runtime = outcomes.rust;
    let arms: Vec<String> = outcomes
        .variants
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
            &crate::forge::static_lowering::PUT_OUTCOMES,
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
            &crate::forge::static_lowering::REMOVE_OUTCOMES,
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
/// `copies`, its body writing the map, kept in the map's owned storage
/// (`to_owned_storage`, docs/adr/0154, D3) so a map whose host lends its slots
/// is walked as any other, and over the map itself otherwise, which nothing in
/// the body changes. Each turn binds `item` (each value of a
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
            let copy = format!("{pad}    let {walked} = {}.to_owned_storage();\n", self.map);
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
            ty: "::sce_portable_bytes::BoundedMap<u32, u64, N0, S0>".to_string(),
            key: AlgorithmValueType::Scalar(SceType::Uint32),
            value,
            key_bound: None,
            value_bound: None,
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
        assert!(
            open.contains("let sce_walk_1 = resets.to_owned_storage();"),
            "{open}"
        );
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

    /// docs/adr/0108, D1, D4 "Storage" and D5: a text or a byte string part of
    /// a parameter's map is held to the bound its type states, or, where it
    /// states none, to one the function takes from its caller; a local's states
    /// its own, and one that does not is no map a local holds. A map is kept in
    /// the storage the function names, or in the default where it names none.
    #[test]
    fn a_text_part_is_held_to_its_stated_bound_or_the_callers() {
        let scalar = |_: &AlgorithmValueType| -> Result<Option<String>, ForgeError> {
            Ok(Some("u32".to_string()))
        };
        let read = |text: &str| AlgorithmValueType::algorithm_from_attr(text).expect("a type");
        let stated = MapSpelling::of(
            &read("map<string[8], uint32>"),
            "N0",
            Some("S0"),
            &scalar,
            Some(0),
        )
        .expect("spelled")
        .expect("held");
        assert_eq!(
            stated.ty,
            "::sce_portable_bytes::BoundedMap<::sce_portable_bytes::SceString<8>, u32, N0, S0>"
        );
        assert_eq!(stated.key_bound.as_deref(), Some("8"));
        assert!(stated.generics.is_empty());
        let callers = MapSpelling::of(
            &read("map<uint32, bytes>"),
            "N2",
            Some("S2"),
            &scalar,
            Some(2),
        )
        .expect("spelled")
        .expect("held");
        assert_eq!(
            callers.ty,
            "::sce_portable_bytes::BoundedMap<u32, ::sce_portable_bytes::SceBytes<V2>, N2, S2>"
        );
        assert_eq!(callers.generics, ["const V2: usize"]);
        let local = MapSpelling::of(&read("set<uint32>[4]"), "4", None, &scalar, None)
            .expect("spelled")
            .expect("held");
        assert_eq!(local.ty, "::sce_portable_bytes::BoundedSet<u32, 4>");
        assert!(
            MapSpelling::of(&read("set<string>[4]"), "4", None, &scalar, None)
                .expect("spelled")
                .is_none()
        );
        assert_eq!(storage_bound("SR"), "SR: ::sce_portable_bytes::MapStorage");
        // docs/adr/0154, D3: a map the function builds is kept in the owned
        // storage of the one it follows, which a lent map has.
        assert_eq!(
            owned_storage("S0"),
            "<S0 as ::sce_portable_bytes::MapStorage>::Owned"
        );
        assert_eq!(
            literal_text("K0", "ab"),
            "::sce_portable_bytes::SceString::<K0>::proven::<{ \"ab\".len() }>(\"ab\")"
        );
    }
}
