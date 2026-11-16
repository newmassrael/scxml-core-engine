// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The events a link carries (the SCE Protocol-Synthesis RFC's link section,
//! "Codegen contract"; docs/adr/0106, item #142): what the Rust and C11 link templates
//! generate the receive path `driver.poll() -> bytes -> framer.decode() ->
//! event inject` and the transmit path `event extract -> encode -> pool slot ->
//! driver.send()` from.
//!
//! A link that declares no `<sce:events>` row is a byte endpoint and needs none
//! of this. A link that declares one is generated against the codecs its rows
//! name, read through the link's own `<sce:import kind="codec">`: the decode
//! and encode are the imported codecs' own functions, and the type, the
//! borrowing and the worst-case size of each are facts the import's enrichment
//! read from the codec's document. A codec named but not imported is refused
//! here, naming the import to add, rather than generated against a guess.
//!
//! Each `<sce:inbound>` row is a rule of its own: every row whose `when` holds
//! for a decoded frame injects its event, in document order, so a frame may
//! raise more than one event and a row without `when` raises its event for
//! every frame the framer decodes.

use crate::filters;
use crate::forge::error::{ForgeError, GenerateError};
use crate::forge::expr::{self, ExprTarget};
use crate::forge::expression_site::ExpressionSite;
use crate::forge::model::{LinkModel, SceType};
use crate::forge::types::InferredType;
use crate::generator::Language;

use super::ImportContext;

/// The name an `<sce:inbound when>` predicate reads the decoded frame by
/// (`when="decoded.msg_id == 0x02"`, the RFC's own example).
pub(crate) const DECODED: &str = "decoded";

/// One event name of a link, with the identifiers each template spells it by.
#[derive(Debug, Clone, serde::Serialize)]
pub(super) struct LinkEventName {
    /// The SCXML event name as the document wrote it.
    event: String,
    /// The Rust enum variant (`ScoutHelloReceived`).
    variant: String,
    /// The C11 enumerator's suffix (`SCOUT_HELLO_RECEIVED`), placed after the
    /// link's own prefix by the template.
    constant: String,
}

/// One `<sce:inbound>` row: the event it injects and its predicate, lowered.
#[derive(Debug, Clone, serde::Serialize)]
pub(super) struct LinkInboundRow {
    #[serde(flatten)]
    name: LinkEventName,
    /// The `when` predicate in the target language over the local `decoded`;
    /// `None` for a row that injects every decoded frame.
    when: Option<String>,
}

/// One `<sce:outbound>` row: the event a caller hands the link and the codec
/// its value is encoded by.
#[derive(Debug, Clone, serde::Serialize)]
pub(super) struct LinkOutboundRow {
    #[serde(flatten)]
    name: LinkEventName,
    /// The event's name in snake case (`scout_query_send`): the C11 send
    /// function's suffix.
    snake: String,
    /// The codec's value type: Rust `Pascal`, C11 `snake_t`.
    codec_type: String,
    /// The codec's function prefix in C11 (`snake`, for `snake_encode_to_buf`).
    codec_prefix: String,
    /// The Rust value type carries a lifetime (`Pascal<'a>`).
    codec_borrowed: bool,
    /// The most bytes the codec encodes to, at least one: the C11 send
    /// function's buffer, which C does not let be empty.
    max_bytes: u64,
}

/// The buffer pool whose slots a link's transmit path encodes into
/// (`<sce:tx-pool ref>`, a buffer-pool document).
#[derive(Debug, Clone, serde::Serialize)]
pub(super) struct LinkTxPool {
    /// The Rust pool type, by its module path (`super::scout_tx_pool::ScoutTxPool`).
    rust_type: String,
    /// The C11 function prefix (`scout_tx_pool`) and macro prefix (`SCOUT_TX_POOL`).
    prefix: String,
    upper: String,
    /// The `#include` of the C11 pool header.
    c_include: String,
}

