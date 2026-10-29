/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * The C11 arm's stress runs of the `broadcast-log` kind, written as histories
 * (RFC §synth-5-S; ADR 0062 section 6, layer 3).
 *
 * This program runs the runtime log of sce/forge/broadcast_log.h on real threads,
 * records every operation each thread made, and writes one history per run in the
 * JSON form every backend writes
 * (tests/forge/conformance/broadcast_history.schema.json). It judges nothing
 * about linearizability: `sce-codegen check-broadcast-history` does, for this
 * backend as for the others, so no backend carries a checker of its own. The
 * build runs this program, then runs the command over what it wrote (ctest
 * fixture `broadcast_histories`).
 *
 * The recording follows the queue recorder's: one counter that every thread reads
 * immediately before and immediately after each call, so a read-modify-write
 * chain on one atomic orders the readings with the calls between them and
 * "returned before invoked" in the record means it in the run.
 *
 * The checker is a general search over the operations of all participants, so a
 * history is kept short: a run publishes tens of entries, and the shapes cover
 * the retentions 1 to 4 and one to three readers. A reader that finds the log
 * empty records that read and then waits, unrecorded, for the head to move: the
 * waiting is a read of the head that the history does not claim, and an empty read
 * changes nothing, so leaving the wait out leaves a history that is judged exactly
 * as the run was. The first reader of a shape opens at sequence number 0, the
 * second at the oldest retained entry and the third at the head, so that all
 * three ways of opening are judged.
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

#include <sce/forge/broadcast_log.h>

/* ---- Runs per shape ---- */

#define RUNS_PER_SHAPE 20
#define DEADLINE_SPINS 400000000ull
#define ENTRY_WORDS 2u
#define MOST_READERS 3u

typedef struct shape {
    uint32_t retention;
    unsigned readers;
    uint64_t publishes;
} shape_t;

/* The retentions 1 to 4, one to three readers: the smallest logs lap their slots
 * the most, and the larger ones leave a reader room to be ahead of the publisher. */
static const shape_t kShapes[] = {
    {1, 2, 80}, {2, 3, 80}, {3, 2, 80}, {1, 1, 120}, {4, 3, 60},
};

/* ---- The recording ---- */

typedef enum op_kind { OP_PUBLISH, OP_OPEN, OP_ITEM, OP_EMPTY, OP_LAGGED, OP_HEAD } op_kind_t;

typedef enum open_from { FROM_OLDEST, FROM_HEAD, FROM_AT } open_from_t;

typedef struct operation {
    op_kind_t kind;
    open_from_t from;
    uint64_t seq;     /* a publish, an item */
    uint64_t value;   /* a publish, an item */
    uint64_t skipped; /* a lagged read */
    uint64_t at;      /* an open from a position */
    uint64_t cursor;  /* an open */
    uint64_t head;    /* a head */
    uint64_t invoked;
    uint64_t returned;
} operation_t;

typedef struct log {
    operation_t *ops;
    size_t count;
    size_t reserved;
} log_t;

static void die(const char *what) {
    (void)fprintf(stderr, "broadcast log history recorder: %s\n", what);
    exit(2);
}

static operation_t *log_next(log_t *log, op_kind_t kind, uint64_t invoked, uint64_t returned) {
    operation_t *op;
    if (log->count == log->reserved) {
        log->reserved = log->reserved == 0u ? 256u : log->reserved * 2u;
        log->ops = (operation_t *)realloc(log->ops, log->reserved * sizeof(operation_t));
        if (log->ops == NULL) {
            die("out of memory recording a history");
        }
    }
    op = &log->ops[log->count++];
    memset(op, 0, sizeof *op);
    op->kind = kind;
    op->invoked = invoked;
    op->returned = returned;
    return op;
}

/* ---- One run ---- */

typedef struct run {
    sce_blog_t log;
    uint64_t *storage;
    uint64_t clock;
    uint64_t total;
    uint32_t failed;
    log_t publisher_log;
    log_t reader_logs[MOST_READERS];
} run_t;

typedef struct actor {
    run_t *run;
    unsigned index;
    pthread_t thread;
} actor_t;

static uint64_t tick(run_t *run) {
    return sce_atomic_fetch_add_acq_rel_u64(&run->clock, 1u) + 1u;
}

