/* SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */
/*
 * The C11 driver of the CBOR pin (tests/forge/cbor_pin/pin.py, docs/adr/0101 step 0).
 *
 *     capture INPUTS OUTPUT
 *
 * Compiled by scripts/capture_cbor_pin.sh against the headers the generator wrote
 * for the pinned codecs and every document they import (`sce-codegen generate
 * --with-imports -l c11`) and the forge runtime's. Each line of INPUTS,
 * `<codec>/<case> <hex>`, is decoded by that codec into a value of its type and
 * the value encoded again into a buffer of the codec's `<NAME>_MAX_BYTES`; OUTPUT
 * gets `<id> ok <hex>` with what the encode wrote, or `<id> refused <words>`
 * naming the status the decode or the encode answered. A value is allocated, not
 * a local: the envelope holds its texts and its data in arrays of their bounds,
 * some nineteen million bytes.
 */

#define _POSIX_C_SOURCE 200809L

#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "broadcast_log_conformance_text_event.h"
#include "codec_cbor_map.h"
#include "codec_cbor_open_enum.h"
#include "codec_cbor_record_list.h"
#include "codec_cbor_record_list_bytes.h"
#include "codec_cbor_record_list_least.h"
#include "codec_cbor_signed_enum.h"
#include "envelope.h"
#include "sce/forge/codec.h"

/* One decode and encode: 1 with `*out` and `*out_len` the bytes written (the
 * caller frees `*out`), 0 with `why` naming the refusal. */
typedef int (*round_trip_fn)(const uint8_t *input, size_t len, uint8_t **out, size_t *out_len, char *why,
                             size_t why_cap);

