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
// fills it with, whether the step `dropped` it, `expect.state`, `expect.ended`,
// `expect.variables` — and stops, naming it, at what it does not, rather than
// passing a scenario it did not replay.
//
// A number spelled with a fraction or an exponent (`0.0`, `0.5`, `1e21`) is a
// real, and one without is an integer, so `0` and `0.0` are two different
// expectations here as they are for every engine that reads the file. A real is
// compared as the 64 bits it is: the expected value is derived from the
// document, and `0.1 + 0.2` is not `0.3`.

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
    // NULL when the step carries none. False, with nothing raised, for a name no
    // event of the machine matches (§scxml-3.12.1) — the name the machine's own
    // `<m>_resolve_event_by_name` refuses, which the machine drops.
    bool (*raise)(void *sm, const char *event, const char *data);
    // 1 when the state is active, 0 when it is not, -1 for a name the machine
    // has no state of.
    int (*in_state)(void *sm, const char *state);
    bool (*ended)(void *sm);
    // False for a name the machine publishes no variable of. A bool reads as 0
    // or 1.
    bool (*variable)(void *sm, const char *name, int64_t *out);
    // The name a variable of an enum holds, as its document declares it; NULL
    // for a variable that is not one. A scenario states such a value as a
    // string, and a driver with no enum may leave this unset.
    const char *(*text)(void *sm, const char *name);
    // The elements of a list variable as numbers — at most `cap` of them written
    // to `out` — and how many it holds in `*len`. False for a name the machine
    // publishes no list of. A scenario states such a value as an array, and a
    // driver with no list may leave this unset.
    bool (*list)(void *sm, const char *name, int64_t *out, size_t cap, size_t *len);
    // The number a record's `field` holds: in the record variable `name`, when
    // `index` is SCE_SCENARIO_WHOLE, or in element `index` of the list of records
    // `name`. False for a name or a field the machine publishes none of, and for
    // an element the list does not hold. A scenario states a record as an object
    // and a list of records as an array of them, and a driver with no record may
    // leave this unset.
    bool (*record_field)(void *sm, const char *name, size_t index, const char *field, int64_t *out);
    // The name an enum field of a record holds, as its document declares it —
    // addressed as `record_field` is; NULL for a field that is not an enum's. A
    // scenario states such a value as a string, and a driver with no enum field
    // may leave this unset.
    const char *(*record_text)(void *sm, const char *name, size_t index, const char *field);
    // How many elements the list of records `name` holds. False for a name the
    // machine publishes no list of records of.
    bool (*record_count)(void *sm, const char *name, size_t *len);
    // The data the done event of the final the run ended at carries — the JSON
    // object of its `<donedata>` pairs, or the JSON string an inline `<content>`
    // spells. A scenario states it as the one or the other, and a driver whose
    // machine has no `<donedata>` may leave this unset.
    const char *(*done_data)(void *sm);
    // The value of a real variable, as the double it is. False for a name the
    // machine publishes no real of. A scenario states such a value with a
    // fraction or an exponent, which is what tells it from an integer, and a
    // driver with no real may leave this unset.
    bool (*real)(void *sm, const char *name, double *out);
    // The elements of a list of reals — at most `cap` of them written to `out` —
    // and how many it holds in `*len`. False for a name the machine publishes no
    // list of reals of. A driver with no list of reals may leave this unset.
    bool (*real_list)(void *sm, const char *name, double *out, size_t cap, size_t *len);
    // The real a record's `field` holds, addressed as `record_field` is. False for
    // a name or a field the machine publishes no real of. A scenario states such a
    // field with a fraction or an exponent, and a driver with no record that holds
    // a real may leave this unset.
    bool (*record_real)(void *sm, const char *name, size_t index, const char *field, double *out);
    // Move the clock the machine measures its delayed sends from forward by `ms`,
    // and run whatever that made due. A scenario states it as an `advance_ms`
    // step, and a driver whose machine waits on no clock may leave this unset: a
    // step that asks for it then fails.
    bool (*advance)(void *sm, int64_t ms);
    // How many states are active, a compound or a parallel one counted with the
    // atomic ones below it. A scenario states the set of them as `configuration`,
    // each named state active and no other, and a driver whose machine does not
    // count them may leave this unset: a step that asks for it then fails.
    size_t (*active_count)(void *sm);
    // What the machine asked of its host since this was last called, as the compact
    // JSON array a scenario's `host_calls` is: each call `{"action":"<name>",
    // "args":[…]}` — the name the document gives the action, and its arguments in
    // the order the document gives them — oldest first, and `[]` for none. Called
    // once per step, so a step is judged on the calls of that step alone. A
    // scenario states its calls with the members in that order, which is what lets
    // the two be compared as text. NULL for a machine whose host is not one that
    // records: a step that states `host_calls` then fails.
    const char *(*host_calls)(void *sm);
} sce_scenario_driver_t;