/// What a link's typed receive and transmit paths are generated from.
#[derive(Debug, Clone, serde::Serialize)]
pub(super) struct LinkEvents {
    /// The framer's import line in the target language.
    framer_include: String,
    /// The framer's value type: Rust `Pascal`, C11 `snake_t`.
    framer_type: String,
    /// The framer's function prefix in C11 (`snake`, for `snake_decode`).
    framer_prefix: String,
    /// The Rust value type carries a lifetime: it borrows the received bytes,
    /// so a decoded frame is handed to the inject callback and not kept.
    framer_borrowed: bool,
    /// The distinct inbound event names, in the order the rows first name them.
    inbound_events: Vec<LinkEventName>,
    /// Every `<sce:inbound>` row, in document order.
    inbound_rows: Vec<LinkInboundRow>,
    /// Every `<sce:outbound>` row, in document order.
    outbound: Vec<LinkOutboundRow>,
    /// Some outbound value type carries a lifetime, so the Rust outbound enum
    /// does.
    outbound_borrowed: bool,
    /// The import lines of the outbound codecs and the enums a predicate may
    /// name, each once and none the framer's.
    includes: Vec<String>,
    /// The most bytes any outbound value encodes to: the size of the transmit
    /// buffer a link with no transmit pool encodes into.
    tx_bytes: u64,
    /// The pool the transmit path encodes into, when the link names one.
    tx_pool: Option<LinkTxPool>,
}

