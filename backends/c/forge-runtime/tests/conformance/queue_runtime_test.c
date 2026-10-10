/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * The C11 arm of the `queue` kind's layer 1 (SCE Protocol-Synthesis RFC
 * §synth-5-P): every scenario in tests/forge/conformance/queue_contract.json, the
 * same ones the Rust, C++, Go, Kotlin and Python arms run, against every runtime
 * queue this backend has for the row, and the properties of those queues the
 * scenarios do not reach: wrapping over many laps, the places of each side, the
 * refusals of a constructor, and that real threads lose nothing and reorder
 * nothing between them.
 *
 * The scenarios arrive as a C source the build writes from the JSON
 * (queue_contract_scenarios.inc): C has no JSON reader within the backend's
 * zero-dependency rule, and a reader written for one test would be a second
 * reading of the contract.
 *
 * A bounded queue for one producer and one consumer is run on the Lamport ring,
 * the SCQ queue at both entry widths and the interrupt-masked ring; any other
 * cardinality on the last three, which is what the generator can select for it
 * on a target of each width.
 */

#include <pthread.h>
#include <sched.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "host_irq.h"
#include <sce/forge/queue.h>
#include <sce/forge/queue_irq.h>
#include <sce/forge/queue_segmented.h>

/* ---- The harness ---- */

static unsigned g_failures = 0;
static unsigned g_checks = 0;

#define CHECK(condition, ...)                                                                                          \
    do {                                                                                                               \
        g_checks++;                                                                                                    \
        if (!(condition)) {                                                                                            \
            g_failures++;                                                                                              \
            (void)fprintf(stderr, "FAIL %s:%d: ", __FILE__, __LINE__);                                                 \
            (void)fprintf(stderr, __VA_ARGS__);                                                                        \
            (void)fputc('\n', stderr);                                                                                 \
        }                                                                                                              \
    } while (0)

/* ---- The element ---- */

/* More than one word, so a copy that used the wrong size is seen. */
typedef struct tracked {
    uint64_t value;
    uint32_t tag;
    uint32_t pad;
} tracked_t;

#define TAG_OF(value) ((uint32_t)((value) * 2654435761u))

static tracked_t make_tracked(uint64_t value) {
    tracked_t t;
    t.value = value;
    t.tag = TAG_OF(value);
    t.pad = 0;
    return t;
}

/* ---- One interface over every queue ---- */

typedef struct subject {
    const char *name;
    void *queue;
    unsigned char *slots;
    uint32_t capacity;
    sce_queue_push_status_t (*push)(void *queue, unsigned char *slots, const void *value);
    int (*pop)(void *queue, unsigned char *slots, void *out);
    int (*producer_acquire)(void *queue);
    void (*producer_release)(void *queue);
    int (*consumer_acquire)(void *queue);
    void (*consumer_release)(void *queue);
} subject_t;

static uint32_t ceil_pow2(uint32_t n) {
    uint32_t size = 1;
    while (size < n) {
        size *= 2u;
    }
    return size;
}

/* -- Lamport ring -- */
static sce_queue_push_status_t spsc_push(void *q, unsigned char *s, const void *v) {
    return sce_queue_spsc_try_push((sce_queue_spsc_t *)q, s, sizeof(tracked_t), v);
}

static int spsc_pop(void *q, unsigned char *s, void *o) {
    return sce_queue_spsc_try_pop((sce_queue_spsc_t *)q, s, sizeof(tracked_t), o);
}

static int spsc_pa(void *q) {
    return sce_queue_spsc_producer_acquire((sce_queue_spsc_t *)q);
}

static void spsc_pr(void *q) {
    sce_queue_spsc_producer_release((sce_queue_spsc_t *)q);
}

static int spsc_ca(void *q) {
    return sce_queue_spsc_consumer_acquire((sce_queue_spsc_t *)q);
}

static void spsc_cr(void *q) {
    sce_queue_spsc_consumer_release((sce_queue_spsc_t *)q);
}

/* -- SCQ, 64-bit entries -- */
static sce_queue_push_status_t scq64_push(void *q, unsigned char *s, const void *v) {
    return sce_queue_scq64_try_push((sce_queue_scq64_t *)q, s, sizeof(tracked_t), v);
}

static int scq64_pop(void *q, unsigned char *s, void *o) {
    return sce_queue_scq64_try_pop((sce_queue_scq64_t *)q, s, sizeof(tracked_t), o);
}

static int scq64_pa(void *q) {
    return sce_queue_scq64_producer_acquire((sce_queue_scq64_t *)q);
}

static void scq64_pr(void *q) {
    sce_queue_scq64_producer_release((sce_queue_scq64_t *)q);
}

static int scq64_ca(void *q) {
    return sce_queue_scq64_consumer_acquire((sce_queue_scq64_t *)q);
}

static void scq64_cr(void *q) {
    sce_queue_scq64_consumer_release((sce_queue_scq64_t *)q);
}

/* -- SCQ, 32-bit entries -- */
static sce_queue_push_status_t scq32_push(void *q, unsigned char *s, const void *v) {
    return sce_queue_scq32_try_push((sce_queue_scq32_t *)q, s, sizeof(tracked_t), v);
}

static int scq32_pop(void *q, unsigned char *s, void *o) {
    return sce_queue_scq32_try_pop((sce_queue_scq32_t *)q, s, sizeof(tracked_t), o);
}

static int scq32_pa(void *q) {
    return sce_queue_scq32_producer_acquire((sce_queue_scq32_t *)q);
}

static void scq32_pr(void *q) {
    sce_queue_scq32_producer_release((sce_queue_scq32_t *)q);
}

static int scq32_ca(void *q) {
    return sce_queue_scq32_consumer_acquire((sce_queue_scq32_t *)q);
}

static void scq32_cr(void *q) {
    sce_queue_scq32_consumer_release((sce_queue_scq32_t *)q);
}

/* -- Interrupt-masked ring -- */
static sce_queue_push_status_t irq_push(void *q, unsigned char *s, const void *v) {
    return sce_queue_irq_try_push((sce_queue_irq_t *)q, s, sizeof(tracked_t), v);
}

static int irq_pop(void *q, unsigned char *s, void *o) {
    return sce_queue_irq_try_pop((sce_queue_irq_t *)q, s, sizeof(tracked_t), o);
}

static int irq_pa(void *q) {
    return sce_queue_irq_producer_acquire((sce_queue_irq_t *)q);
}

static void irq_pr(void *q) {
    sce_queue_irq_producer_release((sce_queue_irq_t *)q);
}

static int irq_ca(void *q) {
    return sce_queue_irq_consumer_acquire((sce_queue_irq_t *)q);
}

static void irq_cr(void *q) {
    sce_queue_irq_consumer_release((sce_queue_irq_t *)q);
}

typedef enum kind { KIND_SPSC, KIND_SCQ64, KIND_SCQ32, KIND_IRQ } kind_t;

/* Everything one subject owns, so it can be freed. */
typedef struct owned {
    subject_t subject;
    void *allocated_entries;
    void *free_entries;
} owned_t;

static const char *kind_name(kind_t kind) {
    switch (kind) {
    case KIND_SPSC:
        return "lamport";
    case KIND_SCQ64:
        return "scq64";
    case KIND_SCQ32:
        return "scq32";
    case KIND_IRQ:
        return "irq";
    }
    return "?";
}

