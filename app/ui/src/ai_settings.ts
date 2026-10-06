// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The AI connection settings: what the person sees of Claude Code and the one connection they
// keep to it, and what pressing a button sends.
//
// The state is kept here and not in the screen's own, for the reason the screen redraws
// everything it shows: a model chosen and not yet saved, or a panel left open, must survive
// the redraw that a model arriving or a request ending causes. What is decided is not decided
// here: whether a way of signing in is used is the build's table (the core says `usable`), and
// who may change the settings is the entrance (the core refuses). This only does not offer
// what it knows would be refused, and says what the core said.
//
// The application starts no sign-in. A person signs in to the official client themselves, in a
// terminal, with a command shown here to copy; the screen asks again when they say they have.

import {
  CLAUDE_CONNECTION_ID,
  claudeConnection,
  connectionForRequest,
  modelChoices,
  readinessOf,
  type Asked,
  type Readiness,
} from "./ai_settings_model";
import type { Api, ConnectionRef } from "./api";
import type {
  Billing,
  Candidate,
  Connection,
  ConnectionListing,
  Described,
  SignInCommand,
} from "./contract";
import { h } from "./dom";
import type { Child } from "./dom";
import type { Key } from "./i18n";
import { CommandFailure } from "./ipc";

/** What the settings need of the screen they are drawn in. */
export interface AiSettingsHost {
  readonly api: Pick<
    Api,
    "listConnections" | "saveConnection" | "setDefaultConnection" | "readClaudeStatus" | "findClients"
  >;
  readonly described: Described;
  readonly t: (key: Key, values?: Record<string, string>) => string;
  /** Draw again: the settings changed what they show. */
  readonly redraw: () => void;
  /** Whether `error` was the server asking for its token, which the screen takes over from here. */
  readonly handled: (error: unknown) => boolean;
  /** An error in words. */
  readonly explain: (error: unknown) => string;
  /** Put text on the clipboard. Absent where there is none, and no button offers it. */
  readonly copy?: ((text: string) => Promise<void>) | undefined;
}

interface Notice {
  readonly tone: "ok" | "warn";
  readonly text: string;
}

export class AiSettings {
  private asked: Asked = { phase: "idle" };
  private listing: ConnectionListing | null = null;
  private busy = false;
  private notice: Notice | null = null;
  private opened = false;
  /** The model chosen in the list and not yet saved; `undefined` is the one that is kept. */
  private draftModel: string | null | undefined = undefined;
  /** What was typed in the field for another model id. */
  private typedModel = "";
  /** The programs the application found, as of the last time it looked. */
  private candidates: readonly Candidate[] = [];
  /** The program chosen in the list and not yet saved; `undefined` is the one that is kept. */
  private draftProgram: string | null | undefined = undefined;
  /** The command the person last pressed copy on, and whether the clipboard took it. */
  private copied: { readonly command: string; readonly ok: boolean } | null = null;

  constructor(private readonly host: AiSettingsHost) {}

  /** Read the connections and ask Claude Code, when this window may. */
  async load(): Promise<void> {
    await this.reload();
    await this.ask();
  }

  /** Read the connections again: another window may have changed them. */
  async reload(): Promise<void> {
    if (!this.host.described.settings) return;
    try {
      this.listing = await this.host.api.listConnections();
    } catch (error) {
      if (this.host.handled(error)) return;
      this.notice = { tone: "warn", text: this.host.explain(error) };
    }
  }

  /** Ask Claude Code who is signed in. Never from a window that may not start a program. */
  async ask(): Promise<void> {
    if (!this.host.described.starts_programs) return;
    this.asked = { phase: "asking" };
    this.copied = null;
    this.host.redraw();
    try {
      this.asked = { phase: "answered", status: await this.host.api.readClaudeStatus() };
    } catch (error) {
      if (this.host.handled(error)) {
        this.asked = { phase: "idle" };
        return;
      }
      this.asked =
        error instanceof CommandFailure
          ? { phase: "refused", kind: error.kind, message: error.message }
          : { phase: "refused", kind: "failed", message: this.host.explain(error) };
    }
    await this.find();
    this.host.redraw();
  }

