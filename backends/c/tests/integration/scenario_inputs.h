// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The inputs of a scenario (docs/adr/0115, D15 and the converse of D8): the
// events the model's environment put on the external queue during the run TLC
// gave of a violation or a witness, each an event from outside (`arrive`) or a
// delayed send come due (`due`), with the count of steps that were over when
// it came. A scenario is a document of the run trace form
// (tests/forge/conformance/run_trace.schema.json) that `sce-codegen formal
// properties --scenarios` writes, and `sce_build::run_trace` is its reader of
// reference; a C test has no JSON library to borrow, so this reads the one
// member a harness needs, `inputs`, and steps over every other member whole,
// whatever its order. It refuses, rather than replay in part, an input with a
// member the form does not have or without one it has, and more inputs than it
// holds.

#ifndef SCE_C_TESTS_SCENARIO_INPUTS_H
#define SCE_C_TESTS_SCENARIO_INPUTS_H

#include <stdbool.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>

#define SCENARIO_INPUTS_MAX 64u
#define SCENARIO_EVENT_CAP 128u
// Room for a whole scenario: the turnstile's are under 8 KiB.
#define SCENARIO_TEXT_CAP 262144u
// How deep the members a reader steps over may nest: a run trace is four deep.
#define SCENARIO_DEPTH_MAX 16

typedef enum {
    SCENARIO_ARRIVE,
    SCENARIO_DUE,
} scenario_input_kind_t;

typedef struct {
    scenario_input_kind_t kind;
    // How many steps of the run were over when it came, from 1.
    size_t after;
    char event[SCENARIO_EVENT_CAP];
} scenario_input_t;

typedef struct {
    scenario_input_t inputs[SCENARIO_INPUTS_MAX];
    size_t count;
} scenario_t;

typedef struct {
    const char *at;
    const char *end;
} scenario_cursor_t;

static void scenario_space(scenario_cursor_t *c) {
    while (c->at < c->end && (*c->at == ' ' || *c->at == '\n' || *c->at == '\r' || *c->at == '\t')) {
        ++c->at;
    }
}

static bool scenario_take(scenario_cursor_t *c, char want) {
    scenario_space(c);
    if (c->at < c->end && *c->at == want) {
        ++c->at;
        return true;
    }
    return false;
}

// A string, into `out` of `cap` bytes, or stepped over when `out` is NULL. A
// name the form holds is printable, and `\"` and `\\` are the escapes one
// read into `out` may have; a string stepped over may have any.
static bool scenario_string(scenario_cursor_t *c, char *out, size_t cap) {
    if (!scenario_take(c, '"')) {
        return false;
    }
    size_t len = 0u;
    while (c->at < c->end && *c->at != '"') {
        char ch = *c->at++;
        if (ch == '\\') {
            if (c->at >= c->end) {
                return false;
            }
            ch = *c->at++;
            if (out != NULL && ch != '"' && ch != '\\') {
                return false;
            }
        }
        if (out != NULL) {
            if (len + 1u >= cap) {
                return false;
            }
            out[len] = ch;
        }
        ++len;
    }
    if (c->at >= c->end) {
        return false;
    }
    ++c->at;
    if (out != NULL) {
        out[len] = '\0';
    }
    return true;
}

// A count: digits, none signed.
static bool scenario_count(scenario_cursor_t *c, size_t *out) {
    scenario_space(c);
    const char *from = c->at;
    size_t value = 0u;
    while (c->at < c->end && *c->at >= '0' && *c->at <= '9') {
        value = value * 10u + (size_t)(*c->at - '0');
        ++c->at;
    }
    *out = value;
    return c->at != from;
}

