// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// What the model panel is in, and the few pure facts it needs.
//
// The screen draws nothing of a model. SCE does (`figures`), and what comes back
// is an SVG the screen puts in an `<img>`: an image never runs script, so a model
// whose text carried markup could not become behaviour in the page, whatever the
// product wrote. This file holds the states the panel moves through and how a
// failure to draw is told from every other failure; `app.ts` holds the order of
// events around them.

import type { Figures, ModelText, Revision, Standing } from "./contract";
import { CommandFailure } from "./ipc";

/** The model a work has, and how it stands to the text now. */
export interface ModelRead {
  readonly model: ModelText;
  readonly standing: Standing;
  /** The text revision that is current, which `standing` was judged against. */
  readonly sourceHead: Revision | null;
}

/** Why SCE did not draw a model that exists. */
export interface DrawFailure {
  /** The core's kind: `sce-refused`, `sce-unavailable`, `sce-timeout` or `sce-failed`. */
  readonly kind: string;
  /** The product's own sentence, or the core's when the product said nothing. */
  readonly message: string;
  /** The product's code (`cli/diagram-does-not-fit`), when it gave one. */
  readonly code: string | null;
}

/** Where the panel is. Each state keeps what the person can still see in it. */
export type ModelPanel =
  | { readonly phase: "reading" }
  | { readonly phase: "none" }
  /** The model is known; SCE is drawing it. */
  | { readonly phase: "drawing"; readonly read: ModelRead }
  | { readonly phase: "drawn"; readonly read: ModelRead; readonly figures: Figures }
  /** SCE would not, or could not: the model is still shown, as text. */
  | { readonly phase: "not-drawn"; readonly read: ModelRead; readonly failure: DrawFailure }
  /** The model could not be read at all. */
  | { readonly phase: "failed"; readonly message: string };

/** The kinds the core gives when the model exists and was not drawn. */
const NOT_DRAWN = new Set(["sce-refused", "sce-unavailable", "sce-timeout", "sce-failed"]);

/**
 * `error` as a failure to draw, or `null` when it is something else (the server
 * unreachable, a token wanted, a work that is gone), which the screen reports the
 * way it reports those everywhere.
 */
export function drawFailureOf(error: unknown): DrawFailure | null {
  if (!(error instanceof CommandFailure) || !NOT_DRAWN.has(error.kind)) return null;
  const detail = error.detail;
  const code =
    typeof detail === "object" && detail !== null && "code" in detail && typeof detail.code === "string"
      ? detail.code
      : null;
  return { kind: error.kind, message: error.message, code };
}

/**
 * `svg` as an address an `<img>` can load. Percent-encoded and not base64, so the
 * address is plain text a test can read and no encoder is needed in the page.
 */
export function svgAddress(svg: string): string {
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
}

/**
 * The sizes a drawing can be shown at, as multiples of the size SCE set it at. A
 * figure is set at a minimum type size meant for paper, which on a screen is
 * small, so it is shown larger by default and scrolled sideways rather than
 * shrunk to fit.
 */
export const ZOOMS: readonly number[] = [1, 1.5, 2, 3];

export const DEFAULT_ZOOM = 1.5;

/** The kept size, if it is one the screen offers; the default otherwise. */
export function zoomFrom(saved: string | null): number {
  const value = Number(saved);
  return saved !== null && ZOOMS.includes(value) ? value : DEFAULT_ZOOM;
}

/** The text a sheet is named by: the file the product wrote, without its extension. */
export function sheetName(file: string): string {
  return file.replace(/\.svg$/i, "");
}
