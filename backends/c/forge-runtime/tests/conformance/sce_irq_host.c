/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * A hosted stand-in for `sce_irq_save` / `sce_irq_restore` (SCE Protocol-Synthesis
 * RFC §synth-5-I, interrupt control), for the tests of the interrupt-masked queue
 * (sce/forge/queue_irq.h). "Interrupts masked" is "the one lock held": a hosted
 * thread that takes the lock excludes the others exactly as a masked interrupt
 * excludes its handler on the single core the row is supported on. It is a test
 * double and nothing else; a target with interrupts supplies the real pair.
 */

#include <pthread.h>

#include "host_irq.h"

static pthread_mutex_t g_irq_lock = PTHREAD_MUTEX_INITIALIZER;

irq_state_t sce_irq_save(void) {
    (void)pthread_mutex_lock(&g_irq_lock);
    return 0u;
}

void sce_irq_restore(irq_state_t p0) {
    (void)p0;
    (void)pthread_mutex_unlock(&g_irq_lock);
}