// Any value, stepped over whole.
static bool scenario_skip(scenario_cursor_t *c, int depth) {
    scenario_space(c);
    if (c->at >= c->end || depth > SCENARIO_DEPTH_MAX) {
        return false;
    }
    const char open = *c->at;
    if (open == '"') {
        return scenario_string(c, NULL, 0u);
    }
    if (open == '{' || open == '[') {
        const char close = open == '{' ? '}' : ']';
        ++c->at;
        if (scenario_take(c, close)) {
            return true;
        }
        for (;;) {
            if (open == '{' && (!scenario_string(c, NULL, 0u) || !scenario_take(c, ':'))) {
                return false;
            }
            if (!scenario_skip(c, depth + 1)) {
                return false;
            }
            if (scenario_take(c, close)) {
                return true;
            }
            if (!scenario_take(c, ',')) {
                return false;
            }
        }
    }
    // A number, `true`, `false` or `null`.
    const char *from = c->at;
    while (c->at < c->end && ((*c->at >= '0' && *c->at <= '9') || (*c->at >= 'a' && *c->at <= 'z') || *c->at == '-' ||
                              *c->at == '+' || *c->at == '.' || *c->at == 'E')) {
        ++c->at;
    }
    return c->at != from;
}

// One input: its kind, the steps over when it came and its event, each once.
static bool scenario_input(scenario_cursor_t *c, scenario_input_t *input) {
    if (!scenario_take(c, '{')) {
        return false;
    }
    bool kind = false;
    bool after = false;
    bool event = false;
    if (scenario_take(c, '}')) {
        return false;
    }
    for (;;) {
        char key[16];
        if (!scenario_string(c, key, sizeof(key)) || !scenario_take(c, ':')) {
            return false;
        }
        if (strcmp(key, "kind") == 0 && !kind) {
            char named[16];
            if (!scenario_string(c, named, sizeof(named))) {
                return false;
            }
            if (strcmp(named, "arrive") == 0) {
                input->kind = SCENARIO_ARRIVE;
            } else if (strcmp(named, "due") == 0) {
                input->kind = SCENARIO_DUE;
            } else {
                return false;
            }
            kind = true;
        } else if (strcmp(key, "after") == 0 && !after) {
            if (!scenario_count(c, &input->after)) {
                return false;
            }
            after = true;
        } else if (strcmp(key, "event") == 0 && !event) {
            if (!scenario_string(c, input->event, sizeof(input->event))) {
                return false;
            }
            event = true;
        } else {
            return false;
        }
        if (scenario_take(c, '}')) {
            return kind && after && event;
        }
        if (!scenario_take(c, ',')) {
            return false;
        }
    }
}

// The inputs of the scenario in `text`, of `len` bytes.
static bool scenario_parse(const char *text, size_t len, scenario_t *out) {
    scenario_cursor_t c = {text, text + len};
    out->count = 0u;
    if (!scenario_take(&c, '{')) {
        return false;
    }
    if (scenario_take(&c, '}')) {
        return true;
    }
    for (;;) {
        char key[32];
        if (!scenario_string(&c, key, sizeof(key)) || !scenario_take(&c, ':')) {
            return false;
        }
        if (strcmp(key, "inputs") == 0) {
            if (!scenario_take(&c, '[')) {
                return false;
            }
            if (!scenario_take(&c, ']')) {
                for (;;) {
                    if (out->count >= SCENARIO_INPUTS_MAX || !scenario_input(&c, &out->inputs[out->count])) {
                        return false;
                    }
                    ++out->count;
                    if (scenario_take(&c, ']')) {
                        break;
                    }
                    if (!scenario_take(&c, ',')) {
                        return false;
                    }
                }
            }
        } else if (!scenario_skip(&c, 0)) {
            return false;
        }
        if (scenario_take(&c, '}')) {
            scenario_space(&c);
            return c.at == c.end;
        }
        if (!scenario_take(&c, ',')) {
            return false;
        }
    }
}

// The inputs of the scenario at `path`.
static bool scenario_read(const char *path, scenario_t *out) {
    static char text[SCENARIO_TEXT_CAP];
    FILE *in = fopen(path, "rb");
    if (in == NULL) {
        return false;
    }
    const size_t len = fread(text, 1u, sizeof(text), in);
    const int closed = fclose(in);
    if (closed != 0 || len == sizeof(text)) {
        return false;
    }
    return scenario_parse(text, len, out);
}

#endif
