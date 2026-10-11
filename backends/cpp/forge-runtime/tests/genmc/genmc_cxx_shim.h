// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The declarations libstdc++'s thread layer asks of <pthread.h> that GenMC's own
// <pthread.h> does not give, force-included before any C++ harness
// (scripts/gates/genmc-queue.sh).
//
// GenMC ships a C <pthread.h>: it declares what a C program under test calls and
// not what libstdc++'s `gthr-default.h` needs. <atomic> and <memory> pull that
// header in, so a C++ source that includes the queue header stops in clang with
// 21 errors before GenMC has seen a line of it (measured 2026-10-11, GenMC
// v0.19.0, libstdc++ 14): three types, seventeen functions and one call whose
// argument types the C declarations do not match. Nothing here is ever called.
// The queue uses `std::atomic`, which clang lowers to the same atomic
// instructions GenMC models, and none of libstdc++'s threads, locks or condition
// variables, so declaring them is enough, and the code GenMC explores is the code
// the runtime ships and not a model of it.
//
// Declarations only. The three symbols the interpreter must be able to resolve
// are DEFINED in genmc_cxx_support.h, which a harness includes after its own
// standard headers.

#ifndef SCE_FORGE_GENMC_CXX_SHIM_H
#define SCE_FORGE_GENMC_CXX_SHIM_H

#include <pthread.h>

extern "C" {

typedef unsigned int pthread_key_t;
typedef int pthread_once_t;

typedef struct {
    long opaque;
} pthread_cond_t;

int pthread_key_create(pthread_key_t *, void (*)(void *));
int pthread_key_delete(pthread_key_t);
void *pthread_getspecific(pthread_key_t);
int pthread_setspecific(pthread_key_t, const void *);
int pthread_once(pthread_once_t *, void (*)(void));
int pthread_equal(pthread_t, pthread_t);
int pthread_detach(pthread_t);
int pthread_cond_wait(pthread_cond_t *, pthread_mutex_t *);
int pthread_cond_timedwait(pthread_cond_t *, pthread_mutex_t *, const void *);
int pthread_cond_signal(pthread_cond_t *);
int pthread_cond_broadcast(pthread_cond_t *);
int pthread_cond_destroy(pthread_cond_t *);
int pthread_mutex_timedlock(pthread_mutex_t *, const void *);
int pthread_mutexattr_init(pthread_mutexattr_t *);
int pthread_mutexattr_settype(pthread_mutexattr_t *, int);
int pthread_mutexattr_destroy(pthread_mutexattr_t *);
int sched_yield(void);

}  // extern "C"

#endif  // SCE_FORGE_GENMC_CXX_SHIM_H
