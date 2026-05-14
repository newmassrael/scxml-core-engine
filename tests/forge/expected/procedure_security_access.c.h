// SCE-MAP: procedure_security_access:1

/* SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure") */
/* Runtime: sce_forge_runtime */
/* Do not edit — regenerate from the source SCXML file. */

#ifndef SCE_FORGE_PROCEDURE_SECURITY_ACCESS_L2_H
#define SCE_FORGE_PROCEDURE_SECURITY_ACCESS_L2_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include <string.h>
#include "sce/forge/limits.h"
#include "sce/forge/procedure.h"

/* ── State and Event enums ─────────────────────────────────────── */

typedef enum {
    PROCEDURE_SECURITY_ACCESS_STATE_SEND_TESTER_PRESENT = 0,
    PROCEDURE_SECURITY_ACCESS_STATE_REQUEST_SEED = 1,
    PROCEDURE_SECURITY_ACCESS_STATE_SEND_KEY = 2,
    PROCEDURE_SECURITY_ACCESS_STATE_RETRY = 3,
    PROCEDURE_SECURITY_ACCESS_STATE_DONE = 4,
    PROCEDURE_SECURITY_ACCESS_STATE_ERROR = 5
} procedure_security_access_state_t;

typedef enum {
    PROCEDURE_SECURITY_ACCESS_EVENT_NONE = 0,
    PROCEDURE_SECURITY_ACCESS_EVENT_ERROR_EXECUTION = 1,
    PROCEDURE_SECURITY_ACCESS_EVENT_FAIL = 2,
    PROCEDURE_SECURITY_ACCESS_EVENT_OK = 3
} procedure_security_access_event_t;

/* ── State struct (one instance per execute() call) ─────────────── */

typedef struct {
    uint32_t ecu_addr;
    sce_forge_bytes_t seed;
    int32_t max_retries;
    int32_t retry_count;
    /* Service handler (set by execute()) */
    sce_forge_procedure_service_handler_t service_handler;
    void *service_handler_user_data;
    /* Helper DI: <sce:helper name="compute_key"> (by-value args, no user_data) */
    sce_forge_bytes_t (*compute_key)(sce_forge_bytes_t);
    /* W3C SCXML 5.10: pending _event.data binding (bytes-typed). */
    sce_forge_bytes_t pending_event_data;
} procedure_security_access_t;

/* ── Done data storage (one static const array per <final> state) ── */

static const sce_forge_procedure_done_param_t procedure_security_access_done_data_done[] = {
    { "result", "success" }
};
static const size_t procedure_security_access_done_data_done_count =
    sizeof(procedure_security_access_done_data_done)
        / sizeof(procedure_security_access_done_data_done[0]);

static const sce_forge_procedure_done_param_t procedure_security_access_done_data_error[] = {
    { "result", "failure" }
};
static const size_t procedure_security_access_done_data_error_count =
    sizeof(procedure_security_access_done_data_error)
        / sizeof(procedure_security_access_done_data_error[0]);

/* ── Static step functions ─────────────────────────────────────── */

