/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * The hosted tests' `irq_state_t`, declared the way a target with its own saved
 * interrupt state declares it (SCE Protocol-Synthesis RFC §synth-5-I names the
 * type and leaves it to the target): the type, and SCE_IRQ_STATE_T_DEFINED so
 * that sce/forge/queue_irq.h does not declare its `uint32_t` default as well.
 * The tests include this header first, so the platform path is the one they run.
 */

#ifndef SCE_FORGE_TEST_HOST_IRQ_H
#define SCE_FORGE_TEST_HOST_IRQ_H

#include <stdint.h>

typedef uint32_t irq_state_t;
#define SCE_IRQ_STATE_T_DEFINED

#endif /* SCE_FORGE_TEST_HOST_IRQ_H */