  /**
   * Ask which programs the application finds, for the person to choose among. Asked with the
   * login, because the two questions are the same one to a person who has just installed one:
   * is it there. A refusal leaves the choice to the application, which is what none found is.
   */
  private async find(): Promise<void> {
    try {
      this.candidates = await this.host.api.findClients();
    } catch (error) {
      if (this.host.handled(error)) return;
      this.candidates = [];
    }
  }

  openPanel(): void {
    this.opened = true;
  }

  /** The connection a request is made for, at the revision read: the default one. */
  connectionForRequest(): ConnectionRef | null {
    return connectionForRequest(this.listing);
  }

  /** What is said beside the generate button: which AI the request will be made for. */
  targetLine(): string {
    const target = this.targetName();
    return target === null
      ? this.host.t("generationWithNone")
      : this.host.t("generationWith", { target });
  }

  /** The name of the default connection, for a line; `null` when there is none. */
  private targetName(): string | null {
    const listing = this.listing;
    const stored = listing?.connections.find((c) => c.connection.id === listing.default);
    if (stored === undefined) return null;
    const { adapter, model, display_name } = stored.connection;
    const modelWords = model ?? this.host.t("aiModelDefault");
    switch (adapter) {
      case "claude-code":
        return `Claude Code (${modelWords})`;
      case "codex":
        return `Codex (${modelWords})`;
      case "local":
        return `${display_name ?? stored.connection.id} (${modelWords})`;
    }
  }

  // ---- what a button sends ----------------------------------------------

  private async save(): Promise<void> {
    if (this.busy || !this.host.described.writes_settings) return;
    this.busy = true;
    this.notice = null;
    this.host.redraw();
    let saved = false;
    try {
      const kept = claudeConnection(this.listing);
      const connection: Connection = {
        id: CLAUDE_CONNECTION_ID,
        adapter: "claude-code",
        display_name: null,
        // What is kept stays unless the person chose another: a save of the model is not a
        // word about the program.
        executable: this.chosenProgram(),
        model: this.chosenModel(),
        auth: "official-login",
        server_url: null,
        // What a person set outside this screen is theirs: saving a model does not drop it.
        limits: kept?.connection.limits ?? { turns: null, seconds: null },
      };
      await this.host.api.saveConnection(connection, kept?.revision ?? null);
      await this.host.api.setDefaultConnection(CLAUDE_CONNECTION_ID, this.listing?.default ?? null);
      this.draftModel = undefined;
      this.typedModel = "";
      this.draftProgram = undefined;
      this.notice = { tone: "ok", text: this.host.t("aiSaved") };
      saved = true;
    } catch (error) {
      if (this.host.handled(error)) return;
      if (error instanceof CommandFailure && (error.kind === "conflict" || error.kind === "moved")) {
        // Somebody else saved first: what is kept now is shown, and nothing of theirs is overwritten.
        this.notice = { tone: "warn", text: this.host.t("aiConflict") };
      } else {
        this.notice = { tone: "warn", text: this.host.t("aiSaveFailed", { detail: this.host.explain(error) }) };
      }
    } finally {
      await this.reload();
      this.busy = false;
      this.host.redraw();
    }
    // What is shown is of the program that is named now, which may be another one.
    if (saved) await this.ask();
  }

  private async copy(command: string): Promise<void> {
    const copy = this.host.copy;
    if (copy === undefined) return;
    try {
      await copy(command);
      this.copied = { command, ok: true };
    } catch {
      this.copied = { command, ok: false };
    }
    this.host.redraw();
  }

  /** The program that would be saved: the one chosen in the list, else the one kept, else none. */
  private chosenProgram(): string | null {
    if (this.draftProgram !== undefined) return this.draftProgram;
    return claudeConnection(this.listing)?.connection.executable ?? null;
  }

  /** The model that would be saved: a typed id wins over the list, and `null` is the client's own. */
  private chosenModel(): string | null {
    const typed = this.typedModel.trim();
    if (typed !== "") return typed;
    if (this.draftModel !== undefined) return this.draftModel;
    return claudeConnection(this.listing)?.connection.model ?? null;
  }