static inline procedure_security_access_event_t procedure_security_access_execute_entry_actions(
    procedure_security_access_t *_st, procedure_security_access_state_t _state) {
    (void)_st;
    switch (_state) {
        case PROCEDURE_SECURITY_ACCESS_STATE_SEND_TESTER_PRESENT: {
            if (_st->service_handler != NULL) {
                sce_forge_procedure_service_request_t _req = {0};
                _req.service = "TesterPresent";
                _req.has_addr = true;
                _req.addr = "";
                sce_forge_procedure_service_response_t _resp =
                    _st->service_handler(&_req, _st->service_handler_user_data);
                _st->pending_event_data = _resp.data;
                return _resp.success ? PROCEDURE_SECURITY_ACCESS_EVENT_OK : PROCEDURE_SECURITY_ACCESS_EVENT_FAIL;
            }
            break;
        }
        case PROCEDURE_SECURITY_ACCESS_STATE_REQUEST_SEED: {
            if (_st->service_handler != NULL) {
                sce_forge_procedure_service_request_t _req = {0};
                _req.service = "SecurityAccess";
                _req.has_subfunc = true;
                _req.subfunc = "0x01";
                sce_forge_procedure_service_response_t _resp =
                    _st->service_handler(&_req, _st->service_handler_user_data);
                _st->pending_event_data = _resp.data;
                return _resp.success ? PROCEDURE_SECURITY_ACCESS_EVENT_OK : PROCEDURE_SECURITY_ACCESS_EVENT_FAIL;
            }
            break;
        }
        case PROCEDURE_SECURITY_ACCESS_STATE_SEND_KEY: {
            if (_st->service_handler != NULL) {
                sce_forge_procedure_service_request_t _req = {0};
                _req.service = "SecurityAccess";
                _req.has_subfunc = true;
                _req.subfunc = "0x02";
                _req.has_payload = true;
                _req.payload = _st->compute_key(_st->seed);
                sce_forge_procedure_service_response_t _resp =
                    _st->service_handler(&_req, _st->service_handler_user_data);
                _st->pending_event_data = _resp.data;
                return _resp.success ? PROCEDURE_SECURITY_ACCESS_EVENT_OK : PROCEDURE_SECURITY_ACCESS_EVENT_FAIL;
            }
            break;
        }
        default: break;
    }
    return PROCEDURE_SECURITY_ACCESS_EVENT_NONE;
}

/* Returns true iff a transition fired; populates *out_next_state, *out_tr_index, *out_has_assigns. */
static inline bool procedure_security_access_process_transition(
    const procedure_security_access_t *_st, procedure_security_access_state_t _state, procedure_security_access_event_t _event,
    procedure_security_access_state_t *out_next, size_t *out_tr_index, bool *out_has_assigns) {
    (void)_st;
    switch (_state) {
        case PROCEDURE_SECURITY_ACCESS_STATE_SEND_TESTER_PRESENT:
            if (_event == PROCEDURE_SECURITY_ACCESS_EVENT_OK) {
                *out_next = PROCEDURE_SECURITY_ACCESS_STATE_REQUEST_SEED;
                *out_tr_index = 0;
                *out_has_assigns = false;
                return true;
            }
            if (_event == PROCEDURE_SECURITY_ACCESS_EVENT_FAIL) {
                *out_next = PROCEDURE_SECURITY_ACCESS_STATE_ERROR;
                *out_tr_index = 1;
                *out_has_assigns = false;
                return true;
            }
            return false;
        case PROCEDURE_SECURITY_ACCESS_STATE_REQUEST_SEED:
            if (_event == PROCEDURE_SECURITY_ACCESS_EVENT_OK) {
                *out_next = PROCEDURE_SECURITY_ACCESS_STATE_SEND_KEY;
                *out_tr_index = 0;
                *out_has_assigns = true;
                return true;
            }
            if (_event == PROCEDURE_SECURITY_ACCESS_EVENT_FAIL) {
                *out_next = PROCEDURE_SECURITY_ACCESS_STATE_RETRY;
                *out_tr_index = 1;
                *out_has_assigns = false;
                return true;
            }
            return false;
        case PROCEDURE_SECURITY_ACCESS_STATE_SEND_KEY:
            if (_event == PROCEDURE_SECURITY_ACCESS_EVENT_OK) {
                *out_next = PROCEDURE_SECURITY_ACCESS_STATE_DONE;
                *out_tr_index = 0;
                *out_has_assigns = false;
                return true;
            }
            if (_event == PROCEDURE_SECURITY_ACCESS_EVENT_FAIL) {
                *out_next = PROCEDURE_SECURITY_ACCESS_STATE_RETRY;
                *out_tr_index = 1;
                *out_has_assigns = false;
                return true;
            }
            return false;
        case PROCEDURE_SECURITY_ACCESS_STATE_RETRY:
            if (_event == PROCEDURE_SECURITY_ACCESS_EVENT_NONE) {
                if (_st->retry_count < _st->max_retries) {
                    *out_next = PROCEDURE_SECURITY_ACCESS_STATE_REQUEST_SEED;
                    *out_tr_index = 0;
                    *out_has_assigns = true;
                    return true;
                }
            }
            if (_event == PROCEDURE_SECURITY_ACCESS_EVENT_NONE) {
                if (_st->retry_count >= _st->max_retries) {
                    *out_next = PROCEDURE_SECURITY_ACCESS_STATE_ERROR;
                    *out_tr_index = 1;
                    *out_has_assigns = false;
                    return true;
                }
            }
            return false;
        default: return false;
    }
}