#define CBOR_PIN_ROUND_TRIP(snake, upper)                                                                              \
    static int round_trip_##snake(const uint8_t *input, size_t len, uint8_t **out, size_t *out_len, char *why,         \
                                  size_t why_cap) {                                                                    \
        snake##_t *value = calloc(1, sizeof *value);                                                                   \
        size_t cap = (size_t)(upper##_MAX_BYTES);                                                                      \
        *out = NULL;                                                                                                   \
        *out_len = 0;                                                                                                  \
        if (value == NULL) {                                                                                           \
            snprintf(why, why_cap, "no memory for the value");                                                         \
            return 0;                                                                                                  \
        }                                                                                                              \
        sce_forge_cursor_t cursor = sce_forge_cursor_init(input, len);                                                 \
        sce_forge_codec_status_t status = snake##_decode(&cursor, value);                                              \
        if (status != SCE_FORGE_CODEC_OK) {                                                                            \
            snprintf(why, why_cap, "decode status %d", (int)status);                                                   \
            free(value);                                                                                               \
            return 0;                                                                                                  \
        }                                                                                                              \
        *out = malloc(cap > 0 ? cap : 1);                                                                              \
        if (*out == NULL) {                                                                                            \
            snprintf(why, why_cap, "no memory for %zu bytes", cap);                                                    \
            free(value);                                                                                               \
            return 0;                                                                                                  \
        }                                                                                                              \
        status = snake##_encode_to_buf(value, *out, cap, out_len);                                                     \
        free(value);                                                                                                   \
        if (status != SCE_FORGE_CODEC_OK) {                                                                            \
            snprintf(why, why_cap, "encode status %d", (int)status);                                                   \
            free(*out);                                                                                                \
            *out = NULL;                                                                                               \
            return 0;                                                                                                  \
        }                                                                                                              \
        return 1;                                                                                                      \
    }

CBOR_PIN_ROUND_TRIP(codec_cbor_map, CODEC_CBOR_MAP)
CBOR_PIN_ROUND_TRIP(broadcast_log_conformance_text_event, BROADCAST_LOG_CONFORMANCE_TEXT_EVENT)
CBOR_PIN_ROUND_TRIP(codec_cbor_open_enum, CODEC_CBOR_OPEN_ENUM)
CBOR_PIN_ROUND_TRIP(codec_cbor_signed_enum, CODEC_CBOR_SIGNED_ENUM)
CBOR_PIN_ROUND_TRIP(codec_cbor_record_list, CODEC_CBOR_RECORD_LIST)
CBOR_PIN_ROUND_TRIP(codec_cbor_record_list_bytes, CODEC_CBOR_RECORD_LIST_BYTES)
CBOR_PIN_ROUND_TRIP(codec_cbor_record_list_least, CODEC_CBOR_RECORD_LIST_LEAST)
CBOR_PIN_ROUND_TRIP(envelope, ENVELOPE)

static const struct {
    const char *name;
    round_trip_fn run;
} CODECS[] = {
    {"codec_cbor_map", round_trip_codec_cbor_map},
    {"broadcast_log_conformance_text_event", round_trip_broadcast_log_conformance_text_event},
    {"codec_cbor_open_enum", round_trip_codec_cbor_open_enum},
    {"codec_cbor_signed_enum", round_trip_codec_cbor_signed_enum},
    {"codec_cbor_record_list", round_trip_codec_cbor_record_list},
    {"codec_cbor_record_list_bytes", round_trip_codec_cbor_record_list_bytes},
    {"codec_cbor_record_list_least", round_trip_codec_cbor_record_list_least},
    {"envelope", round_trip_envelope},
};

static int nibble(char c) {
    if (c >= '0' && c <= '9') {
        return c - '0';
    }
    if (c >= 'a' && c <= 'f') {
        return c - 'a' + 10;
    }
    return -1;
}

/* The case on `line`, its id ended in place, its hex decoded into a new buffer. */
static int parse_line(char *line, const char **id, uint8_t **bytes, size_t *len) {
    char *space = strchr(line, ' ');
    if (space == NULL) {
        return 0;
    }
    *space = '\0';
    *id = line;
    const char *hex = space + 1;
    size_t digits = strcspn(hex, "\r\n");
    if (digits % 2 != 0) {
        return 0;
    }
    *len = digits / 2;
    *bytes = malloc(*len > 0 ? *len : 1);
    if (*bytes == NULL) {
        return 0;
    }
    for (size_t i = 0; i < *len; i++) {
        int high = nibble(hex[2 * i]);
        int low = nibble(hex[2 * i + 1]);
        if (high < 0 || low < 0) {
            free(*bytes);
            return 0;
        }
        (*bytes)[i] = (uint8_t)(high << 4 | low);
    }
    return 1;
}

static round_trip_fn codec_of(const char *id) {
    size_t name_len = strcspn(id, "/");
    for (size_t i = 0; i < sizeof CODECS / sizeof CODECS[0]; i++) {
        if (strlen(CODECS[i].name) == name_len && strncmp(CODECS[i].name, id, name_len) == 0) {
            return CODECS[i].run;
        }
    }
    return NULL;
}

int main(int argc, char **argv) {
    if (argc != 3) {
        fprintf(stderr, "usage: capture INPUTS OUTPUT\n");
        return 2;
    }
    FILE *source = fopen(argv[1], "r");
    FILE *sink = fopen(argv[2], "w");
    if (source == NULL || sink == NULL) {
        fprintf(stderr, "capture: cannot open %s or %s\n", argv[1], argv[2]);
        return 1;
    }
    char *line = NULL;
    size_t line_cap = 0;
    int failed = 0;
    while (!failed && getline(&line, &line_cap, source) >= 0) {
        if (line[0] == '\n' || line[0] == '\0') {
            continue;
        }
        const char *id = NULL;
        uint8_t *input = NULL;
        size_t len = 0;
        round_trip_fn run = NULL;
        if (!parse_line(line, &id, &input, &len) || (run = codec_of(id)) == NULL) {
            fprintf(stderr, "capture: not a line of a pinned codec's case: %.80s\n", line);
            failed = 1;
            break;
        }
        uint8_t *out = NULL;
        size_t out_len = 0;
        char why[96];
        if (run(input, len, &out, &out_len, why, sizeof why)) {
            fprintf(sink, "%s ok ", id);
            for (size_t i = 0; i < out_len; i++) {
                fprintf(sink, "%02x", out[i]);
            }
            fputc('\n', sink);
            free(out);
        } else {
            fprintf(sink, "%s refused %s\n", id, why);
        }
        free(input);
    }
    free(line);
    fclose(source);
    if (fclose(sink) != 0) {
        failed = 1;
    }
    return failed;
}
