/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * The peer core of a device that holds a shared list (ADR 0079), for the lanes that are not Rust.
 *
 * A lane holds the core of one device of one list and speaks to it in UTF-8 texts: the machine's
 * outbound record_list.write and record_list.supersede as the event's name and its JSON payload, the
 * relay's requests and responses as the protocol's JSON, what a round came to as one JSON text, and
 * what the device keeps between runs as one text. An administrator's device opens a list and grants,
 * admits and takes off devices; any device reads what it stands at in the list as one JSON text.
 * The core does no I/O: the lane carries each request
 * to the relay and gives the response back, and it reads no clock and starts no thread. The shapes of
 * the texts are in docs/SCE_SHARED_LIST_PEER_ABI.md; no structure is laid out here a second time.
 *
 * Every function that does something returns SCE_PEER_OK (0) or a status, and the text of what went
 * wrong is in sce_peer_last_error() for the thread. A buffer the library returns is given back with
 * sce_peer_buf_free(); an empty buffer (ptr NULL, len 0) is how a call says there is nothing to give.
 * A buffer's text is not NUL-terminated: it is len bytes. A handle is used by one thread at a time.
 *
 * Written by hand and kept equal to the library's exports by a test.
 */

#ifndef SCE_PEER_H
#define SCE_PEER_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define SCE_PEER_ABI_VERSION 2u

#define SCE_PEER_OK 0
#define SCE_PEER_ERR_NOTHING_ASKED 1
#define SCE_PEER_ERR_OUTCOME_NOT_TAKEN 2
#define SCE_PEER_ERR_NO_GROUP_DEVICE 3
#define SCE_PEER_ERR_GROUP 4
#define SCE_PEER_ERR_UNREADABLE 5
#define SCE_PEER_ERR_NO_ADMINISTRATOR 6
#define SCE_PEER_ERR_NO_PERSON 7
#define SCE_PEER_ERR_OPENED 8
#define SCE_PEER_ERR_LOG 9
#define SCE_PEER_ERR_NOT_A_MEMBER 10
#define SCE_PEER_ERR_ADMITTED_THROUGH_GROUP 11
#define SCE_PEER_ERR_NO_KEY_PACKAGE 12
#define SCE_PEER_ERR_DRAWN_KEYS 13
#define SCE_PEER_ERR_NULL_ARGUMENT 20
#define SCE_PEER_ERR_BAD_TEXT 21
#define SCE_PEER_ERR_PANIC 22
#define SCE_PEER_ERR_BAD_ARGUMENT 23

/* What the core did with a send (sce_peer_outbound). */
#define SCE_PEER_KEPT_TAKEN 0
#define SCE_PEER_KEPT_HELD 1
#define SCE_PEER_KEPT_NOT_SIGNED 2
#define SCE_PEER_KEPT_REFUSED 3

/* The kinds of round (sce_peer_start). */
#define SCE_PEER_ROUND_SYNC 0
#define SCE_PEER_ROUND_RESYNC 1
#define SCE_PEER_ROUND_REREAD 2

/* Whether the hub of a list sce_peer_open_list opens is trusted. */
#define SCE_PEER_PROFILE_UNTRUSTED 0
#define SCE_PEER_PROFILE_TRUSTED_HUB 1

typedef struct ScePeer ScePeer;

/* A buffer the library made; give it back with sce_peer_buf_free. */
typedef struct ScePeerBuf {
    uint8_t *ptr;
    size_t len;
} ScePeerBuf;

uint32_t sce_peer_abi_version(void);
const char *sce_peer_last_error(void);
void sce_peer_buf_free(ScePeerBuf buf);

/* The core of the device of whoever makes the list, or of a device invited with the hash of the
 * list's genesis entry (32 bytes). NULL on failure. */
ScePeer *sce_peer_open(const char *list);
ScePeer *sce_peer_invited(const char *list, const uint8_t *genesis /* 32 bytes */);
void sce_peer_free(ScePeer *peer);