static void *publish_all(void *arg) {
    actor_t *a = (actor_t *)arg;
    run_t *run = a->run;
    sce_blog_publisher_t publisher;
    uint64_t n;
    if (!sce_blog_publisher_acquire(&run->log, &publisher)) {
        (void)sce_atomic_fetch_add_acq_rel_u32(&run->failed, 1u);
        return NULL;
    }
    for (n = 0; n < run->total; n++) {
        uint64_t entry[ENTRY_WORDS];
        uint64_t invoked;
        uint64_t returned;
        uint64_t seq;
        unsigned k;
        operation_t *op;
        for (k = 0; k < ENTRY_WORDS; k++) {
            entry[k] = n + 1u;
        }
        invoked = tick(run);
        seq = sce_blog_publish(&publisher, entry);
        returned = tick(run);
        if (seq != n) {
            (void)sce_atomic_fetch_add_acq_rel_u32(&run->failed, 1u);
            break;
        }
        op = log_next(&run->publisher_log, OP_PUBLISH, invoked, returned);
        op->seq = seq;
        op->value = n + 1u;
        if (n % 4u == 3u) {
            uint64_t head;
            invoked = tick(run);
            head = sce_blog_head(&run->log);
            returned = tick(run);
            op = log_next(&run->publisher_log, OP_HEAD, invoked, returned);
            op->head = head;
        }
        /* Give the readers the processor, so that they are sometimes ahead of
         * the publisher and sometimes far behind it. */
        if (n % 8u == 7u) {
            (void)sched_yield();
        }
    }
    sce_blog_publisher_release(&publisher);
    return NULL;
}

static void *read_all(void *arg) {
    actor_t *a = (actor_t *)arg;
    run_t *run = a->run;
    log_t *log = &run->reader_logs[a->index];
    sce_blog_reader_t reader;
    open_from_t from = (open_from_t)((a->index == 0u) ? FROM_AT : (a->index == 1u ? FROM_OLDEST : FROM_HEAD));
    uint64_t invoked = tick(run);
    uint64_t returned;
    uint64_t spins = 0;
    uint64_t reads = 0;
    operation_t *op;
    if (from == FROM_AT) {
        reader = sce_blog_reader_at(&run->log, 0u);
    } else if (from == FROM_OLDEST) {
        reader = sce_blog_reader_oldest(&run->log);
    } else {
        reader = sce_blog_reader_head(&run->log);
    }
    returned = tick(run);
    op = log_next(log, OP_OPEN, invoked, returned);
    op->from = from;
    op->at = 0u;
    op->cursor = reader.cursor;
    while (reader.cursor < run->total) {
        uint64_t out[ENTRY_WORDS];
        sce_blog_read_t result;
        unsigned k;
        invoked = tick(run);
        result = sce_blog_read(&run->log, &reader, out);
        returned = tick(run);
        if (result.status == SCE_BLOG_ITEM) {
            op = log_next(log, OP_ITEM, invoked, returned);
            op->seq = result.seq;
            op->value = out[0];
            for (k = 1; k < ENTRY_WORDS; k++) {
                if (out[k] != out[0]) {
                    (void)sce_atomic_fetch_add_acq_rel_u32(&run->failed, 1u);
                }
            }
            spins = 0;
        } else if (result.status == SCE_BLOG_LAGGED) {
            op = log_next(log, OP_LAGGED, invoked, returned);
            op->skipped = result.skipped;
            spins = 0;
        } else {
            (void)log_next(log, OP_EMPTY, invoked, returned);
            /* Wait, unrecorded, for the head to move past the cursor. */
            while (sce_blog_head(&run->log) <= reader.cursor) {
                if (++spins > DEADLINE_SPINS) {
                    (void)sce_atomic_fetch_add_acq_rel_u32(&run->failed, 1u);
                    return NULL;
                }
                (void)sched_yield();
            }
        }
        if (++reads % 8u == 0u) {
            uint64_t head;
            invoked = tick(run);
            head = sce_blog_head(&run->log);
            returned = tick(run);
            op = log_next(log, OP_HEAD, invoked, returned);
            op->head = head;
        }
    }
    return NULL;
}

/* ---- The JSON form ---- */

