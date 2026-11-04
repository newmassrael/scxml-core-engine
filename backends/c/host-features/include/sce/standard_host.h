/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/**
 * @file standard_host.h
 * @brief The reference host of the standard host features
 *        (docs/SCE_HOST_FEATURES.md): clock, key-value store, notifications
 *        and HTTP, as the operations a generated machine's host-run invokes
 *        ask for.
 *
 * A `sce_hf_host_t` is the whole of the four features' semantics as one object.
 * It is told the start of an operation (its type and the request's text
 * parameters) and answers with an outcome: a completion, a refusal, or nothing
 * yet. It knows no machine, so the same object serves a generated machine
 * through `sce_hf_invoker`, a test, or another embedding. Time comes from a
 * clock and the network from a transport, both the embedding's to supply.
 *
 * It is a hosted reference: it allocates. A build that serves the HTTP
 * operation sets `SCE_MAX_DATA_LEN` to at least SCE_HF_MIN_DATA_LEN, because a
 * result record is carried in one data slot of the machine.
 */

#ifndef SCE_C_STANDARD_HOST_H
#define SCE_C_STANDARD_HOST_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "sce/host_processor.h"

#ifdef __cplusplus
extern "C" {
#endif

/** The number of standard operation types. */
#define SCE_HF_TYPE_COUNT 12

/** Every operation type of the standard host features, in the order the
    specification lists them. */
extern const char *const sce_hf_types[SCE_HF_TYPE_COUNT];

/** The data slot a result of the largest record needs: a body of 16384 bytes
    and headers of 4096, as JSON text. */
#define SCE_HF_MIN_DATA_LEN 40000

/** What a host answers a start with. */
typedef enum sce_hf_kind_e {
    /** A completion: `data` is the result record, as the JSON a completion
        carries. A failure is such a record whose `error` is a code of the
        operation's closed set. */
    SCE_HF_DONE = 0,
    /** The host would not start the operation; `data` is the reason, as the
        JSON `error.execution` carries. */
    SCE_HF_REFUSE = 1,
    /** The host has started the operation and answers later, or never. */
    SCE_HF_HOLD = 2
} sce_hf_kind_t;

typedef struct sce_hf_outcome_s {
    sce_hf_kind_t kind;
    /** Owned by the outcome; release with `sce_hf_outcome_free`. */
    char *data;
} sce_hf_outcome_t;

void sce_hf_outcome_free(sce_hf_outcome_t *outcome);

/** Both readings of one instant. */
typedef struct sce_hf_reading_s {
    /** Milliseconds since 1970-01-01T00:00:00Z, POSIX time. */
    int64_t utc_ms;
    /** Milliseconds since an origin fixed for the life of the process. */
    int64_t monotonic_ms;
} sce_hf_reading_t;

/** The time zone a host reads local time in. */
typedef struct sce_hf_zone_s {
    /** An IANA identifier, or empty when only an offset is known. */
    char id[65];
    /** Seconds east of UTC in force now. */
    int32_t utc_offset_s;
} sce_hf_zone_t;

/** Where a host's time comes from. A source that cannot be read answers
    false, which the host reports as `unavailable`. */
typedef struct sce_hf_clock_s {
    bool (*read)(void *user_data, sce_hf_reading_t *out);
    bool (*zone)(void *user_data, sce_hf_zone_t *out);
    void *user_data;
} sce_hf_clock_t;

/** The operating system's clock. */
sce_hf_clock_t sce_hf_system_clock(void);

/** One header field: its name and its value. */
typedef struct sce_hf_header_s {
    char *name;
    char *value;
} sce_hf_header_t;

/** What the host hands its transport: the request, its header lines already
    split into name and value. Owned by the host for the call. */
typedef struct sce_hf_http_request_s {
    const char *method;
    const char *url;
    const sce_hf_header_t *headers;
    size_t header_count;
    const char *body;
} sce_hf_http_request_t;

/** What a transport answers: the response as it came. The transport fills it;
    the host releases it with `sce_hf_http_response_free`. */
typedef struct sce_hf_http_response_s {
    int status;
    sce_hf_header_t *headers;
    size_t header_count;
    unsigned char *body;
    size_t body_len;
} sce_hf_http_response_t;

void sce_hf_http_response_free(sce_hf_http_response_t *response);

/** A transport's answer to one exchange. */
typedef enum sce_hf_exchange_e {
    /** The response as it came, in `out`. */
    SCE_HF_EXCHANGE_RESPONSE = 0,
    /** No response was received: name resolution, connection, TLS, or the
        connection ended. */
    SCE_HF_EXCHANGE_FAILURE = 1,
    /** The transport has not answered; the host holds the exchange until it
        is cancelled. */
    SCE_HF_EXCHANGE_HOLD = 2
} sce_hf_exchange_t;

/** Carries one exchange. */
typedef struct sce_hf_transport_s {
    sce_hf_exchange_t (*exchange)(void *user_data, const sce_hf_http_request_t *request, sce_hf_http_response_t *out);
    void *user_data;
} sce_hf_transport_t;

typedef struct sce_hf_host_s sce_hf_host_t;

/** A host reading `clock` and carrying HTTP over `transport`, with an empty
    store, nothing posted, and the notification permission undetermined. */
sce_hf_host_t *sce_hf_host_new(sce_hf_clock_t clock, sce_hf_transport_t transport);
void sce_hf_host_free(sce_hf_host_t *host);

/** Inject a failure: the operation `operation` fails with `code`, once the
    request has been accepted. */
void sce_hf_set_fault(sce_hf_host_t *host, const char *operation, const char *code);
/** Ends every injected failure. */
void sce_hf_clear_faults(sce_hf_host_t *host);
/** Bound the store to `keys` keys; a write of a new key past it fails
    `quota_exceeded`. */
void sce_hf_set_quota(sce_hf_host_t *host, size_t keys);
/** The notification permission, as the platform says it: `granted`, `denied`
    or `undetermined`. */
void sce_hf_set_permission(sce_hf_host_t *host, const char *state);
/** What the user chooses when asked: `granted`, `denied`, or `none` for a host
    with no user to ask. */
void sce_hf_set_user_decision(sce_hf_host_t *host, const char *decision);

/** The store, in key order. */
size_t sce_hf_store_count(const sce_hf_host_t *host);
void sce_hf_store_entry(const sce_hf_host_t *host, size_t index, const char **key, const char **value);
/** The ids of the waiting and of the shown notifications, in the order posted. */
size_t sce_hf_waiting_count(const sce_hf_host_t *host);
const char *sce_hf_waiting_id(const sce_hf_host_t *host, size_t index);
size_t sce_hf_shown_count(const sce_hf_host_t *host);
const char *sce_hf_shown_id(const sce_hf_host_t *host, size_t index);
/** The invocations the host was told to stop, in the order told. */
size_t sce_hf_cancelled_count(const sce_hf_host_t *host);
const char *sce_hf_cancelled_id(const sce_hf_host_t *host, size_t index);

/** The starts the host was given since the log was last cleared, oldest first,
    refused ones included. */
size_t sce_hf_start_count(const sce_hf_host_t *host);
const char *sce_hf_start_operation(const sce_hf_host_t *host, size_t index);
const char *sce_hf_start_id(const sce_hf_host_t *host, size_t index);
size_t sce_hf_start_param_count(const sce_hf_host_t *host, size_t index);
void sce_hf_start_param(const sce_hf_host_t *host, size_t index, size_t param, const char **name, const char **value);
void sce_hf_clear_starts(sce_hf_host_t *host);

/** Start the operation of invocation `invoke_id` with the parameters
    `params[0..param_count)`.
 *
 * False for a type that is not one of the standard host features: the host
 * serves none of it. Otherwise the arguments are judged first (a refusal starts
 * nothing), then a failure the embedding injected, then the operation itself,
 * and `out` holds the answer. */
bool sce_hf_start(sce_hf_host_t *host, const char *operation, const char *invoke_id,
                  const sce_host_send_param_t *params, size_t param_count, sce_hf_outcome_t *out);

/** The host is told to stop invocation `invoke_id`, because its state exited
    or its deadline passed. What it held for it is dropped. */
void sce_hf_cancel(sce_hf_host_t *host, const char *invoke_id);

/* ── Generated text (docs/SCE_HOST_FEATURES.md, section 6) ──────────────
 *
 * The host judges the request (an empty prompt, `max_tokens` zero, a
 * temperature past 2000 thousandths, an answer schema the machine's table does
 * not hold), then asks its provider for a generation, and never cuts the
 * answer: one past the bound of `text` fails `answer_too_large`. No provider
 * here reaches a model. The scripted provider gives the answers a scenario
 * wrote; the exhaustive provider gives the outcome class a test names at each
 * call (docs/adr/0087, section 9.4). */

/** The operation that generates a text. */
#define SCE_HF_MODEL_GENERATE "sce-host-model-generate"
/** The bound of a result's `text`, in bytes (`model_answer`). */
#define SCE_HF_ANSWER_MAX_BYTES 4096u
/** The highest sampling temperature a request may ask for, in thousandths. */
#define SCE_HF_TEMPERATURE_MAX_MILLI 2000u

/** A generation's request, as a provider reads it. `answer_schema` is the
    machine's answer schema the request names, NULL for a free text. */
typedef struct sce_hf_generate_request_s {
    const char *prompt;
    const sce_answer_schema_t *answer_schema;
    uint32_t max_tokens;
    uint32_t temperature_milli;
} sce_hf_generate_request_t;

/** What a provider made of one generation. */
typedef enum sce_hf_generation_kind_e {
    /** An answer: the completion's fields but `error`. */
    SCE_HF_GENERATION_ANSWER = 0,
    /** A failure, by its `code` of the operation's closed set. */
    SCE_HF_GENERATION_FAILURE = 1,
    /** The host declines to start it (`invalid_argument`). */
    SCE_HF_GENERATION_REFUSAL = 2,
    /** No answer yet, or ever: a deadline or a cancel ends the call. */
    SCE_HF_GENERATION_HOLD = 3
} sce_hf_generation_kind_t;

/** A generation. Its texts are the provider's, valid until it is asked again;
    the host copies what it answers before then. */
typedef struct sce_hf_generation_s {
    sce_hf_generation_kind_t kind;
    const char *text;
    size_t text_len;
    const char *finish;
    bool constrained;
    const char *model;
    uint32_t prompt_tokens;
    uint32_t answer_tokens;
    const char *code;
} sce_hf_generation_t;

/** Where a host's generations come from: a model on the device, a remote one,
    or a script. */
typedef struct sce_hf_model_provider_s {
    sce_hf_generation_t (*generate)(void *user_data, const sce_hf_generate_request_t *request);
    void *user_data;
} sce_hf_model_provider_t;

/** Generate texts with `provider`, whose `user_data` outlives the host, from
    here on. A host starts with a scripted provider of its own that has no
    answer. */
void sce_hf_set_model_provider(sce_hf_host_t *host, sce_hf_model_provider_t provider);

/** Resolve a generation's `answer_schema` through the `count` schemas at
    `schemas`, the table of the machine this host serves
    (`<machine>_answer_schemas()`), constant storage the host does not copy. A
    request that names an id the table does not hold is refused. */
void sce_hf_set_answer_schemas(sce_hf_host_t *host, const sce_answer_schema_t *schemas, size_t count);

/** A provider that gives the answers it was told, one a generation, in order,
    counting a token as one byte. It is constrained only when the request names
    an answer schema; with no answer left it fails `unavailable`. */
typedef struct sce_hf_scripted_provider_s sce_hf_scripted_provider_t;

sce_hf_scripted_provider_t *sce_hf_scripted_provider_new(void);
void sce_hf_scripted_provider_free(sce_hf_scripted_provider_t *provider);
/** The provider as a host takes it. */
sce_hf_model_provider_t sce_hf_scripted_provider(sce_hf_scripted_provider_t *provider);
/** Forget the answers not given yet. */
void sce_hf_scripted_clear_answers(sce_hf_scripted_provider_t *provider);
/** Give `text[0..len)` with `finish` (`stop` or `length`) after the others. */
void sce_hf_scripted_push_answer(sce_hf_scripted_provider_t *provider, const char *text, size_t len,
                                 const char *finish);
void sce_hf_scripted_set_constrained(sce_hf_scripted_provider_t *provider, bool constrained);
/** What the provider says answered; `scripted` until set. */
void sce_hf_scripted_set_name(sce_hf_scripted_provider_t *provider, const char *name);
/** The most tokens a prompt and `max_tokens` may take together; 0 for none. */
void sce_hf_scripted_set_context(sce_hf_scripted_provider_t *provider, uint64_t context);
/** Whether the provider holds every generation unanswered. */
void sce_hf_scripted_set_hold(sce_hf_scripted_provider_t *provider, bool hold);

/** One outcome class of a generation (docs/adr/0087, section 9.4), as the table
    every lane reads names it (tests/host_features/model_outcome_classes.json):
    an answer with `text` and `finish`, a failure with `code`, a refusal, or a
    hold the test ends by a deadline or a cancel. */
typedef struct sce_hf_outcome_class_s {
    sce_hf_generation_kind_t kind;
    const char *text;
    const char *finish;
    const char *code;
} sce_hf_outcome_class_t;

/** A provider that stands for a model that may give any outcome at any call:
    each generation takes the next class of the sequence the test named. */
typedef struct sce_hf_exhaustive_provider_s sce_hf_exhaustive_provider_t;

/** A provider answering `sequence[0..count)`, which it copies. */
sce_hf_exhaustive_provider_t *sce_hf_exhaustive_provider_new(const sce_hf_outcome_class_t *sequence, size_t count);
void sce_hf_exhaustive_provider_free(sce_hf_exhaustive_provider_t *provider);
/** The provider as a host takes it. */
sce_hf_model_provider_t sce_hf_exhaustive_provider(sce_hf_exhaustive_provider_t *provider);

/** The handler to register under every type of `sce_hf_types`, with the host
    as `user_data`. A start is answered with its completion (a failure is a
    completion whose `error` is a code) or its refusal, or not at all when the
    host holds it. */
void sce_hf_invoker(void *user_data, const sce_host_invoke_event_t *event, sce_host_invoke_response_t *out);

/** Register `sce_hf_invoker` for every standard type in `registry`. False when
    the registry cannot hold them all (`SCE_MAX_HOST_INVOKERS`). */
bool sce_hf_register(sce_host_invoker_registry_t *registry, sce_hf_host_t *host);

#ifdef __cplusplus
}
#endif

#endif /* SCE_C_STANDARD_HOST_H */
