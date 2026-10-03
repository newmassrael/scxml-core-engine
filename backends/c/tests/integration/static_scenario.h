// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The scenarios of datamodel="sce-static" (docs/SCE_ACCEPTED_SUBSET.md §2.15),
// replayed against a generated C machine.
//
// `sce-build/tests/fixtures/static_datamodel/scenarios/<machine>.json` is the
// one answer every engine is held to — Kotlin, Rust, Go, Python, C++ and the
// Interpreter read the same file — and its expected values are derived from the
// document, not observed from a backend. A scenario a C test wrote out again by
// hand would be a second answer, free to drift from it, so this reads the file.
//
// The reader below is a tokenizer for the one shape those files have, small
// enough to own: a C test has no JSON library to borrow, and the other C tests
// that read a shared table scan it for its keys, which would take a step's
// `expect` for any other object that spelled `state`. It reads what a step
// carries — `event`, an event's `data` as the compact JSON text every producer
// fills it with, `expect.state`, `expect.ended`, `expect.variables` — and stops,
// naming it, at what it does not (a real number), rather than passing a
// scenario it did not replay.

#ifndef SCE_C_TESTS_STATIC_SCENARIO_H
#define SCE_C_TESTS_STATIC_SCENARIO_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// One generated machine as a scenario sees it: events and states by their
// document name, and the published variables by theirs.
typedef struct {
    const char *machine;
    void *sm;
    // The event goes in, and the macrostep it starts runs to its end. `data` is
    // the event's payload as the JSON text every producer fills `data` with, or
    // NULL when the step carries none. False for a name the machine has no event
    // of.
    bool (*raise)(void *sm, const char *event, const char *data);
    // 1 when the state is active, 0 when it is not, -1 for a name the machine
    // has no state of.
    int (*in_state)(void *sm, const char *state);
    bool (*ended)(void *sm);
    // False for a name the machine publishes no variable of. A bool reads as 0
    // or 1.
    bool (*variable)(void *sm, const char *name, int64_t *out);
} sce_scenario_driver_t;

typedef struct {
    const char *at;
} sce_scenario_cursor_t;

static void sce_scenario_space(sce_scenario_cursor_t *c) {
    while (*c->at == ' ' || *c->at == '\n' || *c->at == '\r' || *c->at == '\t') {
        c->at++;
    }
}

static bool sce_scenario_take(sce_scenario_cursor_t *c, char want) {
    sce_scenario_space(c);
    if (*c->at != want) {
        return false;
    }
    c->at++;
    return true;
}

// A string, into `out` when there is one (`cap` bytes, terminator included).
// `\uXXXX` is skipped and read as `?`: no key or name the machine is asked
// about is spelled with one.
static bool sce_scenario_string(sce_scenario_cursor_t *c, char *out, size_t cap) {
    sce_scenario_space(c);
    if (*c->at != '"') {
        return false;
    }
    c->at++;
    size_t n = 0;
    while (*c->at != '\0' && *c->at != '"') {
        char ch = *c->at++;
        if (ch == '\\') {
            ch = *c->at++;
            switch (ch) {
            case 'n':
                ch = '\n';
                break;
            case 't':
                ch = '\t';
                break;
            case 'u':
                for (int i = 0; i < 4 && *c->at != '\0'; ++i) {
                    c->at++;
                }
                ch = '?';
                break;
            case '"':
            case '\\':
            case '/':
                break;
            default:
                return false;
            }
        }
        if (out != NULL) {
            if (n + 1 >= cap) {
                return false;
            }
            out[n] = ch;
        }
        n++;
    }
    if (*c->at != '"') {
        return false;
    }
    c->at++;
    if (out != NULL) {
        out[n] = '\0';
    }
    return true;
}

static bool sce_scenario_skip_value(sce_scenario_cursor_t *c);

static bool sce_scenario_skip_members(sce_scenario_cursor_t *c, char close, bool keyed) {
    if (sce_scenario_take(c, close)) {
        return true;
    }
    for (;;) {
        if (keyed && (!sce_scenario_string(c, NULL, 0) || !sce_scenario_take(c, ':'))) {
            return false;
        }
        if (!sce_scenario_skip_value(c)) {
            return false;
        }
        if (sce_scenario_take(c, close)) {
            return true;
        }
        if (!sce_scenario_take(c, ',')) {
            return false;
        }
    }
}

static bool sce_scenario_skip_value(sce_scenario_cursor_t *c) {
    sce_scenario_space(c);
    switch (*c->at) {
    case '"':
        return sce_scenario_string(c, NULL, 0);
    case '{':
        c->at++;
        return sce_scenario_skip_members(c, '}', true);
    case '[':
        c->at++;
        return sce_scenario_skip_members(c, ']', false);
    case '\0':
        return false;
    default:
        // A number, true, false or null: up to whatever ends it.
        while (*c->at != '\0' && *c->at != ',' && *c->at != '}' && *c->at != ']' && *c->at != ' ' && *c->at != '\n' &&
               *c->at != '\r' && *c->at != '\t') {
            c->at++;
        }
        return true;
    }
}