// `index` of a record that is a variable of its own, not an element of a list.
#define SCE_SCENARIO_WHOLE ((size_t)-1)

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

// The four hex digits at `p` as the number they spell, false at the first that is not one.
static bool sce_scenario_hex4(const char *p, unsigned long *code) {
    *code = 0;
    for (int i = 0; i < 4; ++i) {
        const char d = p[i];
        unsigned digit;
        if (d >= '0' && d <= '9') {
            digit = (unsigned)(d - '0');
        } else if (d >= 'a' && d <= 'f') {
            digit = (unsigned)(d - 'a') + 10u;
        } else if (d >= 'A' && d <= 'F') {
            digit = (unsigned)(d - 'A') + 10u;
        } else {
            return false;
        }
        *code = (*code << 4) | digit;
    }
    return true;
}

// The UTF-8 of the character at `code`, written to `out`: how many bytes it is.
static size_t sce_scenario_utf8(unsigned long code, char out[4]) {
    if (code < 0x80ul) {
        out[0] = (char)code;
        return 1u;
    }
    if (code < 0x800ul) {
        out[0] = (char)(0xC0ul | (code >> 6));
        out[1] = (char)(0x80ul | (code & 0x3Ful));
        return 2u;
    }
    if (code < 0x10000ul) {
        out[0] = (char)(0xE0ul | (code >> 12));
        out[1] = (char)(0x80ul | ((code >> 6) & 0x3Ful));
        out[2] = (char)(0x80ul | (code & 0x3Ful));
        return 3u;
    }
    out[0] = (char)(0xF0ul | (code >> 18));
    out[1] = (char)(0x80ul | ((code >> 12) & 0x3Ful));
    out[2] = (char)(0x80ul | ((code >> 6) & 0x3Ful));
    out[3] = (char)(0x80ul | (code & 0x3Ful));
    return 4u;
}