/* Builds a subject of `kind`. `places` is how many producers and consumers may
 * be held at once on a many side; ring_slots is the SCQ ring size (zero: the
 * smallest that holds both numbers). Returns zero when the queue refuses the
 * shape. */
static int subject_build(owned_t *o, kind_t kind, uint32_t capacity, uint32_t places, uint32_t ring_slots) {
    memset(o, 0, sizeof *o);
    o->subject.name = kind_name(kind);
    o->subject.capacity = capacity;
    o->subject.slots = (unsigned char *)calloc(capacity, sizeof(tracked_t));
    if (ring_slots == 0u) {
        ring_slots = ceil_pow2(capacity > places ? capacity : places);
    }
    switch (kind) {
    case KIND_SPSC: {
        sce_queue_spsc_t *q = (sce_queue_spsc_t *)calloc(1, sizeof *q);
        o->subject.queue = q;
        o->subject.push = spsc_push;
        o->subject.pop = spsc_pop;
        o->subject.producer_acquire = spsc_pa;
        o->subject.producer_release = spsc_pr;
        o->subject.consumer_acquire = spsc_ca;
        o->subject.consumer_release = spsc_cr;
        return sce_queue_spsc_init(q, capacity);
    }
    case KIND_SCQ64: {
        sce_queue_scq64_t *q = (sce_queue_scq64_t *)calloc(1, sizeof *q);
        o->allocated_entries = calloc(2u * (size_t)ring_slots, sizeof(uint64_t));
        o->free_entries = calloc(2u * (size_t)ring_slots, sizeof(uint64_t));
        o->subject.queue = q;
        o->subject.push = scq64_push;
        o->subject.pop = scq64_pop;
        o->subject.producer_acquire = scq64_pa;
        o->subject.producer_release = scq64_pr;
        o->subject.consumer_acquire = scq64_ca;
        o->subject.consumer_release = scq64_cr;
        return sce_queue_scq64_init(q, capacity, ring_slots, (uint64_t *)o->allocated_entries,
                                    (uint64_t *)o->free_entries);
    }
    case KIND_SCQ32: {
        sce_queue_scq32_t *q = (sce_queue_scq32_t *)calloc(1, sizeof *q);
        o->allocated_entries = calloc(2u * (size_t)ring_slots, sizeof(uint32_t));
        o->free_entries = calloc(2u * (size_t)ring_slots, sizeof(uint32_t));
        o->subject.queue = q;
        o->subject.push = scq32_push;
        o->subject.pop = scq32_pop;
        o->subject.producer_acquire = scq32_pa;
        o->subject.producer_release = scq32_pr;
        o->subject.consumer_acquire = scq32_ca;
        o->subject.consumer_release = scq32_cr;
        return sce_queue_scq32_init(q, capacity, ring_slots, (uint32_t *)o->allocated_entries,
                                    (uint32_t *)o->free_entries);
    }
    case KIND_IRQ: {
        sce_queue_irq_t *q = (sce_queue_irq_t *)calloc(1, sizeof *q);
        o->subject.queue = q;
        o->subject.push = irq_push;
        o->subject.pop = irq_pop;
        o->subject.producer_acquire = irq_pa;
        o->subject.producer_release = irq_pr;
        o->subject.consumer_acquire = irq_ca;
        o->subject.consumer_release = irq_cr;
        return sce_queue_irq_init(q, capacity, places, places);
    }
    }
    return 0;
}

static void subject_free(owned_t *o) {
    free(o->subject.slots);
    free(o->subject.queue);
    free(o->allocated_entries);
    free(o->free_entries);
    memset(o, 0, sizeof *o);
}

/* ---- The contract scenarios ---- */

typedef enum op { OP_CAPACITY, OP_PUSH, OP_POP, OP_DESTROY } op_t;

typedef struct step {
    op_t op;
    int64_t value;  /* push: the value */
    int64_t expect; /* capacity: the number; push: 1 ok, 0 full, 2 out of memory; pop: the value,
                       or -1 empty */
} step_t;

typedef struct scenario {
    const char *id;
    int producers_one;
    int consumers_one;
    int intrusive;
    int segmented;
    uint32_t capacity; /* an intrusive scenario: the nodes the caller owns, the stub besides;
                          a segmented one: the elements a segment holds */
    size_t segments;   /* a segmented scenario: how many segments the allocator gives in all
                          (the first included), or 0 when it never refuses */
    const step_t *steps;
    unsigned step_count;
} scenario_t;

#include "queue_contract_scenarios.inc"

static void run_scenario(subject_t *s, const scenario_t *scenario) {
    unsigned i;
    for (i = 0; i < scenario->step_count; i++) {
        const step_t *step = &scenario->steps[i];
        switch (step->op) {
        case OP_CAPACITY:
            CHECK((int64_t)s->capacity == step->expect, "%s/%s step %u: capacity %u, want %lld", s->name, scenario->id,
                  i, (unsigned)s->capacity, (long long)step->expect);
            break;
        case OP_PUSH: {
            const tracked_t element = make_tracked((uint64_t)step->value);
            const sce_queue_push_status_t got = s->push(s->queue, s->slots, &element);
            const sce_queue_push_status_t want = step->expect == 1 ? SCE_QUEUE_PUSH_OK : SCE_QUEUE_PUSH_FULL;
            CHECK(got == want, "%s/%s step %u: push gave %d, want %d", s->name, scenario->id, i, (int)got, (int)want);
            break;
        }
        case OP_POP: {
            tracked_t got;
            const int popped = s->pop(s->queue, s->slots, &got);
            if (step->expect < 0) {
                CHECK(!popped, "%s/%s step %u: expected the queue to be empty", s->name, scenario->id, i);
            } else {
                CHECK(popped && got.value == (uint64_t)step->expect && got.tag == TAG_OF(got.value),
                      "%s/%s step %u: popped (%d, %llu), want %lld", s->name, scenario->id, i, popped,
                      (unsigned long long)got.value, (long long)step->expect);
            }
            break;
        }
        case OP_DESTROY:
            /* Destruction of held elements is a step for languages with
             * destructors. A C queue holds plain values in the caller's
             * storage; there is nothing to run. */
            return;
        }
    }
}

static void every_contract_scenario_holds(void) {
    size_t i;
    CHECK(g_scenario_count > 0u, "a contract with no scenarios checks nothing");
    for (i = 0; i < g_scenario_count; i++) {
        const scenario_t *scenario = &g_scenarios[i];
        kind_t kinds[4];
        unsigned kind_count = 0;
        unsigned k;
        if (scenario->intrusive) {
            continue; /* every_intrusive_scenario_holds */
        }
        if (scenario->segmented) {
            continue; /* every_segmented_scenario_holds */
        }
        if (scenario->producers_one && scenario->consumers_one) {
            kinds[kind_count++] = KIND_SPSC;
        }
        kinds[kind_count++] = KIND_SCQ64;
        kinds[kind_count++] = KIND_SCQ32;
        kinds[kind_count++] = KIND_IRQ;
        for (k = 0; k < kind_count; k++) {
            owned_t o;
            /* One place a side is all a scenario holds at a time; a `many` side
             * is given the capacity so the row is not the reason it fails. */
            const uint32_t places = (scenario->producers_one && scenario->consumers_one) ? 1u : scenario->capacity;
            CHECK(subject_build(&o, kinds[k], scenario->capacity, places, 0), "%s/%s: the shape is refused",
                  kind_name(kinds[k]), scenario->id);
            run_scenario(&o.subject, scenario);
            subject_free(&o);
        }
    }
}

