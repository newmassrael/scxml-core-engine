/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * The peer core of a device that holds a shared list (ADR 0079), for the lanes that are not Rust.
 *
 * A lane holds the core of one device of one list and speaks to it in UTF-8 texts: the machine's
 * outbound record_list.change, one change whole, as the event's name and its JSON payload, the
 * relay's requests and responses as the protocol's JSON, what a round came to as one JSON text, and
 * what the device keeps between runs, its secrets with the rest, as one sealed state under a data key
 * the lane wraps with its platform's key store. An administrator's device opens a list and grants,
 * admits and takes off devices; any device reads what it stands at in the list as one JSON text.
 * The core does no I/O: the lane carries each request
 * to the relay and gives the response back, and it reads no clock and starts no thread. A lane that
 * does not carry the texts itself runs an exchange over a link the library holds (the relay's WebSocket
 * binding, or a transport the lane writes as one function), on a thread of its own, by a deadline and
 * until a cancellation. The shapes of
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

#define SCE_PEER_ABI_VERSION 8u

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
#define SCE_PEER_ERR_NOT_OPENED 14
#define SCE_PEER_ERR_NO_RANDOMNESS 15
#define SCE_PEER_ERR_GRANTED 16
#define SCE_PEER_ERR_INVITATION 17
#define SCE_PEER_ERR_NOT_JOINED 18
#define SCE_PEER_ERR_NULL_ARGUMENT 20
#define SCE_PEER_ERR_BAD_TEXT 21
#define SCE_PEER_ERR_PANIC 22
#define SCE_PEER_ERR_BAD_ARGUMENT 23
#define SCE_PEER_ERR_CANCELLED 24
#define SCE_PEER_ERR_DEADLINE 25
#define SCE_PEER_ERR_SINK_TAKEN 26

/* What a host's transport says of one exchange (ScePeerExchangeFn). RATE_LIMITED: the relay's
 * endpoint refused the connection for the rate (an HTTP 429), and the text given with sce_peer_reply
 * is the wait it asked for, in milliseconds, as decimal digits. */
#define SCE_PEER_TRANSPORT_ANSWERED 0
#define SCE_PEER_TRANSPORT_LINK 1
#define SCE_PEER_TRANSPORT_UNAUTHORIZED 2
#define SCE_PEER_TRANSPORT_DEADLINE 3
#define SCE_PEER_TRANSPORT_CANCELLED 4
#define SCE_PEER_TRANSPORT_RATE_LIMITED 5

/* What a host's pause says of the wait it was asked for (ScePeerPauseFn). */
#define SCE_PEER_PAUSE_WAITED 0
#define SCE_PEER_PAUSE_WOKEN 1

/* The levels of what the library logs (sce_peer_set_log_sink). */
#define SCE_PEER_LOG_OFF 0
#define SCE_PEER_LOG_ERROR 1
#define SCE_PEER_LOG_WARN 2
#define SCE_PEER_LOG_INFO 3
#define SCE_PEER_LOG_DEBUG 4
#define SCE_PEER_LOG_TRACE 5

/* What the core did with a send (sce_peer_outbound). */
#define SCE_PEER_KEPT_TAKEN 0
#define SCE_PEER_KEPT_HELD 1
#define SCE_PEER_KEPT_NOT_SIGNED 2
#define SCE_PEER_KEPT_REFUSED 3
#define SCE_PEER_KEPT_WAITING 4

/* The kinds of round (sce_peer_start). */
#define SCE_PEER_ROUND_SYNC 0
#define SCE_PEER_ROUND_RESYNC 1
#define SCE_PEER_ROUND_REREAD 2

/* Whether the hub of a list sce_peer_open_list opens is trusted. */
#define SCE_PEER_PROFILE_UNTRUSTED 0
#define SCE_PEER_PROFILE_TRUSTED_HUB 1

/* Which authorities a link of sce_peer_link_wss_believing believes: the ones it is given alone, or
 * those and the public ones. */