// An integer, or true / false as 1 / 0. A real number is refused, not rounded.
static bool sce_scenario_number(sce_scenario_cursor_t *c, int64_t *out) {
    sce_scenario_space(c);
    if (strncmp(c->at, "true", 4) == 0) {
        c->at += 4;
        *out = 1;
        return true;
    }
    if (strncmp(c->at, "false", 5) == 0) {
        c->at += 5;
        *out = 0;
        return true;
    }
    char *end = NULL;
    long long value = strtoll(c->at, &end, 10);
    if (end == c->at || *end == '.' || *end == 'e' || *end == 'E') {
        return false;
    }
    c->at = end;
    *out = (int64_t)value;
    return true;
}

// `from` up to `to`, without the whitespace between its tokens: the compact
// text an event's `data` is written in, which is what every producer hands a
// machine. Whitespace inside a string stays. False when it does not fit `cap`.
static bool sce_scenario_compact(const char *from, const char *to, char *out, size_t cap) {
    size_t n = 0;
    bool in_string = false;
    for (const char *p = from; p < to; ++p) {
        const char ch = *p;
        if (in_string) {
            if (ch == '\\' && p + 1 < to) {
                if (n + 2 >= cap) {
                    return false;
                }
                out[n++] = ch;
                out[n++] = *++p;
                continue;
            }
            in_string = ch != '"';
        } else if (ch == ' ' || ch == '\n' || ch == '\r' || ch == '\t') {
            continue;
        } else if (ch == '"') {
            in_string = true;
        }
        if (n + 1 >= cap) {
            return false;
        }
        out[n++] = ch;
    }
    out[n] = '\0';
    return true;
}

static int sce_scenario_fail(const sce_scenario_driver_t *d, int step, const char *what) {
    (void)fprintf(stderr, "%s: FAIL [step %d] - %s\n", d->machine, step, what);
    return 1;
}

// What a step expects, read from its `expect` object — the object `c` is at.
static int sce_scenario_expect(sce_scenario_cursor_t *c, const sce_scenario_driver_t *d, int step) {
    int bad = 0;
    char key[64];
    char message[256];
    if (!sce_scenario_take(c, '{')) {
        return sce_scenario_fail(d, step, "`expect` is not an object");
    }
    if (sce_scenario_take(c, '}')) {
        return 0;
    }
    for (;;) {
        if (!sce_scenario_string(c, key, sizeof(key)) || !sce_scenario_take(c, ':')) {
            return sce_scenario_fail(d, step, "`expect` has a member with no name");
        }
        if (strcmp(key, "state") == 0) {
            char state[64];
            if (!sce_scenario_string(c, state, sizeof(state))) {
                return sce_scenario_fail(d, step, "`state` is not a string");
            }
            const int active = d->in_state(d->sm, state);
            if (active != 1) {
                (void)snprintf(message, sizeof(message), "state `%s` is %s", state,
                               active < 0 ? "not one of this machine's states" : "not active");
                bad |= sce_scenario_fail(d, step, message);
            }
        } else if (strcmp(key, "ended") == 0) {
            int64_t want = 0;
            if (!sce_scenario_number(c, &want)) {
                return sce_scenario_fail(d, step, "`ended` is not a bool");
            }
            if (d->ended(d->sm) != (want != 0)) {
                (void)snprintf(message, sizeof(message), "ended is %s, want %s", want != 0 ? "false" : "true",
                               want != 0 ? "true" : "false");
                bad |= sce_scenario_fail(d, step, message);
            }
        } else if (strcmp(key, "variables") == 0) {
            if (!sce_scenario_take(c, '{')) {
                return sce_scenario_fail(d, step, "`variables` is not an object");
            }
            if (!sce_scenario_take(c, '}')) {
                for (;;) {
                    char name[64];
                    int64_t want = 0;
                    if (!sce_scenario_string(c, name, sizeof(name)) || !sce_scenario_take(c, ':') ||
                        !sce_scenario_number(c, &want)) {
                        return sce_scenario_fail(d, step, "a variable is not a name and an integer or a bool");
                    }
                    int64_t got = 0;
                    if (!d->variable(d->sm, name, &got)) {
                        (void)snprintf(message, sizeof(message), "the machine publishes no variable `%s`", name);
                        bad |= sce_scenario_fail(d, step, message);
                    } else if (got != want) {
                        (void)snprintf(message, sizeof(message), "`%s` is %lld, want %lld", name, (long long)got,
                                       (long long)want);
                        bad |= sce_scenario_fail(d, step, message);
                    }
                    if (sce_scenario_take(c, '}')) {
                        break;
                    }
                    if (!sce_scenario_take(c, ',')) {
                        return sce_scenario_fail(d, step, "`variables` is not well formed");
                    }
                }
            }
        } else {
            (void)snprintf(message, sizeof(message), "`expect` carries `%s`, which this reader does not replay", key);
            return sce_scenario_fail(d, step, message);
        }
        if (sce_scenario_take(c, '}')) {
            return bad;
        }
        if (!sce_scenario_take(c, ',')) {
            return sce_scenario_fail(d, step, "`expect` is not well formed");
        }
    }
}