/* ---- The intrusive list ---- */

/* A node: more than the link, so a queue that wrote past it or moved the
 * payload is seen. */
typedef struct node {
    uint64_t payload;
    uint32_t link;
    uint32_t tag;
} node_t;

#define NODE_PAYLOAD(index) ((uint64_t)(index) * 40503u + 7u)

typedef enum list_kind { LIST_ATOMIC, LIST_IRQ } list_kind_t;

static const char *list_name(list_kind_t kind) {
    return kind == LIST_ATOMIC ? "intrusive" : "intrusive_irq";
}

/* The list over `nodes` nodes and one more for the stub, `many_consumers` as the
 * scenario states. Returns zero when the shape is refused. */
typedef struct list {
    list_kind_t kind;
    sce_queue_intrusive_t atomic;
    sce_queue_intrusive_irq_t irq;
} list_t;

static int list_build(list_t *l, list_kind_t kind, node_t *nodes, uint32_t len, int many_consumers) {
    l->kind = kind;
    if (kind == LIST_ATOMIC) {
        return sce_queue_intrusive_init(&l->atomic, nodes, len, sizeof(node_t), offsetof(node_t, link), len - 1u,
                                        many_consumers);
    }
    return sce_queue_intrusive_irq_init(&l->irq, nodes, len, sizeof(node_t), offsetof(node_t, link), len - 1u,
                                        many_consumers);
}

static void list_push(list_t *l, uint32_t index) {
    if (l->kind == LIST_ATOMIC) {
        sce_queue_intrusive_push(&l->atomic, index);
    } else {
        sce_queue_intrusive_irq_push(&l->irq, index);
    }
}

static int list_pop(list_t *l, uint32_t *index) {
    return l->kind == LIST_ATOMIC ? sce_queue_intrusive_try_pop(&l->atomic, index)
                                  : sce_queue_intrusive_irq_try_pop(&l->irq, index);
}

static void run_intrusive_scenario(list_t *l, const scenario_t *scenario) {
    unsigned i;
    for (i = 0; i < scenario->step_count; i++) {
        const step_t *step = &scenario->steps[i];
        switch (step->op) {
        case OP_PUSH:
            list_push(l, (uint32_t)step->value);
            break;
        case OP_POP: {
            uint32_t index = 0;
            const int popped = list_pop(l, &index);
            if (step->expect < 0) {
                CHECK(!popped, "%s/%s step %u: expected the queue to be empty, popped node %u", list_name(l->kind),
                      scenario->id, i, (unsigned)index);
            } else {
                CHECK(popped && index == (uint32_t)step->expect, "%s/%s step %u: popped (%d, node %u), want node %lld",
                      list_name(l->kind), scenario->id, i, popped, (unsigned)index, (long long)step->expect);
            }
            break;
        }
        case OP_CAPACITY: /* an intrusive queue has no capacity */
        case OP_DESTROY:
            return;
        }
    }
}

static void every_intrusive_scenario_holds(void) {
    size_t i;
    unsigned ran = 0;
    for (i = 0; i < g_scenario_count; i++) {
        const scenario_t *scenario = &g_scenarios[i];
        list_kind_t kind;
        if (!scenario->intrusive) {
            continue;
        }
        for (kind = LIST_ATOMIC; kind <= LIST_IRQ; kind++) {
            node_t nodes[16];
            list_t l;
            uint32_t n;
            const uint32_t len = scenario->capacity + 1u; /* the nodes, and the stub */
            CHECK(len <= 16u, "%s: more nodes than the test array", scenario->id);
            for (n = 0; n < len; n++) {
                nodes[n].payload = NODE_PAYLOAD(n);
                nodes[n].link = 0xDEADBEEFu;
                nodes[n].tag = 0xA5A5A5A5u;
            }
            CHECK(list_build(&l, kind, nodes, len, !scenario->consumers_one), "%s/%s: the shape is refused",
                  list_name(kind), scenario->id);
            run_intrusive_scenario(&l, scenario);
            for (n = 0; n < len; n++) {
                CHECK(nodes[n].payload == NODE_PAYLOAD(n) && nodes[n].tag == 0xA5A5A5A5u,
                      "%s/%s: the queue wrote outside the link of node %u", list_name(kind), scenario->id, (unsigned)n);
            }
            ran++;
        }
    }
    CHECK(ran > 0u, "a contract with no intrusive scenario checks nothing of the intrusive list");
}

static void an_intrusive_queue_refuses_a_shape_it_cannot_run(void) {
    node_t nodes[4];
    sce_queue_intrusive_t q;
    sce_queue_intrusive_irq_t irq;
    CHECK(!sce_queue_intrusive_init(&q, nodes, 4, sizeof(node_t), offsetof(node_t, link), 4, 0),
          "a stub outside the array is refused");
    CHECK(!sce_queue_intrusive_init(&q, nodes, SCE_QUEUE_NIL, sizeof(node_t), offsetof(node_t, link), 0, 0),
          "a length that reaches the value for none is refused");
    CHECK(!sce_queue_intrusive_init(&q, nodes, 4, sizeof(node_t), sizeof(node_t) - 2u, 3, 0),
          "a link that does not fit its node is refused");
    CHECK(!sce_queue_intrusive_init(&q, nodes, 4, sizeof(node_t), offsetof(node_t, link) + 1u, 3, 0),
          "a link not aligned for a 32-bit word is refused");
    CHECK(!sce_queue_intrusive_irq_init(&irq, nodes, 4, sizeof(node_t), offsetof(node_t, link), 4, 0),
          "the critical-section list refuses a stub outside the array too");
}

/* ---- The segmented queues ---- */

/* The system allocator behind the contract a segmented queue is injected with,
 * giving at most `limit` blocks in all (the contract's `segments`, 0 meaning never
 * refuse) and counting the blocks it has out, so a scenario sees a refusal and a
 * test sees a leak. The counts are atomics: the queue calls the allocator from
 * several threads. */
typedef struct test_allocator {
    size_t limit;
    size_t given;
    size_t live;
} test_allocator_t;

static void *test_allocate(void *context, size_t size, size_t align) {
    test_allocator_t *a = (test_allocator_t *)context;
    size_t seen = sce_atomic_load_acquire_usize(&a->given);
    void *block;
    for (;;) {
        size_t old;
        if (a->limit != 0u && seen >= a->limit) {
            return NULL;
        }
        old = sce_atomic_cas_strong_acq_rel_usize(&a->given, seen, seen + 1u);
        if (old == seen) {
            break;
        }
        seen = old;
    }
    block = aligned_alloc(align, sce_queue_align_up(size, align));
    if (block != NULL) {
        (void)sce_atomic_fetch_add_acq_rel_usize(&a->live, 1u);
    }
    return block;
}

