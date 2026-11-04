# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""Generated text (docs/SCE_HOST_FEATURES.md, section 6; docs/adr/0087, section
4): the semantics of ``sce-host-model-generate`` over a provider the embedding
supplies.

The host judges the request (an empty prompt, ``max_tokens`` zero, a temperature
past 2000 thousandths, an answer schema the machine's table does not hold), then
asks its `ModelProvider` for a `Generation`, and never cuts the answer: one past
the bound of ``text`` fails ``answer_too_large``. No provider here reaches a
model. `ScriptedProvider` gives the answers a scenario wrote;
`ExhaustiveProvider` gives the outcome class a test names at each call
(docs/adr/0087, section 9.4).

The Python runtime has no log facility, so the branches here say nothing; each
outcome is visible in the record it answers.
"""

from __future__ import annotations

from collections import deque
from dataclasses import dataclass, field
from typing import TYPE_CHECKING, Deque, Dict, List, Optional, Protocol, Sequence

from .records import HOLD, INVALID_ARGUMENT, REFUSE, Outcome, failure, record

if TYPE_CHECKING:
    from sce_runtime import AnswerSchema

    from .host import ReferenceHost

#: The operation that generates a text.
MODEL_GENERATE = "sce-host-model-generate"

#: The bound of a result's ``text``, in bytes (``model_answer`` of
#: ``sce:std/host/model_generate_result.scxml``).
ANSWER_MAX_BYTES = 4096

#: The highest sampling temperature a request may ask for, in thousandths.
TEMPERATURE_MAX_MILLI = 2000

#: A generation's ends, as a provider names them.
ANSWERED = "answered"
FAILED = "failed"
REFUSED = "refused"
HELD = "held"


@dataclass(frozen=True)
class GenerateRequest:
    """A generation's request, as a provider reads it. `answer_schema` is the
    machine's answer schema the request names, `None` for a free text."""

    prompt: str
    answer_schema: Optional["AnswerSchema"]
    max_tokens: int
    temperature_milli: int


@dataclass(frozen=True)
class Generation:
    """What a provider made of one generation: `kind` one of `ANSWERED` (the
    completion's fields but ``error``), `FAILED` (`code`), `REFUSED` or
    `HELD`."""

    kind: str
    text: str = ""
    finish: str = ""
    constrained: bool = False
    model: str = ""
    prompt_tokens: int = 0
    answer_tokens: int = 0
    code: str = ""


class ModelProvider(Protocol):
    """Where a host's generations come from: a model on the device, a remote
    one, or a script."""

    def generate(self, request: GenerateRequest) -> Generation: ...


def tokens(text: str) -> int:
    """A text's tokens as the reference providers count them: its UTF-8 bytes."""
    return len(text.encode("utf-8"))


@dataclass
class ScriptedAnswer:
    """One answer a scripted provider gives; `finish` is ``stop`` or ``length``."""

    text: str
    finish: str = "stop"


@dataclass
class ScriptedProvider:
    """A provider that gives the answers it was told, one a generation, in order.
    It is `constrained` only when the request names an answer schema; `context`
    is the most tokens a prompt and ``max_tokens`` may take together, 0 for none;
    `hold` holds every generation unanswered."""

    answers: Deque[ScriptedAnswer] = field(default_factory=deque)
    constrained: bool = False
    name: str = "scripted"
    context: int = 0
    hold: bool = False

    def generate(self, request: GenerateRequest) -> Generation:
        if self.hold:
            return Generation(HELD)
        prompt_tokens = tokens(request.prompt)
        if self.context and prompt_tokens + request.max_tokens > self.context:
            return Generation(FAILED, code="context_exceeded")
        if not self.answers:
            return Generation(FAILED, code="unavailable")
        answer = self.answers.popleft()
        return Generation(
            ANSWERED,
            text=answer.text,
            finish=answer.finish,
            constrained=self.constrained and request.answer_schema is not None,
            model=self.name,
            prompt_tokens=prompt_tokens,
            answer_tokens=tokens(answer.text),
        )


@dataclass(frozen=True)
class OutcomeClass:
    """One outcome class of a generation (docs/adr/0087, section 9.4), as the
    table every lane reads names it (tests/host_features/model_outcome_classes.json):
    `kind` `ANSWERED` (a completion with `text` and `finish`), `FAILED` (`code`),
    `REFUSED`, or `HELD` (the test ends the call by a deadline or a cancel)."""

    kind: str
    text: str = ""
    finish: str = ""
    code: str = ""


class ExhaustiveProvider:
    """A provider that stands for a model that may give any outcome at any call:
    each generation takes the next class of the sequence the test named."""

    def __init__(self, sequence: Sequence[OutcomeClass]) -> None:
        self._sequence: Deque[OutcomeClass] = deque(sequence)

    def generate(self, request: GenerateRequest) -> Generation:
        if not self._sequence:
            return Generation(FAILED, code="unavailable")
        cls = self._sequence.popleft()
        if cls.kind == ANSWERED:
            return Generation(
                ANSWERED,
                text=cls.text,
                finish=cls.finish,
                model="exhaustive",
                prompt_tokens=tokens(request.prompt),
                answer_tokens=tokens(cls.text),
            )
        if cls.kind == FAILED:
            return Generation(FAILED, code=cls.code)
        return Generation(cls.kind)


def judge(
    schemas: List["AnswerSchema"], params: Dict[str, str]
) -> Optional[GenerateRequest]:
    """The request the host accepts, or `None` for one it refuses: an empty
    prompt, ``max_tokens`` zero, a temperature past `TEMPERATURE_MAX_MILLI`, or
    an answer schema the table does not hold."""
    prompt = params.get("prompt", "")
    try:
        max_tokens = int(params.get("max_tokens", ""))
        temperature = int(params.get("temperature_milli", ""))
    except ValueError:
        return None
    if not prompt or max_tokens <= 0 or not 0 <= temperature <= TEMPERATURE_MAX_MILLI:
        return None
    answer_schema = None
    named = params.get("answer_schema", "")
    if named:
        answer_schema = next((s for s in schemas if s.id == named), None)
        if answer_schema is None:
            return None
    return GenerateRequest(prompt, answer_schema, max_tokens, temperature)


def generate(host: "ReferenceHost", invoke_id: str, params: Dict[str, str]) -> Outcome:
    """Generate a text for invocation `invoke_id`, whose request the host
    accepted."""
    request = judge(host.answer_schemas, params)
    assert request is not None, "the request was judged before the generation"
    generation = host.provider.generate(request)
    if generation.kind == ANSWERED:
        if tokens(generation.text) > ANSWER_MAX_BYTES:
            return failure(MODEL_GENERATE, "answer_too_large")
        return record(
            {
                "text": generation.text,
                "constrained": generation.constrained,
                "model": generation.model,
                "prompt_tokens": generation.prompt_tokens,
                "answer_tokens": generation.answer_tokens,
                "finish": generation.finish,
            }
        )
    if generation.kind == FAILED:
        return failure(MODEL_GENERATE, generation.code)
    if generation.kind == REFUSED:
        return Outcome(REFUSE, INVALID_ARGUMENT)
    host.held.append(invoke_id)
    return Outcome(HOLD)


__all__ = [
    "ANSWERED",
    "ANSWER_MAX_BYTES",
    "ExhaustiveProvider",
    "FAILED",
    "GenerateRequest",
    "Generation",
    "HELD",
    "MODEL_GENERATE",
    "ModelProvider",
    "OutcomeClass",
    "REFUSED",
    "ScriptedAnswer",
    "ScriptedProvider",
    "TEMPERATURE_MAX_MILLI",
]
