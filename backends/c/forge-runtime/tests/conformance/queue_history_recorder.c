/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * The C11 arm's stress runs of the `queue` kind, written as histories (SCE
 * Protocol-Synthesis RFC §synth-5-P, verification layer 2).
 *
 * This program runs the runtime queues on real threads, records every attempt each
 * thread made, and writes one history per run in the JSON form every backend
 * writes (tests/forge/conformance/queue_history.schema.json). It judges nothing
 * about linearizability: `sce-codegen check-queue-history` does, for this backend
 * as for the others, so no backend carries a checker of its own. The build runs
 * this program, then runs the command over what it wrote (ctest fixture
 * `queue_histories`).
 *
 * The recording follows the Rust arm's: one counter that every thread reads
 * immediately before and immediately after each call, so a read-modify-write chain
 * on one atomic orders the readings with the calls between them and "returned
 * before invoked" in the record means it in the run. Every attempt is recorded, the
 * refused pushes and the empty pops too, because those are results the checker must
 * account for.
 *
 * Which refusal a history is judged under is the queue's: the Lamport ring and the
 * interrupt-masked ring refuse a push only at capacity (`at-capacity`, the strict
 * reading); SCQ may also refuse while other participants' operations hold slots
 * (`while-slots-are-held`).
 */

/* mkdir and sched_yield are POSIX, and the build asks for strict C11. */
#define _POSIX_C_SOURCE 200809L

#include <errno.h>
#include <pthread.h>
#include <sched.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>

#include "host_irq.h"
#include <sce/forge/queue.h>
#include <sce/forge/queue_irq.h>

/* ---- Runs per shape ---- */

/* Runs of each Lamport capacity: a Lamport run pushes thousands of values and its
 * history is megabytes, and the ring has no schedule a second run reaches that the
 * first does not. */
#define RUNS_PER_SPSC_CAPACITY 3
#define VALUES_PER_SPSC_RUN 2000u
/* Runs of each many-participant shape: short runs, many of them, because the
 * schedules that matter are a small fraction of those a machine gives. */
#define RUNS_PER_SHAPE 25
#define DEADLINE_SPINS 400000000ull

typedef struct shape {
    uint32_t capacity;
    uint32_t ring_slots;
    unsigned producers;
    unsigned consumers;
    uint64_t per_producer;
} shape_t;

/* The Rust arm's shapes. A ring is at least as large as the number of participants
 * working it, so the small capacities ride on rings sized for their threads. */
static const shape_t kShapes[] = {
    {1, 2, 2, 2, 150}, {3, 4, 2, 2, 150}, {8, 8, 2, 2, 150}, {2, 4, 3, 1, 120}, {4, 4, 1, 3, 120}, {5, 8, 2, 2, 150},
};

/* ---- The recording ---- */

typedef struct operation {
    int is_push;
    int has_value;
    const char *outcome;
    uint64_t value;
    uint64_t invoked;
    uint64_t returned;
} operation_t;

typedef struct log {
    operation_t *ops;
    size_t count;
    size_t reserved;
} log_t;

static void log_add(log_t *log, int is_push, int has_value, uint64_t value, const char *outcome, uint64_t invoked,
                    uint64_t returned) {
    operation_t *op;
    if (log->count == log->reserved) {
        log->reserved = log->reserved == 0u ? 256u : log->reserved * 2u;
        log->ops = (operation_t *)realloc(log->ops, log->reserved * sizeof(operation_t));
        if (log->ops == NULL) {
            (void)fprintf(stderr, "out of memory recording a history\n");
            exit(2);
        }
    }
    op = &log->ops[log->count++];
    op->is_push = is_push;
    op->has_value = has_value;
    op->value = value;
    op->outcome = outcome;
    op->invoked = invoked;
    op->returned = returned;
}

/* ---- One interface over every queue ---- */

typedef struct element {
    uint64_t value;
} element_t;

