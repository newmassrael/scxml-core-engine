// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Why a machine's script session could not be established, which refuses its
//! start (docs/adr/0096).
//!
//! A machine whose document needs a datamodel runs its conditions, assignments
//! and scripts in a script session. Establishing it is three steps in this
//! order: create the session, load the shared library into it (the ECMAScript
//! semantics, the JSON builtins and the DOM read surface), and bind the system
//! variables `_sessionid`, `_name` and `_ioprocessors`. A machine started over
//! a session one of them failed for would run with no datamodel, every
//! condition an error and every assignment lost, and the document could not
//! even handle it: its means of handling an error runs in the session that
//! failed. So the start is refused, and the host is handed this value.
//!
//! The policy names the failure its start can have
//! ([`StatePolicy::StartFailure`](crate::StatePolicy::StartFailure)): this type
//! for a machine with a script engine, and [`core::convert::Infallible`] for
//! one without, whose start cannot be refused this way. A `no_std` build has
//! no script engine, so it never makes one; the type is in both profiles
//! because the refusals that carry it ([`RootStartRefusal`](crate::RootStartRefusal),
//! [`ConfigurationRejection`](crate::ConfigurationRejection)) are.

use core::fmt;

/// The step of establishing a script session that failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScriptSessionStage {
    /// The script engine could not create the session.
    Create,
    /// The shared library did not load into the new session, which the engine
    /// then did not keep. A library chunk that does not load is a defect of
    /// the build that shipped it.
    Library,
    /// The system variables `_sessionid`, `_name` and `_ioprocessors` could not
    /// be bound; the session was destroyed.
    SystemVariables,
}

impl ScriptSessionStage {
    /// The step's name, the same word on every backend: `create`, `library`
    /// or `system-variables`.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Library => "library",
            Self::SystemVariables => "system-variables",
        }
    }
}

impl fmt::Display for ScriptSessionStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A machine's script session could not be established, so its start was
/// refused: no state was entered, no executable content ran, no event was
/// taken, and what the failed steps made was released.
///
/// The engine's words are kept only where there is an engine to say them: a
/// `no_std` build has no script engine and never makes this value, and there
/// it is the step alone. Holding a bounded string for the words would make
/// every refusal that carries this value
/// ([`RootStartRefusal`](crate::RootStartRefusal),
/// [`ConfigurationRejection`](crate::ConfigurationRejection)) that string's
/// size on a device's stack, for a value it cannot have. `#[non_exhaustive]`
/// keeps the field `std` adds an addition (docs/adr/0053): no crate builds
/// this value from its fields, so none is broken when the feature is unified
/// on.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ScriptSessionFailure {
    /// The step that failed.
    pub stage: ScriptSessionStage,
    /// What the script engine said.
    #[cfg(feature = "std")]
    pub message: std::string::String,
}

#[cfg(feature = "std")]
impl ScriptSessionFailure {
    /// A failure of `stage`, in the engine's words `message`.
    pub fn new(stage: ScriptSessionStage, message: impl Into<String>) -> Self {
        Self {
            stage,
            message: message.into(),
        }
    }

    /// What [`IScriptEngine::create_session`](crate::IScriptEngine::create_session)
    /// failed with: a shared library that did not load
    /// ([`ScriptError::LibraryNotLoaded`](crate::ScriptError::LibraryNotLoaded))
    /// is the `library` step, any other failure the `create` step.
    pub fn of_creation(error: &crate::ScriptError) -> Self {
        let stage = match error {
            crate::ScriptError::LibraryNotLoaded(_) => ScriptSessionStage::Library,
            _ => ScriptSessionStage::Create,
        };
        Self::new(stage, error.to_string())
    }

    /// What [`IScriptEngine::setup_system_variables`](crate::IScriptEngine::setup_system_variables)
    /// failed with: the `system-variables` step.
    pub fn of_system_variables(error: &crate::ScriptError) -> Self {
        Self::new(ScriptSessionStage::SystemVariables, error.to_string())
    }
}

impl fmt::Display for ScriptSessionFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the script session could not be established at step '{}'",
            self.stage
        )?;
        #[cfg(feature = "std")]
        write!(f, ": {}", self.message)?;
        Ok(())
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ScriptSessionFailure {}

/// The start of a machine without a script engine cannot be refused for its
/// session, so its failure type has no value; this lets a refusal that carries
/// a [`ScriptSessionFailure`] be written once for both kinds of policy.
impl From<core::convert::Infallible> for ScriptSessionFailure {
    fn from(never: core::convert::Infallible) -> Self {
        match never {}
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn each_step_has_the_name_every_backend_spells() {
        assert_eq!(ScriptSessionStage::Create.to_string(), "create");
        assert_eq!(ScriptSessionStage::Library.to_string(), "library");
        assert_eq!(
            ScriptSessionStage::SystemVariables.to_string(),
            "system-variables"
        );
    }

    #[test]
    fn a_library_that_did_not_load_is_the_library_step() {
        let library =
            ScriptSessionFailure::of_creation(&crate::ScriptError::LibraryNotLoaded("x".into()));
        assert_eq!(library.stage, ScriptSessionStage::Library);
        let create =
            ScriptSessionFailure::of_creation(&crate::ScriptError::EngineError("y".into()));
        assert_eq!(create.stage, ScriptSessionStage::Create);
        let bind = ScriptSessionFailure::of_system_variables(&crate::ScriptError::RuntimeError(
            "z".into(),
        ));
        assert_eq!(bind.stage, ScriptSessionStage::SystemVariables);
    }

    #[test]
    fn the_failure_names_its_step_and_the_engines_words() {
        let failure = ScriptSessionFailure::new(ScriptSessionStage::Create, "out of memory");
        assert_eq!(
            failure.to_string(),
            "the script session could not be established at step 'create': out of memory"
        );
    }
}