static void write_operation(FILE *file, const operation_t *op) {
    switch (op->kind) {
    case OP_PUBLISH:
        (void)fprintf(file, "{\"call\":\"publish\",\"value\":%llu,\"seq\":%llu", (unsigned long long)op->value,
                      (unsigned long long)op->seq);
        break;
    case OP_OPEN:
        if (op->from == FROM_AT) {
            (void)fprintf(file, "{\"call\":\"open\",\"from\":\"at\",\"at\":%llu,\"cursor\":%llu",
                          (unsigned long long)op->at, (unsigned long long)op->cursor);
        } else {
            (void)fprintf(file, "{\"call\":\"open\",\"from\":\"%s\",\"cursor\":%llu",
                          op->from == FROM_OLDEST ? "oldest" : "head", (unsigned long long)op->cursor);
        }
        break;
    case OP_ITEM:
        (void)fprintf(file, "{\"call\":\"read\",\"outcome\":\"item\",\"seq\":%llu,\"value\":%llu",
                      (unsigned long long)op->seq, (unsigned long long)op->value);
        break;
    case OP_EMPTY:
        (void)fputs("{\"call\":\"read\",\"outcome\":\"empty\"", file);
        break;
    case OP_LAGGED:
        (void)fprintf(file, "{\"call\":\"read\",\"outcome\":\"lagged\",\"skipped\":%llu",
                      (unsigned long long)op->skipped);
        break;
    case OP_HEAD:
        (void)fprintf(file, "{\"call\":\"head\",\"head\":%llu", (unsigned long long)op->head);
        break;
    }
    (void)fprintf(file, ",\"invoked\":%llu,\"returned\":%llu}", (unsigned long long)op->invoked,
                  (unsigned long long)op->returned);
}

static void write_participant(FILE *file, const log_t *log) {
    size_t at;
    (void)fputc('[', file);
    for (at = 0; at < log->count; at++) {
        if (at != 0u) {
            (void)fputc(',', file);
        }
        write_operation(file, &log->ops[at]);
    }
    (void)fputc(']', file);
}

/* Writes `<dir>/<name>.json`. */
static void write_history(const char *dir, const char *name, uint32_t retention, const run_t *run, unsigned readers) {
    char path[1024];
    FILE *file;
    unsigned who;
    (void)snprintf(path, sizeof path, "%s/%s.json", dir, name);
    file = fopen(path, "w");
    if (file == NULL) {
        (void)fprintf(stderr, "cannot write %s\n", path);
        exit(2);
    }
    (void)fprintf(file, "{\"version\":1,\"retention\":%u,\"publisher\":", (unsigned)retention);
    write_participant(file, &run->publisher_log);
    (void)fputs(",\"readers\":[", file);
    for (who = 0; who < readers; who++) {
        if (who != 0u) {
            (void)fputc(',', file);
        }
        write_participant(file, &run->reader_logs[who]);
    }
    (void)fputs("]}\n", file);
    if (fclose(file) != 0) {
        (void)fprintf(stderr, "cannot finish %s\n", path);
        exit(2);
    }
}

/* One run of a publisher and `readers` readers on a log that retains `retention`
 * entries, written as `<dir>/<name>.json`. */
static void record_run(const char *dir, const char *name, uint32_t retention, unsigned readers, uint64_t publishes) {
    run_t *run = (run_t *)calloc(1, sizeof *run);
    actor_t actors[MOST_READERS + 1u];
    const uint32_t slots = retention + 1u;
    unsigned i;
    if (run == NULL || readers > MOST_READERS) {
        die("a run needs memory and at most the readers it has logs for");
    }
    run->storage = (uint64_t *)calloc(SCE_BLOG_STORAGE_WORDS(slots, ENTRY_WORDS), sizeof(uint64_t));
    if (run->storage == NULL || !sce_blog_init(&run->log, run->storage, slots, ENTRY_WORDS)) {
        die("the log refused its shape");
    }
    run->total = publishes;
    for (i = 0; i < readers; i++) {
        actors[i].run = run;
        actors[i].index = i;
        (void)pthread_create(&actors[i].thread, NULL, read_all, &actors[i]);
    }
    actors[readers].run = run;
    actors[readers].index = 0;
    (void)pthread_create(&actors[readers].thread, NULL, publish_all, &actors[readers]);
    for (i = 0; i <= readers; i++) {
        (void)pthread_join(actors[i].thread, NULL);
    }
    if (run->failed != 0u) {
        (void)fprintf(stderr, "%s: an entry was torn, a sequence number was wrong or a reader missed the deadline\n",
                      name);
        exit(1);
    }
    write_history(dir, name, retention, run, readers);
    free(run->publisher_log.ops);
    for (i = 0; i < readers; i++) {
        free(run->reader_logs[i].ops);
    }
    free(run->storage);
    free(run);
}

int main(int argc, char **argv) {
    const char *dir;
    char name[128];
    size_t s;
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
    for (s = 0; s < sizeof kShapes / sizeof kShapes[0]; s++) {
        for (run = 0; run < RUNS_PER_SHAPE; run++) {
            (void)snprintf(name, sizeof name, "c11_broadcast_r%u_c%u_n%llu_%d", (unsigned)kShapes[s].retention,
                           kShapes[s].readers, (unsigned long long)kShapes[s].publishes, run);
            record_run(dir, name, kShapes[s].retention, kShapes[s].readers, kShapes[s].publishes);
        }
    }
    return 0;
}