static void test_deallocate(void *context, void *block, size_t size, size_t align) {
    test_allocator_t *a = (test_allocator_t *)context;
    (void)size;
    (void)align;
    free(block);
    (void)sce_atomic_fetch_sub_acq_rel_usize(&a->live, 1u);
}

static sce_queue_allocator_t test_allocator_of(test_allocator_t *counting, sce_queue_progress_t progress) {
    sce_queue_allocator_t allocator;
    allocator.context = counting;
    allocator.allocate = test_allocate;
    allocator.deallocate = test_deallocate;
    allocator.progress = progress;
    return allocator;
}

static size_t live_blocks(const test_allocator_t *a) {
    return sce_atomic_load_acquire_usize(&a->live);
}

/* One interface over the two segmented queues, so one reading of the steps serves
 * both. */
typedef struct segmented_subject {
    void *queue;
    uint32_t producer_hazard;
    uint32_t consumer_hazard;
    sce_queue_push_status_t (*push)(struct segmented_subject *s, const tracked_t *element);
    int (*pop)(struct segmented_subject *s, tracked_t *out);
} segmented_subject_t;

static sce_queue_push_status_t linked_subject_push(segmented_subject_t *s, const tracked_t *element) {
    return sce_queue_linked_try_push((sce_queue_linked_t *)s->queue, element);
}

static int linked_subject_pop(segmented_subject_t *s, tracked_t *out) {
    return sce_queue_linked_try_pop((sce_queue_linked_t *)s->queue, out);
}

static sce_queue_push_status_t lscq_subject_push(segmented_subject_t *s, const tracked_t *element) {
    return sce_queue_lscq_try_push((sce_queue_lscq_t *)s->queue, s->producer_hazard, element);
}

static int lscq_subject_pop(segmented_subject_t *s, tracked_t *out) {
    return sce_queue_lscq_try_pop((sce_queue_lscq_t *)s->queue, s->consumer_hazard, out);
}

/* The steps of a segmented scenario; `name` says which queue runs them. A
 * `destroy` step ends the run, and the caller destroys its queue and checks that
 * every segment went back. */
static void run_segmented_steps(segmented_subject_t *s, const char *name, const scenario_t *scenario) {
    unsigned i;
    for (i = 0; i < scenario->step_count; i++) {
        const step_t *step = &scenario->steps[i];
        switch (step->op) {
        case OP_PUSH: {
            const tracked_t element = make_tracked((uint64_t)step->value);
            const sce_queue_push_status_t got = s->push(s, &element);
            const sce_queue_push_status_t want = step->expect == 1 ? SCE_QUEUE_PUSH_OK : SCE_QUEUE_PUSH_OUT_OF_MEMORY;
            CHECK(got == want, "%s/%s step %u: push gave %d, want %d", name, scenario->id, i, (int)got, (int)want);
            break;
        }
        case OP_POP: {
            tracked_t got;
            const int popped = s->pop(s, &got);
            if (step->expect < 0) {
                CHECK(!popped, "%s/%s step %u: expected the queue to be empty", name, scenario->id, i);
            } else {
                CHECK(popped && got.value == (uint64_t)step->expect && got.tag == TAG_OF(got.value),
                      "%s/%s step %u: popped (%d, %llu), want %lld", name, scenario->id, i, popped,
                      (unsigned long long)got.value, (long long)step->expect);
            }
            break;
        }
        case OP_CAPACITY: /* a segmented queue has no capacity */
        case OP_DESTROY:
            return;
        }
    }
}

static void run_linked_scenario(const scenario_t *scenario) {
    test_allocator_t counting = {scenario->segments, 0u, 0u};
    sce_queue_allocator_t allocator = test_allocator_of(&counting, SCE_QUEUE_PROGRESS_BLOCKING);
    sce_queue_linked_t q;
    segmented_subject_t s;
    CHECK(sce_queue_linked_init(&q, &allocator, SCE_QUEUE_PROGRESS_BLOCKING, sizeof(tracked_t), _Alignof(tracked_t),
                                scenario->capacity),
          "linked/%s: the shape is refused", scenario->id);
    CHECK(sce_queue_linked_producer_acquire(&q) && sce_queue_linked_consumer_acquire(&q), "linked/%s: one place a side",
          scenario->id);
    s.queue = &q;
    s.producer_hazard = 0u;
    s.consumer_hazard = 0u;
    s.push = linked_subject_push;
    s.pop = linked_subject_pop;
    run_segmented_steps(&s, "linked", scenario);
    sce_queue_linked_producer_release(&q);
    sce_queue_linked_consumer_release(&q);
    sce_queue_linked_destroy(&q);
    CHECK(live_blocks(&counting) == 0u, "linked/%s: the queue did not give back every segment", scenario->id);
}

static void run_lscq_scenario(const scenario_t *scenario) {
    test_allocator_t counting = {scenario->segments, 0u, 0u};
    sce_queue_allocator_t allocator = test_allocator_of(&counting, SCE_QUEUE_PROGRESS_BLOCKING);
    sce_hazard_slot_t slots[4];
    sce_hazard_domain_t domain;
    sce_queue_lscq_t q;
    segmented_subject_t s;
    CHECK(sce_hazard_domain_init(&domain, slots, 4u), "lscq/%s: the domain is refused", scenario->id);
    CHECK(sce_queue_lscq_init(&q, &domain, &allocator, SCE_QUEUE_PROGRESS_BLOCKING, sizeof(tracked_t),
                              _Alignof(tracked_t), scenario->capacity, 2u),
          "lscq/%s: the shape is refused", scenario->id);
    CHECK(sce_queue_lscq_producer_acquire(&q, &s.producer_hazard) &&
              sce_queue_lscq_consumer_acquire(&q, &s.consumer_hazard),
          "lscq/%s: a place a side", scenario->id);
    s.queue = &q;
    s.push = lscq_subject_push;
    s.pop = lscq_subject_pop;
    run_segmented_steps(&s, "lscq", scenario);
    sce_queue_lscq_producer_release(&q, s.producer_hazard);
    sce_queue_lscq_consumer_release(&q, s.consumer_hazard);
    sce_queue_lscq_destroy(&q);
    CHECK(live_blocks(&counting) == 0u, "lscq/%s: the queue did not give back every segment", scenario->id);
    CHECK(sce_hazard_waiting(&domain) == 0u, "lscq/%s: the domain freed every retired segment", scenario->id);
}

static void every_segmented_scenario_holds(void) {
    size_t i;
    unsigned ran = 0;
    for (i = 0; i < g_scenario_count; i++) {
        const scenario_t *scenario = &g_scenarios[i];
        if (!scenario->segmented) {
            continue;
        }
        if (scenario->producers_one && scenario->consumers_one) {
            run_linked_scenario(scenario);
        } else {
            run_lscq_scenario(scenario);
        }
        ran++;
    }
    CHECK(ran > 0u, "a contract with no segmented scenario checks nothing of the segmented queues");
}