/* The device's person: the node the log names them by, and the key the 32-byte seed makes. */
int32_t sce_peer_set_identity(ScePeer *peer, uint64_t node, const uint8_t *seed /* 32 bytes */);
/* The administrator's key the device holds, from its 32-byte seed: a secret, given again before a restore. */
int32_t sce_peer_set_administrator(ScePeer *peer, const uint8_t *seed /* 32 bytes */);
/* Before the first exchange: the machine is a hub's. */
int32_t sce_peer_for_a_hub(ScePeer *peer);
/* Before the first exchange: the core holds the device of node in the list's MLS group, a new one
 * when len is 0, or the one sce_peer_group_state gave. */
int32_t sce_peer_with_group(ScePeer *peer, uint64_t node, const uint8_t *seed /* 32 bytes */, const uint8_t *state,
                            size_t len);
/* What the group's device keeps: secret, for the host's store. sce_peer_save does not hold it. */
int32_t sce_peer_group_state(ScePeer *peer, ScePeerBuf *out);

/* An administrator's acts. Opening a list writes the genesis and the grant of the device's person, makes
 * the group when the core holds a device of it, and sets *genesis to the hash (32 bytes) a person is
 * invited with. A role is 1 to 3. On a list keyed by its group a device is admitted with
 * sce_peer_add_member (the grant and the commit are one change), from the newest key package of it a
 * round read, and taken off the list and the group at once. The entries go to the relay with the next
 * round, and the commits and Welcomes after it. */
int32_t sce_peer_open_list(ScePeer *peer, int32_t profile /* SCE_PEER_PROFILE_* */, uint8_t *genesis /* 32 bytes */);
int32_t sce_peer_grant(ScePeer *peer, uint64_t node, uint32_t role, const uint8_t *key /* 32 bytes */);
int32_t sce_peer_add_member(ScePeer *peer, uint64_t node, uint32_t role, const uint8_t *key /* 32 bytes */);
int32_t sce_peer_remove_member(ScePeer *peer, uint64_t node);

/* What the device stands at in its list, as one JSON text. */
int32_t sce_peer_standing(ScePeer *peer, ScePeerBuf *out);

/* What the machine sent: *kept is SCE_PEER_KEPT_*, and *reason (may be NULL) says why when refused. */
int32_t sce_peer_outbound(ScePeer *peer, const char *event, const char *data, int32_t *kept, ScePeerBuf *reason);

/* An exchange with the relay: begun, then each request polled and answered (or failed) until the
 * poll gives an empty buffer, then its outcome taken once. */
int32_t sce_peer_start(ScePeer *peer, int32_t kind /* SCE_PEER_ROUND_* */);
int32_t sce_peer_publish_checkpoint(ScePeer *peer, int64_t before, uint32_t digest);
int32_t sce_peer_publish_key_package(ScePeer *peer);
int32_t sce_peer_poll_request(ScePeer *peer, ScePeerBuf *out);
int32_t sce_peer_respond(ScePeer *peer, const char *response);
int32_t sce_peer_fail(ScePeer *peer, const char *reason);
int32_t sce_peer_take_outcome(ScePeer *peer, ScePeerBuf *out);

/* The event that says an instant is settled, as the JSON pair [name, payload]; empty when it is not. */
int32_t sce_peer_floor_event(ScePeer *peer, int64_t before, uint32_t digest, ScePeerBuf *out);

/* What the device keeps, as one text, and taking it back into a core just made for the same list.
 * *kept (may be NULL) is how many operations and checkpoints the log verified. A restored device
 * signs nothing new until a round has read its own chain back from the relay. */
int32_t sce_peer_save(ScePeer *peer, ScePeerBuf *out);
int32_t sce_peer_restore(ScePeer *peer, const char *saved, uint64_t *kept);

#ifdef __cplusplus
}
#endif

#endif /* SCE_PEER_H */