static inline sce_forge_procedure_raise_t procedure_security_access_execute_transition_actions(
    procedure_security_access_t *_st, procedure_security_access_state_t _source, size_t _tr_index) {
    (void)_st; (void)_source; (void)_tr_index;
    sce_forge_procedure_raise_t _r = { false, 0 };
    if (_source == PROCEDURE_SECURITY_ACCESS_STATE_REQUEST_SEED) {
        if (_tr_index == 0) {
            {
                sce_forge_bytes_t _scope_tmp = _st->pending_event_data;
                if (_scope_tmp.len > 64) {
                    _r.raised = true;
                    _r.event = PROCEDURE_SECURITY_ACCESS_EVENT_ERROR_EXECUTION;
                    return _r;
                }
                _st->seed = _scope_tmp;
            }
        }
    }
    if (_source == PROCEDURE_SECURITY_ACCESS_STATE_RETRY) {
        if (_tr_index == 0) {
            _st->retry_count = _st->retry_count + 1;
        }
    }
    return _r;
}

/* ── Main run function ─────────────────────────────────────────── */

static inline sce_forge_procedure_run_result_t procedure_security_access_execute(
    sce_forge_procedure_service_handler_t _handler,
    void *_handler_user_data,
    sce_forge_bytes_t (*compute_key)(sce_forge_bytes_t),
    uint32_t ecu_addr) {
    procedure_security_access_t _st = {0};
    _st.service_handler = _handler;
    _st.service_handler_user_data = _handler_user_data;
    _st.compute_key = compute_key;
    _st.ecu_addr = ecu_addr;
    _st.max_retries = 3;
    _st.retry_count = 0;

    procedure_security_access_state_t _current = PROCEDURE_SECURITY_ACCESS_STATE_SEND_TESTER_PRESENT;
    procedure_security_access_event_t _event = procedure_security_access_execute_entry_actions(&_st, _current);

    for (int _iter = 0; _iter < 1000; ++_iter) {
        switch (_current) {
            case PROCEDURE_SECURITY_ACCESS_STATE_DONE: {
                sce_forge_procedure_run_result_t _result = { true, "done", NULL, 0 };
                _result.done_data = procedure_security_access_done_data_done;
                _result.done_data_count = procedure_security_access_done_data_done_count;
                /* Execute donedata side effects via entry-actions
                 * dispatch (mirrors cpp parity even though _event is
                 * unused for final states). */
                (void)procedure_security_access_execute_entry_actions(&_st, _current);
                return _result;
            }
            case PROCEDURE_SECURITY_ACCESS_STATE_ERROR: {
                sce_forge_procedure_run_result_t _result = { true, "error", NULL, 0 };
                _result.done_data = procedure_security_access_done_data_error;
                _result.done_data_count = procedure_security_access_done_data_error_count;
                /* Execute donedata side effects via entry-actions
                 * dispatch (mirrors cpp parity even though _event is
                 * unused for final states). */
                (void)procedure_security_access_execute_entry_actions(&_st, _current);
                return _result;
            }
            default: break;
        }

        procedure_security_access_state_t _next;
        size_t _tr_index;
        bool _has_assigns;
        if (!procedure_security_access_process_transition(&_st, _current, _event, &_next, &_tr_index, &_has_assigns)) {
            break;
        }
        if (_has_assigns) {
            sce_forge_procedure_raise_t _r =
                procedure_security_access_execute_transition_actions(&_st, _current, _tr_index);
            if (_r.raised) {
                /* Re-pump source state with the raised event. */
                _event = (procedure_security_access_event_t)_r.event;
                continue;
            }
        }
        _current = _next;
        _event = procedure_security_access_execute_entry_actions(&_st, _current);
    }

    sce_forge_procedure_run_result_t _result = { false, "", NULL, 0 };
    return _result;
}


#endif  /* SCE_FORGE_PROCEDURE_SECURITY_ACCESS_L2_H */