#define SCE_PEER_ROOTS_ONLY 0
#define SCE_PEER_ROOTS_AND_PUBLIC 1

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
/* The core of the device an invitation's text (len bytes of UTF-8, as the QR code holds it) invites:
 * sce_peer_invited for its list and genesis. NULL on failure. */
ScePeer *sce_peer_invited_by(const uint8_t *invitation, size_t len);
void sce_peer_free(ScePeer *peer);

/* The name of a list to open at a relay that takes a creation with no token (ADR 0080 section 3),
 * into out as UTF-8: n1.<salt>.<digest>, which certifies the administrator's key the 32-byte seed
 * makes. salt is 1 to 32 bytes of the host's; with salt_len 0 the library draws 16. The host opens
 * the core with this name, gives it the same seed as its administrator, opens the list, and its
 * login creates the list at such a relay. */
int32_t sce_peer_certified_name(const uint8_t *administrator /* 32 bytes */, const uint8_t *salt, size_t salt_len,
                                ScePeerBuf *out);

/* The device's person: the node the log names them by, and the key the 32-byte seed makes. */
int32_t sce_peer_set_identity(ScePeer *peer, uint64_t node, const uint8_t *seed /* 32 bytes */);
/* The administrator's key the device holds, from its 32-byte seed: a secret, given once and kept in the
 * sealed state. */
int32_t sce_peer_set_administrator(ScePeer *peer, const uint8_t *seed /* 32 bytes */);
/* Before the first exchange: the machine is a hub's. */
int32_t sce_peer_for_a_hub(ScePeer *peer);
/* Before the first exchange of a device with no sealed state yet: the core holds a new device of node
 * in the list's MLS group, which the sealed state keeps from then on. */
int32_t sce_peer_with_group(ScePeer *peer, uint64_t node, const uint8_t *seed /* 32 bytes */);

/* An administrator's acts. Opening a list writes the genesis and the grant of the device's person, makes
 * the group when the core holds a device of it with the entry that names its first epoch (so that the
 * list is sealed from its first write), and sets *genesis to the hash (32 bytes) a person is invited
 * with. A role is 1 to 3. On a list keyed by its group a device is admitted with sce_peer_add_member
 * (the grant and the commit are one change), from the newest key package of it a round read, and is
 * given the keys of the epochs before its own; it is taken off the list and the group at once. The
 * entries go to the relay with the next round, and the commits, Welcomes and keys given after it. */
int32_t sce_peer_open_list(ScePeer *peer, int32_t profile /* SCE_PEER_PROFILE_* */, uint8_t *genesis /* 32 bytes */);
int32_t sce_peer_grant(ScePeer *peer, uint64_t node, uint32_t role, const uint8_t *key /* 32 bytes */);
int32_t sce_peer_add_member(ScePeer *peer, uint64_t node, uint32_t role, const uint8_t *key /* 32 bytes */);
int32_t sce_peer_remove_member(ScePeer *peer, uint64_t node);

/* What the device stands at in its list, as one JSON text. */
int32_t sce_peer_standing(ScePeer *peer, ScePeerBuf *out);

/* What the machine of a copy that keeps no journal sent: *kept is SCE_PEER_KEPT_*, and *reason (may be
 * NULL) says why when refused or kept waiting. A send the device may not sign yet waits, with every
 * send after it, and the round that reads what it waits for signs it (the outcome's taken) or
 * withdraws it with every send after it (the outcome's withdrawn). */
int32_t sce_peer_outbound(ScePeer *peer, const char *event, const char *data, int32_t *kept, ScePeerBuf *reason);

/* The outbox of a copy that keeps a journal. The change the copy offered (record_list.made's payload)
 * is signed in the call: *kept is the verdict the lane gives the copy (TAKEN accepted, HELD held,
 * WAITING waiting, REFUSED refused), and *reason (may be NULL) says why for the last two. What is
 * signed is a record, given as a JSON array of {seq, event, data}, that the lane appends to its
 * journal; no round gives the relay an operation of the device from the first record not recorded
 * on, until the lane says it recorded every one up to through. A lane started again gives its
 * journal's records back, in the same form, after the sealed state: *kept (may be NULL) is how many
 * the core did not hold. */
