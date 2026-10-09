/* SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial */
/* SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael */

/*
 * The hosted tests' `irq_state_t`: the platform type sce/forge/queue_irq.h
 * requires to be defined before it is included (SCE Protocol-Synthesis RFC
 * §synth-5-I names the type and leaves it to the target).
 */

#ifndef SCE_FORGE_TEST_HOST_IRQ_H
#define SCE_FORGE_TEST_HOST_IRQ_H

#include <stdint.h>

typedef uint32_t irq_state_t;

#endif /* SCE_FORGE_TEST_HOST_IRQ_H */