typedef struct subject {
    void *queue;
    unsigned char *slots;
    sce_queue_push_status_t (*push)(void *queue, unsigned char *slots, const void *value);
    int (*pop)(void *queue, unsigned char *slots, void *out);
    int (*producer_acquire)(void *queue);
    void (*producer_release)(void *queue);
    int (*consumer_acquire)(void *queue);
    void (*consumer_release)(void *queue);
} subject_t;

static sce_queue_push_status_t spsc_push(void *q, unsigned char *s, const void *v) {
    return sce_queue_spsc_try_push((sce_queue_spsc_t *)q, s, sizeof(element_t), v);
}

static int spsc_pop(void *q, unsigned char *s, void *o) {
    return sce_queue_spsc_try_pop((sce_queue_spsc_t *)q, s, sizeof(element_t), o);
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

static sce_queue_push_status_t scq64_push(void *q, unsigned char *s, const void *v) {
    return sce_queue_scq64_try_push((sce_queue_scq64_t *)q, s, sizeof(element_t), v);
}

static int scq64_pop(void *q, unsigned char *s, void *o) {
    return sce_queue_scq64_try_pop((sce_queue_scq64_t *)q, s, sizeof(element_t), o);
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

static sce_queue_push_status_t scq32_push(void *q, unsigned char *s, const void *v) {
    return sce_queue_scq32_try_push((sce_queue_scq32_t *)q, s, sizeof(element_t), v);
}

static int scq32_pop(void *q, unsigned char *s, void *o) {
    return sce_queue_scq32_try_pop((sce_queue_scq32_t *)q, s, sizeof(element_t), o);
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

static sce_queue_push_status_t irq_push(void *q, unsigned char *s, const void *v) {
    return sce_queue_irq_try_push((sce_queue_irq_t *)q, s, sizeof(element_t), v);
}

static int irq_pop(void *q, unsigned char *s, void *o) {
    return sce_queue_irq_try_pop((sce_queue_irq_t *)q, s, sizeof(element_t), o);
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

typedef struct owned {
    subject_t subject;
    void *allocated_entries;
    void *free_entries;
} owned_t;

static void die(const char *what) {
    (void)fprintf(stderr, "queue history recorder: %s\n", what);
    exit(2);
}

/* `places` is how many producers and consumers may be held at once. */
static void subject_build(owned_t *o, kind_t kind, uint32_t capacity, uint32_t ring_slots, uint32_t places) {
    memset(o, 0, sizeof *o);
    o->subject.slots = (unsigned char *)calloc(capacity, sizeof(element_t));
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
        if (!sce_queue_spsc_init(q, capacity)) {
            die("the Lamport ring refused its shape");
        }
        break;
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
        if (!sce_queue_scq64_init(q, capacity, ring_slots, (uint64_t *)o->allocated_entries,
                                  (uint64_t *)o->free_entries)) {
            die("the 64-bit SCQ refused its shape");
        }
        break;
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
        if (!sce_queue_scq32_init(q, capacity, ring_slots, (uint32_t *)o->allocated_entries,
                                  (uint32_t *)o->free_entries)) {
            die("the 32-bit SCQ refused its shape");
        }
        break;
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
        if (!sce_queue_irq_init(q, capacity, places, places)) {
            die("the interrupt-masked ring refused its shape");
        }
        break;
    }
    }
}

static void subject_free(owned_t *o) {
    free(o->subject.slots);
    free(o->subject.queue);
    free(o->allocated_entries);
    free(o->free_entries);
}

/* ---- One run ---- */

typedef struct run {
    subject_t *subject;
    uint64_t clock;
    uint64_t delivered;
    uint64_t total;
    uint64_t per_producer;
    unsigned producers;
    uint32_t failed;
    log_t *logs; /* producers + consumers of them */
} run_t;

typedef struct actor {
    run_t *run;
    unsigned index;
    pthread_t thread;
} actor_t;

static uint64_t tick(run_t *run) {
    return sce_atomic_fetch_add_acq_rel_u64(&run->clock, 1u) + 1u;
}

static void *produce(void *arg) {
    actor_t *a = (actor_t *)arg;
    run_t *run = a->run;
    subject_t *s = run->subject;
    log_t *log = &run->logs[a->index];
    uint64_t i;
    if (!s->producer_acquire(s->queue)) {
        (void)sce_atomic_fetch_add_acq_rel_u32(&run->failed, 1u);
        return NULL;
    }
    for (i = 0; i < run->per_producer; i++) {
        element_t element;
        uint64_t spins = 0;
        element.value = (uint64_t)a->index * run->per_producer + i + 1u;
        for (;;) {
            uint64_t invoked;
            uint64_t returned;
            sce_queue_push_status_t status;
            if (++spins > DEADLINE_SPINS) {
                (void)sce_atomic_fetch_add_acq_rel_u32(&run->failed, 1u);
                s->producer_release(s->queue);
                return NULL;
            }
            invoked = tick(run);
            status = s->push(s->queue, s->slots, &element);
            returned = tick(run);
            if (status == SCE_QUEUE_PUSH_OK) {
                log_add(log, 1, 1, element.value, "pushed", invoked, returned);
                break;
            }
            log_add(log, 1, 1, element.value, "full", invoked, returned);
            (void)sched_yield();
        }
    }
    s->producer_release(s->queue);
    return NULL;
}

static void *consume(void *arg) {
    actor_t *a = (actor_t *)arg;
    run_t *run = a->run;
    subject_t *s = run->subject;
    log_t *log = &run->logs[run->producers + a->index];
    uint64_t spins = 0;
    if (!s->consumer_acquire(s->queue)) {
        (void)sce_atomic_fetch_add_acq_rel_u32(&run->failed, 1u);
        return NULL;
    }
    while (sce_atomic_load_acquire_u64(&run->delivered) < run->total) {
        element_t got;
        const uint64_t invoked = tick(run);
        const int popped = s->pop(s->queue, s->slots, &got);
        const uint64_t returned = tick(run);
        if (popped) {
            spins = 0;
            log_add(log, 0, 1, got.value, "popped", invoked, returned);
            (void)sce_atomic_fetch_add_acq_rel_u64(&run->delivered, 1u);
        } else {
            log_add(log, 0, 0, 0, "empty", invoked, returned);
            if (++spins > DEADLINE_SPINS) {
                (void)sce_atomic_fetch_add_acq_rel_u32(&run->failed, 1u);
                break;
            }
            (void)sched_yield();
        }
    }
    s->consumer_release(s->queue);
    return NULL;
}

/* Writes `<dir>/<name>.json`. */
static void write_history(const char *dir, const char *name, uint32_t capacity, const char *refusal, const log_t *logs,
                          unsigned participants) {
    char path[1024];
    FILE *file;
    unsigned who;
    (void)snprintf(path, sizeof path, "%s/%s.json", dir, name);
    file = fopen(path, "w");
    if (file == NULL) {
        (void)fprintf(stderr, "cannot write %s\n", path);
        exit(2);
    }
    (void)fprintf(file, "{\"version\":1,\"capacity\":%u,\"refusal\":\"%s\",\"participants\":[", (unsigned)capacity,
                  refusal);
    for (who = 0; who < participants; who++) {
        size_t at;
        (void)fputs(who == 0u ? "[" : ",[", file);
        for (at = 0; at < logs[who].count; at++) {
            const operation_t *op = &logs[who].ops[at];
            (void)fprintf(file, "%s{\"call\":\"%s\"", at == 0u ? "" : ",", op->is_push ? "push" : "pop");
            if (op->has_value) {
                (void)fprintf(file, ",\"value\":%llu", (unsigned long long)op->value);
            }
            (void)fprintf(file, ",\"outcome\":\"%s\",\"invoked\":%llu,\"returned\":%llu}", op->outcome,
                          (unsigned long long)op->invoked, (unsigned long long)op->returned);
        }
        (void)fputc(']', file);
    }
    (void)fputs("]}\n", file);
    if (fclose(file) != 0) {
        (void)fprintf(stderr, "cannot finish %s\n", path);
        exit(2);
    }
}

/* One run of `producers` and `consumers` on `kind`, written as `<dir>/<name>.json`. */
static void record_run(const char *dir, const char *name, kind_t kind, uint32_t capacity, uint32_t ring_slots,
                       unsigned producers, unsigned consumers, uint64_t per_producer, const char *refusal) {
    owned_t o;
    run_t run;
    unsigned participants = producers + consumers;
    actor_t *actors = (actor_t *)calloc(participants, sizeof(actor_t));
    unsigned i;
    subject_build(&o, kind, capacity, ring_slots, producers > consumers ? producers : consumers);
    memset(&run, 0, sizeof run);
    run.subject = &o.subject;
    run.total = (uint64_t)producers * per_producer;
    run.per_producer = per_producer;
    run.producers = producers;
    run.logs = (log_t *)calloc(participants, sizeof(log_t));
    for (i = 0; i < producers; i++) {
        actors[i].run = &run;
        actors[i].index = i;
        (void)pthread_create(&actors[i].thread, NULL, produce, &actors[i]);
    }
    for (i = 0; i < consumers; i++) {
        actors[producers + i].run = &run;
        actors[producers + i].index = i;
        (void)pthread_create(&actors[producers + i].thread, NULL, consume, &actors[producers + i]);
    }
    for (i = 0; i < participants; i++) {
        (void)pthread_join(actors[i].thread, NULL);
    }
    if (run.failed != 0u) {
        (void)fprintf(stderr, "%s: the elements did not all arrive before the deadline\n", name);
        exit(1);
    }
    write_history(dir, name, capacity, refusal, run.logs, participants);
    for (i = 0; i < participants; i++) {
        free(run.logs[i].ops);
    }
    free(run.logs);
    free(actors);
    subject_free(&o);
}

int main(int argc, char **argv) {
    static const uint32_t spsc_capacities[] = {1u, 2u, 3u, 8u};

    static const struct {
        kind_t kind;
        const char *name;
        const char *refusal;
    } many[] = {
        {KIND_SCQ64, "scq64", "while-slots-are-held"},
        {KIND_SCQ32, "scq32", "while-slots-are-held"},
        {KIND_IRQ, "irq", "at-capacity"},
    };

    const char *dir;
    char name[128];
    size_t c;
    size_t s;
    size_t m;
    int run;
    if (argc != 2) {
        (void)fprintf(stderr, "usage: %s <directory the histories are written to>\n", argv[0]);
        return 2;
    }
    dir = argv[1];
    if (mkdir(dir, 0777) != 0 && errno != EEXIST) {
        (void)fprintf(stderr, "cannot create %s\n", dir);
        return 2;
    }
    for (c = 0; c < sizeof spsc_capacities / sizeof spsc_capacities[0]; c++) {
        for (run = 0; run < RUNS_PER_SPSC_CAPACITY; run++) {
            (void)snprintf(name, sizeof name, "c11_lamport_n%u_%d", (unsigned)spsc_capacities[c], run);
            record_run(dir, name, KIND_SPSC, spsc_capacities[c], 0u, 1u, 1u, VALUES_PER_SPSC_RUN, "at-capacity");
        }
    }
    for (m = 0; m < sizeof many / sizeof many[0]; m++) {
        for (s = 0; s < sizeof kShapes / sizeof kShapes[0]; s++) {
            const shape_t *shape = &kShapes[s];
            for (run = 0; run < RUNS_PER_SHAPE; run++) {
                (void)snprintf(name, sizeof name, "c11_%s_n%u_r%u_p%u_c%u_%d", many[m].name, (unsigned)shape->capacity,
                               (unsigned)shape->ring_slots, shape->producers, shape->consumers, run);
                record_run(dir, name, many[m].kind, shape->capacity, shape->ring_slots, shape->producers,
                           shape->consumers, shape->per_producer, many[m].refusal);
            }
        }
    }
    return 0;
}
