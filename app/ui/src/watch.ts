// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// How the screen is woken to ask whether the work moved under it.
//
// The screen asks the core (`read_work_heads`) and does not wait to be told: the same
// question goes through the desktop shell and the browser shell, so neither needs a
// permission, a stream or a file watcher the other does not, and a change that was missed
// is found by the next question instead of being lost with a notice.

/** A timer the screen does not own, so a test can hold it and let it go. */
export interface Ticker {
  /** Call `run` once after `ms` milliseconds. The function it returns cancels the call. */
  after(ms: number, run: () => void): () => void;
}

export const browserTicker: Ticker = {
  after(ms, run) {
    const timer = setTimeout(run, ms);
    return () => clearTimeout(timer);
  },
};

/** How long the screen waits between two questions while the core answers. */
export const WATCH_MS = 2000;

/** The longest it waits after the core has failed to answer several times in a row. */
export const WATCH_MAX_MS = 30000;

/** The wait after a question that failed: twice the last one, up to [`WATCH_MAX_MS`]. */
export function nextDelay(afterFailure: number): number {
  return Math.min(afterFailure * 2, WATCH_MAX_MS);
}
