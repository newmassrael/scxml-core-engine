// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
//! Generated text (docs/SCE_HOST_FEATURES.md, section 6; docs/adr/0087, section
//! 4): the semantics of `sce-host-model-generate` over a provider the embedding
//! supplies.
//!
//! The host judges the request (an empty prompt, `max_tokens` zero, a
//! temperature past 2000 thousandths, an answer schema the machine's table does
//! not hold), then asks its [`ModelProvider`] for a [`Generation`], and never cuts
//! the answer: one past the bound of `text` fails `answer_too_large`. No provider
//! here reaches a model. [`ScriptedProvider`] gives the answers a scenario wrote;
//! [`ExhaustiveProvider`] gives the outcome class a test names at each call
//! (docs/adr/0087, section 9.4).

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use sce_rust_runtime::AnswerSchema;

use crate::{
    failure, record, Outcome, Params, ReferenceHost, Transport, INVALID_ARGUMENT, MODEL_GENERATE,
};

/// The bound of a result's `text`, in bytes (`model_answer` of
/// `sce:std/host/model_generate_result.scxml`).
pub const ANSWER_MAX_BYTES: usize = 4096;

/// The highest sampling temperature a request may ask for, in thousandths.
pub const TEMPERATURE_MAX_MILLI: u32 = 2000;

/// A generation's request, as a provider reads it.
#[derive(Debug, Clone, Copy)]
pub struct GenerateRequest<'a> {
    pub prompt: &'a str,
    /// The machine's answer schema the request names, `None` for a free text.
    pub answer_schema: Option<&'a AnswerSchema>,
    pub max_tokens: u32,
    pub temperature_milli: u32,
}

/// What a provider makes of one generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Generation {
    /// An answer: the completion's fields but `error`.
    Answer {
        text: String,
        finish: String,
        constrained: bool,
        model: String,
        prompt_tokens: u32,
        answer_tokens: u32,
    },
    /// A failure, by its code of the operation's closed set.
    Failure(String),
    /// The host declines to start it (`invalid_argument`).
    Refusal,
    /// No answer yet, or ever: a deadline or a cancel ends the call.
    Hold,
}

/// Where a host's generations come from: a model on the device, a remote one,
/// or a script.
pub trait ModelProvider {
    fn generate(&mut self, request: &GenerateRequest<'_>) -> Generation;
}

impl<P: ModelProvider> ModelProvider for Arc<Mutex<P>> {
    fn generate(&mut self, request: &GenerateRequest<'_>) -> Generation {
        self.lock()
            .expect("the provider is not left mid-update")
            .generate(request)
    }
}

/// One answer a scripted provider gives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptedAnswer {
    pub text: String,
    /// `stop` or `length`.
    pub finish: String,
}

/// A provider that gives the answers it was told, one a generation, in order.
/// It counts a token as one byte of UTF-8.
#[derive(Debug, Clone)]
pub struct ScriptedProvider {
    pub answers: VecDeque<ScriptedAnswer>,
    /// Whether it decodes under a grammar; it does only when the request names
    /// an answer schema.
    pub constrained: bool,
    /// What it says answered.
    pub name: String,
    /// The most tokens a prompt and `max_tokens` may take together; 0 for none.
    pub context: u64,
    /// Whether it holds every generation unanswered.
    pub hold: bool,
}

impl Default for ScriptedProvider {
    fn default() -> Self {
        Self {
            answers: VecDeque::new(),
            constrained: false,
            name: "scripted".to_string(),
            context: 0,
            hold: false,
        }
    }
}

impl ModelProvider for ScriptedProvider {
    fn generate(&mut self, request: &GenerateRequest<'_>) -> Generation {
        if self.hold {
            sce_rust_runtime::sce_log_info!("scripted provider: holds the generation");
            return Generation::Hold;
        }
        let prompt_tokens = tokens(request.prompt);
        if self.context != 0
            && u64::from(prompt_tokens) + u64::from(request.max_tokens) > self.context
        {
            sce_rust_runtime::sce_log_info!(
                "scripted provider: {prompt_tokens} prompt tokens and {} more pass a context of {}",
                request.max_tokens,
                self.context
            );
            return Generation::Failure("context_exceeded".to_string());
        }
        let Some(answer) = self.answers.pop_front() else {
            sce_rust_runtime::sce_log_info!("scripted provider: no answer left to give");
            return Generation::Failure("unavailable".to_string());
        };
        sce_rust_runtime::sce_log_debug!(
            "scripted provider: answers {} bytes, finish {}",
            answer.text.len(),
            answer.finish
        );
        Generation::Answer {
            answer_tokens: tokens(&answer.text),
            text: answer.text,
            finish: answer.finish,
            constrained: self.constrained && request.answer_schema.is_some(),
            model: self.name.clone(),
            prompt_tokens,
        }
    }
}

/// One outcome class of a generation (docs/adr/0087, section 9.4), as the
/// table every lane reads names it (`tests/host_features/model_outcome_classes.json`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutcomeClass {
    /// A completion with this text and `finish`.
    Answer { text: String, finish: String },
    /// A failure with this code.
    Failure(String),
    /// The host declines to start it.
    Refusal,
    /// No answer: the test ends the call by a deadline or a cancel.
    Hold,
}

/// A provider that stands for a model that may give any outcome at any call:
/// each generation takes the next class of the sequence the test named.
#[derive(Debug, Clone, Default)]
pub struct ExhaustiveProvider {
    sequence: VecDeque<OutcomeClass>,
}

