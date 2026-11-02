/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * Replays the recorded scenario of the peer core (backends/rust/peer-capi/tests/fixtures) through the
 * C ABI, from C, with no relay: the cores of two devices are made, given the administrator's acts, the
 * machine's sends and the rounds' answers, and one is sealed and made again from its sealed state with
 * no key given again; every text a core gives (each request, each outcome, each standing, the genesis,
 * the sealed state, the reason of a refusal) must be the recorded one.
 *
 * The scenario is read as whitespace-separated tokens (the Rust test writes them from the JSON file).
 * A text is written as x followed by its UTF-8 bytes in hex (so "x" is the empty text). First the
 * list and the number of devices, then a line per device: <node> <seed> <genesis or -> <administrator's
 * seed or ->. Then one line per step, each naming its device by its place:
 *   make <d>
 *   open <d> <untrusted|trusted_hub> <genesis>   the list opened, and the hash of its genesis in hex
 *   grant <d> <node> <role> <key>                 the key in hex
 *   remove <d> <node>
 *   standing <d> <text>
 *   save <d> <key> <state>          the state sealed under the data key, both in hex
 *   restore <d> <key> <state> <count>
 *                                   the device's core made again given nothing but the list (and the
 *                                   genesis), and the state taken back for its node under the key
 *   outbound <d> <event> <data> <taken|held|not_signed|refused> <reason>
 *   start <d> <sync|resync|reread>
 *   request <d> <text>      the core asks this
 *   respond <d> <text>      the relay answered this
 *   fail <d> <reason>       the transport failed
 *   outcome <d> <text>      the core asks nothing more, and the exchange came to this
 */

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "sce_peer.h"

#define MAX_DEVICES 8

static char *script;
static size_t at;
static int steps;

static void die(const char *what) {
    fprintf(stderr, "replay: step %d: %s (%s)\n", steps, what, sce_peer_last_error());
    exit(1);
}

static void check(int32_t status, const char *what) {
    if (status != SCE_PEER_OK) {
        die(what);
    }
}