/// Resolve the events of `m` for `lang`, or `None` for a link that declares no
/// `<sce:events>` row.
pub(super) fn resolve(
    m: &LinkModel,
    imports: &[ImportContext],
    lang: Language,
    options: &crate::ForgeCompileOptions,
) -> Result<Option<LinkEvents>, ForgeError> {
    if m.inbound.is_empty() && m.outbound.is_empty() {
        return Ok(None);
    }
    let framer = imported_codec(m, imports, &m.framer, "<sce:framer ref>")?;
    if let Some(why) = takes_caller_inputs(framer, Direction::Decode) {
        return Err(refused(
            m,
            format!(
                "its framer '{framer}' {why}, and a frame the link receives says nothing it \
                 does not carry: a framer's decode takes the bytes alone",
                framer = m.framer
            ),
        ));
    }

    let mut includes: Vec<String> = Vec::new();
    let mut names = EventNames::new(m);

    let decoded_fields: Vec<(String, SceType)> = framer
        .member_field_types
        .iter()
        .filter_map(|(key, ty)| {
            key.split_once('.')
                .map(|(_, field)| (format!("{DECODED}.{field}"), ty.clone()))
        })
        .collect();
    let type_ctx = crate::forge::type_ctx::link_inbound(&decoded_fields, imports);
    let renamed: Vec<(String, String)> = decoded_fields
        .iter()
        .map(|(path, _)| {
            let field = path.split_once('.').map_or(path.as_str(), |(_, f)| f);
            (
                path.clone(),
                format!("{DECODED}.{}", filters::to_snake_case(field.to_string())),
            )
        })
        .collect();
    let renames: std::collections::HashMap<&str, &str> = renamed
        .iter()
        .map(|(from, to)| (from.as_str(), to.as_str()))
        .collect();
    let target = match lang {
        Language::C11 => ExprTarget::C,
        _ => ExprTarget::Rust,
    };

    let mut inbound_events: Vec<LinkEventName> = Vec::new();
    let mut inbound_rows = Vec::with_capacity(m.inbound.len());
    for row in &m.inbound {
        let name = names.name(&row.event, Side::Inbound)?;
        if !inbound_events.iter().any(|e| e.event == row.event) {
            inbound_events.push(name.clone());
        }
        let when = match &row.when {
            Some(source) => {
                let site = ExpressionSite::new(source, row.when_spelling.as_ref());
                Some(
                    expr::transpile_into(
                        site.source,
                        target,
                        &type_ctx,
                        &renames,
                        InferredType::Bool,
                    )
                    .map_err(|refusal| site.place(refusal))?,
                )
            }
            None => None,
        };
        inbound_rows.push(LinkInboundRow { name, when });
    }

    let mut outbound = Vec::with_capacity(m.outbound.len());
    let mut tx_bytes: u64 = 0;
    for row in &m.outbound {
        if outbound
            .iter()
            .any(|given: &LinkOutboundRow| given.name.event == row.event)
        {
            return Err(refused(
                m,
                format!(
                    "it declares the outbound event '{event}' twice, and a value handed to the \
                     link for it is encoded by one codec",
                    event = row.event
                ),
            ));
        }
        let name = names.name(&row.event, Side::Outbound)?;
        let codec = imported_codec(m, imports, &row.encode, "<sce:outbound encode>")?;
        if let Some(why) = takes_caller_inputs(codec, Direction::Encode) {
            return Err(refused(
                m,
                format!(
                    "the codec '{codec}' its outbound event '{event}' is encoded by {why}, and \
                     the event's value is all the link is handed to encode",
                    codec = row.encode,
                    event = row.event
                ),
            ));
        }
        // `imported_codec` admits only an enriched import, which has its size.
        let max_bytes = codec.codec_max_bytes.unwrap_or(0);
        tx_bytes = tx_bytes.max(max_bytes);
        if codec.alias != framer.alias && !includes.contains(&codec.include_stmt) {
            includes.push(codec.include_stmt.clone());
        }
        outbound.push(LinkOutboundRow {
            snake: filters::to_snake_case(row.event.clone()),
            name,
            codec_type: codec_type(codec, lang),
            codec_prefix: codec.namespace.clone(),
            codec_borrowed: codec.codec_is_borrowed,
            max_bytes: max_bytes.max(1),
        });
    }

    // A predicate may compare a decoded field with a value of an enum the link
    // imports; its spelling names the enum's unit.
    for imp in imports.iter().filter(|imp| imp.kind == "enum") {
        if !includes.contains(&imp.include_stmt) {
            includes.push(imp.include_stmt.clone());
        }
    }

    let tx_pool = if outbound.is_empty() {
        None
    } else {
        m.tx_pool
            .as_deref()
            .map(|pool| tx_pool_of(pool, imports, lang, options))
    };

    let outbound_borrowed = outbound.iter().any(|row| row.codec_borrowed);
    Ok(Some(LinkEvents {
        framer_include: framer.include_stmt.clone(),
        framer_type: codec_type(framer, lang),
        framer_prefix: framer.namespace.clone(),
        framer_borrowed: framer.codec_is_borrowed,
        inbound_events,
        inbound_rows,
        outbound,
        outbound_borrowed,
        includes,
        tx_bytes,
        tx_pool,
    }))
}

/// The codec import `alias` names, enriched from its document, or the refusal
/// that says to import it.
fn imported_codec<'i>(
    m: &LinkModel,
    imports: &'i [ImportContext],
    alias: &str,
    named_by: &str,
) -> Result<&'i ImportContext, ForgeError> {
    match imports
        .iter()
        .find(|imp| imp.alias == alias && imp.kind == "codec")
    {
        Some(imp) if imp.codec_max_bytes.is_some() => Ok(imp),
        _ => Err(refused(
            m,
            format!(
                "it declares events, and the codec '{alias}' its {named_by} names is not an \
                 import this generation read: the decode and encode of a link's events are \
                 generated from the codec's document, which the link reads through \
                 <sce:import as=\"{alias}\" kind=\"codec\" src=\"...\"/>"
            ),
        )),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Decode,
    Encode,
}

