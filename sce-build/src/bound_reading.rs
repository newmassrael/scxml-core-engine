// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What the proofs of a machine's bounds read of its document alike: which
//! states can be active together, the places a `<foreach>` walks and how many
//! passes it makes, and the basis of a bound in lines of a comment.
//!
//! The internal queue's depth (docs/adr/0076) and the delayed-send queue's
//! capacity (docs/adr/0132) are each proven from the document by a walk of its
//! own. The walks count different things, and read these the same way; one
//! reading here keeps the two proofs from disagreeing about what a loop is or
//! when two states can both be active.

use crate::forge::event_schema_check::EVENT_DATA_PATH;
use crate::forge::value_path::{self, Place};
use crate::forge::value_type::TypeScope;
use crate::model::{Action, SCXMLModel, Transition};

/// The states of a document, as which can be in one configuration together.
pub(crate) struct Tree<'m> {
    model: &'m SCXMLModel,
}

impl<'m> Tree<'m> {
    /// The states of `model`.
    pub(crate) fn of(model: &'m SCXMLModel) -> Self {
        Self { model }
    }

    /// `id` and every state above it, nearest first. A parent chain that loops
    /// is cut where it repeats, as the parser refuses one elsewhere.
    fn up_from<'s>(&'s self, id: &'s str) -> Vec<&'s str> {
        let mut chain = vec![id];
        let mut current = id;
        while let Some(parent) = self
            .model
            .states
            .get(current)
            .and_then(|state| state.parent.as_deref())
        {
            if chain.contains(&parent) {
                break;
            }
            chain.push(parent);
            current = parent;
        }
        chain
    }

    /// Whether `a` and `b` can be in one configuration: one is the other or
    /// above it, or the nearest state above both is a `<parallel>`, whose
    /// regions are all active at once.
    pub(crate) fn coexist(&self, a: &str, b: &str) -> bool {
        let above_a = self.up_from(a);
        if above_a.contains(&b) {
            return true;
        }
        let above_b = self.up_from(b);
        if above_b.contains(&a) {
            return true;
        }
        above_a
            .iter()
            .find(|state| above_b.contains(*state))
            .and_then(|common| self.model.states.get(*common))
            .is_some_and(|common| common.is_parallel)
    }
}

/// Each name that holds a value where a walk of executable content stands,
/// innermost last: the document's variables, `_event.data` in a transition
/// taken on an event whose schema it imports, and the item of each
/// `<foreach>` around it, typed in the scope
/// [`crate::forge::value_path::statechart_scope`] gives.
pub(crate) struct BlockPlaces {
    scope: TypeScope,
    places: Vec<(String, Place)>,
}

impl BlockPlaces {
    /// The places of `model` outside any block: its variables.
    pub(crate) fn of(model: &SCXMLModel) -> Self {
        let places = model
            .variables
            .iter()
            .chain(model.states.values().flat_map(|s| s.datamodel.iter()))
            .filter_map(|var| Some((var.id.clone(), value_path::variable_place(var)?)))
            .collect();
        Self {
            scope: value_path::statechart_scope(model),
            places,
        }
    }

    /// The place `path` (written as [`value_path`] writes one) names where the
    /// walk stands ([`value_path::resolve`]).
    fn place(&self, path: &str) -> Option<Place> {
        value_path::resolve(
            Some(&self.scope),
            |name| {
                self.places
                    .iter()
                    .rev()
                    .find(|(held, _)| held == name)
                    .map(|(_, place)| place)
            },
            path,
        )
    }

    /// Hold `_event.data` as the payload of the event `transition` is taken
    /// on, when the document imports its schema; whether it was held, so the
    /// walk lets it go ([`Self::release`]) when it leaves the content.
    pub(crate) fn enter_transition(&mut self, model: &SCXMLModel, transition: &Transition) -> bool {
        let payload = model
            .imported_event_schemas
            .get(&transition.event)
            .and_then(|schema| value_path::payload_place(model, &schema.name));
        let held = payload.is_some();
        if let Some(payload) = payload {
            self.places.push((EVENT_DATA_PATH.to_string(), payload));
        }
        held
    }

    /// The list a `<foreach>` walks, as the place its `array` names, when it
    /// names one.
    pub(crate) fn walked_list(&self, action: &Action) -> Option<Place> {
        crate::forge::expr::written_place(&action.array).and_then(|path| self.place(&path))
    }

    /// Hold `place` under `name`, a `<foreach>`'s item in its body.
    pub(crate) fn hold(&mut self, name: &str, place: Place) {
        self.places.push((name.trim().to_string(), place));
    }

    /// Let go of the innermost place held.
    pub(crate) fn release(&mut self) {
        self.places.pop();
    }
}

/// How many passes a `<foreach>` over `list` makes at most: the bound of the
/// list's type, wherever the list is held (a variable typed `list<T>[N]`, a
/// list of the payload, a list an enclosing loop's item holds), the number the
/// build holds after its limit name (docs/adr/0086 section 6.3). `None` for a
/// list whose bound the model does not state.
pub(crate) fn passes(list: Option<&Place>) -> Option<usize> {
    list.and_then(Place::list_bound)
        .and_then(|n| usize::try_from(n).ok())
}

/// `text` in lines of at most `width` characters, broken between words.
pub(crate) fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && line.len() + 1 + word.len() > width {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}