  // ---- what is drawn -----------------------------------------------------

  /** The panel, or `null` when this window has neither settings to read nor a program to ask. */
  view(): HTMLElement | null {
    const { described, t } = this.host;
    if (!described.settings && !described.starts_programs) return null;
    const readiness = readinessOf(described, this.asked);
    const target = this.targetName();
    return h(
      "details",
      {
        id: "ai-settings",
        class: "ai-settings",
        open: this.opened,
        ontoggle: (event) => {
          this.opened = (event.target as HTMLDetailsElement).open;
        },
      },
      h("summary", {}, `${t("aiTitle")}: ${target ?? t("aiNoneChosen")}`),
      ...this.body(readiness),
      this.notice === null
        ? null
        : h(
            "p",
            { class: this.notice.tone === "ok" ? "banner banner-ok" : "banner banner-warn", role: "status" },
            this.notice.text,
          ),
    );
  }

  private body(readiness: Readiness): Child[] {
    const t = this.host.t;
    switch (readiness.kind) {
      case "not-here":
        return [h("p", { class: "muted" }, t("aiNotHere"))];
      case "unasked":
        return [h("p", { class: "muted" }, t("aiNotAsked")), this.recheck("aiAsk")];
      case "asking":
        return [h("p", { class: "muted", role: "status" }, t("aiAsking"))];
      case "no-client":
        return [h("p", {}, t("aiNoClient")), this.recheck("aiRecheck")];
      case "client-unverified":
        return [h("p", {}, t("aiClientUnverified")), this.program(), this.recheck("aiRecheck")];
      case "unknown":
        return [
          h("p", {}, t("aiUnknown", { reason: readiness.reason })),
          this.program(),
          this.recheck("aiRecheck"),
        ];
      case "signed-out":
        return [
          h("p", {}, t("aiSignedOut")),
          this.commands(readiness.commands),
          this.program(),
          this.recheck("aiRecheck"),
        ];
      case "not-used":
        return [
          h("p", {}, t("aiNotUsed", { reason: t(NOT_USED_WORDS[readiness.reason]) })),
          this.commands(this.signInCommands()),
          this.program(),
          this.recheck("aiRecheck"),
        ];
      case "ready":
        return this.ready(readiness);
    }
  }

