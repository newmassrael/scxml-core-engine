// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// What the sections of the AI settings draw the same way: the words for how a way of signing in is
// billed and why one is not used, and the choice of which program a connection runs. Said once, so
// that the screen for one client cannot say them differently from the screen for another.

import type { Billing, Candidate } from "./contract";
import { h } from "./dom";
import type { Key } from "./i18n";

export const SIGN_IN_WORDS: Record<Billing, Key> = {
  subscription: "aiSignInSubscription",
  usage: "aiSignInUsage",
  provider: "aiSignInProvider",
};

export const BILLING_WORDS: Record<Billing, Key> = {
  subscription: "aiBillingSubscription",
  usage: "aiBillingUsage",
  provider: "aiBillingProvider",
};

export const NOT_USED_WORDS: Record<"forbidden" | "unconfirmed" | "switched-off", Key> = {
  forbidden: "aiReasonForbidden",
  unconfirmed: "aiReasonUnconfirmed",
  "switched-off": "aiReasonSwitchedOff",
};

/** What the choice of a program needs. */
export interface ProgramChoice {
  /** The id of the list, which a test and a label find it by. */
  readonly id: string;
  readonly t: (key: Key, values?: Record<string, string>) => string;
  /** The program that would be saved: the one chosen in the list, else the one kept, else none. */
  readonly current: string | null;
  /** The programs the application found, as of the last time it looked. */
  readonly candidates: readonly Candidate[];
  /** The person chose one in the list; `null` is the application's own choice. */
  readonly choose: (program: string | null) => void;
}

/**
 * Which program a connection runs: the application's own choice, or one of the programs it found.
 * A program the connection names that is not found now stays in the list, said to be gone, so that
 * a save does not drop it unseen and the person can see why a run waits.
 */
export function programChoice(options: ProgramChoice): HTMLElement {
  const { t, current, candidates } = options;
  const known = new Set(candidates.map((c) => c.path));
  const select = h(
    "select",
    {
      id: options.id,
      "aria-label": t("aiProgram"),
      onchange: (event) => {
        const value = (event.target as HTMLSelectElement).value;
        options.choose(value === "" ? null : value);
      },
    },
    h("option", { value: "", selected: current === null }, t("aiProgramAutomatic")),
    ...candidates.map((c) =>
      h(
        "option",
        { value: c.path, selected: current === c.path },
        t("aiProgramCandidate", { path: c.path, version: c.version }),
      ),
    ),
    current !== null && !known.has(current)
      ? h("option", { value: current, selected: true }, `${current} (${t("aiProgramGone")})`)
      : null,
  );
  return h("label", { class: "ai-program" }, h("span", {}, `${t("aiProgram")} `), select);
}