int32_t sce_peer_made(ScePeer *peer, const char *data, int32_t *kept, ScePeerBuf *reason);
int32_t sce_peer_take_records(ScePeer *peer, ScePeerBuf *out);
int32_t sce_peer_recorded(ScePeer *peer, uint64_t through);
int32_t sce_peer_restore_records(ScePeer *peer, const char *records, uint64_t *kept);

/* What a machine built again is given, as one JSON object {roster, changes}: every change the core
 * holds in causal order with its tag (author, seq, context), from the cut after on (a JSON array of
 * [node, seq] pairs, or NULL for the whole history), then the device's own changes that wait,
 * provisional. The stable cut is such an array: what every member is known to have been told. A
 * device that has not written for a while signs a mark (*seq the number it took, 0 when none was
 * owed), which the next round gives the relay. *wall_time and *counter are the highest stamp of the
 * device's own changes the core holds (0 and 0 for none), and *read_back 1 once it read its own
 * chain back; any of the three may be NULL. */
int32_t sce_peer_history(ScePeer *peer, const char *after, ScePeerBuf *out);
int32_t sce_peer_stable_cut(ScePeer *peer, ScePeerBuf *out);

/* What the parts of changes signed before a change was one unit make together, as one JSON object
 * {restores, cut}: the state as record_list.restore payloads (JSON texts of at most parts values
 * each, the build's shared_parts; empty when no value is live), and for each device the highest
 * number among its parts ([node, seq] pairs). A machine built again is given the restores first,
 * then the history. A state sealed before a change was one unit is opened by sce_peer_restore_sealed
 * as it is and upgraded; the next save is of this core's version. */
int32_t sce_peer_fold(ScePeer *peer, uint32_t parts, ScePeerBuf *out);
int32_t sce_peer_mark(ScePeer *peer, uint64_t *seq);
int32_t sce_peer_own_stamp(ScePeer *peer, int64_t *wall_time, uint32_t *counter, int32_t *read_back);

/* An exchange with the relay: begun, then each request polled and answered (or failed) until the
 * poll gives an empty buffer, then its outcome taken once. */
int32_t sce_peer_start(ScePeer *peer, int32_t kind /* SCE_PEER_ROUND_* */);
int32_t sce_peer_publish_checkpoint(ScePeer *peer, int64_t before, uint32_t digest);
int32_t sce_peer_publish_key_package(ScePeer *peer);
int32_t sce_peer_poll_request(ScePeer *peer, ScePeerBuf *out);
int32_t sce_peer_respond(ScePeer *peer, const char *response);
int32_t sce_peer_fail(ScePeer *peer, const char *reason);
int32_t sce_peer_take_outcome(ScePeer *peer, ScePeerBuf *out);

/* A login at the relay named audience (the name the device was configured with, never the one the
 * relay says): an exchange as a round is, over a link that presents no token or one that expired,
 * whose outcome holds "token", which the lane gives the link with sce_peer_link_set_token. At a relay
 * that takes a creation with no token, the login of a list the relay does not hold creates it (a list
 * just opened under sce_peer_certified_name's name, or one that lapsed there), and the next round
 * gives the relay back what the device holds. */
int32_t sce_peer_login(ScePeer *peer, const char *audience);

/* Invitations (ADR 0080 section 4). An administrator's device invites the device of a node the log
 * never granted, at the relay of address relay and name audience, from now_ms for valid_ms (at most
 * a day): *out is the text for a QR code (the relay's address, the signed invitation and its
 * secret, drawn here, or the 32 bytes the host gives with _under). sce_peer_invitation_read gives
 * what an invitation says as one JSON text (relay, audience, list, node, genesis, expires_at_ms),
 * its secret left out. The invited device's core (sce_peer_invited_by) joins: an exchange as a
 * round is, over a link that presents no token, and *fingerprint is the digits its person compares
 * with the administrator's device's. sce_peer_joined gives the devices that joined under the
 * core's invitation of node, as a JSON array of {key, fingerprint}, empty when none; the device is
 * then admitted with sce_peer_add_member or sce_peer_grant and that key, and with no other. */
