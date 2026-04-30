/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure") */
/* Runtime: sce_forge_runtime */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_CROSSFILE_PROCEDURE_FILTER_L2_H
#define SCE_FORGE_CROSSFILE_PROCEDURE_FILTER_L2_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>
#include "sce/forge/limits.h"
#include "sce/forge/procedure.h"
#include "filter_low_pass.h"

/* ── State and Event enums ─────────────────────────────────────── */

typedef enum {
    CROSSFILE_PROCEDURE_FILTER_STATE_SAMPLE = 0,
    CROSSFILE_PROCEDURE_FILTER_STATE_DONE = 1
} crossfile_procedure_filter_state_t;

typedef enum {
    CROSSFILE_PROCEDURE_FILTER_EVENT_NONE = 0,
    CROSSFILE_PROCEDURE_FILTER_EVENT_ERROR_EXECUTION = 1,
    CROSSFILE_PROCEDURE_FILTER_EVENT_FAIL = 2,
    CROSSFILE_PROCEDURE_FILTER_EVENT_OK = 3
} crossfile_procedure_filter_event_t;

/* ── State struct (one instance per execute() call) ─────────────── */

typedef struct {
    double raw_sample;
    double smoothed;
    /* Imported kind members (cross-file composition).
       Each stateful import (codec/filter/observer/validator/procedure) is
       embedded by-value; the procedure dispatches method calls through
       `&_st->{member_name}` (see expr::lower_stateful_import_calls). */
    filter_low_pass_t smoother_;
    /* Service handler (set by execute()) */
    sce_forge_procedure_service_handler_t service_handler;
    void *service_handler_user_data;
    /* W3C SCXML 5.10: pending _event.data binding (bytes-typed). */
    sce_forge_bytes_t pending_event_data;
} crossfile_procedure_filter_t;

/* ── Done data storage (one static const array per <final> state) ── */

static const sce_forge_procedure_done_param_t crossfile_procedure_filter_done_data_done[] = {
    { "result", "success" }
};
static const size_t crossfile_procedure_filter_done_data_done_count =
    sizeof(crossfile_procedure_filter_done_data_done)
        / sizeof(crossfile_procedure_filter_done_data_done[0]);

/* ── Static step functions ─────────────────────────────────────── */

static inline crossfile_procedure_filter_event_t crossfile_procedure_filter_execute_entry_actions(
    crossfile_procedure_filter_t *_st, crossfile_procedure_filter_state_t _state) {
    (void)_st;
    switch (_state) {
        default: break;
    }
    return CROSSFILE_PROCEDURE_FILTER_EVENT_NONE;
}

/* Returns true iff a transition fired; populates *out_next_state, *out_tr_index, *out_has_assigns. */
static inline bool crossfile_procedure_filter_process_transition(
    const crossfile_procedure_filter_t *_st, crossfile_procedure_filter_state_t _state, crossfile_procedure_filter_event_t _event,
    crossfile_procedure_filter_state_t *out_next, size_t *out_tr_index, bool *out_has_assigns) {
    (void)_st;
    switch (_state) {
        case CROSSFILE_PROCEDURE_FILTER_STATE_SAMPLE:
            if (_event == CROSSFILE_PROCEDURE_FILTER_EVENT_NONE) {
                *out_next = CROSSFILE_PROCEDURE_FILTER_STATE_DONE;
                *out_tr_index = 0;
                *out_has_assigns = true;
                return true;
            }
            return false;
        default: return false;
    }
}

static inline sce_forge_procedure_raise_t crossfile_procedure_filter_execute_transition_actions(
    crossfile_procedure_filter_t *_st, crossfile_procedure_filter_state_t _source, size_t _tr_index) {
    (void)_st; (void)_source; (void)_tr_index;
    sce_forge_procedure_raise_t _r = { false, 0 };
    if (_source == CROSSFILE_PROCEDURE_FILTER_STATE_SAMPLE) {
        if (_tr_index == 0) {
            _st->smoothed = filter_low_pass_update(&_st->smoother_, _st->raw_sample);
        }
    }
    return _r;
}

/* ── Main run function ─────────────────────────────────────────── */

static inline sce_forge_procedure_run_result_t crossfile_procedure_filter_execute(
    sce_forge_procedure_service_handler_t _handler,
    void *_handler_user_data,
    double raw_sample) {
    crossfile_procedure_filter_t _st = {0};
    _st.service_handler = _handler;
    _st.service_handler_user_data = _handler_user_data;
    _st.raw_sample = raw_sample;

    crossfile_procedure_filter_state_t _current = CROSSFILE_PROCEDURE_FILTER_STATE_SAMPLE;
    crossfile_procedure_filter_event_t _event = crossfile_procedure_filter_execute_entry_actions(&_st, _current);

    for (int _iter = 0; _iter < 1000; ++_iter) {
        switch (_current) {
            case CROSSFILE_PROCEDURE_FILTER_STATE_DONE: {
                sce_forge_procedure_run_result_t _result = { true, "done", NULL, 0 };
                _result.done_data = crossfile_procedure_filter_done_data_done;
                _result.done_data_count = crossfile_procedure_filter_done_data_done_count;
                /* Execute donedata side effects via entry-actions
                 * dispatch (mirrors cpp parity even though _event is
                 * unused for final states). */
                (void)crossfile_procedure_filter_execute_entry_actions(&_st, _current);
                return _result;
            }
            default: break;
        }

        crossfile_procedure_filter_state_t _next;
        size_t _tr_index;
        bool _has_assigns;
        if (!crossfile_procedure_filter_process_transition(&_st, _current, _event, &_next, &_tr_index, &_has_assigns)) {
            break;
        }
        if (_has_assigns) {
            sce_forge_procedure_raise_t _r =
                crossfile_procedure_filter_execute_transition_actions(&_st, _current, _tr_index);
            if (_r.raised) {
                /* Re-pump source state with the raised event. */
                _event = (crossfile_procedure_filter_event_t)_r.event;
                continue;
            }
        }
        _current = _next;
        _event = crossfile_procedure_filter_execute_entry_actions(&_st, _current);
    }

    sce_forge_procedure_run_result_t _result = { false, "", NULL, 0 };
    return _result;
}


#endif  /* SCE_FORGE_CROSSFILE_PROCEDURE_FILTER_L2_H */