  /**
   * Which Claude Code this connection runs: the application's own choice, or one of the programs
   * it found. A program the connection names that is not found now stays in the list, said to be
   * gone, so that a save does not drop it unseen and the person can see why a run waits.
   */
  private programChoice(): HTMLElement {
    const t = this.host.t;
    const current = this.chosenProgram();
    const known = new Set(this.candidates.map((c) => c.path));
    const select = h(
      "select",
      {
        id: "ai-program",
        "aria-label": t("aiProgram"),
        onchange: (event) => {
          const value = (event.target as HTMLSelectElement).value;
          this.draftProgram = value === "" ? null : value;
        },
      },
      h("option", { value: "", selected: current === null }, t("aiProgramAutomatic")),
      ...this.candidates.map((c) =>
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

  /**
   * The program choice with a button that keeps it, for the states where nobody is signed in to
   * the program that answered, which is when another one is what the person wants. Where a model
   * can be chosen the choice is in that state's own save.
   */
  private program(): HTMLElement {
    const t = this.host.t;
    return h(
      "div",
      { class: "ai-program-block" },
      this.programChoice(),
      this.host.described.writes_settings
        ? h(
            "button",
            { id: "ai-save-program", type: "button", disabled: this.busy, onclick: () => void this.save() },
            t("aiUseProgram"),
          )
        : null,
    );
  }

  /** The commands the core gave the last time it was asked; none before it has been. */
  private signInCommands(): readonly SignInCommand[] {
    return this.asked.phase === "answered" ? this.asked.status.sign_in : [];
  }

  private recheck(label: Key): HTMLElement {
    return h(
      "button",
      { id: "ai-recheck", type: "button", class: "quiet", onclick: () => void this.ask() },
      this.host.t(label),
    );
  }

  /** The commands that sign in, each with how it bills and a button to copy it. */
  private commands(commands: readonly SignInCommand[]): HTMLElement {
    const t = this.host.t;
    return h(
      "ul",
      { class: "ai-commands" },
      ...commands.map((entry) =>
        h(
          "li",
          {},
          h("span", { class: "muted" }, `${t(SIGN_IN_WORDS[entry.billing])} `),
          h("code", {}, entry.command),
          " ",
          this.host.copy === undefined
            ? null
            : h(
                "button",
                {
                  type: "button",
                  class: "quiet",
                  "data-copy": entry.command,
                  onclick: () => void this.copy(entry.command),
                },
                t("aiCopy"),
              ),
          this.copied?.command === entry.command
            ? h("span", { class: "muted", role: "status" }, ` ${t(this.copied.ok ? "aiCopied" : "aiCopyFailed")}`)
            : null,
        ),
      ),
    );
  }

  private ready(readiness: Extract<Readiness, { kind: "ready" }>): Child[] {
    const t = this.host.t;
    const kept = claudeConnection(this.listing);
    // The list shows what is chosen in it; a model id typed beside it wins when it is saved.
    const current = this.draftModel !== undefined ? this.draftModel : (kept?.connection.model ?? null);
    const select = h(
      "select",
      {
        id: "ai-model",
        "aria-label": t("aiModel"),
        onchange: (event) => {
          const value = (event.target as HTMLSelectElement).value;
          this.draftModel = value === "" ? null : value;
          this.typedModel = "";
        },
      },
      ...modelChoices(kept?.connection.model ?? null).map((choice) =>
        h(
          "option",
          { value: choice.model ?? "", selected: choice.model === current },
          choice.model === null ? t("aiModelDefault") : choice.model,
        ),
      ),
    );
    const typed = h("input", {
      id: "ai-model-id",
      type: "text",
      maxlength: "200",
      spellcheck: "false",
      autocomplete: "off",
      "aria-label": t("aiModelId"),
      placeholder: t("aiModelId"),
      oninput: (event) => {
        this.typedModel = (event.target as HTMLInputElement).value;
      },
    });
    typed.value = this.typedModel;
    return [
      h(
        "dl",
        { class: "ai-facts" },
        h("dt", {}, t("aiClient")),
        h("dd", {}, t("aiClientInstalled", { version: readiness.version, path: readiness.path })),
        h("dt", {}, t("aiAccount")),
        h("dd", {}, t("aiAccountSignedIn")),
        h("dt", {}, t("aiAuth")),
        h(
          "dd",
          {},
          t(BILLING_WORDS[readiness.billing]),
          readiness.environment === null
            ? null
            : h(
                "div",
                { class: "muted" },
                t("aiFromEnvironment", { name: readiness.environment }),
                " ",
                t("aiEnvironmentNote"),
              ),
        ),
        h("dt", {}, t("aiModel")),
        h("dd", {}, select),
      ),
      h(
        "details",
        { class: "advanced" },
        h("summary", {}, t("aiAdvanced")),
        this.programChoice(),
        h("p", { class: "muted" }, t("aiModelNote")),
        typed,
      ),
      this.host.described.writes_settings
        ? h(
            "div",
            { class: "choices" },
            h(
              "button",
              { id: "ai-save", type: "button", disabled: this.busy, onclick: () => void this.save() },
              t(this.busy ? "aiSaving" : "aiSave"),
            ),
            this.recheck("aiRecheck"),
          )
        : this.recheck("aiRecheck"),
    ];
  }
}

const SIGN_IN_WORDS: Record<Billing, Key> = {
  subscription: "aiSignInSubscription",
  usage: "aiSignInUsage",
  provider: "aiSignInProvider",
};

const BILLING_WORDS: Record<Billing, Key> = {
  subscription: "aiBillingSubscription",
  usage: "aiBillingUsage",
  provider: "aiBillingProvider",
};

const NOT_USED_WORDS: Record<"forbidden" | "unconfirmed" | "switched-off", Key> = {
  forbidden: "aiReasonForbidden",
  unconfirmed: "aiReasonUnconfirmed",
  "switched-off": "aiReasonSwitchedOff",
};
