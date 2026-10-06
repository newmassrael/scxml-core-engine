// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The part of the AI settings that is Codex: what the core says of it, which source its credential
// comes from, and the one connection to it that the settings keep.
//
// Codex differs from Claude Code in two ways the screen must not hide. A connection to it takes its
// credential from one of three sources (the official client's login, a login the application keeps
// of its own, or a key in the environment), and each is asked on its own. And until this build has
// verified the version that is installed, a request made for Codex never runs, whoever is signed
// in: so the screen says, in one line, whether a request made for this connection would run and,
// if not, what it waits for, and does not offer to make a connection the default that would only
// wait. What is decided is not decided here: the core says what is usable and what is verified.

import {
  CODEX_CONNECTION_ID,
  CODEX_SOURCES,
  accountOf,
  chosenSource,
  codexConnection,
  codexReadinessOf,
  outlookOf,
  type Asked,
  type Because,
  type Outlook,
} from "./ai_settings_model";
import { BILLING_WORDS, NOT_USED_WORDS, programChoice } from "./ai_settings_view";
import type {
  Candidate,
  ClaudeClient,
  CodexSource,
  CodexStatus,
  Connection,
  ConnectionListing,
  Described,
  SignInCommand,
} from "./contract";
import { h } from "./dom";
import type { Child } from "./dom";
import type { Key } from "./i18n";

/** What the Codex section needs of the settings it is drawn in. */
export interface CodexSectionEnv {
  readonly t: (key: Key, values?: Record<string, string>) => string;
  readonly described: Described;
  /** The connections as last read. */
  readonly listing: () => ConnectionListing | null;
  readonly busy: () => boolean;
  /** The person pressed a save. */
  readonly save: () => void;
  /** Draw again: what is chosen changes what is shown. */
  readonly redraw: () => void;
  /** The button that asks again, with the words `label`. */
  readonly recheck: (label: Key) => HTMLElement;
  /** The commands that sign in, each with how it bills and a button to copy it. */
  readonly commands: (commands: readonly SignInCommand[]) => HTMLElement;
}

const SOURCE_WORDS: Record<CodexSource, Key> = {
  "official-login": "aiSourceOfficial",
  "app-store": "aiSourceStore",
  "env-api-key": "aiSourceKey",
};

const WHY_WORDS: Record<Because, Key> = {
  "no-client": "aiWhyNoClient",
  "client-unverified": "aiWhyClientUnverified",
  unsupported: "aiWhyUnsupported",
  "support-unknown": "aiWhySupportUnknown",
  "signed-out": "aiWhySignedOut",
  "not-used": "aiWhyNotUsed",
  "account-unknown": "aiWhyAccountUnknown",
};

export class CodexSection {
  private asked: Asked<CodexStatus> = { phase: "idle" };
  /** The Codex programs the application found, as of the last time it looked. */
  private candidates: readonly Candidate[] = [];
  /** The model typed and not yet saved; `undefined` is the one that is kept. */
  private draftModel: string | undefined = undefined;
  /** The source chosen in the list and not yet saved; `undefined` is the one that is kept. */
  private draftSource: CodexSource | undefined = undefined;
  /** The program chosen in the list and not yet saved; `undefined` is the one that is kept. */
  private draftProgram: string | null | undefined = undefined;

  constructor(private readonly env: CodexSectionEnv) {}

  /** Whether the core has not been asked yet: the screen asks the first time Codex is chosen. */
  get unasked(): boolean {
    return this.asked.phase === "idle";
  }

  setAsked(asked: Asked<CodexStatus>): void {
    this.asked = asked;
  }

  setCandidates(candidates: readonly Candidate[]): void {
    this.candidates = candidates;
  }

  /** Whether this window can say anything of Codex: the settings are the desktop window's. */
  get here(): boolean {
    return codexReadinessOf(this.env.described, this.asked).kind !== "not-here";
  }

  /** The connection a save would write: what is chosen, over what is kept, over nothing. */
  connection(): Connection {
    const kept = codexConnection(this.env.listing());
    return {
      id: CODEX_CONNECTION_ID,
      adapter: "codex",
      display_name: null,
      executable: this.chosenProgram(),
      model: this.chosenModel(),
      auth: chosenSource(this.status(), kept, this.draftSource),
      server_url: null,
      // What a person set outside this screen is theirs: saving a model does not drop it.
      limits: kept?.connection.limits ?? { turns: null, seconds: null },
    };
  }