/// What a codec's decode or encode asks its caller for besides the bytes or
/// the value, said as the clause a refusal reads; `None` when it asks nothing.
///
/// A flag input the value says (ADR 0065 §6 R4a) is taken by the decode but
/// not by the encode; a variant tag the caller supplies is taken by the decode.
fn takes_caller_inputs(codec: &ImportContext, direction: Direction) -> Option<String> {
    let flags: Vec<&str> = codec
        .codec_flag_inputs
        .iter()
        .map(|input| input.name.as_str())
        .filter(|name| {
            direction == Direction::Decode
                || !codec.codec_says_inputs.iter().any(|s| s.as_str() == *name)
        })
        .collect();
    if !flags.is_empty() {
        return Some(format!(
            "takes the flag input(s) {} from its caller",
            flags.join(", ")
        ));
    }
    if direction == Direction::Decode && codec.codec_variant_is_caller_tag {
        return Some("takes its variant's tag from its caller".to_string());
    }
    None
}

fn codec_type(codec: &ImportContext, lang: Language) -> String {
    match lang {
        Language::C11 => codec.member_type.clone(),
        _ => codec.type_name.clone(),
    }
}

/// The pool `pool` names: by its import when the link imports it (the import's
/// document name is the pool's), by its own name when the build lists it.
fn tx_pool_of(
    pool: &str,
    imports: &[ImportContext],
    lang: Language,
    options: &crate::ForgeCompileOptions,
) -> LinkTxPool {
    let name = imports
        .iter()
        .find(|imp| imp.alias == pool && imp.kind == "buffer-pool")
        .map_or(pool, |imp| imp.document_name.as_str());
    let identity = super::forge_import_identity(name, &lang, true, options);
    LinkTxPool {
        rust_type: format!("super::{}::{}", identity.namespace, identity.type_name),
        prefix: filters::to_snake_case(name.to_string()),
        upper: super::to_upper_snake(name),
        c_include: format!(
            "#include \"{}.h\"",
            filters::to_snake_case(name.to_string())
        ),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Inbound,
    Outbound,
}

/// The identifiers of a link's event names, each name given one, and two
/// names that would be given the same refused.
struct EventNames<'m> {
    link: &'m LinkModel,
    given: Vec<GivenName>,
}

/// A name [`EventNames`] gave identifiers to.
struct GivenName {
    side: Side,
    event: String,
    variant: String,
    constant: String,
}

impl<'m> EventNames<'m> {
    fn new(link: &'m LinkModel) -> Self {
        Self {
            link,
            given: Vec::new(),
        }
    }

    fn name(&mut self, event: &str, side: Side) -> Result<LinkEventName, ForgeError> {
        let variant = filters::to_pascal_case(event.to_string());
        let constant = filters::to_upper_snake_case(event.to_string());
        let spellable = variant
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic())
            && variant
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
            && constant
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_');
        if !spellable {
            return Err(refused(
                self.link,
                format!(
                    "its event '{event}' has no spelling as an identifier: an event a link \
                     carries is written of letters, digits, '.', '-' and '_', beginning with a \
                     letter"
                ),
            ));
        }
        // The Rust variant and the C11 enumerator are each one spelling of the
        // name, and two names may meet in either (`AB` and `ab` differ as
        // variants and are both the enumerator `AB`).
        if let Some(given) = self.given.iter().find(|given| {
            given.side == side
                && given.event != event
                && (given.variant == variant || given.constant == constant)
        }) {
            return Err(refused(
                self.link,
                format!(
                    "its events '{other}' and '{event}' are spelled alike in generated code \
                     ('{variant}', '{constant}'); rename one",
                    other = given.event
                ),
            ));
        }
        if !self
            .given
            .iter()
            .any(|given| given.side == side && given.event == event)
        {
            self.given.push(GivenName {
                side,
                event: event.to_string(),
                variant: variant.clone(),
                constant: constant.clone(),
            });
        }
        Ok(LinkEventName {
            event: event.to_string(),
            variant,
            constant,
        })
    }
}

fn refused(m: &LinkModel, why: String) -> ForgeError {
    ForgeError::from(GenerateError::InvalidConfig(format!(
        "link '{name}': {why}",
        name = m.name
    )))
}
