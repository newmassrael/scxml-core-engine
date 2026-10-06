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
  CODEX_CONNECTION_ID,
  claudeConnection,
  codexConnection,
  connectionForRequest,
  defaultKind,
  localConnection,
  modelChoices,
  readinessOf,
  type Asked,
  type ClientKind,
  type Readiness,
} from "./ai_settings_model";
import { BILLING_WORDS, NOT_USED_WORDS, SIGN_IN_WORDS, programChoice } from "./ai_settings_view";
import type { Api, ConnectionRef } from "./api";
import { CodexSection } from "./codex_settings";
import { ServerSection } from "./server_settings";
import type { Candidate, Connection, ConnectionListing, Described, SignInCommand } from "./contract";
import { h } from "./dom";
import type { Child } from "./dom";
import type { Key } from "./i18n";
import { CommandFailure } from "./ipc";

/** What the settings need of the screen they are drawn in. */
export interface AiSettingsHost {
  readonly api: Pick<
    Api,
    | "listConnections"
    | "saveConnection"
    | "setDefaultConnection"
    | "readClaudeStatus"
    | "readCodexStatus"
    | "findClients"
    | "readServerStatus"
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
  /** The client the person chose to look at; `null` is the one the default connection is for. */
  private kindDraft: ClientKind | null = null;
  /** What is Codex's in these settings, which has its own state to keep. */
  private readonly codex: CodexSection;
  /** What is a model server's in these settings, which has its own state to keep. */
  private readonly server: ServerSection;

  constructor(private readonly host: AiSettingsHost) {
    this.codex = new CodexSection({
      t: host.t,
      described: host.described,
      listing: () => this.listing,
      busy: () => this.busy,
      save: (makeDefault) => void this.save(makeDefault),
      redraw: () => host.redraw(),
      recheck: (label) => this.recheck(label),
      commands: (commands) => this.commands(commands),
    });
    this.server = new ServerSection({
      t: host.t,
      described: host.described,
      listing: () => this.listing,
      busy: () => this.busy,
      save: () => void this.save(true),
      redraw: () => host.redraw(),
      check: (address) => void this.askServer(address),
    });
  }

  /** Read the connections and ask the client that is shown, when this window may. */
  async load(): Promise<void> {
    await this.reload();
    await this.ask();
  }

  /** The client the settings show: the one the person chose, else the one the default connection is for. */
  private kind(): ClientKind {
    return this.kindDraft ?? defaultKind(this.listing);
  }