int32_t sce_peer_invite(ScePeer *peer, const char *relay, const char *audience, uint64_t node, uint64_t now_ms,
                        uint64_t valid_ms, ScePeerBuf *out);
int32_t sce_peer_invite_under(ScePeer *peer, const char *relay, const char *audience, uint64_t node, uint64_t now_ms,
                              uint64_t valid_ms, const uint8_t *secret /* 32 bytes */, ScePeerBuf *out);
int32_t sce_peer_invitation_read(const uint8_t *invitation, size_t len, ScePeerBuf *out);
int32_t sce_peer_join(ScePeer *peer, const uint8_t *invitation, size_t len, ScePeerBuf *fingerprint);
int32_t sce_peer_joined(ScePeer *peer, uint64_t node, ScePeerBuf *out);

/* What the device keeps, its secrets with the rest, as one sealed state, and taking it back into a
 * core just made for the same list and given nothing else.
 *
 * sce_peer_save_sealed seals it under a data key drawn for this save alone and sets the 32 bytes at
 * key to that key: the lane wraps it with a key its platform's key store holds and does not export,
 * keeps the wrapped key beside the state, and zeroes its own copy. sce_peer_save_sealed_under seals
 * it under a 32-byte key the lane gives (a key it drew itself, or a recording's fixed one); the same
 * key and state give the same bytes. sce_peer_restore_sealed takes the state back for the device of
 * node under the unwrapped key: SCE_PEER_ERR_NOT_OPENED when it does not open under that key for
 * this list and node, SCE_PEER_ERR_UNREADABLE when the bytes are no sealed state, and either way
 * nothing is taken. *kept (may be NULL) is how many operations and checkpoints the log verified. A
 * restored device signs nothing new until a round has read its own chain back from the relay. */
int32_t sce_peer_save_sealed(ScePeer *peer, ScePeerBuf *state, uint8_t *key /* 32 bytes */);
int32_t sce_peer_save_sealed_under(ScePeer *peer, const uint8_t *key /* 32 bytes */, ScePeerBuf *state);
int32_t sce_peer_restore_sealed(ScePeer *peer, uint64_t node, const uint8_t *state, size_t len,
                                const uint8_t *key /* 32 bytes */, uint64_t *kept);

/* A link to the relay that the library holds, and a run of an exchange over it.
 *
 * sce_peer_run runs the exchange begun on a core (sce_peer_start, a publication) to its end over a
 * link, on the calling thread (a background worker's), and returns SCE_PEER_OK when it ended, or
 * SCE_PEER_ERR_CANCELLED or SCE_PEER_ERR_DEADLINE when it stopped within_ms milliseconds from the
 * call (UINT64_MAX: no deadline) or once cancel (may be NULL) was raised; the outcome is then taken
 * with sce_peer_take_outcome. A relay's refusal of a request for the rate is waited out and the
 * request put again while the wait ends before the deadline; one that would not ends the exchange
 * with the refusal. The wait is the system's (the thread sleeps, woken by cancel), or, for a link
 * whose transport the host carries and that was given a pause (sce_peer_link_set_pause), the
 * host's, on its own clock: a host whose time is not the system's gives one.
 * The core and the link are used by that thread alone while it runs, but that any thread may raise the
 * run's cancellation and tell the link the network changed.
 *
 * A link is the relay's WebSocket binding (base "ws://host:port" or "wss://host:port", and the token
 * the device's login earned, or NULL or "" for none: a link a device logs in over, given its token
 * afterwards with sce_peer_link_set_token, which gives up a connection dialled with another and is
 * refused for a host's own transport), believing the public certificate authorities, or, made with
 * sce_peer_link_wss_believing, the authorities in the len bytes at authorities (one or more
 * certificates in PEM, or one in DER, as a file of the application holds them), alone
 * (SCE_PEER_ROOTS_ONLY) or with the public ones (SCE_PEER_ROOTS_AND_PUBLIC); or a transport the host
 * carries itself: exchange is called on the thread
 * that runs, one call at a time, with the request (len bytes of UTF-8, not NUL-terminated), the time
 * it may take (UINT64_MAX when there is no deadline) and a reply; it gives the response, or the reason
 * it has none, with sce_peer_reply, and returns SCE_PEER_TRANSPORT_*. It calls nothing else of the
 * library on that link or core. release (may be NULL) is called once with context when the link is
 * freed. A link's notices are one JSON array: what became of its connection, and what the relay
 * pushed. */