static void a_segmented_queue_refuses_what_it_cannot_run(void) {
    test_allocator_t counting = {0u, 0u, 0u};
    sce_queue_allocator_t allocator = test_allocator_of(&counting, SCE_QUEUE_PROGRESS_BLOCKING);
    sce_queue_allocator_t lock_free = test_allocator_of(&counting, SCE_QUEUE_PROGRESS_LOCK_FREE);
    test_allocator_t none = {1u, 1u, 0u}; /* has given its one block already */
    sce_queue_allocator_t spent = test_allocator_of(&none, SCE_QUEUE_PROGRESS_BLOCKING);
    sce_hazard_slot_t slots[2];
    sce_hazard_domain_t domain;
    sce_queue_linked_t linked;
    sce_queue_lscq_t lscq;
    CHECK(sce_hazard_domain_init(&domain, slots, 2u), "a domain of two slots");
    CHECK(!sce_hazard_domain_init(&domain, slots, 0u), "a domain of no slot is refused");

    /* The allocator gives less progress than the document declared for it. */
    CHECK(!sce_queue_linked_init(&linked, &allocator, SCE_QUEUE_PROGRESS_LOCK_FREE, sizeof(tracked_t),
                                 _Alignof(tracked_t), 2u),
          "linked: a blocking allocator under a lock-free declaration is refused");
    CHECK(!sce_queue_lscq_init(&lscq, &domain, &allocator, SCE_QUEUE_PROGRESS_LOCK_FREE, sizeof(tracked_t),
                               _Alignof(tracked_t), 2u, 2u),
          "lscq: a blocking allocator under a lock-free declaration is refused");
    CHECK(live_blocks(&counting) == 0u, "a refused construction takes no block");
    CHECK(sce_queue_lscq_init(&lscq, &domain, &lock_free, SCE_QUEUE_PROGRESS_LOCK_FREE, sizeof(tracked_t),
                              _Alignof(tracked_t), 2u, 2u),
          "lscq: a lock-free allocator meets a lock-free declaration");
    sce_queue_lscq_destroy(&lscq);

    /* A shape the rings refuse. */
    CHECK(!sce_queue_lscq_init(&lscq, &domain, &allocator, SCE_QUEUE_PROGRESS_BLOCKING, sizeof(tracked_t),
                               _Alignof(tracked_t), 4u, 2u),
          "lscq: a ring smaller than the segment is refused");
    CHECK(!sce_queue_lscq_init(&lscq, &domain, &allocator, SCE_QUEUE_PROGRESS_BLOCKING, sizeof(tracked_t),
                               _Alignof(tracked_t), 2u, 3u),
          "lscq: a ring that is not a power of two is refused");
    CHECK(!sce_queue_linked_init(&linked, &allocator, SCE_QUEUE_PROGRESS_BLOCKING, sizeof(tracked_t), 3u, 2u),
          "linked: an alignment that is not a power of two is refused");

    /* An allocator that refuses the first segment leaves nothing built. */
    CHECK(!sce_queue_linked_init(&linked, &spent, SCE_QUEUE_PROGRESS_BLOCKING, sizeof(tracked_t), _Alignof(tracked_t),
                                 2u),
          "linked: an allocator that refuses the first segment is refused");
    CHECK(!sce_queue_lscq_init(&lscq, &domain, &spent, SCE_QUEUE_PROGRESS_BLOCKING, sizeof(tracked_t),
                               _Alignof(tracked_t), 2u, 2u),
          "lscq: an allocator that refuses the first segment is refused");
    CHECK(live_blocks(&none) == 0u, "and takes no block");
}

/* An LSCQ queue hands out `ring_slots` places a side and no more than the domain
 * has slots for, and a place and a slot come back when a handle is released. */
static void an_lscq_queue_hands_out_no_more_places_than_its_ring_and_domain_have(void) {
    test_allocator_t counting = {0u, 0u, 0u};
    sce_queue_allocator_t allocator = test_allocator_of(&counting, SCE_QUEUE_PROGRESS_BLOCKING);
    sce_hazard_slot_t slots[4];
    sce_hazard_domain_t domain;
    sce_queue_lscq_t q;
    uint32_t hazards[4];
    uint32_t extra;
    unsigned i;
    CHECK(sce_hazard_domain_init(&domain, slots, 4u), "a domain of four slots");
    CHECK(sce_queue_lscq_init(&q, &domain, &allocator, SCE_QUEUE_PROGRESS_BLOCKING, sizeof(tracked_t),
                              _Alignof(tracked_t), 2u, 4u),
          "the queue");
    for (i = 0; i < 3u; i++) {
        CHECK(sce_queue_lscq_producer_acquire(&q, &hazards[i]), "three producers fit the domain");
    }
    CHECK(sce_queue_lscq_consumer_acquire(&q, &hazards[3]), "three producers and one consumer: four slots");
    CHECK(!sce_queue_lscq_producer_acquire(&q, &extra) && !sce_queue_lscq_consumer_acquire(&q, &extra),
          "the domain has four slots, all taken");
    sce_queue_lscq_producer_release(&q, hazards[2]);
    CHECK(sce_queue_lscq_consumer_acquire(&q, &extra), "a released handle gives its slot back");
    sce_queue_lscq_consumer_release(&q, extra);
    sce_queue_lscq_consumer_release(&q, hazards[3]);
    sce_queue_lscq_producer_release(&q, hazards[1]);
    sce_queue_lscq_producer_release(&q, hazards[0]);
    sce_queue_lscq_destroy(&q);
    CHECK(live_blocks(&counting) == 0u, "every segment went back");
}

/* ---- Properties ---- */

static void order_holds_across_many_laps(void) {
    kind_t kinds[4];
    unsigned k;
    kinds[0] = KIND_SPSC;
    kinds[1] = KIND_SCQ64;
    kinds[2] = KIND_SCQ32;
    kinds[3] = KIND_IRQ;
    for (k = 0; k < 4; k++) {
        owned_t o;
        uint64_t pushed = 0;
        uint64_t popped = 0;
        unsigned round;
        CHECK(subject_build(&o, kinds[k], 3, 1, 0), "%s: capacity 3 is refused", kind_name(kinds[k]));
        CHECK(o.subject.producer_acquire(o.subject.queue) && o.subject.consumer_acquire(o.subject.queue),
              "%s: a fresh queue hands out a place a side", o.subject.name);
        for (round = 0; round < 1000u; round++) {
            tracked_t element;
            tracked_t got;
            element = make_tracked(pushed);
            while (o.subject.push(o.subject.queue, o.subject.slots, &element) == SCE_QUEUE_PUSH_OK) {
                pushed++;
                element = make_tracked(pushed);
            }
            while (o.subject.pop(o.subject.queue, o.subject.slots, &got)) {
                CHECK(got.value == popped, "%s: popped %llu, want %llu", o.subject.name, (unsigned long long)got.value,
                      (unsigned long long)popped);
                popped++;
            }
        }
        CHECK(pushed == popped, "%s: %llu pushed, %llu popped", o.subject.name, (unsigned long long)pushed,
              (unsigned long long)popped);
        CHECK(popped >= 3000u, "%s: the run went round the ring many times", o.subject.name);
        subject_free(&o);
    }
}