static char *next(void) {
    while (script[at] == ' ' || script[at] == '\n' || script[at] == '\t' || script[at] == '\r') {
        at++;
    }
    if (script[at] == '\0') {
        return NULL;
    }
    char *start = script + at;
    while (script[at] != '\0' && script[at] != ' ' && script[at] != '\n' && script[at] != '\t' && script[at] != '\r') {
        at++;
    }
    if (script[at] != '\0') {
        script[at++] = '\0';
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
    uint8_t *bytes = malloc(n + 1);
    for (size_t i = 0; i < n; i++) {
        bytes[i] = (uint8_t)(nibble(hex[2 * i]) * 16 + nibble(hex[2 * i + 1]));
    }
    bytes[n] = 0;
    *length = n;
    return bytes;
}

/* A text token (x and hex) as a NUL-terminated string the caller frees. */
static char *text_of(const char *token) {
    if (token[0] != 'x') {
        die("a text is expected");
    }
    size_t n;
    return (char *)unhex(token + 1, &n);
}

/* The text of a buffer the library gave, given back; NULL for an empty buffer. */
static char *taken(ScePeerBuf buf) {
    if (buf.ptr == NULL) {
        return NULL;
    }
    char *text = malloc(buf.len + 1);
    memcpy(text, buf.ptr, buf.len);
    text[buf.len] = '\0';
    sce_peer_buf_free(buf);
    return text;
}

/* The text a core gave (NULL is the empty text) must be the recorded one; both are freed. */
static void expect_text(char *got, char *want, const char *what) {
    const char *have = got != NULL ? got : "";
    if (strcmp(have, want) != 0) {
        fprintf(stderr, "replay: step %d: %s is not the recorded one\n  got:  %s\n  want: %s\n", steps, what, have,
                want);
        exit(1);
    }
    free(got);
    free(want);
}

static int32_t kept_of(const char *name) {
    if (strcmp(name, "taken") == 0) {
        return SCE_PEER_KEPT_TAKEN;
    }
    if (strcmp(name, "held") == 0) {
        return SCE_PEER_KEPT_HELD;
    }
    if (strcmp(name, "not_signed") == 0) {
        return SCE_PEER_KEPT_NOT_SIGNED;
    }
    if (strcmp(name, "refused") == 0) {
        return SCE_PEER_KEPT_REFUSED;
    }
    die("an answer to a send the replay does not know");
    return -1;
}

static int32_t profile_of(const char *name) {
    if (strcmp(name, "untrusted") == 0) {
        return SCE_PEER_PROFILE_UNTRUSTED;
    }
    if (strcmp(name, "trusted_hub") == 0) {
        return SCE_PEER_PROFILE_TRUSTED_HUB;
    }
    die("a profile the replay does not know");
    return -1;
}

/* A copy of a token, for the caller to free. */
static char *copy_of(const char *token) {
    size_t n = strlen(token);
    char *copy = malloc(n + 1);
    memcpy(copy, token, n + 1);
    return copy;
}

/* The bytes as lowercase hex, NUL-terminated, for the caller to free. */
static char *hex_of(const uint8_t *bytes, size_t n) {
    static const char digits[] = "0123456789abcdef";
    char *text = malloc(2 * n + 1);
    for (size_t i = 0; i < n; i++) {
        text[2 * i] = digits[bytes[i] >> 4];
        text[2 * i + 1] = digits[bytes[i] & 0x0f];
    }
    text[2 * n] = '\0';
    return text;
}

/* A data key token: 32 bytes in hex, for the caller to free. */
static uint8_t *data_key_of(const char *token) {
    size_t n;
    uint8_t *key = unhex(token, &n);
    if (n != 32) {
        die("a data key is not 32 bytes");
    }
    return key;
}

static int32_t kind_of(const char *name) {
    if (strcmp(name, "sync") == 0) {
        return SCE_PEER_ROUND_SYNC;
    }
    if (strcmp(name, "resync") == 0) {
        return SCE_PEER_ROUND_RESYNC;
    }
    if (strcmp(name, "reread") == 0) {
        return SCE_PEER_ROUND_REREAD;
    }
    die("a kind of round the replay does not know");
    return -1;
}

int main(int argc, char **argv) {
    if (argc != 2) {
        fprintf(stderr, "usage: replay <script>\n");
        return 2;
    }
    if (sce_peer_abi_version() != SCE_PEER_ABI_VERSION) {
        die("the library is another ABI version");
    }
    FILE *file = fopen(argv[1], "rb");
    if (file == NULL) {
        die("the script cannot be opened");
    }
    fseek(file, 0, SEEK_END);
    long size = ftell(file);
    fseek(file, 0, SEEK_SET);
    script = calloc((size_t)size + 1, 1);
    if (fread(script, 1, (size_t)size, file) != (size_t)size) {
        die("the script cannot be read");
    }
    fclose(file);

    char *list = text_of(need());
    unsigned long devices = strtoul(need(), NULL, 10);
    if (devices == 0 || devices > MAX_DEVICES) {
        die("the number of devices is not one the replay holds");
    }
    uint64_t nodes[MAX_DEVICES];
    uint8_t *seeds[MAX_DEVICES];
    uint8_t *geneses[MAX_DEVICES];
    uint8_t *administrators[MAX_DEVICES];
    ScePeer *peers[MAX_DEVICES] = {NULL};
    for (unsigned long d = 0; d < devices; d++) {
        size_t n;
        nodes[d] = strtoull(need(), NULL, 10);
        seeds[d] = unhex(need(), &n);
        if (n != 32) {
            die("a seed is not 32 bytes");
        }
        char *genesis = need();
        geneses[d] = NULL;
        if (strcmp(genesis, "-") != 0) {
            geneses[d] = unhex(genesis, &n);
            if (n != 32) {
                die("a genesis hash is not 32 bytes");
            }
        }
        char *administrator = need();
        administrators[d] = NULL;
        if (strcmp(administrator, "-") != 0) {
            administrators[d] = unhex(administrator, &n);
            if (n != 32) {
                die("an administrator's seed is not 32 bytes");
            }
        }
    }

    char *op;
    while ((op = next()) != NULL) {
        unsigned long d = strtoul(need(), NULL, 10);
        if (d >= devices) {
            die("a step names no device");
        }
        if (strcmp(op, "make") == 0) {
            sce_peer_free(peers[d]);
            peers[d] = geneses[d] == NULL ? sce_peer_open(list) : sce_peer_invited(list, geneses[d]);
            if (peers[d] == NULL) {
                die("the core is not made");
            }
            check(sce_peer_set_identity(peers[d], nodes[d], seeds[d]), "set_identity");
            if (administrators[d] != NULL) {
                check(sce_peer_set_administrator(peers[d], administrators[d]), "set_administrator");
            }
            steps++;
            continue;
        }
        if (strcmp(op, "restore") == 0) {
            /* The device started again: a core given nothing but the list and the genesis. */
            uint8_t *key = data_key_of(need());
            size_t len;
            uint8_t *state = unhex(need(), &len);
            uint64_t want = strtoull(need(), NULL, 10);
            sce_peer_free(peers[d]);
            peers[d] = geneses[d] == NULL ? sce_peer_open(list) : sce_peer_invited(list, geneses[d]);
            if (peers[d] == NULL) {
                die("the core is not made");
            }
            uint64_t count = UINT64_MAX;
            check(sce_peer_restore_sealed(peers[d], nodes[d], state, len, key, &count), "restore_sealed");
            if (count != want) {
                die("the restore kept another number of operations than was recorded");
            }
            free(key);
            free(state);
            steps++;
            continue;
        }
        ScePeer *peer = peers[d];
        if (peer == NULL) {
            die("the device's core is not made");
        }
        ScePeerBuf out = {NULL, 0};
        if (strcmp(op, "open") == 0) {
            int32_t profile = profile_of(need());
            char *want = copy_of(need());
            uint8_t genesis[32];
            check(sce_peer_open_list(peer, profile, genesis), "open_list");
            expect_text(hex_of(genesis, sizeof genesis), want, "the genesis");
        } else if (strcmp(op, "grant") == 0) {
            uint64_t node = strtoull(need(), NULL, 10);
            uint32_t role = (uint32_t)strtoul(need(), NULL, 10);
            size_t n;
            uint8_t *key = unhex(need(), &n);
            if (n != 32) {
                die("a key is not 32 bytes");
            }
            check(sce_peer_grant(peer, node, role, key), "grant");
            free(key);
        } else if (strcmp(op, "remove") == 0) {
            check(sce_peer_remove_member(peer, strtoull(need(), NULL, 10)), "remove_member");
        } else if (strcmp(op, "standing") == 0) {
            char *want = text_of(need());
            check(sce_peer_standing(peer, &out), "standing");
            expect_text(taken(out), want, "the standing");
        } else if (strcmp(op, "outbound") == 0) {
            char *event = text_of(need());
            char *data = text_of(need());
            int32_t want = kept_of(need());
            char *reason = text_of(need());
            int32_t kept = -1;
            check(sce_peer_outbound(peer, event, data, &kept, &out), "outbound");
            if (kept != want) {
                die("the core did not keep the send as recorded");
            }
            expect_text(taken(out), reason, "the reason of the refusal");
            free(event);
            free(data);
        } else if (strcmp(op, "start") == 0) {
            check(sce_peer_start(peer, kind_of(need())), "start");
        } else if (strcmp(op, "request") == 0) {
            char *want = text_of(need());
            check(sce_peer_poll_request(peer, &out), "poll_request");
            if (out.ptr == NULL) {
                die("the core asks nothing where a request was recorded");
            }
            expect_text(taken(out), want, "the request");
        } else if (strcmp(op, "respond") == 0) {
            char *response = text_of(need());
            check(sce_peer_respond(peer, response), "respond");
            free(response);
        } else if (strcmp(op, "fail") == 0) {
            char *reason = text_of(need());
            check(sce_peer_fail(peer, reason), "fail");
            free(reason);
        } else if (strcmp(op, "outcome") == 0) {
            char *want = text_of(need());
            check(sce_peer_poll_request(peer, &out), "poll_request");
            if (out.ptr != NULL) {
                die("the core still asks where the exchange was over");
            }
            check(sce_peer_take_outcome(peer, &out), "take_outcome");
            if (out.ptr == NULL) {
                die("no exchange ended where an outcome was recorded");
            }
            expect_text(taken(out), want, "the outcome");
        } else if (strcmp(op, "save") == 0) {
            uint8_t *key = data_key_of(need());
            char *want = copy_of(need());
            check(sce_peer_save_sealed_under(peer, key, &out), "save_sealed_under");
            if (out.ptr == NULL) {
                die("the sealed state is empty");
            }
            char *got = hex_of(out.ptr, out.len);
            sce_peer_buf_free(out);
            expect_text(got, want, "the sealed state");
            free(key);
        } else {
            die("a step the replay does not know");
        }
        steps++;
    }

    for (unsigned long d = 0; d < devices; d++) {
        sce_peer_free(peers[d]);
        free(seeds[d]);
        free(geneses[d]);
        free(administrators[d]);
    }
    free(list);
    free(script);
    printf("replayed %d steps\n", steps);
    return 0;
}
