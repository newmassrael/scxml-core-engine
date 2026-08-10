// SCE-MAP: crossfile_procedure_codec_mutate:3 :: _forge_body

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure") */
/* Runtime: sce_forge_runtime */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CROSSFILE_PROCEDURE_CODEC_MUTATE_L2_H
#define SCE_FORGE_CROSSFILE_PROCEDURE_CODEC_MUTATE_L2_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>
#include "sce/forge/limits.h"
#include "sce/forge/procedure.h"
#include "codec_simple_frame.h"

/* ── State and Event enums ─────────────────────────────────────── */

typedef enum {
    CROSSFILE_PROCEDURE_CODEC_MUTATE_STATE_INIT = 0,
    CROSSFILE_PROCEDURE_CODEC_MUTATE_STATE_SEND = 1,
    CROSSFILE_PROCEDURE_CODEC_MUTATE_STATE_DONE = 2,
    CROSSFILE_PROCEDURE_CODEC_MUTATE_STATE_ERROR = 3
} crossfile_procedure_codec_mutate_state_t;

typedef enum {
    CROSSFILE_PROCEDURE_CODEC_MUTATE_EVENT_NONE = 0,
    CROSSFILE_PROCEDURE_CODEC_MUTATE_EVENT_ERROR_EXECUTION = 1,
    CROSSFILE_PROCEDURE_CODEC_MUTATE_EVENT_FAIL = 2,
    CROSSFILE_PROCEDURE_CODEC_MUTATE_EVENT_OK = 3
} crossfile_procedure_codec_mutate_event_t;

/* ── Cross-file import wrappers ─────────────────────────────────── */
/* Codec imports expose `encode(self*, writer*)` as a free function
   that writes into a caller-owned `sce_forge_writer_t`. The
   procedure's `<send sce:payload>` slot is the runtime's stack-bounded
   `sce_forge_bytes_t`. A per-procedure inline wrapper bridges the
   two by running the codec's encode over a writer that targets the
   bytes struct's data buffer (worst-case bounded by
   SCE_FORGE_BYTES_DEFAULT_MAX). The wrapper name embeds both the
   procedure and the alias so distinct procedures importing the same
   codec do not collide. Other stateful kinds (filter, observer, ...)
   emit the wrapper-free direct dispatch via
   `expr::ImportLowering::methods` and produce no entries here. */
static inline sce_forge_bytes_t crossfile_procedure_codec_mutate__frame_encode(const codec_simple_frame_t *self) {
    sce_forge_bytes_t _r;
    sce_forge_writer_t _wr = sce_forge_writer_init_buf(_r.data, SCE_FORGE_BYTES_DEFAULT_MAX);
    (void)codec_simple_frame_encode(self, &_wr);
    _r.len = _wr.pos;
    return _r;
}

/* ── State struct (one instance per execute() call) ─────────────── */

typedef struct {
    uint8_t msg_id;
    /* Imported kind members (cross-file composition).
       Each stateful import (codec/filter/observer/validator/procedure) is
       embedded by-value; the procedure dispatches method calls through
       `&_st->{member_name}` (see expr::lower_stateful_import_calls). */
    codec_simple_frame_t frame_;
    /* Service handler (set by execute()) */
    sce_forge_procedure_service_handler_t service_handler;
    void *service_handler_user_data;
    /* W3C SCXML 5.10: pending _event.data binding (bytes-typed). */
    sce_forge_bytes_t pending_event_data;
} crossfile_procedure_codec_mutate_t;

/* ── Done data storage (one static const array per <final> state) ── */

static const sce_forge_procedure_done_param_t crossfile_procedure_codec_mutate_done_data_done[] = {
    { "result", "success" }
};
static const size_t crossfile_procedure_codec_mutate_done_data_done_count =
    sizeof(crossfile_procedure_codec_mutate_done_data_done)
        / sizeof(crossfile_procedure_codec_mutate_done_data_done[0]);

static const sce_forge_procedure_done_param_t crossfile_procedure_codec_mutate_done_data_error[] = {
    { "result", "failure" }
};
static const size_t crossfile_procedure_codec_mutate_done_data_error_count =
    sizeof(crossfile_procedure_codec_mutate_done_data_error)
        / sizeof(crossfile_procedure_codec_mutate_done_data_error[0]);

/* ── Static step functions ─────────────────────────────────────── */

static inline crossfile_procedure_codec_mutate_event_t crossfile_procedure_codec_mutate_execute_entry_actions(
    crossfile_procedure_codec_mutate_t *_st, crossfile_procedure_codec_mutate_state_t _state) {
    (void)_st;
    switch (_state) {
        case CROSSFILE_PROCEDURE_CODEC_MUTATE_STATE_SEND: {
            if (_st->service_handler != NULL) {
                sce_forge_procedure_service_request_t _req = {0};
                _req.service = "transport";
                _req.has_payload = true;
                _req.payload = crossfile_procedure_codec_mutate__frame_encode(&_st->frame_);
                sce_forge_procedure_service_response_t _resp =
                    _st->service_handler(&_req, _st->service_handler_user_data);
                _st->pending_event_data = _resp.data;
                return _resp.success ? CROSSFILE_PROCEDURE_CODEC_MUTATE_EVENT_OK : CROSSFILE_PROCEDURE_CODEC_MUTATE_EVENT_FAIL;
            }
            break;
        }
        default: break;
    }
    return CROSSFILE_PROCEDURE_CODEC_MUTATE_EVENT_NONE;
}