static void exactly_the_capacity_fits_when_nothing_else_runs(void) {
    static const uint32_t capacities[] = {1u, 2u, 3u, 5u, 8u};
    kind_t kinds[3];
    unsigned k;
    unsigned c;
    kinds[0] = KIND_SCQ64;
    kinds[1] = KIND_SCQ32;
    kinds[2] = KIND_IRQ;
    for (k = 0; k < 3; k++) {
        for (c = 0; c < sizeof capacities / sizeof capacities[0]; c++) {
            owned_t o;
            unsigned round;
            uint64_t next = 0;
            CHECK(subject_build(&o, kinds[k], capacities[c], 1, 0), "%s: capacity %u is refused", kind_name(kinds[k]),
                  (unsigned)capacities[c]);
            for (round = 0; round < 200u; round++) {
                uint32_t accepted = 0;
                tracked_t element = make_tracked(next);
                tracked_t got;
                while (o.subject.push(o.subject.queue, o.subject.slots, &element) == SCE_QUEUE_PUSH_OK) {
                    next++;
                    accepted++;
                    element = make_tracked(next);
                }
                CHECK(accepted == capacities[c], "%s: %u fit, want exactly %u", o.subject.name, (unsigned)accepted,
                      (unsigned)capacities[c]);
                while (o.subject.pop(o.subject.queue, o.subject.slots, &got)) {
                }
            }
            subject_free(&o);
        }
    }
}

static void a_side_hands_out_no_more_places_than_it_has(void) {
    owned_t o;
    unsigned taken;
    /* Lamport: one place a side. */
    CHECK(subject_build(&o, KIND_SPSC, 2, 1, 0), "lamport capacity 2");
    CHECK(o.subject.producer_acquire(o.subject.queue), "first producer");
    CHECK(!o.subject.producer_acquire(o.subject.queue), "a second producer is not this algorithm");
    CHECK(o.subject.consumer_acquire(o.subject.queue), "first consumer");
    CHECK(!o.subject.consumer_acquire(o.subject.queue), "a second consumer is not this algorithm");
    o.subject.producer_release(o.subject.queue);
    o.subject.consumer_release(o.subject.queue);
    CHECK(o.subject.producer_acquire(o.subject.queue), "a released producer place is free again");
    CHECK(o.subject.consumer_acquire(o.subject.queue), "a released consumer place is free again");
    subject_free(&o);

    /* SCQ at both widths and the interrupt ring: as many as the ring has slots. */
    {
        static const kind_t kinds[] = {KIND_SCQ64, KIND_SCQ32, KIND_IRQ};
        unsigned k;
        for (k = 0; k < 3; k++) {
            CHECK(subject_build(&o, kinds[k], 2, 4, 4), "%s: capacity 2, 4 places", kind_name(kinds[k]));
            for (taken = 0; taken < 4u; taken++) {
                CHECK(o.subject.producer_acquire(o.subject.queue), "%s: producer %u", o.subject.name, taken);
                CHECK(o.subject.consumer_acquire(o.subject.queue), "%s: consumer %u", o.subject.name, taken);
            }
            CHECK(!o.subject.producer_acquire(o.subject.queue), "%s: a fifth producer has no place", o.subject.name);
            CHECK(!o.subject.consumer_acquire(o.subject.queue), "%s: a fifth consumer has no place", o.subject.name);
            o.subject.producer_release(o.subject.queue);
            CHECK(o.subject.producer_acquire(o.subject.queue), "%s: a released place is free again", o.subject.name);
            subject_free(&o);
        }
    }
}

static void a_constructor_refuses_a_shape_it_cannot_run(void) {
    owned_t o;
    sce_queue_spsc_t spsc;
    sce_queue_irq_t irq;
    CHECK(!sce_queue_spsc_init(&spsc, 0u), "lamport capacity 0");
    CHECK(!sce_queue_spsc_init(&spsc, 0x80000000u), "lamport 2N does not fit in 32 bits");
    CHECK(sce_queue_spsc_init(&spsc, 0x7FFFFFFFu), "lamport 2N-1 fits");
    CHECK(!sce_queue_irq_init(&irq, 0u, 1u, 1u), "irq capacity 0");
    CHECK(!sce_queue_irq_init(&irq, 2u, 0u, 1u), "irq producer places 0");
    CHECK(!sce_queue_irq_init(&irq, 2u, 1u, 0u), "irq consumer places 0");
    /* SCQ: capacity below one, a ring that is not a power of two, a ring below the
     * capacity, a ring larger than the entry width can number. */
    CHECK(!subject_build(&o, KIND_SCQ64, 0, 1, 2), "scq64 capacity 0");
    subject_free(&o);
    CHECK(!subject_build(&o, KIND_SCQ64, 2, 1, 3), "scq64 ring 3");
    subject_free(&o);
    CHECK(!subject_build(&o, KIND_SCQ64, 4, 1, 2), "scq64 ring below capacity");
    subject_free(&o);
    CHECK(!subject_build(&o, KIND_SCQ32, 2, 1, 3), "scq32 ring 3");
    subject_free(&o);
    CHECK(!subject_build(&o, KIND_SCQ32, 4, 1, 2), "scq32 ring below capacity");
    subject_free(&o);
    {
        /* Not built: the entry arrays of the largest ring are gigabytes. The
         * checks run before the arrays are touched. */
        sce_queue_scq64_t q64;
        sce_queue_scq32_t q32;
        CHECK(!sce_queue_scq64_init(&q64, 2u, (1u << 30) * 2u, NULL, NULL), "scq64 ring beyond 2^30");
        CHECK(!sce_queue_scq32_init(&q32, 2u, (1u << 15) * 2u, NULL, NULL), "scq32 ring beyond 2^15");
    }
}

static void the_wrap_bound_is_the_entry_widths(void) {
    CHECK(SCE_QUEUE_WRAP_BOUND_OPS_64 == ((uint64_t)1 << 62), "the 64-bit bound is 2^62");
    CHECK(SCE_QUEUE_WRAP_BOUND_OPS_32 == ((uint64_t)1 << 30), "the 32-bit bound is 2^30");
}

/* ---- Threads ---- */

#define DEADLINE_SPINS 400000000ull

typedef struct stress {
    subject_t *subject;
    unsigned producers;
    unsigned consumers;
    uint64_t per_producer;
    uint64_t delivered; /* guarded by the atomics: read through sce_atomic_load_acquire_u64 */
    uint64_t total;
    /* Per consumer, per producer: the last value seen (+1; 0 = none) and the count. */
    uint64_t *last_plus_one;
    uint64_t *counts;
    uint32_t failed;
} stress_t;

/* Several threads report a failure; the count is an atomic and not a plain
 * increment. */
static void note_failure(stress_t *st) {
    (void)sce_atomic_fetch_add_acq_rel_u32(&st->failed, 1u);
}

typedef struct worker {
    stress_t *stress;
    unsigned index;
    pthread_t thread;
} worker_t;

static void *produce(void *arg) {
    worker_t *w = (worker_t *)arg;
    stress_t *st = w->stress;
    subject_t *s = st->subject;
    uint64_t i;
    if (!s->producer_acquire(s->queue)) {
        note_failure(st);
        return NULL;
    }
    for (i = 0; i < st->per_producer; i++) {
        const tracked_t element = make_tracked((uint64_t)w->index * st->per_producer + i);
        uint64_t spins = 0;
        while (s->push(s->queue, s->slots, &element) != SCE_QUEUE_PUSH_OK) {
            if (++spins > DEADLINE_SPINS) {
                note_failure(st);
                s->producer_release(s->queue);
                return NULL;
            }
            (void)sched_yield();
        }
    }
    s->producer_release(s->queue);
    return NULL;
}

