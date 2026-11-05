// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

/**
 * @file ThreadSupport.h
 * @brief Whether this build can start a thread: the one question the engine's
 *        execution model is chosen by.
 *
 * The engine runs in one of two models (ARCHITECTURE.md, Platform Support):
 *
 * - threaded: the scheduler fires a delayed `<send>` on its own timer thread
 *   when it falls due (§scxml-6.2), events are processed on a worker thread,
 *   and scripts run on the script engine's worker;
 * - host-driven: nothing runs unless a thread the host owns calls in. A delayed
 *   event fires when the machine's thread polls the scheduler, or when the host
 *   moves a MANUAL-mode clock on.
 *
 * Which one a build gets is a fact about the build, not about the platform it
 * targets. A native build has threads. An Emscripten build has them exactly
 * when it is compiled with `-pthread`, which defines `__EMSCRIPTEN_PTHREADS__`:
 * the Node programs of the emscripten tree are, a browser module without
 * cross-origin isolation (no SharedArrayBuffer) is not. Choosing the model by
 * `__EMSCRIPTEN__` instead gave a Node build that had threads the host-driven
 * model, under which a delayed event could not fire while the machine's thread
 * was held.
 */
#if defined(__EMSCRIPTEN__) && !defined(__EMSCRIPTEN_PTHREADS__)
#define SCE_HAS_THREADS 0
#else
#define SCE_HAS_THREADS 1
#endif
