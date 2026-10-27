/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * Replays the recorded scenario of the group key agreement (backends/rust/mls/tests/fixtures) through
 * the C ABI, from C: a device is loaded as it was before it joined, and is given the Welcome and the
 * commits with the membership log as data. The keys it derives must be the recorded ones, until the
 * commit that removes it, after which it derives none.
 *
 * The scenario is read as whitespace-separated tokens (the Rust test writes them from the JSON
 * file): list NODE SEED STATE then one line per step, each ending in its log:
 *   join <welcome> <commit> <epoch> <epoch_key> <section> <section_key> LOG
 *   apply <commit> <epoch> <epoch_key> LOG
 *   removed <commit> LOG
 * where LOG is: <epoch> <n> (<node> <key>)xn <m> (<epoch> <hash>)xm.
 */

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "sce_mls.h"

static char *text;
static size_t at;

static void die(const char *what) {
    fprintf(stderr, "replay: %s (%s)\n", what, sce_mls_last_error());
    exit(1);
}

static char *next(void) {
    while (text[at] == ' ' || text[at] == '\n' || text[at] == '\t' || text[at] == '\r') {
        at++;
    }
    if (text[at] == '\0') {
        return NULL;
    }
    char *start = text + at;
    while (text[at] != '\0' && text[at] != ' ' && text[at] != '\n' && text[at] != '\t' && text[at] != '\r') {
        at++;
    }
    if (text[at] != '\0') {
        text[at++] = '\0';
    }
    return start;
}

static char *need(void) {
    char *token = next();
    if (token == NULL) {
        die("the scenario ends too soon");
    }
    return token;
}

static int nibble(char c) {
    if (c >= '0' && c <= '9') {
        return c - '0';
    }
    if (c >= 'a' && c <= 'f') {
        return c - 'a' + 10;
    }
    die("a hex digit is expected");
    return 0;
}

static uint8_t *unhex(const char *hex, size_t *length) {
    size_t n = strlen(hex) / 2;
    uint8_t *bytes = malloc(n ? n : 1);
    for (size_t i = 0; i < n; i++) {
        bytes[i] = (uint8_t)(nibble(hex[2 * i]) * 16 + nibble(hex[2 * i + 1]));
    }
    *length = n;
    return bytes;
}

static void expect_key(const uint8_t *got, const char *want_hex, const char *what) {
    size_t n;
    uint8_t *want = unhex(want_hex, &n);
    if (n != 32 || memcmp(got, want, 32) != 0) {
        fprintf(stderr, "replay: %s is not the recorded key\n", what);
        exit(1);
    }
    free(want);
}

static SceMlsLog *read_log(void) {
    SceMlsLog *log = sce_mls_log_new((uint32_t)strtoul(need(), NULL, 10));
    unsigned long members = strtoul(need(), NULL, 10);
    for (unsigned long i = 0; i < members; i++) {
        uint64_t node = strtoull(need(), NULL, 10);
        size_t n;
        uint8_t *key = unhex(need(), &n);
        if (n != 32 || sce_mls_log_add_member(log, node, key) != SCE_MLS_OK) {
            die("add_member");
        }
        free(key);
    }
    unsigned long commits = strtoul(need(), NULL, 10);
    for (unsigned long i = 0; i < commits; i++) {
        uint32_t epoch = (uint32_t)strtoul(need(), NULL, 10);
        size_t n;
        uint8_t *hash = unhex(need(), &n);
        if (n != 32 || sce_mls_log_name_commit(log, epoch, hash) != SCE_MLS_OK) {
            die("name_commit");
        }
        free(hash);
    }
    return log;
}

int main(int argc, char **argv) {
    if (argc != 2) {
        fprintf(stderr, "usage: replay <script>\n");
        return 2;
    }
    if (sce_mls_abi_version() != SCE_MLS_ABI_VERSION) {
        die("the library is another ABI version");
    }
    FILE *file = fopen(argv[1], "rb");
    if (file == NULL) {
        die("the script cannot be opened");
    }
    fseek(file, 0, SEEK_END);
    long size = ftell(file);
    fseek(file, 0, SEEK_SET);
    text = calloc((size_t)size + 1, 1);
    if (fread(text, 1, (size_t)size, file) != (size_t)size) {
        die("the script cannot be read");
    }
    fclose(file);

    char *list = strdup(need());
    uint64_t node = strtoull(need(), NULL, 10);
    size_t seed_length, state_length;
    uint8_t *seed = unhex(need(), &seed_length);
    uint8_t *state = unhex(need(), &state_length);
    SceMlsDevice *device = sce_mls_device_from_bytes(list, node, seed, state, state_length);
    if (device == NULL) {
        die("the device does not load");
    }

    int steps = 0;
    char *op;
    while ((op = next()) != NULL) {
        uint8_t key[32];
        uint64_t epoch = 0;
        size_t commit_length;
        if (strcmp(op, "join") == 0) {
            size_t welcome_length;
            uint8_t *welcome = unhex(need(), &welcome_length);
            uint8_t *commit = unhex(need(), &commit_length);
            unsigned long want_epoch = strtoul(need(), NULL, 10);
            char *want_key = need();
            char *section = need();
            char *want_section = need();
            SceMlsLog *log = read_log();
            if (sce_mls_join(device, welcome, welcome_length, commit, commit_length, log, &epoch) != SCE_MLS_OK) {
                die("join");
            }
            if (epoch != want_epoch) {
                die("the epoch joined is not the recorded one");
            }
            if (sce_mls_epoch_key(device, key) != SCE_MLS_OK) {
                die("epoch_key");
            }
            expect_key(key, want_key, "the key of the epoch joined");
            if (sce_mls_section_key(device, section, key) != SCE_MLS_OK) {
                die("section_key");
            }
            expect_key(key, want_section, "the key of the section");
            sce_mls_log_free(log);
            free(welcome);
            free(commit);
        } else if (strcmp(op, "apply") == 0) {
            uint8_t *commit = unhex(need(), &commit_length);
            unsigned long want_epoch = strtoul(need(), NULL, 10);
            char *want_key = need();
            SceMlsLog *log = read_log();
            int32_t removed = -1;
            if (sce_mls_apply_commit(device, commit, commit_length, log, &removed, &epoch) != SCE_MLS_OK) {
                die("apply_commit");
            }
            if (removed != 0 || epoch != want_epoch) {
                die("the commit is not the recorded epoch");
            }
            if (sce_mls_epoch_key(device, key) != SCE_MLS_OK) {
                die("epoch_key");
            }
            expect_key(key, want_key, "the key of the epoch applied");
            sce_mls_log_free(log);
            free(commit);
        } else if (strcmp(op, "removed") == 0) {
            uint8_t *commit = unhex(need(), &commit_length);
            SceMlsLog *log = read_log();
            int32_t removed = -1;
            if (sce_mls_apply_commit(device, commit, commit_length, log, &removed, &epoch) != SCE_MLS_OK) {
                die("apply_commit of the removal");
            }
            if (removed != 1) {
                die("the commit did not remove the device");
            }
            if (sce_mls_is_active(device) != 0) {
                die("a removed device is still active");
            }
            if (sce_mls_epoch_key(device, key) != SCE_MLS_ERR_REMOVED) {
                die("a removed device derived a key");
            }
            sce_mls_log_free(log);
            free(commit);
        } else {
            die("a step the replay does not know");
        }
        steps++;
    }

    sce_mls_device_free(device);
    printf("replayed %d steps\n", steps);
    return steps == 3 ? 0 : 1;
}