static void *consume(void *arg) {
    worker_t *w = (worker_t *)arg;
    stress_t *st = w->stress;
    subject_t *s = st->subject;
    uint64_t *last = st->last_plus_one + (size_t)w->index * st->producers;
    uint64_t *count = st->counts + (size_t)w->index * st->producers;
    uint64_t spins = 0;
    if (!s->consumer_acquire(s->queue)) {
        note_failure(st);
        return NULL;
    }
    while (sce_atomic_load_acquire_u64(&st->delivered) < st->total) {
        tracked_t got;
        if (s->pop(s->queue, s->slots, &got)) {
            const unsigned origin = (unsigned)(got.value / st->per_producer);
            spins = 0;
            if (got.tag != TAG_OF(got.value) || origin >= st->producers || got.value + 1u <= last[origin]) {
                note_failure(st);
                break;
            }
            last[origin] = got.value + 1u;
            count[origin]++;
            (void)sce_atomic_fetch_add_acq_rel_u64(&st->delivered, 1u);
        } else {
            if (++spins > DEADLINE_SPINS) {
                note_failure(st);
                break;
            }
            (void)sched_yield();
        }
    }
    s->consumer_release(s->queue);
    return NULL;
}

/* `producers` producers and `consumers` consumers on real threads through
 * `subject`, each producer pushing per_producer distinct values. Nothing is lost,
 * nothing arrives twice, and one producer's elements reach one consumer in the
 * order they were pushed. */
static void threads_lose_nothing_and_reorder_nothing(subject_t *subject, unsigned producers, unsigned consumers,
                                                     uint64_t per_producer) {
    stress_t st;
    worker_t *workers = (worker_t *)calloc(producers + consumers, sizeof(worker_t));
    unsigned i;
    unsigned p;
    memset(&st, 0, sizeof st);
    st.subject = subject;
    st.producers = producers;
    st.consumers = consumers;
    st.per_producer = per_producer;
    st.total = (uint64_t)producers * per_producer;
    st.last_plus_one = (uint64_t *)calloc((size_t)consumers * producers, sizeof(uint64_t));
    st.counts = (uint64_t *)calloc((size_t)consumers * producers, sizeof(uint64_t));
    for (i = 0; i < producers; i++) {
        workers[i].stress = &st;
        workers[i].index = i;
        (void)pthread_create(&workers[i].thread, NULL, produce, &workers[i]);
    }
    for (i = 0; i < consumers; i++) {
        worker_t *w = &workers[producers + i];
        w->stress = &st;
        w->index = i;
        (void)pthread_create(&w->thread, NULL, consume, w);
    }
    for (i = 0; i < producers + consumers; i++) {
        (void)pthread_join(workers[i].thread, NULL);
    }
    CHECK(st.failed == 0u, "%s %up/%uc: %u failures (a lost, repeated or reordered element, or a deadline)",
          subject->name, producers, consumers, st.failed);
    for (p = 0; p < producers; p++) {
        uint64_t sum = 0;
        for (i = 0; i < consumers; i++) {
            sum += st.counts[(size_t)i * producers + p];
        }
        CHECK(sum == per_producer, "%s %up/%uc: producer %u's elements arrived %llu times, want %llu", subject->name,
              producers, consumers, p, (unsigned long long)sum, (unsigned long long)per_producer);
    }
    free(st.last_plus_one);
    free(st.counts);
    free(workers);
}

static void real_threads_lose_nothing_and_reorder_nothing(void) {
    owned_t o;
    static const kind_t kinds[] = {KIND_SCQ64, KIND_SCQ32, KIND_IRQ};
    unsigned k;
    /* Lamport: one producer, one consumer. */
    CHECK(subject_build(&o, KIND_SPSC, 4, 1, 0), "lamport capacity 4");
    threads_lose_nothing_and_reorder_nothing(&o.subject, 1, 1, 50000u);
    subject_free(&o);
    for (k = 0; k < 3; k++) {
        CHECK(subject_build(&o, kinds[k], 8, 3, 8), "%s: capacity 8", kind_name(kinds[k]));
        threads_lose_nothing_and_reorder_nothing(&o.subject, 3, 3, 20000u);
        subject_free(&o);
    }
}

/* ---- Threads through the segmented queues ---- */

#define SEGMENTED_COUNT 100000u

typedef struct linked_run {
    sce_queue_linked_t *queue;
    uint32_t failed;
    uint64_t *got;
} linked_run_t;

static void *linked_produce(void *arg) {
    linked_run_t *run = (linked_run_t *)arg;
    uint64_t i;
    for (i = 0; i < SEGMENTED_COUNT; i++) {
        const tracked_t element = make_tracked(i);
        if (sce_queue_linked_try_push(run->queue, &element) != SCE_QUEUE_PUSH_OK) {
            (void)sce_atomic_fetch_add_acq_rel_u32(&run->failed, 1u);
            return NULL;
        }
    }
    return NULL;
}

static void *linked_consume(void *arg) {
    linked_run_t *run = (linked_run_t *)arg;
    uint64_t taken = 0;
    unsigned long long spins = 0;
    while (taken < SEGMENTED_COUNT) {
        tracked_t element;
        if (sce_queue_linked_try_pop(run->queue, &element)) {
            spins = 0;
            run->got[taken++] = element.value;
        } else {
            if (++spins > DEADLINE_SPINS) {
                (void)sce_atomic_fetch_add_acq_rel_u32(&run->failed, 1u);
                return NULL;
            }
            (void)sched_yield();
        }
    }
    return NULL;
}

/* One producer and one consumer through linked Lamport rings of three-element
 * segments: order and count, and every segment given back. */
static void linked_rings_lose_nothing_between_threads(void) {
    test_allocator_t counting = {0u, 0u, 0u};
    sce_queue_allocator_t allocator = test_allocator_of(&counting, SCE_QUEUE_PROGRESS_BLOCKING);
    sce_queue_linked_t q;
    linked_run_t run;
    pthread_t producer;
    pthread_t consumer;
    uint64_t i;
    run.queue = &q;
    run.failed = 0u;
    run.got = (uint64_t *)calloc(SEGMENTED_COUNT, sizeof(uint64_t));
    CHECK(run.got != NULL, "memory for the run");
    CHECK(
        sce_queue_linked_init(&q, &allocator, SCE_QUEUE_PROGRESS_BLOCKING, sizeof(tracked_t), _Alignof(tracked_t), 3u),
        "the queue");
    CHECK(sce_queue_linked_producer_acquire(&q) && sce_queue_linked_consumer_acquire(&q), "a place a side");
    (void)pthread_create(&producer, NULL, linked_produce, &run);
    (void)pthread_create(&consumer, NULL, linked_consume, &run);
    (void)pthread_join(producer, NULL);
    (void)pthread_join(consumer, NULL);
    CHECK(run.failed == 0u, "the elements all arrived before the deadline");
    for (i = 0; i < SEGMENTED_COUNT; i++) {
        if (run.got[i] != i) {
            CHECK(0, "element %llu came out as %llu: not in the order pushed", (unsigned long long)i,
                  (unsigned long long)run.got[i]);
            break;
        }
    }
    sce_queue_linked_producer_release(&q);
    sce_queue_linked_consumer_release(&q);
    sce_queue_linked_destroy(&q);
    CHECK(live_blocks(&counting) == 0u, "the queue gave back every segment");
    free(run.got);
}