/* Returns true iff a transition fired; populates *out_next_state, *out_tr_index, *out_has_assigns. */
static inline bool crossfile_procedure_codec_mutate_process_transition(
    const crossfile_procedure_codec_mutate_t *_st, crossfile_procedure_codec_mutate_state_t _state, crossfile_procedure_codec_mutate_event_t _event,
    crossfile_procedure_codec_mutate_state_t *out_next, size_t *out_tr_index, bool *out_has_assigns) {
    (void)_st;
    switch (_state) {
        case CROSSFILE_PROCEDURE_CODEC_MUTATE_STATE_INIT:
            if (_event == CROSSFILE_PROCEDURE_CODEC_MUTATE_EVENT_NONE) {
                *out_next = CROSSFILE_PROCEDURE_CODEC_MUTATE_STATE_SEND;
                *out_tr_index = 0;
                *out_has_assigns = true;
                return true;
            }
            return false;
        case CROSSFILE_PROCEDURE_CODEC_MUTATE_STATE_SEND:
            if (_event == CROSSFILE_PROCEDURE_CODEC_MUTATE_EVENT_OK) {
                *out_next = CROSSFILE_PROCEDURE_CODEC_MUTATE_STATE_DONE;
                *out_tr_index = 0;
                *out_has_assigns = false;
                return true;
            }
            if (_event == CROSSFILE_PROCEDURE_CODEC_MUTATE_EVENT_FAIL) {
                *out_next = CROSSFILE_PROCEDURE_CODEC_MUTATE_STATE_ERROR;
                *out_tr_index = 1;
                *out_has_assigns = false;
                return true;
            }
            return false;
        default: return false;
    }
}

static inline sce_forge_procedure_raise_t crossfile_procedure_codec_mutate_execute_transition_actions(
    crossfile_procedure_codec_mutate_t *_st, crossfile_procedure_codec_mutate_state_t _source, size_t _tr_index) {
    (void)_st; (void)_source; (void)_tr_index;
    sce_forge_procedure_raise_t _r = { false, 0 };
    if (_source == CROSSFILE_PROCEDURE_CODEC_MUTATE_STATE_INIT) {
        if (_tr_index == 0) {
            _st->frame_.msg_id = _st->msg_id;
        }
    }
    return _r;
}

/* ── Main run function ─────────────────────────────────────────── */

static inline sce_forge_procedure_run_result_t crossfile_procedure_codec_mutate_execute(
    sce_forge_procedure_service_handler_t _handler,
    void *_handler_user_data,
    uint8_t msg_id) {
    crossfile_procedure_codec_mutate_t _st = {0};
    _st.service_handler = _handler;
    _st.service_handler_user_data = _handler_user_data;
    _st.msg_id = msg_id;

    crossfile_procedure_codec_mutate_state_t _current = CROSSFILE_PROCEDURE_CODEC_MUTATE_STATE_INIT;
    crossfile_procedure_codec_mutate_event_t _event = crossfile_procedure_codec_mutate_execute_entry_actions(&_st, _current);

    for (int _iter = 0; _iter < 1000; ++_iter) {
        switch (_current) {
            case CROSSFILE_PROCEDURE_CODEC_MUTATE_STATE_DONE: {
                sce_forge_procedure_run_result_t _result = { true, "done", NULL, 0 };
                _result.done_data = crossfile_procedure_codec_mutate_done_data_done;
                _result.done_data_count = crossfile_procedure_codec_mutate_done_data_done_count;
                /* Execute donedata side effects via entry-actions
                 * dispatch (mirrors cpp parity even though _event is
                 * unused for final states). */
                (void)crossfile_procedure_codec_mutate_execute_entry_actions(&_st, _current);
                return _result;
            }
            case CROSSFILE_PROCEDURE_CODEC_MUTATE_STATE_ERROR: {
                sce_forge_procedure_run_result_t _result = { true, "error", NULL, 0 };
                _result.done_data = crossfile_procedure_codec_mutate_done_data_error;
                _result.done_data_count = crossfile_procedure_codec_mutate_done_data_error_count;
                /* Execute donedata side effects via entry-actions
                 * dispatch (mirrors cpp parity even though _event is
                 * unused for final states). */
                (void)crossfile_procedure_codec_mutate_execute_entry_actions(&_st, _current);
                return _result;
            }
            default: break;
        }

        crossfile_procedure_codec_mutate_state_t _next;
        size_t _tr_index;
        bool _has_assigns;
        if (!crossfile_procedure_codec_mutate_process_transition(&_st, _current, _event, &_next, &_tr_index, &_has_assigns)) {
            break;
        }
        if (_has_assigns) {
            sce_forge_procedure_raise_t _r =
                crossfile_procedure_codec_mutate_execute_transition_actions(&_st, _current, _tr_index);
            if (_r.raised) {
                /* Re-pump source state with the raised event. */
                _event = (crossfile_procedure_codec_mutate_event_t)_r.event;
                continue;
            }
        }
        _current = _next;
        _event = crossfile_procedure_codec_mutate_execute_entry_actions(&_st, _current);
    }

    sce_forge_procedure_run_result_t _result = { false, "", NULL, 0 };
    return _result;
}


#endif  /* SCE_FORGE_CROSSFILE_PROCEDURE_CODEC_MUTATE_L2_H */
