// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Each `test-port-type` a suite reaches as a TTCN-3 message port type
//! (docs/SCE_TEST_PROFILE.md §2.2, §9.2 E3, §11.3): its messages in document
//! order, each with its direction stated, so the port type's `in` and `out`
//! lists are stated explicitly, as Eclipse Titan requires
//! (ADR 0091 answer 1).
//!
//! What is written is a model of the port type ([`PortTypeView`]); the
//! template `ttcn3/port_type.ttcn.jinja2` spells it.

use serde::Serialize;

use super::behaviour::Source;
use super::names::{Declared, Namespace};
use super::reach::Reach;
use super::types::{type_expr, Site, TypeExpr};
use crate::forge::error::{ForgeError, Located};
use crate::forge::model::ForgeDocument;
use crate::forge::value_type::TypeOf;

/// A port type as the export writes it.
#[derive(Debug, Clone, Serialize)]
pub struct PortTypeView {
    pub name: String,
    pub messages: Vec<MessageView>,
    /// The port type's document, which E3 translates whole.
    pub source: Source,
}

/// One message of a port type: its direction (`in`, `out` or `inout`) and
/// the record it is a value of.
#[derive(Debug, Clone, Serialize)]
pub struct MessageView {
    pub direction: &'static str,
    #[serde(rename = "type")]
    pub ty: TypeExpr,
}

/// Each port type the suite reaches, in the order of their names, every name
/// it writes entered in `names`.
pub(super) fn port_types(
    reach: &Reach,
    names: &mut Namespace,
) -> Result<Vec<PortTypeView>, Located<ForgeError>> {
    let mut views = Vec::new();
    for document in reach.sorted() {
        let ForgeDocument::TestPortType(port_type) = &document.parsed.document else {
            continue;
        };
        let origin = document.origin();
        names.definition(
            &document.name,
            Declared::read("the port type", origin.clone()),
        )?;
        let mut messages = Vec::new();
        for message in &port_type.messages {
            let site = Site {
                origin: origin.at(message.source_line),
                subject: format!(
                    "test-port-type '{}': the message '{}'",
                    port_type.name, message.schema
                ),
            };
            messages.push(MessageView {
                direction: message.direction.as_attr(),
                ty: type_expr(
                    &TypeOf::Record {
                        alias: message.schema.clone(),
                    },
                    &document.scope,
                    &site,
                )?,
            });
        }
        views.push(PortTypeView {
            name: document.name.clone(),
            messages,
            source: document.source("E3"),
        });
    }
    Ok(views)
}