  /** The person chose a client to look at. It is asked the first time it is looked at. */
  private async choose(kind: ClientKind): Promise<void> {
    this.kindDraft = kind;
    this.copied = null;
    this.host.redraw();
    const unasked =
      kind === "codex" ? this.codex.unasked : kind === "local" ? this.server.unasked : this.asked.phase === "idle";
    if (unasked) await this.ask();
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

  /** Ask the client that is shown who is signed in. Never from a window that may not start a program. */
  async ask(): Promise<void> {
    if (!this.host.described.starts_programs) return;
    this.copied = null;
    switch (this.kind()) {
      case "codex":
        await this.askCodex();
        break;
      case "local":
        await this.askServer(this.server.address());
        break;
      case "claude-code":
        await this.askClaude();
        break;
    }
  }

  /**
   * Ask the server at `address` what it is. Nothing is asked of a server nobody named: there is
   * no address to call, and the screen says it has not been checked.
   */
  private async askServer(address: string): Promise<void> {
    if (!this.host.described.starts_programs || address === "") return;
    this.server.setAsked({ phase: "asking" });
    this.host.redraw();
    try {
      this.server.setAsked({ phase: "answered", status: await this.host.api.readServerStatus(address) });
    } catch (error) {
      if (this.host.handled(error)) {
        this.server.setAsked({ phase: "idle" });
        return;
      }
      this.server.setAsked(refusal(error, this.host));
    }
    this.host.redraw();
  }

  private async askClaude(): Promise<void> {
    this.asked = { phase: "asking" };
    this.host.redraw();
    try {
      // Asked about the connection these settings edit, which is not always the default: the
      // program that answers is the one that connection names.
      this.asked = { phase: "answered", status: await this.host.api.readClaudeStatus(CLAUDE_CONNECTION_ID) };
    } catch (error) {
      if (this.host.handled(error)) {
        this.asked = { phase: "idle" };
        return;
      }
      this.asked = refusal(error, this.host);
    }
    await this.find();
    this.host.redraw();
  }

  private async askCodex(): Promise<void> {
    this.codex.setAsked({ phase: "asking" });
    this.host.redraw();
    try {
      this.codex.setAsked({
        phase: "answered",
        status: await this.host.api.readCodexStatus(CODEX_CONNECTION_ID),
      });
    } catch (error) {
      if (this.host.handled(error)) {
        this.codex.setAsked({ phase: "idle" });
        return;
      }
      this.codex.setAsked(refusal(error, this.host));
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
      const found = await this.host.api.findClients();
      this.candidates = found.claude;
      this.codex.setCandidates(found.codex);
    } catch (error) {
      if (this.host.handled(error)) return;
      this.candidates = [];
      this.codex.setCandidates([]);
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

  /**
   * Keep the connection that is shown. `makeDefault` also makes it the default, which is the
   * person's word for which AI a request is made for: only the save that is offered when a request
   * made for the connection would run says it. Keeping the program a connection runs does not: it
   * is for a connection that would not run (nobody is signed in, a version this build did not
   * verify), and a default that only waits would leave every later request waiting for it.
   */
  private async save(makeDefault: boolean): Promise<void> {
    if (this.busy || !this.host.described.writes_settings) return;
    this.busy = true;
    this.notice = null;
    this.host.redraw();
    let saved = false;
    const kind = this.kind();
    try {
      const kept =
        kind === "codex"
          ? codexConnection(this.listing)
          : kind === "local"
            ? localConnection(this.listing)
            : claudeConnection(this.listing);
      const connection =
        kind === "codex" ? this.codex.connection() : kind === "local" ? this.server.connection() : this.claudeDraft();
      await this.host.api.saveConnection(connection, kept?.revision ?? null);
      if (makeDefault) await this.host.api.setDefaultConnection(connection.id, this.listing?.default ?? null);
      if (kind === "codex") {
        this.codex.reset();
      } else if (kind === "local") {
        this.server.reset();
      } else {
        this.draftModel = undefined;
        this.typedModel = "";
        this.draftProgram = undefined;
      }
      this.notice = { tone: "ok", text: this.host.t(makeDefault ? "aiSaved" : "aiSavedNotDefault") };
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
    // What is shown is of the program that is named now, which may be another one; a server that
    // was just kept is not asked again, because the person has just seen what it said.
    if (saved && kind !== "local") await this.ask();
  }

  /** The connection to Claude Code that a save would write: what is chosen, over what is kept. */
  private claudeDraft(): Connection {
    const kept = claudeConnection(this.listing);
    return {
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
    const kind = this.kind();
    const readiness = readinessOf(described, this.asked);
    const target = this.targetName();
    // A window that may not ask anything has no client to choose between.
    const here =
      kind === "codex" ? this.codex.here : kind === "local" ? this.server.here : readiness.kind !== "not-here";
    const chooser = here ? this.kindChooser() : null;
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
      chooser,
      ...(kind === "codex" ? this.codex.body() : kind === "local" ? this.server.body() : this.body(readiness)),
      this.notice === null
        ? null
        : h(
            "p",
            { class: this.notice.tone === "ok" ? "banner banner-ok" : "banner banner-warn", role: "status" },
            this.notice.text,
          ),
    );
  }

  /** The ways to reach a model these settings edit a connection to, one of which is shown at a time. */
  private kindChooser(): HTMLElement {
    const t = this.host.t;
    const shown = this.kind();
    const option = (kind: ClientKind, label: string) =>
      h(
        "label",
        { class: "ai-kind-option" },
        h("input", {
          id: `ai-kind-${kind}`,
          type: "radio",
          name: "ai-kind",
          value: kind,
          checked: shown === kind,
          onchange: () => void this.choose(kind),
        }),
        ` ${label}`,
      );
    return h(
      "fieldset",
      { class: "ai-kind" },
      h("legend", {}, t("aiKind")),
      option("claude-code", "Claude Code"),
      option("codex", "Codex"),
      option("local", t("aiServer")),
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
    return programChoice({
      id: "ai-program",
      t: this.host.t,
      current: this.chosenProgram(),
      candidates: this.candidates,
      choose: (program) => {
        this.draftProgram = program;
      },
    });
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
            { id: "ai-save-program", type: "button", disabled: this.busy, onclick: () => void this.save(false) },
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
              { id: "ai-save", type: "button", disabled: this.busy, onclick: () => void this.save(true) },
              t(this.busy ? "aiSaving" : "aiSave"),
            ),
            this.recheck("aiRecheck"),
          )
        : this.recheck("aiRecheck"),
    ];
  }
}

/** What a refused or failed asking is, in the words the settings keep it in. */
function refusal<S>(error: unknown, host: AiSettingsHost): Asked<S> {
  return error instanceof CommandFailure
    ? { phase: "refused", kind: error.kind, message: error.message }
    : { phase: "refused", kind: "failed", message: host.explain(error) };
}