  /** What was saved is what is kept now: nothing is left of what was chosen. */
  reset(): void {
    this.draftModel = undefined;
    this.draftSource = undefined;
    this.draftProgram = undefined;
  }

  private status(): CodexStatus | null {
    return this.asked.phase === "answered" ? this.asked.status : null;
  }

  /** The program that would be saved: the one chosen in the list, else the one kept, else none. */
  private chosenProgram(): string | null {
    if (this.draftProgram !== undefined) return this.draftProgram;
    return codexConnection(this.env.listing())?.connection.executable ?? null;
  }

  /** The model that would be saved: what was typed, else the one kept; empty is the client's own. */
  private chosenModel(): string | null {
    const typed = this.draftModel ?? codexConnection(this.env.listing())?.connection.model ?? "";
    const model = typed.trim();
    return model === "" ? null : model;
  }

  // ---- what is drawn -----------------------------------------------------

  body(): Child[] {
    const t = this.env.t;
    const readiness = codexReadinessOf(this.env.described, this.asked);
    switch (readiness.kind) {
      case "not-here":
        return [h("p", { class: "muted" }, t("aiNotHere"))];
      case "unasked":
        return [h("p", { class: "muted" }, t("aiCodexNotAsked")), this.env.recheck("aiCodexAsk")];
      case "asking":
        return [h("p", { class: "muted", role: "status" }, t("aiCodexAsking"))];
      case "unknown":
        return [h("p", {}, t("aiCodexUnknown", { reason: readiness.reason })), this.env.recheck("aiRecheck")];
      case "answered":
        return this.answered(readiness.status);
    }
  }

  private answered(status: CodexStatus): Child[] {
    const t = this.env.t;
    const client = status.client;
    if (client.state === "missing") return [h("p", {}, t("aiCodexNoClient")), this.env.recheck("aiRecheck")];
    if (client.state === "unverified") {
      return [h("p", {}, t("aiCodexClientUnverified")), this.programBlock(), this.env.recheck("aiRecheck")];
    }
    const kept = codexConnection(this.env.listing());
    const source = chosenSource(status, kept, this.draftSource);
    const outlook = outlookOf(status, source);
    return [
      this.facts(status, client, source, outlook.runs),
      ...(outlook.runs ? [h("p", { class: "muted" }, t("aiCodexModelNote"))] : []),
      this.outlookLine(outlook),
      ...this.guidance(status, source),
      ...(outlook.runs ? this.controls() : [this.programBlock(), this.env.recheck("aiRecheck")]),
    ];
  }

  private facts(
    status: CodexStatus,
    client: Extract<ClaudeClient, { state: "installed" }>,
    source: CodexSource,
    runs: boolean,
  ): HTMLElement {
    const t = this.env.t;
    const account = accountOf(status, source);
    const rows: Child[] = [
      h("dt", {}, t("aiClient")),
      h("dd", {}, t("aiClientInstalled", { version: client.version, path: client.path })),
      h("dt", {}, t("aiSupport")),
      h("dd", {}, this.supportWords(status)),
      h("dt", {}, t("aiSource")),
      h("dd", {}, this.sourceChoice(source)),
    ];
    if (account?.state === "signed-in" && account.billing !== null) {
      rows.push(
        h("dt", {}, t("aiAuth")),
        h(
          "dd",
          {},
          t(BILLING_WORDS[account.billing]),
          account.environment === null
            ? null
            : h("div", { class: "muted" }, t("aiFromEnvironment", { name: account.environment })),
        ),
      );
    }
    if (runs) rows.push(h("dt", {}, t("aiModel")), h("dd", {}, this.modelField()));
    return h("dl", { class: "ai-facts" }, ...rows);
  }

  private supportWords(status: CodexStatus): string {
    const t = this.env.t;
    switch (status.support.state) {
      case "verified":
        return t("aiSupportVerified");
      case "unverified":
        return t("aiSupportUnverified", { reason: status.support.reason });
      case "unknown":
        return t("aiSupportUnknown", { reason: status.support.reason });
    }
  }

