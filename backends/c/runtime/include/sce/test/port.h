/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/**
 * @file port.h
 * @brief A port of a test component (docs/SCE_TEST_PROFILE.md §2.2): a FIFO
 *        queue of received messages and the snapshot of its head
 *        (ES 201 873-4 §8.3.3)
 *
 * The C11 port of `sce/include/test/Port.h` and `sce_rust_runtime::test::Port`.
 * Its siblings hold each queue in a deque that grows; a machine without an
 * allocator cannot, so every queued message of a run lives in one pool of
 * `SCE_TEST_MAX_QUEUED_MESSAGES` slots the run owns, and a port is a list of
 * the slots it holds, in arrival order. A message that finds the pool full is
 * refused, never dropped: the run ends with `error` and says which bound it
 * met (sce/test/test_case_run.h).
 *
 * A message carries its value as the bytes of the value the generated code
 * holds, copied out of the sender's frame, beside the C name of its type: a
 * template's matcher reads it back as that type, and matching is on the value,
 * never on an encoding (ADR 0091 C2). It carries the record type the test log
 * names it by and writes it with too (sce/test/canonical.h), which a send and
 * an input give with the value (`sce_test_message_value_t`). Header-only and
 * allocation-free.
 */

#ifndef SCE_TEST_PORT_H
#define SCE_TEST_PORT_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

#include "sce/host_processor.h"
#include "sce/test/canonical.h"
#include "sce/types.h"