#define LSCQ_PER_PRODUCER 4000u

typedef struct lscq_run {
    sce_queue_lscq_t *queue;
    unsigned producers;
    uint64_t total;
    uint64_t delivered;
    uint32_t failed;
    /* Per consumer, then per producer: the last sequence seen (+1; 0 = none) and
     * how many were seen. */
    uint64_t *last_plus_one;
    uint64_t *counts;
    unsigned consumers;
} lscq_run_t;

typedef struct lscq_worker {
    lscq_run_t *run;
    unsigned index;
    pthread_t thread;
} lscq_worker_t;

static void *lscq_produce(void *arg) {
    lscq_worker_t *w = (lscq_worker_t *)arg;
    lscq_run_t *run = w->run;
    uint32_t hazard;
    uint64_t i;
    if (!sce_queue_lscq_producer_acquire(run->queue, &hazard)) {
        (void)sce_atomic_fetch_add_acq_rel_u32(&run->failed, 1u);
        return NULL;
    }
    for (i = 0; i < LSCQ_PER_PRODUCER; i++) {
        /* The value carries the producer in its top bits and the sequence below. */
        const tracked_t element = make_tracked(((uint64_t)w->index << 32) | i);
        if (sce_queue_lscq_try_push(run->queue, hazard, &element) != SCE_QUEUE_PUSH_OK) {
            (void)sce_atomic_fetch_add_acq_rel_u32(&run->failed, 1u);
            break;
        }
    }
    sce_queue_lscq_producer_release(run->queue, hazard);
    return NULL;
}

static void *lscq_consume(void *arg) {
    lscq_worker_t *w = (lscq_worker_t *)arg;
    lscq_run_t *run = w->run;
    uint32_t hazard;
    unsigned long long spins = 0;
    if (!sce_queue_lscq_consumer_acquire(run->queue, &hazard)) {
        (void)sce_atomic_fetch_add_acq_rel_u32(&run->failed, 1u);
        return NULL;
    }
    while (sce_atomic_load_acquire_u64(&run->delivered) < run->total) {
        tracked_t element;
        if (sce_queue_lscq_try_pop(run->queue, hazard, &element)) {
            const unsigned producer = (unsigned)(element.value >> 32);
            const uint64_t sequence = element.value & 0xFFFFFFFFu;
            uint64_t *last = &run->last_plus_one[(size_t)w->index * run->producers + producer];
            spins = 0;
            if (*last != 0u && sequence + 1u <= *last) {
                (void)sce_atomic_fetch_add_acq_rel_u32(&run->failed, 1u);
            }
            *last = sequence + 1u;
            run->counts[(size_t)w->index * run->producers + producer]++;
            (void)sce_atomic_fetch_add_acq_rel_u64(&run->delivered, 1u);
        } else {
            if (++spins > DEADLINE_SPINS) {
                (void)sce_atomic_fetch_add_acq_rel_u32(&run->failed, 1u);
                break;
            }
            (void)sched_yield();
        }
    }
    sce_queue_lscq_consumer_release(run->queue, hazard);
    return NULL;
}

/* Producers and consumers of an LSCQ queue of two-element segments: nothing is lost
 * or duplicated, each consumer sees each producer's elements in the order they went
 * in, and every segment, the retired ones included, goes back. */
static void lscq_loses_nothing_between_threads(unsigned producers, unsigned consumers) {
    test_allocator_t counting = {0u, 0u, 0u};
    sce_queue_allocator_t allocator = test_allocator_of(&counting, SCE_QUEUE_PROGRESS_BLOCKING);
    sce_hazard_slot_t slots[8];
    sce_hazard_domain_t domain;
    sce_queue_lscq_t q;
    lscq_run_t run;
    lscq_worker_t workers[8];
    unsigned i;
    unsigned p;
    memset(&run, 0, sizeof run);
    run.queue = &q;
    run.producers = producers;
    run.consumers = consumers;
    run.total = (uint64_t)producers * LSCQ_PER_PRODUCER;
    run.last_plus_one = (uint64_t *)calloc((size_t)consumers * producers, sizeof(uint64_t));
    run.counts = (uint64_t *)calloc((size_t)consumers * producers, sizeof(uint64_t));
    CHECK(run.last_plus_one != NULL && run.counts != NULL, "memory for the run");
    CHECK(sce_hazard_domain_init(&domain, slots, 8u), "the domain");
    CHECK(sce_queue_lscq_init(&q, &domain, &allocator, SCE_QUEUE_PROGRESS_BLOCKING, sizeof(tracked_t),
                              _Alignof(tracked_t), 2u, 4u),
          "the queue");
    for (i = 0; i < consumers; i++) {
        workers[i].run = &run;
        workers[i].index = i;
        (void)pthread_create(&workers[i].thread, NULL, lscq_consume, &workers[i]);
    }
    for (p = 0; p < producers; p++) {
        workers[consumers + p].run = &run;
        workers[consumers + p].index = p;
        (void)pthread_create(&workers[consumers + p].thread, NULL, lscq_produce, &workers[consumers + p]);
    }
    for (i = 0; i < consumers + producers; i++) {
        (void)pthread_join(workers[i].thread, NULL);
    }
    CHECK(run.failed == 0u, "the elements arrived in order before the deadline");
    for (p = 0; p < producers; p++) {
        uint64_t seen = 0;
        for (i = 0; i < consumers; i++) {
            seen += run.counts[(size_t)i * producers + p];
        }
        CHECK(seen == LSCQ_PER_PRODUCER, "producer %u: %llu of %u elements arrived", p, (unsigned long long)seen,
              (unsigned)LSCQ_PER_PRODUCER);
    }
    sce_queue_lscq_destroy(&q);
    CHECK(live_blocks(&counting) == 0u, "every segment went back, the retired ones included");
    CHECK(sce_hazard_waiting(&domain) == 0u, "the domain freed every retired segment");
    free(run.last_plus_one);
    free(run.counts);
}

static void an_lscq_queue_loses_nothing_between_threads(void) {
    lscq_loses_nothing_between_threads(2u, 2u);
    lscq_loses_nothing_between_threads(1u, 3u);
    lscq_loses_nothing_between_threads(3u, 1u);
}

int main(void) {
    every_contract_scenario_holds();
    every_intrusive_scenario_holds();
    an_intrusive_queue_refuses_a_shape_it_cannot_run();
    every_segmented_scenario_holds();
    a_segmented_queue_refuses_what_it_cannot_run();
    an_lscq_queue_hands_out_no_more_places_than_its_ring_and_domain_have();
    linked_rings_lose_nothing_between_threads();
    an_lscq_queue_loses_nothing_between_threads();
    order_holds_across_many_laps();
    exactly_the_capacity_fits_when_nothing_else_runs();
    a_side_hands_out_no_more_places_than_it_has();
    a_constructor_refuses_a_shape_it_cannot_run();
    the_wrap_bound_is_the_entry_widths();
    real_threads_lose_nothing_and_reorder_nothing();
    (void)printf("queue runtime (C11): %u check(s), %u failure(s)\n", g_checks, g_failures);
    return g_failures == 0u ? 0 : 1;
}
