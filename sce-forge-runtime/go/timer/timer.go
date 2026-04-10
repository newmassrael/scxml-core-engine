// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

// Package timer provides the Timer HAL — an interface that the user
// implements once per platform. See SCE_FORGE.md Section 4.10.
//
// Go has no embedded heap constraint, so the callback shape is a plain
// closure (`func()`) that naturally captures user state — there is no need
// for the C-style function-pointer + context pattern used by the C++ and
// Rust runtimes. The behavioural contract is identical: the user supplies a
// concrete Timer implementation that schedules callbacks against the host
// platform's clock and event loop.
package timer

// Callback is the function value invoked when a timer fires.
type Callback func()

// Timer is the platform timer interface.
//
// Implementations must be re-entrant safe: a previously scheduled callback
// must not run after Cancel returns. Timer object lifetime is owned by the
// user; generated code holds references and never destroys them.
type Timer interface {
	StartPeriodic(intervalMs uint32, callback Callback)
	StartOneShot(delayMs uint32, callback Callback)
	Cancel()
}