// One step: the event it sends, if any, and then what it expects of the
// machine. The members come in the file's order, so `expect` is remembered and
// read once the event has been sent.
static int sce_scenario_step(sce_scenario_cursor_t *c, const sce_scenario_driver_t *d, int step) {
    char key[64];
    char event[64] = "";
    char data[256];
    bool has_event = false;
    bool has_data = false;
    sce_scenario_cursor_t expect = {NULL};
    if (!sce_scenario_take(c, '{')) {
        return sce_scenario_fail(d, step, "a step is not an object");
    }
    if (!sce_scenario_take(c, '}')) {
        for (;;) {
            if (!sce_scenario_string(c, key, sizeof(key)) || !sce_scenario_take(c, ':')) {
                return sce_scenario_fail(d, step, "a step has a member with no name");
            }
            if (strcmp(key, "event") == 0) {
                if (!sce_scenario_string(c, event, sizeof(event))) {
                    return sce_scenario_fail(d, step, "`event` is not a string");
                }
                has_event = true;
            } else if (strcmp(key, "data") == 0) {
                sce_scenario_space(c);
                const char *from = c->at;
                if (!sce_scenario_skip_value(c) || !sce_scenario_compact(from, c->at, data, sizeof(data))) {
                    return sce_scenario_fail(d, step, "`data` is not well formed, or is longer than the buffer");
                }
                has_data = true;
            } else if (strcmp(key, "expect") == 0) {
                sce_scenario_space(c);
                expect = *c;
                if (!sce_scenario_skip_value(c)) {
                    return sce_scenario_fail(d, step, "`expect` is not well formed");
                }
            } else if (strcmp(key, "note") == 0) {
                if (!sce_scenario_skip_value(c)) {
                    return sce_scenario_fail(d, step, "`note` is not well formed");
                }
            } else {
                char message[128];
                (void)snprintf(message, sizeof(message), "a step carries `%s`, which this reader does not replay", key);
                return sce_scenario_fail(d, step, message);
            }
            if (sce_scenario_take(c, '}')) {
                break;
            }
            if (!sce_scenario_take(c, ',')) {
                return sce_scenario_fail(d, step, "a step is not well formed");
            }
        }
    }
    if (has_event && !d->raise(d->sm, event, has_data ? data : NULL)) {
        char message[128];
        (void)snprintf(message, sizeof(message), "the machine has no event `%s`", event);
        return sce_scenario_fail(d, step, message);
    }
    return expect.at == NULL ? 0 : sce_scenario_expect(&expect, d, step);
}

// Replay the scenario at `path` against `d`: the number of steps that failed,
// and the count of steps it replayed in `*replayed`, so a scenario that lost its
// steps cannot pass by having none.
static int sce_scenario_replay(const char *path, const sce_scenario_driver_t *d, int *replayed) {
    static char text[16384];
    FILE *file = fopen(path, "rb");
    if (file == NULL) {
        (void)fprintf(stderr, "%s: FAIL - cannot read %s\n", d->machine, path);
        return 1;
    }
    const size_t len = fread(text, 1, sizeof(text) - 1, file);
    (void)fclose(file);
    text[len] = '\0';

    sce_scenario_cursor_t c = {text};
    char key[64];
    int bad = 0;
    *replayed = 0;
    if (!sce_scenario_take(&c, '{')) {
        return sce_scenario_fail(d, 0, "the scenario is not an object");
    }
    for (;;) {
        if (!sce_scenario_string(&c, key, sizeof(key)) || !sce_scenario_take(&c, ':')) {
            return sce_scenario_fail(d, 0, "the scenario has a member with no name");
        }
        if (strcmp(key, "machine") == 0) {
            char machine[64];
            if (!sce_scenario_string(&c, machine, sizeof(machine)) || strcmp(machine, d->machine) != 0) {
                return sce_scenario_fail(d, 0, "the scenario is of another machine");
            }
        } else if (strcmp(key, "steps") == 0) {
            if (!sce_scenario_take(&c, '[')) {
                return sce_scenario_fail(d, 0, "`steps` is not an array");
            }
            for (int step = 1;; ++step) {
                bad |= sce_scenario_step(&c, d, step);
                (*replayed)++;
                if (sce_scenario_take(&c, ']')) {
                    break;
                }
                if (!sce_scenario_take(&c, ',')) {
                    return sce_scenario_fail(d, step, "`steps` is not well formed");
                }
            }
        } else if (!sce_scenario_skip_value(&c)) {
            return sce_scenario_fail(d, 0, "the scenario is not well formed");
        }
        if (sce_scenario_take(&c, '}')) {
            return bad;
        }
        if (!sce_scenario_take(&c, ',')) {
            return sce_scenario_fail(d, 0, "the scenario is not well formed");
        }
    }
}

#endif