typedef struct ScePeerLink ScePeerLink;
typedef struct ScePeerCancel ScePeerCancel;
typedef struct ScePeerReply ScePeerReply;

typedef int32_t (*ScePeerExchangeFn)(void *context, const uint8_t *request, size_t len, uint64_t timeout_ms,
                                     ScePeerReply *reply);
typedef void (*ScePeerReleaseFn)(void *context);
/* A host's wait on its own clock: wait wait_ms of the host's time and return SCE_PEER_PAUSE_WAITED,
 * or return SCE_PEER_PAUSE_WOKEN earlier, at the latest once the host raises the run's cancellation.
 * Called with the transport's context, on the thread that runs, between two exchanges; it calls
 * nothing of the library on that link or core. */
typedef int32_t (*ScePeerPauseFn)(void *context, uint64_t wait_ms);

ScePeerLink *sce_peer_link_wss(const char *base, const char *token);
ScePeerLink *sce_peer_link_wss_believing(const char *base, const char *token, const uint8_t *authorities, size_t len,
                                         int32_t roots /* SCE_PEER_ROOTS_* */);
ScePeerLink *sce_peer_link_host(ScePeerExchangeFn exchange, ScePeerReleaseFn release, void *context);
void sce_peer_link_free(ScePeerLink *link);
int32_t sce_peer_link_network_changed(const ScePeerLink *link);
int32_t sce_peer_link_take_notices(ScePeerLink *link, ScePeerBuf *out);
int32_t sce_peer_link_set_token(ScePeerLink *link, const char *token);
/* The pause of a link whose transport the host carries (NULL: the system's clock, the default);
 * SCE_PEER_ERR_BAD_ARGUMENT for a link of the library's own transport. Not while a run uses it. */
int32_t sce_peer_link_set_pause(ScePeerLink *link, ScePeerPauseFn pause);
int32_t sce_peer_reply(ScePeerReply *reply, const uint8_t *text, size_t len);

ScePeerCancel *sce_peer_cancel_new(void);
int32_t sce_peer_cancel_raise(const ScePeerCancel *cancel);
void sce_peer_cancel_free(ScePeerCancel *cancel);

int32_t sce_peer_run(ScePeer *peer, ScePeerLink *link, uint64_t within_ms, const ScePeerCancel *cancel);

/* The sink of what the library logs, process-wide: sink is called with each record at max_level
 * (SCE_PEER_LOG_*) and the levels before it, the module that decided it and the record as one line of
 * key=value words (NUL-terminated UTF-8, valid for the call), and user. It may be called from any
 * thread that calls the library, and from several at once, and calls nothing of the library. A later
 * call replaces the sink, and NULL or SCE_PEER_LOG_OFF removes it; once the call returns, the sink it
 * replaced is called no more. A record names counts, codes, nodes, durations and a list by a digest;
 * never a token, a key, a signature or anything a list holds. A record of another crate the library
 * links is given at SCE_PEER_LOG_WARN and SCE_PEER_LOG_ERROR only. SCE_PEER_ERR_SINK_TAKEN when another
 * sink was installed in the process first. */
typedef void (*ScePeerLogFn)(int32_t level, const char *target, const char *message, void *user);
int32_t sce_peer_set_log_sink(int32_t max_level, ScePeerLogFn sink, void *user);

#ifdef __cplusplus
}
#endif

#endif /* SCE_PEER_H */