  /** Which of the three places the credential comes from; changing it changes what is shown of the account. */
  private sourceChoice(chosen: CodexSource): HTMLElement {
    const t = this.env.t;
    return h(
      "select",
      {
        id: "ai-codex-source",
        "aria-label": t("aiSource"),
        onchange: (event) => {
          this.draftSource = (event.target as HTMLSelectElement).value as CodexSource;
          this.env.redraw();
        },
      },
      ...CODEX_SOURCES.map((source) =>
        h("option", { value: source, selected: source === chosen }, t(SOURCE_WORDS[source])),
      ),
    );
  }

  private modelField(): HTMLElement {
    const t = this.env.t;
    const kept = codexConnection(this.env.listing())?.connection.model ?? "";
    const field = h("input", {
      id: "ai-codex-model",
      type: "text",
      maxlength: "200",
      spellcheck: "false",
      autocomplete: "off",
      "aria-label": t("aiModelId"),
      placeholder: t("aiModelDefault"),
      oninput: (event) => {
        this.draftModel = (event.target as HTMLInputElement).value;
      },
    });
    field.value = this.draftModel ?? kept;
    return field;
  }

  /** One line: whether a request made for this connection would run, and if not what it waits for. */
  private outlookLine(outlook: Outlook): HTMLElement {
    const t = this.env.t;
    return outlook.runs
      ? h("p", { class: "banner banner-ok ai-outlook", role: "status" }, t("aiOutlookRuns"))
      : h(
          "p",
          { class: "banner banner-warn ai-outlook", role: "status" },
          t("aiOutlookWaits", { why: t(WHY_WORDS[outlook.because]) }),
        );
  }

  /** What to do about the account of the source that is chosen, when it is not one a request can run on. */
  private guidance(status: CodexStatus, source: CodexSource): Child[] {
    const t = this.env.t;
    const account = accountOf(status, source);
    if (account === null) return [];
    const entries = status.sign_in.filter((entry) => entry.source === source);
    const home = entries.find((entry) => entry.home !== null)?.home ?? null;
    const homeNote = home === null ? [] : [h("p", { class: "muted" }, t("aiCodexHome", { home }))];
    switch (account.state) {
      case "unknown":
        return [h("p", {}, t("aiCodexAccountUnknown", { reason: account.reason }))];
      case "signed-out":
        // A key in the environment is not signed in to: it is set where the application starts.
        return source === "env-api-key"
          ? [h("p", {}, t("aiCodexSignedOutKey", { name: status.key_variable }))]
          : [h("p", {}, t("aiCodexSignedOutLogin")), this.env.commands(entries), ...homeNote];
      case "signed-in": {
        if (account.usable) return [];
        const reason = account.decision.decision === "refuse" ? account.decision.reason : "unconfirmed";
        return [
          h("p", {}, t("aiCodexNotUsed", { reason: t(NOT_USED_WORDS[reason]) })),
          this.env.commands(entries),
          ...homeNote,
        ];
      }
    }
  }

  private programSelect(): HTMLElement {
    return programChoice({
      id: "ai-codex-program",
      t: this.env.t,
      current: this.chosenProgram(),
      candidates: this.candidates,
      choose: (program) => {
        this.draftProgram = program;
      },
    });
  }

  /**
   * The program choice with a button that keeps it, for the states where a request would not run,
   * which is when another program is what the person wants: a version this build verified may be
   * another one.
   */
  private programBlock(): HTMLElement {
    const t = this.env.t;
    return h(
      "div",
      { class: "ai-program-block" },
      this.programSelect(),
      this.env.described.writes_settings
        ? h(
            "button",
            { id: "ai-save-program", type: "button", disabled: this.env.busy(), onclick: () => this.env.save() },
            t("aiUseProgram"),
          )
        : null,
    );
  }

  /** The program choice and the save, for a connection a request would run on. */
  private controls(): Child[] {
    const t = this.env.t;
    return [
      h("details", { class: "advanced" }, h("summary", {}, t("aiProgram")), this.programSelect()),
      this.env.described.writes_settings
        ? h(
            "div",
            { class: "choices" },
            h(
              "button",
              { id: "ai-save", type: "button", disabled: this.env.busy(), onclick: () => this.env.save() },
              t(this.env.busy() ? "aiSaving" : "aiSave"),
            ),
            this.env.recheck("aiRecheck"),
          )
        : this.env.recheck("aiRecheck"),
    ];
  }
}
