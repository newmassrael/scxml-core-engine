// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The module parameters of a test run (docs/SCE_TEST_PROFILE.md §11.5, §11.8;
//! ETSI ES 201 873-1 §8.2): the value a suite gives each, which every component
//! that declares a module parameter of that id reads into its variable before
//! its machine starts.
//!
//! A value is held as the one JSON value model a saved state is made of
//! ([`crate::json::Value`]). The generated suite writes each default through
//! [`crate::saved_state::SavedValue::to_saved`] of its declared type, and the
//! generated component reads it back through
//! [`crate::saved_state::SavedValue::from_saved`] of the type it declares, which
//! are the reads a restored variable is held by: a value of another type, or
//! past its bound, is refused by name rather than converted. The runtime knows
//! the type of no module parameter, so it is written once for every suite.

use std::collections::BTreeMap;

use crate::json::Value;
use crate::saved_state::StateRefusal;

/// A module parameter's value: the saved form of a value of its declared type.
pub type ModuleParameterValue = Value;

/// The module parameters a run hands its components, by id.
///
/// Generated from a `test-suite` with each parameter at its default, and given
/// other values by the caller of a run where it overrides one (-1 §8.2.1).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModuleParameters {
    values: BTreeMap<String, ModuleParameterValue>,
}

impl ModuleParameters {
    /// Gives the module parameter `id` the value `value`, in place of any it had.
    pub fn set(&mut self, id: impl Into<String>, value: ModuleParameterValue) {
        self.values.insert(id.into(), value);
    }

    /// Whether the module parameter `id` has a value.
    pub fn has(&self, id: &str) -> bool {
        self.values.contains_key(id)
    }

    /// The value of the module parameter `id`; refused, naming it, when it has
    /// none, since no value is made up in its place.
    pub fn of(&self, id: &str) -> Result<&ModuleParameterValue, StateRefusal> {
        self.values.get(id).ok_or_else(|| {
            StateRefusal::new(format!("no value is given for the module parameter '{id}'"))
        })
    }

    /// Every module parameter that has a value, by id.
    pub fn values(&self) -> &BTreeMap<String, ModuleParameterValue> {
        &self.values
    }
}