// A string, into `out` when there is one (`cap` bytes, terminator included).
// A `\uXXXX` escape is the UTF-8 of the character it names, a surrogate pair
// the one character the two name together: a value a scenario states is spelled
// with them, a byte above 0x7F as its character.
static bool sce_scenario_string(sce_scenario_cursor_t *c, char *out, size_t cap) {
    sce_scenario_space(c);
    if (*c->at != '"') {
        return false;
    }
    c->at++;
    size_t n = 0;
    while (*c->at != '\0' && *c->at != '"') {
        char piece[4];
        size_t pieces = 1u;
        piece[0] = *c->at++;
        if (piece[0] == '\\') {
            const char escape = *c->at++;
            switch (escape) {
            case 'n':
                piece[0] = '\n';
                break;
            case 't':
                piece[0] = '\t';
                break;
            case 'u': {
                unsigned long code = 0;
                if (!sce_scenario_hex4(c->at, &code)) {
                    return false;
                }
                c->at += 4;
                if (code >= 0xD800ul && code <= 0xDBFFul) {
                    unsigned long low = 0;
                    if (c->at[0] != '\\' || c->at[1] != 'u' || !sce_scenario_hex4(c->at + 2, &low) || low < 0xDC00ul ||
                        low > 0xDFFFul) {
                        return false;
                    }
                    c->at += 6;
                    code = 0x10000ul + ((code - 0xD800ul) << 10) + (low - 0xDC00ul);
                } else if (code >= 0xDC00ul && code <= 0xDFFFul) {
                    return false;
                }
                pieces = sce_scenario_utf8(code, piece);
                break;
            }
            case '"':
            case '\\':
            case '/':
                piece[0] = escape;
                break;
            default:
                return false;
            }
        }
        for (size_t i = 0; i < pieces; ++i) {
            if (out != NULL) {
                if (n + 1 >= cap) {
                    return false;
                }
                out[n] = piece[i];
            }
            n++;
        }
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

// Whether the number at the cursor is spelled as a real: with a fraction or an
// exponent after its digits.
static bool sce_scenario_is_real(sce_scenario_cursor_t *c) {
    sce_scenario_space(c);
    const char *p = c->at;
    if (*p == '-' || *p == '+') {
        ++p;
    }
    while (*p >= '0' && *p <= '9') {
        ++p;
    }
    return *p == '.' || *p == 'e' || *p == 'E';
}

// A real, as the double its text is the nearest to. `strtod` reads the "C"
// locale's decimal point, which this program never leaves.
static bool sce_scenario_real(sce_scenario_cursor_t *c, double *out) {
    sce_scenario_space(c);
    char *end = NULL;
    const double value = strtod(c->at, &end);
    if (end == c->at) {
        return false;
    }
    c->at = end;
    *out = value;
    return true;
}

// Whether two reals are the same 64 bits: `==` would call -0.0 and 0.0 one value
// and NaN none.
static bool sce_scenario_same_real(double got, double want) {
    _Static_assert(sizeof(double) == sizeof(uint64_t), "a double is 64 bits");
    uint64_t got_bits = 0;
    uint64_t want_bits = 0;
    memcpy(&got_bits, &got, sizeof(got_bits));
    memcpy(&want_bits, &want, sizeof(want_bits));
    return got_bits == want_bits;
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

// One value of a flat JSON object: a bool or an integer, or a string.
typedef struct {
    bool is_text;
    int64_t number;
    char text[128];
} sce_scenario_value_t;

static bool sce_scenario_read_value(sce_scenario_cursor_t *c, sce_scenario_value_t *value) {
    sce_scenario_space(c);
    if (*c->at == '"') {
        value->is_text = true;
        return sce_scenario_string(c, value->text, sizeof(value->text));
    }
    value->is_text = false;
    return sce_scenario_number(c, &value->number);
}

// The member `key` of the flat object `json` — one of bools, integers and strings,
// as a machine's `<donedata>` writes it — and how many members it has. False when
// the text is not such an object, or has no `key`.
static bool sce_scenario_find_member(const char *json, const char *key, sce_scenario_value_t *out, size_t *count) {
    sce_scenario_cursor_t c = {json};
    char name[64];
    bool found = false;
    *count = 0;
    if (!sce_scenario_take(&c, '{')) {
        return false;
    }
    if (sce_scenario_take(&c, '}')) {
        return false;
    }
    for (;;) {
        sce_scenario_value_t value;
        if (!sce_scenario_string(&c, name, sizeof(name)) || !sce_scenario_take(&c, ':') ||
            !sce_scenario_read_value(&c, &value)) {
            return false;
        }
        ++*count;
        if (strcmp(name, key) == 0) {
            *out = value;
            found = true;
        }
        if (sce_scenario_take(&c, '}')) {
            return found;
        }
        if (!sce_scenario_take(&c, ',')) {
            return false;
        }
    }
}

// What a step states of the done event's data, against what the machine's final
// wrote: one value — the string an inline `<content>` spells, or the bool, integer
// or string a `<content expr>` names — or the pairs of a `<donedata>`, each stated
// pair there and equal and nothing else.
static int sce_scenario_expect_done_data(sce_scenario_cursor_t *c, const sce_scenario_driver_t *d, int step) {
    int bad = 0;
    char key[64];
    char message[256];
    size_t stated = 0;
    const char *got = d->done_data == NULL ? NULL : d->done_data(d->sm);
    sce_scenario_space(c);
    if (*c->at != '{') {
        sce_scenario_value_t want;
        sce_scenario_value_t have;
        sce_scenario_cursor_t written = {got == NULL ? "" : got};
        if (!sce_scenario_read_value(c, &want)) {
            return sce_scenario_fail(d, step, "`donedata` is not a well formed value");
        }
        if (got == NULL) {
            return sce_scenario_fail(d, step, "the machine publishes no done data");
        }
        if (!sce_scenario_read_value(&written, &have) || have.is_text != want.is_text ||
            (want.is_text ? strcmp(have.text, want.text) != 0 : have.number != want.number)) {
            (void)snprintf(message, sizeof(message), "the done data `%s` is not the value the step states", got);
            bad |= sce_scenario_fail(d, step, message);
        }
        return bad;
    }
    if (!sce_scenario_take(c, '{')) {
        return sce_scenario_fail(d, step, "`donedata` is not an object");
    }
    if (got == NULL) {
        bad |= sce_scenario_fail(d, step, "the machine publishes no done data");
    }
    if (!sce_scenario_take(c, '}')) {
        for (;;) {
            sce_scenario_value_t want;
            sce_scenario_value_t have;
            size_t members = 0;
            if (!sce_scenario_string(c, key, sizeof(key)) || !sce_scenario_take(c, ':') ||
                !sce_scenario_read_value(c, &want)) {
                return sce_scenario_fail(d, step, "a done-data pair is not a bool, an integer or a string");
            }
            ++stated;
            if (got != NULL) {
                if (!sce_scenario_find_member(got, key, &have, &members)) {
                    (void)snprintf(message, sizeof(message), "the done data `%s` has no `%s`", got, key);
                    bad |= sce_scenario_fail(d, step, message);
                } else if (have.is_text != want.is_text ||
                           (want.is_text ? strcmp(have.text, want.text) != 0 : have.number != want.number)) {
                    (void)snprintf(message, sizeof(message), "the done data `%s` holds a different `%s`", got, key);
                    bad |= sce_scenario_fail(d, step, message);
                }
            }
            if (sce_scenario_take(c, '}')) {
                break;
            }
            if (!sce_scenario_take(c, ',')) {
                return sce_scenario_fail(d, step, "`donedata` is not well formed");
            }
        }
    }
    if (got != NULL) {
        sce_scenario_value_t ignored;
        size_t members = 0;
        (void)sce_scenario_find_member(got, "", &ignored, &members);
        if (members != stated) {
            (void)snprintf(message, sizeof(message), "the done data `%s` holds %zu pair(s), want %zu", got, members,
                           stated);
            bad |= sce_scenario_fail(d, step, message);
        }
    }
    return bad;
}

// A record stated as an object of numbers, bools and enum names, against the record variable `name`
// or element `index` of the list of records `name`. A well-formed object sets
// `*well_formed`, whatever the machine holds; the rest of the file is read only
// when it is.
static int sce_scenario_expect_record(sce_scenario_cursor_t *c, const sce_scenario_driver_t *d, int step,
                                      const char *name, size_t index, bool *well_formed) {
    int bad = 0;
    char field[64];
    char message[256];
    *well_formed = false;
    if (!sce_scenario_take(c, '{')) {
        return sce_scenario_fail(d, step, "a record is not an object");
    }
    if (sce_scenario_take(c, '}')) {
        *well_formed = true;
        return 0;
    }
    for (;;) {
        char where[160];
        char want_text[64];
        int64_t want = 0;
        if (!sce_scenario_string(c, field, sizeof(field)) || !sce_scenario_take(c, ':')) {
            return sce_scenario_fail(d, step, "a record has a field with no name");
        }
        if (index == SCE_SCENARIO_WHOLE) {
            (void)snprintf(where, sizeof(where), "`%s.%s`", name, field);
        } else {
            (void)snprintf(where, sizeof(where), "`%s`[%zu].%s", name, index, field);
        }
        sce_scenario_space(c);
        if (*c->at == '"') {
            // An enum field is stated as the name its document declares.
            if (!sce_scenario_string(c, want_text, sizeof(want_text))) {
                return sce_scenario_fail(d, step, "a record field's name is not a string");
            }
            const char *got_text = d->record_text == NULL ? NULL : d->record_text(d->sm, name, index, field);
            if (got_text == NULL) {
                (void)snprintf(message, sizeof(message), "the machine publishes no enum field %s", where);
                bad |= sce_scenario_fail(d, step, message);
            } else if (strcmp(got_text, want_text) != 0) {
                (void)snprintf(message, sizeof(message), "%s is `%s`, want `%s`", where, got_text, want_text);
                bad |= sce_scenario_fail(d, step, message);
            }
        } else if (sce_scenario_is_real(c)) {
            // A real field, compared as the 64 bits it is.
            double want_real = 0;
            double got_real = 0;
            if (!sce_scenario_real(c, &want_real)) {
                return sce_scenario_fail(d, step, "a record field's real is not a number");
            }
            if (d->record_real == NULL || !d->record_real(d->sm, name, index, field, &got_real)) {
                (void)snprintf(message, sizeof(message), "the machine publishes no real field %s", where);
                bad |= sce_scenario_fail(d, step, message);
            } else if (!sce_scenario_same_real(got_real, want_real)) {
                (void)snprintf(message, sizeof(message), "%s is %.17g, want %.17g", where, got_real, want_real);
                bad |= sce_scenario_fail(d, step, message);
            }
        } else {
            if (!sce_scenario_number(c, &want)) {
                return sce_scenario_fail(d, step, "a record field is not an integer, a bool or an enum name");
            }
            int64_t got = 0;
            if (d->record_field == NULL || !d->record_field(d->sm, name, index, field, &got)) {
                (void)snprintf(message, sizeof(message), "the machine publishes no field %s", where);
                bad |= sce_scenario_fail(d, step, message);
            } else if (got != want) {
                (void)snprintf(message, sizeof(message), "%s is %lld, want %lld", where, (long long)got,
                               (long long)want);
                bad |= sce_scenario_fail(d, step, message);
            }
        }
        if (sce_scenario_take(c, '}')) {
            *well_formed = true;
            return bad;
        }
        if (!sce_scenario_take(c, ',')) {
            return sce_scenario_fail(d, step, "a record is not well formed");
        }
    }
}

// What a step expects, read from its `expect` object — the object `c` is at.
// `asked` is what the machine told its host in this step, or NULL for a driver
// that records none.
static int sce_scenario_expect(sce_scenario_cursor_t *c, const sce_scenario_driver_t *d, int step, const char *asked) {
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
        } else if (strcmp(key, "configuration") == 0) {
            // Every active state, a compound or a parallel one with the atomic
            // ones below it, as a set: each named state is active and no other is.
            // The order the scenario lists them in is not part of the answer.
            size_t listed = 0;
            if (!sce_scenario_take(c, '[')) {
                return sce_scenario_fail(d, step, "`configuration` is not an array");
            }
            if (!sce_scenario_take(c, ']')) {
                for (;;) {
                    char state[64];
                    if (!sce_scenario_string(c, state, sizeof(state))) {
                        return sce_scenario_fail(d, step, "`configuration` holds something that is no state id");
                    }
                    ++listed;
                    const int active = d->in_state(d->sm, state);
                    if (active != 1) {
                        (void)snprintf(message, sizeof(message), "state `%s` is %s", state,
                                       active < 0 ? "not one of this machine's states" : "not active");
                        bad |= sce_scenario_fail(d, step, message);
                    }
                    if (sce_scenario_take(c, ']')) {
                        break;
                    }
                    if (!sce_scenario_take(c, ',')) {
                        return sce_scenario_fail(d, step, "`configuration` is not well formed");
                    }
                }
            }
            if (d->active_count == NULL) {
                bad |= sce_scenario_fail(d, step, "this driver counts no active states, so it cannot hold a set");
            } else if (d->active_count(d->sm) != listed) {
                (void)snprintf(message, sizeof(message), "%zu state(s) are active, `configuration` names %zu",
                               d->active_count(d->sm), listed);
                bad |= sce_scenario_fail(d, step, message);
            }
        } else if (strcmp(key, "host_calls") == 0) {
            // The calls the machine made of its host in this step, as the compact
            // text the driver writes them in.
            char want[1024];
            sce_scenario_space(c);
            const char *from = c->at;
            if (!sce_scenario_skip_value(c) || !sce_scenario_compact(from, c->at, want, sizeof(want))) {
                return sce_scenario_fail(d, step, "`host_calls` is not well formed, or is longer than the buffer");
            }
            if (asked == NULL) {
                bad |=
                    sce_scenario_fail(d, step, "this driver records no host calls, so it cannot hold a step to them");
            } else if (strcmp(asked, want) != 0) {
                (void)snprintf(message, sizeof(message), "the machine asked its host %.100s, not %.100s", asked, want);
                bad |= sce_scenario_fail(d, step, message);
            }
        } else if (strcmp(key, "donedata") == 0) {
            const int mismatch = sce_scenario_expect_done_data(c, d, step);
            if (mismatch != 0) {
                bad |= mismatch;
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
                    char want_text[64];
                    int64_t want = 0;
                    if (!sce_scenario_string(c, name, sizeof(name)) || !sce_scenario_take(c, ':')) {
                        return sce_scenario_fail(d, step, "a variable has no name");
                    }
                    sce_scenario_space(c);
                    if (*c->at == '{') {
                        // A record is stated as an object of its fields.
                        bool well_formed = false;
                        const int mismatch =
                            sce_scenario_expect_record(c, d, step, name, SCE_SCENARIO_WHOLE, &well_formed);
                        if (!well_formed) {
                            return mismatch;
                        }
                        bad |= mismatch;
                        if (sce_scenario_take(c, '}')) {
                            break;
                        }
                        if (!sce_scenario_take(c, ',')) {
                            return sce_scenario_fail(d, step, "`variables` is not well formed");
                        }
                        continue;
                    }
                    if (*c->at == '[') {
                        // A list is stated as its elements, in order.
                        enum { MAX_LIST = 16 };

                        int64_t want_list[MAX_LIST];
                        size_t want_len = 0;
                        (void)sce_scenario_take(c, '[');
                        sce_scenario_space(c);
                        if (*c->at == '{') {
                            // Records: each element an object, the count the list's own.
                            size_t count = 0;
                            for (;;) {
                                bool well_formed = false;
                                const int mismatch = sce_scenario_expect_record(c, d, step, name, count, &well_formed);
                                if (!well_formed) {
                                    return mismatch;
                                }
                                bad |= mismatch;
                                ++count;
                                if (sce_scenario_take(c, ']')) {
                                    break;
                                }
                                if (!sce_scenario_take(c, ',')) {
                                    return sce_scenario_fail(d, step, "a list is not well formed");
                                }
                            }
                            size_t held = 0;
                            if (d->record_count == NULL || !d->record_count(d->sm, name, &held)) {
                                (void)snprintf(message, sizeof(message),
                                               "the machine publishes no list of records `%s`", name);
                                bad |= sce_scenario_fail(d, step, message);
                            } else if (held != count) {
                                (void)snprintf(message, sizeof(message), "`%s` holds %zu element(s), want %zu", name,
                                               held, count);
                                bad |= sce_scenario_fail(d, step, message);
                            }
                            if (sce_scenario_take(c, '}')) {
                                break;
                            }
                            if (!sce_scenario_take(c, ',')) {
                                return sce_scenario_fail(d, step, "`variables` is not well formed");
                            }
                            continue;
                        }
                        if (sce_scenario_is_real(c)) {
                            // A list of reals: each element spelled with a fraction or an
                            // exponent, compared as the 64 bits it is.
                            double want_reals[MAX_LIST];
                            size_t want_real_len = 0;
                            for (;;) {
                                if (want_real_len == MAX_LIST || !sce_scenario_real(c, &want_reals[want_real_len])) {
                                    return sce_scenario_fail(d, step,
                                                             "a list of reals holds more than the reader keeps, or "
                                                             "an element that is not a real");
                                }
                                ++want_real_len;
                                if (sce_scenario_take(c, ']')) {
                                    break;
                                }
                                if (!sce_scenario_take(c, ',')) {
                                    return sce_scenario_fail(d, step, "a list is not well formed");
                                }
                            }
                            double got_reals[MAX_LIST];
                            size_t got_real_len = 0;
                            if (d->real_list == NULL ||
                                !d->real_list(d->sm, name, got_reals, MAX_LIST, &got_real_len)) {
                                (void)snprintf(message, sizeof(message), "the machine publishes no list of reals `%s`",
                                               name);
                                bad |= sce_scenario_fail(d, step, message);
                            } else if (got_real_len != want_real_len) {
                                (void)snprintf(message, sizeof(message), "`%s` holds %zu element(s), want %zu", name,
                                               got_real_len, want_real_len);
                                bad |= sce_scenario_fail(d, step, message);
                            } else {
                                for (size_t i = 0; i < want_real_len; ++i) {
                                    if (!sce_scenario_same_real(got_reals[i], want_reals[i])) {
                                        (void)snprintf(message, sizeof(message), "`%s`[%zu] is %.17g, want %.17g", name,
                                                       i, got_reals[i], want_reals[i]);
                                        bad |= sce_scenario_fail(d, step, message);
                                        break;
                                    }
                                }
                            }
                            if (sce_scenario_take(c, '}')) {
                                break;
                            }
                            if (!sce_scenario_take(c, ',')) {
                                return sce_scenario_fail(d, step, "`variables` is not well formed");
                            }
                            continue;
                        }
                        if (!sce_scenario_take(c, ']')) {
                            for (;;) {
                                if (want_len == MAX_LIST || !sce_scenario_number(c, &want_list[want_len])) {
                                    return sce_scenario_fail(d, step,
                                                             "a list holds more than the reader keeps, or "
                                                             "an element that is not an integer or a bool");
                                }
                                ++want_len;
                                if (sce_scenario_take(c, ']')) {
                                    break;
                                }
                                if (!sce_scenario_take(c, ',')) {
                                    return sce_scenario_fail(d, step, "a list is not well formed");
                                }
                            }
                        }
                        int64_t got_list[MAX_LIST];
                        size_t got_len = 0;
                        // An empty array is a list of either kind: a machine whose
                        // list holds records answers by its count.
                        bool found = d->list != NULL && d->list(d->sm, name, got_list, MAX_LIST, &got_len);
                        if (!found && want_len == 0 && d->record_count != NULL) {
                            found = d->record_count(d->sm, name, &got_len);
                        }
                        if (!found && want_len == 0 && d->real_list != NULL) {
                            double none[MAX_LIST];
                            found = d->real_list(d->sm, name, none, MAX_LIST, &got_len);
                        }
                        if (!found) {
                            (void)snprintf(message, sizeof(message), "the machine publishes no list `%s`", name);
                            bad |= sce_scenario_fail(d, step, message);
                        } else if (got_len != want_len) {
                            (void)snprintf(message, sizeof(message), "`%s` holds %zu element(s), want %zu", name,
                                           got_len, want_len);
                            bad |= sce_scenario_fail(d, step, message);
                        } else {
                            for (size_t i = 0; i < want_len; ++i) {
                                if (got_list[i] != want_list[i]) {
                                    (void)snprintf(message, sizeof(message), "`%s`[%zu] is %lld, want %lld", name, i,
                                                   (long long)got_list[i], (long long)want_list[i]);
                                    bad |= sce_scenario_fail(d, step, message);
                                    break;
                                }
                            }
                        }
                        if (sce_scenario_take(c, '}')) {
                            break;
                        }
                        if (!sce_scenario_take(c, ',')) {
                            return sce_scenario_fail(d, step, "`variables` is not well formed");
                        }
                        continue;
                    }
                    if (*c->at == '"') {
                        // An enum is stated as the name its document declares.
                        if (!sce_scenario_string(c, want_text, sizeof(want_text))) {
                            return sce_scenario_fail(d, step, "a variable's name is not a string");
                        }
                        const char *got_text = d->text == NULL ? NULL : d->text(d->sm, name);
                        if (got_text == NULL) {
                            (void)snprintf(message, sizeof(message), "the machine publishes no enum variable `%s`",
                                           name);
                            bad |= sce_scenario_fail(d, step, message);
                        } else if (strcmp(got_text, want_text) != 0) {
                            (void)snprintf(message, sizeof(message), "`%s` is `%s`, want `%s`", name, got_text,
                                           want_text);
                            bad |= sce_scenario_fail(d, step, message);
                        }
                        if (sce_scenario_take(c, '}')) {
                            break;
                        }
                        if (!sce_scenario_take(c, ',')) {
                            return sce_scenario_fail(d, step, "`variables` is not well formed");
                        }
                        continue;
                    }
                    if (sce_scenario_is_real(c)) {
                        double want_real = 0;
                        double got_real = 0;
                        if (!sce_scenario_real(c, &want_real)) {
                            return sce_scenario_fail(d, step, "a real variable's value is not a number");
                        }
                        if (d->real == NULL || !d->real(d->sm, name, &got_real)) {
                            (void)snprintf(message, sizeof(message), "the machine publishes no real variable `%s`",
                                           name);
                            bad |= sce_scenario_fail(d, step, message);
                        } else if (!sce_scenario_same_real(got_real, want_real)) {
                            (void)snprintf(message, sizeof(message), "`%s` is %.17g, want %.17g", name, got_real,
                                           want_real);
                            bad |= sce_scenario_fail(d, step, message);
                        }
                        if (sce_scenario_take(c, '}')) {
                            break;
                        }
                        if (!sce_scenario_take(c, ',')) {
                            return sce_scenario_fail(d, step, "`variables` is not well formed");
                        }
                        continue;
                    }
                    if (!sce_scenario_number(c, &want)) {
                        return sce_scenario_fail(d, step, "a variable is not an integer, a bool or an enum name");
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
    // The time the step moves the machine's clock on by, once its event is sent.
    int64_t advance_ms = 0;
    bool has_advance = false;
    // The step expects the machine to drop its event, so that no event of the
    // machine matches the name. Without it, a name nothing matches is a misspelt
    // step.
    bool dropped = false;
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
            } else if (strcmp(key, "dropped") == 0) {
                int64_t want = 0;
                if (!sce_scenario_number(c, &want)) {
                    return sce_scenario_fail(d, step, "`dropped` is not a bool");
                }
                dropped = want != 0;
            } else if (strcmp(key, "advance_ms") == 0) {
                if (!sce_scenario_number(c, &advance_ms) || advance_ms < 0) {
                    return sce_scenario_fail(d, step, "`advance_ms` is not a time");
                }
                has_advance = true;
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
    if (has_event && d->raise(d->sm, event, has_data ? data : NULL) == dropped) {
        char message[128];
        (void)snprintf(message, sizeof(message), "the machine's events %s `%s`", dropped ? "match" : "do not match",
                       event);
        return sce_scenario_fail(d, step, message);
    }
    if (has_advance) {
        if (d->advance == NULL) {
            return sce_scenario_fail(d, step, "the machine waits on no clock a step could move");
        }
        if (!d->advance(d->sm, advance_ms)) {
            return sce_scenario_fail(d, step, "the machine's clock could not be moved");
        }
    }
    // What the machine asked of its host in this step. Read on every step, so that
    // a step which states nothing does not hand its calls to the next.
    const char *asked = d->host_calls == NULL ? NULL : d->host_calls(d->sm);
    return expect.at == NULL ? 0 : sce_scenario_expect(&expect, d, step, asked);
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