#ifdef __cplusplus
extern "C" {
#endif

/** An instant of the logical clock `NOW`: microseconds since the test case
 *  started (docs/SCE_TEST_PROFILE.md §3.3). */
typedef int64_t sce_test_instant_t;

/** A length of logical time, in microseconds. */
typedef int64_t sce_test_duration_t;

/* The most bytes the value of one message holds. A record of the C11 event
   schema unit is laid out statically (a text in a buffer of its bound, a list
   in an array of its count), so a build whose records are larger raises it
   (`-DSCE_TEST_MAX_VALUE_SIZE=N`); a send of a larger value is refused by name. */
#ifndef SCE_TEST_MAX_VALUE_SIZE
#define SCE_TEST_MAX_VALUE_SIZE 256u
#endif

/* How many messages wait in the queues of one run at once, over all its ports.
   A request and its answer are one message each, and the profile's examples
   hold at most two at a time; sixteen leaves room for a burst a system adapter
   delivers in one round. A send or an input that finds the pool full is refused
   by name. */
#ifndef SCE_TEST_MAX_QUEUED_MESSAGES
#define SCE_TEST_MAX_QUEUED_MESSAGES 16u
#endif

/** The slot no message is in: the end of a queue and of the pool's free list. */
#define SCE_TEST_NO_MESSAGE SIZE_MAX

/**
 * A value a message carries and the record type the test log names and writes
 * it by (docs/SCE_TEST_PROFILE.md §12.5): the C11 form of Rust's
 * `message_value` and C++'s `SCE::Test::messageValue`.
 *
 * `value` is the value as the code that holds it lends it, read back by its C
 * type; its canonical form is written by the types its document declares, which
 * its C type alone does not say, so `record_type` writes it. A generated send
 * gives the value of its template's unit with the unit's `<name>_record_type`,
 * and so does a system adapter for each input it hands the run.
 */
typedef struct sce_test_message_value_s {
    sce_host_value_t value;
    const sce_test_record_type_t *record_type;
} sce_test_message_value_t;

/** `value` as a message's value, of the record type `record_type`. */
static inline sce_test_message_value_t sce_test_message_value(sce_host_value_t value,
                                                              const sce_test_record_type_t *record_type) {
    sce_test_message_value_t made;
    made.value = value;
    made.record_type = record_type;
    return made;
}

/** `*value` lent as a request's value, as a generated test-port send lends it
 *  to the test-port processor (`sce_host_send_request_t::value`); valid for as
 *  long as `*value` and what it lends. */
static inline sce_host_value_t sce_test_message_value_lent(const sce_test_message_value_t *value) {
    return SCE_HOST_VALUE(sce_test_message_value_t, value);
}

/**
 * The message value a request carries, as `sce_test_message_value_lent` lent it;
 * for a request whose value is anything else, that value of no record type,
 * which a send refuses (`SCE_TEST_MESSAGE_UNTYPED`).
 */
static inline sce_test_message_value_t sce_test_message_value_carried(sce_host_value_t carried) {
    if (carried.type != NULL && carried.data != NULL && carried.size == sizeof(sce_test_message_value_t) &&
        strcmp(carried.type, "sce_test_message_value_t") == 0) {
        return *(const sce_test_message_value_t *)carried.data;
    }
    return sce_test_message_value(carried, NULL);
}

/**
 * A message on a port: the event name of its record, the record's value and
 * its record type.
 *
 * `type` is the C name of the value's type as its sender named it
 * (`sce_host_value_t::type`), NULL for a message that carries no value; the
 * value is its `size` bytes, held aligned for any type. `record_type` names the
 * value and writes its canonical form in the test log.
 */
typedef struct sce_test_message_s {
    char event[SCE_MAX_ID_LEN];
    const char *type;
    size_t size;
    const sce_test_record_type_t *record_type;

    union {
        max_align_t align;
        unsigned char bytes[SCE_TEST_MAX_VALUE_SIZE];
    } value;
} sce_test_message_t;

/** Whether a message holds what it was given, or which bound refused it. */
typedef enum sce_test_message_fit_e {
    SCE_TEST_MESSAGE_HELD = 0,
    /** The event name is longer than `SCE_MAX_ID_LEN` holds. */
    SCE_TEST_MESSAGE_EVENT_TOO_LONG,
    /** The value is larger than `SCE_TEST_MAX_VALUE_SIZE`. */
    SCE_TEST_MESSAGE_VALUE_TOO_LARGE,
    /** The value names no record type, so the test log could not name it or
        write it. */
    SCE_TEST_MESSAGE_UNTYPED
} sce_test_message_fit_t;

/** Whether `record_type` names a record and writes its values. */
static inline bool sce_test_record_type_complete(const sce_test_record_type_t *record_type) {
    return record_type != NULL && record_type->name != NULL && record_type->name[0] != '\0' &&
           record_type->canonical != NULL;
}

/**
 * Write the message of the record `event` whose value is `value` into `out`,
 * the value's bytes copied out of the lender's frame.
 *
 * Refuses rather than cuts: an event name or a value past its bound, or a value
 * of no record type, leaves `out` empty and says which, since a shortened name
 * could match another record's branch and a message the test log cannot write
 * would leave the log short of it.
 */
static inline sce_test_message_fit_t sce_test_message_set(sce_test_message_t *out, const char *event,
                                                          sce_test_message_value_t value) {
    memset(out, 0, sizeof(*out));
    if (strlen(event) >= sizeof(out->event)) {
        return SCE_TEST_MESSAGE_EVENT_TOO_LONG;
    }
    if (value.value.size > sizeof(out->value.bytes)) {
        return SCE_TEST_MESSAGE_VALUE_TOO_LARGE;
    }
    if (!sce_test_record_type_complete(value.record_type)) {
        return SCE_TEST_MESSAGE_UNTYPED;
    }
    memcpy(out->event, event, strlen(event) + 1u);
    out->type = value.value.type;
    out->size = value.value.size;
    out->record_type = value.record_type;
    if (value.value.size > 0u) {
        memcpy(out->value.bytes, value.value.data, value.value.size);
    }
    return SCE_TEST_MESSAGE_HELD;
}

/**
 * The value of `message`, when it is of the C type named `type`, `size` bytes
 * long; NULL when it is of another type, or carries none, which a template of
 * that type does not match.
 */
static inline const void *sce_test_message_value_as(const sce_test_message_t *message, const char *type, size_t size) {
    if (message == NULL || message->type == NULL || type == NULL || message->size != size ||
        strcmp(message->type, type) != 0) {
        return NULL;
    }
    return message->value.bytes;
}

/** The value of `message` as a `const Type *`, or NULL when it is no `Type`. */
#define SCE_TEST_MESSAGE_VALUE_AS(Type, message)                                                                       \
    ((const Type *)sce_test_message_value_as((message), #Type, sizeof(Type)))

/**
 * The slots every queued message of a run is held in: each slot is a message,
 * and `next` links a slot to the one behind it in its port's queue, or to the
 * next free slot.
 */
typedef struct sce_test_message_pool_s {
    sce_test_message_t slots[SCE_TEST_MAX_QUEUED_MESSAGES];
    size_t next[SCE_TEST_MAX_QUEUED_MESSAGES];
    size_t free_head;
} sce_test_message_pool_t;

/** Every slot of `pool` free. */
static inline void sce_test_message_pool_init(sce_test_message_pool_t *pool) {
    for (size_t slot = 0u; slot < SCE_TEST_MAX_QUEUED_MESSAGES; ++slot) {
        pool->next[slot] = slot + 1u < SCE_TEST_MAX_QUEUED_MESSAGES ? slot + 1u : SCE_TEST_NO_MESSAGE;
    }
    pool->free_head = 0u;
}

/**
 * One component port: its id, its queue and its snapshot (-4 §8.3.3).
 *
 * Only the component that owns the port removes from it, so between the
 * snapshot and the removal of a branch the head is the message the snapshot
 * saw; what others append goes to the back. The snapshot records only whether
 * the queue held a message: a message that arrives in an empty queue after the
 * snapshot is not in it.
 */
typedef struct sce_test_port_s {
    /** The port's id, a string that outlives the port (the configuration's). */
    const char *id;
    size_t head;
    size_t tail;
    size_t count;
    bool snapshot_holds;
} sce_test_port_t;

/** An empty port of the id `id`. */
static inline void sce_test_port_init(sce_test_port_t *port, const char *id) {
    port->id = id;
    port->head = SCE_TEST_NO_MESSAGE;
    port->tail = SCE_TEST_NO_MESSAGE;
    port->count = 0u;
    port->snapshot_holds = false;
}

/**
 * A message a connected port sent, or one from the SUT on a mapped port,
 * appended in arrival order (-4 §8.3.3.1), copied into a slot of `pool`.
 * False, with the queue as it was, when every slot of `pool` is taken.
 */
static inline bool sce_test_port_enqueue(sce_test_port_t *port, sce_test_message_pool_t *pool,
                                         const sce_test_message_t *message) {
    const size_t slot = pool->free_head;
    if (slot == SCE_TEST_NO_MESSAGE) {
        return false;
    }
    pool->free_head = pool->next[slot];
    pool->slots[slot] = *message;
    pool->next[slot] = SCE_TEST_NO_MESSAGE;
    if (port->tail == SCE_TEST_NO_MESSAGE) {
        port->head = slot;
    } else {
        pool->next[port->tail] = slot;
    }
    port->tail = slot;
    port->count++;
    return true;
}

/** `SNAP-VALUE`: the head of the queue, taken in a round's snapshot step
 *  (docs/SCE_TEST_PROFILE.md §3.2 step 3). */
static inline void sce_test_port_take_snapshot(sce_test_port_t *port) {
    port->snapshot_holds = port->count > 0u;
}

/** The message the last snapshot saw at the head, or NULL. */
static inline const sce_test_message_t *sce_test_port_snapshot(const sce_test_port_t *port,
                                                               const sce_test_message_pool_t *pool) {
    return port->snapshot_holds && port->head != SCE_TEST_NO_MESSAGE ? &pool->slots[port->head] : NULL;
}

/** The message at the head of the queue, whatever the snapshot saw, or NULL. */
static inline const sce_test_message_t *sce_test_port_head(const sce_test_port_t *port,
                                                           const sce_test_message_pool_t *pool) {
    return port->head != SCE_TEST_NO_MESSAGE ? &pool->slots[port->head] : NULL;
}

/**
 * `remove-from-port` of a receive branch that was taken on the snapshot
 * (-4 §9.37.1, P4 step 1): the head leaves the queue, copied into `out` where
 * `out` is not NULL, and its slot is free again. False when the queue is empty,
 * which a receive branch whose guard saw a message never finds.
 */
static inline bool sce_test_port_remove_head(sce_test_port_t *port, sce_test_message_pool_t *pool,
                                             sce_test_message_t *out) {
    const size_t slot = port->head;
    if (slot == SCE_TEST_NO_MESSAGE) {
        return false;
    }
    if (out != NULL) {
        *out = pool->slots[slot];
    }
    port->head = pool->next[slot];
    if (port->head == SCE_TEST_NO_MESSAGE) {
        port->tail = SCE_TEST_NO_MESSAGE;
    }
    port->count--;
    port->snapshot_holds = false;
    pool->next[slot] = pool->free_head;
    pool->free_head = slot;
    return true;
}

/** Every message of the queue back to `pool`: the port's component is removed. */
static inline void sce_test_port_clear(sce_test_port_t *port, sce_test_message_pool_t *pool) {
    while (sce_test_port_remove_head(port, pool, NULL)) {
    }
}

/** How many messages the queue holds. */
static inline size_t sce_test_port_size(const sce_test_port_t *port) {
    return port->count;
}

#ifdef __cplusplus
}
#endif

#endif /* SCE_TEST_PORT_H */