impl ExhaustiveProvider {
    /// A provider that answers `sequence`, one class a generation, in order.
    pub fn new(sequence: Vec<OutcomeClass>) -> Self {
        Self {
            sequence: sequence.into(),
        }
    }
}

impl ModelProvider for ExhaustiveProvider {
    fn generate(&mut self, request: &GenerateRequest<'_>) -> Generation {
        let Some(class) = self.sequence.pop_front() else {
            sce_rust_runtime::sce_log_warn!(
                "exhaustive provider: asked past the sequence the test named"
            );
            return Generation::Failure("unavailable".to_string());
        };
        sce_rust_runtime::sce_log_debug!("exhaustive provider: answers {class:?}");
        match class {
            OutcomeClass::Answer { text, finish } => Generation::Answer {
                answer_tokens: tokens(&text),
                text,
                finish,
                constrained: false,
                model: "exhaustive".to_string(),
                prompt_tokens: tokens(request.prompt),
            },
            OutcomeClass::Failure(code) => Generation::Failure(code),
            OutcomeClass::Refusal => Generation::Refusal,
            OutcomeClass::Hold => Generation::Hold,
        }
    }
}

/// A text's tokens as the reference providers count them: its bytes.
fn tokens(text: &str) -> u32 {
    u32::try_from(text.len()).unwrap_or(u32::MAX)
}

/// A request the host accepts: the answer schema it names, `max_tokens` and the
/// temperature.
type Judged<'a> = (Option<&'a AnswerSchema>, u32, u32);

/// The machine's answer schema `answer_schema` names with the request's
/// numbers, or why the request is refused: an empty prompt, `max_tokens` zero,
/// a temperature past [`TEMPERATURE_MAX_MILLI`], or an id the table does not
/// hold.
fn judge<'a>(schemas: &'a [AnswerSchema], params: &Params) -> Result<Judged<'a>, &'static str> {
    let argument = |name: &str| params.get(name).map(String::as_str).unwrap_or("");
    let (prompt, answer_schema) = (argument("prompt"), argument("answer_schema"));
    let (max_tokens, temperature_milli) = (argument("max_tokens"), argument("temperature_milli"));
    if prompt.is_empty() {
        return Err("the prompt is empty");
    }
    let max_tokens = match max_tokens.parse::<u32>() {
        Ok(0) | Err(_) => return Err("max_tokens is not a count above zero"),
        Ok(n) => n,
    };
    let temperature_milli = match temperature_milli.parse::<u32>() {
        Ok(n) if n <= TEMPERATURE_MAX_MILLI => n,
        _ => return Err("the temperature is past 2000 thousandths"),
    };
    if answer_schema.is_empty() {
        return Ok((None, max_tokens, temperature_milli));
    }
    schemas
        .iter()
        .find(|schema| schema.id == answer_schema)
        .map(|schema| (Some(schema), max_tokens, temperature_milli))
        .ok_or("the answer schema is not one of the machine's")
}

/// Whether the host declines to start a generation with `params`, which it
/// says why it does.
pub(crate) fn refuses(schemas: &[AnswerSchema], id: &str, params: &Params) -> bool {
    match judge(schemas, params) {
        Ok(_) => false,
        Err(why) => {
            sce_rust_runtime::sce_log_info!("{MODEL_GENERATE} {id}: refused, {why}");
            true
        }
    }
}

/// Generate a text for invocation `id`, whose request the host accepted
/// ([`refuses`]).
pub(crate) fn generate<T: Transport>(
    host: &mut ReferenceHost<T>,
    id: &str,
    params: &Params,
) -> Outcome {
    let prompt = params.get("prompt").cloned().unwrap_or_default();
    let Ok((schema, max_tokens, temperature_milli)) = judge(&host.answer_schemas, params) else {
        unreachable!("the request was judged before the generation")
    };
    let request = GenerateRequest {
        prompt: &prompt,
        answer_schema: schema,
        max_tokens,
        temperature_milli,
    };
    let generation = host.provider.generate(&request);
    match generation {
        Generation::Answer {
            text,
            finish,
            constrained,
            model,
            prompt_tokens,
            answer_tokens,
        } => {
            if text.len() > ANSWER_MAX_BYTES {
                sce_rust_runtime::sce_log_info!(
                    "sce-host-model-generate {id}: an answer of {} bytes is past {ANSWER_MAX_BYTES}",
                    text.len()
                );
                return failure(MODEL_GENERATE, "answer_too_large");
            }
            sce_rust_runtime::sce_log_debug!(
                "sce-host-model-generate {id}: answered by {model}, finish {finish}"
            );
            record(serde_json::json!({
                "text": text,
                "constrained": constrained,
                "model": model,
                "prompt_tokens": prompt_tokens,
                "answer_tokens": answer_tokens,
                "finish": finish,
            }))
        }
        Generation::Failure(code) => {
            sce_rust_runtime::sce_log_info!("sce-host-model-generate {id}: fails {code}");
            failure(MODEL_GENERATE, &code)
        }
        Generation::Refusal => {
            sce_rust_runtime::sce_log_info!(
                "sce-host-model-generate {id}: refused by the provider"
            );
            Outcome::Refuse(INVALID_ARGUMENT)
        }
        Generation::Hold => {
            sce_rust_runtime::sce_log_debug!("sce-host-model-generate {id}: held");
            host.held.push(id.to_string());
            Outcome::Hold
        }
    }
}
