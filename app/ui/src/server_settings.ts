// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The part of the AI settings that is a model server the person runs (Ollama, LM Studio, llama.cpp,
// vLLM: anything that speaks the OpenAI chat protocol with tools), and the one connection to it that
// the settings keep.
//
// A server is no program to find, so nothing is installed or signed in to: the person names it, says
// where it is, and the application asks it what it is and which models it lists before the person
// saves. What is shown of where it is decided by the core (`reach`, `tls`), because the address is
// all that is known of it and the rule for reading one is the core's. What matters to the person who
// is about to press save is where a specification goes, so that is said beside the address whatever
// the answer, and said louder when it goes to another computer over a connection nobody encrypts.
//
// A server that wants a key cannot be used: this build has no place to keep one. That is said, and
// saving is not offered for an address that is known to be one, instead of saving a connection that
// only waits.

import { answeringOf, localConnection, LOCAL_CONNECTION_ID, type Asked } from "./ai_settings_model";
import { NO_LIMITS } from "./contract";
import type { ConnectionListing, Connection, Described, ServerStatus } from "./contract";
import { h } from "./dom";
import type { Child } from "./dom";
import type { Key } from "./i18n";

/** What the server section needs of the settings it is drawn in. */
export interface ServerSectionEnv {
  readonly t: (key: Key, values?: Record<string, string>) => string;
  readonly described: Described;
  /** The connections as last read. */
  readonly listing: () => ConnectionListing | null;
  readonly busy: () => boolean;
  /** The person pressed a save. */
  readonly save: () => void;
  /** Draw again: what is typed changes what is shown. */
  readonly redraw: () => void;
  /** The person asked the server what it is, at this address. */
  readonly check: (address: string) => void;
}

/** The servers a person is most likely to run, and where each listens when it is started as its makers say. */
const PRESETS: readonly { readonly name: string; readonly address: string }[] = [
  { name: "Ollama", address: "http://127.0.0.1:11434/v1" },
  { name: "LM Studio", address: "http://127.0.0.1:1234/v1" },
  { name: "llama.cpp", address: "http://127.0.0.1:8080/v1" },
  { name: "vLLM", address: "http://127.0.0.1:8000/v1" },
];

/** The id of what the server said, which a screen can bring into view. */
const STATUS_ID = "ai-server-status";

export class ServerSection {
  private asked: Asked<ServerStatus> = { phase: "idle" };
  /** The name typed and not yet saved; `undefined` is the one that is kept. */
  private draftName: string | undefined = undefined;
  /** The address typed and not yet saved; `undefined` is the one that is kept. */
  private draftAddress: string | undefined = undefined;
  /** The model typed or chosen and not yet saved; `undefined` is the one that is kept. */
  private draftModel: string | undefined = undefined;
  /** The buttons as drawn, so that typing can enable them without drawing the fields again. */
  private saveButton: HTMLButtonElement | null = null;
  private checkButtonDrawn: HTMLButtonElement | null = null;

  constructor(private readonly env: ServerSectionEnv) {}

  setAsked(asked: Asked<ServerStatus>): void {
    this.asked = asked;
  }

  /**
   * Bring what the server said into view. The panel is longer than the sidebar is tall, and a
   * button that asks (a server's button among the usual ones is above the answer) leaves the
   * answer below the fold, where a person who pressed it does not look.
   */
  reveal(): void {
    document.getElementById(STATUS_ID)?.scrollIntoView?.({ block: "nearest" });
  }

  /** Whether this window can say anything of a server: asking calls an address, which is the desktop window's. */
  get here(): boolean {
    return answeringOf(this.env.described, this.asked).kind !== "not-here";
  }

  /** The address the settings show: what was typed, else the one kept, else none. */
  address(): string {
    return (this.draftAddress ?? localConnection(this.env.listing())?.connection.server_url ?? "").trim();
  }

  /** Whether the server at the kept address has not been asked since the screen opened. */
  get unasked(): boolean {
    return this.asked.phase === "idle";
  }

  /** The connection a save would write: what is typed, over what is kept. */
  connection(): Connection {
    const kept = localConnection(this.env.listing());
    return {
      id: kept?.connection.id ?? LOCAL_CONNECTION_ID,
      adapter: "local",
      display_name: this.name(),
      executable: null,
      model: this.model() === "" ? null : this.model(),
      // A server that wants a key cannot be used by this build, so none is asked for.
      auth: "none",
      server_url: this.address(),
      // What a person set outside this screen is theirs: saving a model does not drop it.
      limits: kept?.connection.limits ?? NO_LIMITS,
    };
  }

  /** What was saved is what is kept now: nothing is left of what was typed. */
  reset(): void {
    this.draftName = undefined;
    this.draftAddress = undefined;
    this.draftModel = undefined;
  }

  private name(): string {
    return (this.draftName ?? localConnection(this.env.listing())?.connection.display_name ?? "").trim();
  }

  private model(): string {
    return (this.draftModel ?? localConnection(this.env.listing())?.connection.model ?? "").trim();
  }

  /** What the server said, when it was asked at the address that is shown now and not another. */
  private status(): ServerStatus | null {
    return this.asked.phase === "answered" && this.asked.status.address === this.address()
      ? this.asked.status
      : null;
  }

  /** Whether what is typed is enough to keep, and is not known to be a server this build cannot use. */
  private canSave(): boolean {
    if (this.name() === "" || this.address() === "" || this.model() === "") return false;
    const said = this.status();
    return said === null || (said.state !== "needs-key" && said.state !== "certificate");
  }

  // ---- what is drawn -----------------------------------------------------

  body(): Child[] {
    const t = this.env.t;
    if (!this.here) return [h("p", { class: "muted" }, t("aiNotHere"))];
    return [
      h("p", { class: "muted" }, t("aiServerIntro")),
      this.kept(),
      h(
        "dl",
        { class: "ai-facts" },
        h("dt", {}, t("aiServerName")),
        h("dd", {}, this.nameField()),
        h("dt", {}, t("aiServerAddress")),
        h("dd", {}, this.addressField(), this.presets()),
        h("dt", {}, t("aiServerState")),
        h("dd", { id: STATUS_ID }, this.checkButton(), ...this.statusLines()),
        h("dt", {}, t("aiModel")),
        h("dd", {}, this.modelField()),
      ),
      h("p", { class: "muted" }, t("aiServerSentTo")),
      ...this.warnings(),
      this.saveControls(),
    ];
  }

  /** What is kept now, one line, so that what is typed below can be told from it. */
  private kept(): Child {
    const kept = localConnection(this.env.listing())?.connection;
    if (kept === undefined) return null;
    return h(
      "p",
      { class: "ai-server-kept muted" },
      this.env.t("aiServerKept", {
        name: kept.display_name ?? kept.id,
        address: kept.server_url ?? "",
        model: kept.model ?? this.env.t("aiServerNoModel"),
      }),
    );
  }

  private nameField(): HTMLElement {
    const t = this.env.t;
    const field = h("input", {
      id: "ai-server-name",
      type: "text",
      maxlength: "80",
      autocomplete: "off",
      "aria-label": t("aiServerName"),
      placeholder: t("aiServerNamePlaceholder"),
      oninput: (event) => {
        this.draftName = (event.target as HTMLInputElement).value;
        this.refreshSave();
      },
    });
    field.value = this.draftName ?? localConnection(this.env.listing())?.connection.display_name ?? "";
    return h("div", {}, field, h("div", { class: "muted" }, t("aiServerNameNote")));
  }

  private addressField(): HTMLElement {
    const t = this.env.t;
    const field = h("input", {
      id: "ai-server-address",
      type: "text",
      maxlength: "300",
      spellcheck: "false",
      autocomplete: "off",
      "aria-label": t("aiServerAddress"),
      placeholder: "http://127.0.0.1:11434/v1",
      oninput: (event) => {
        this.draftAddress = (event.target as HTMLInputElement).value;
        this.refreshSave();
      },
      // What was said of the address that was there is not said of this one: drawn again once the
      // person has left the field, not at each key, which would take the cursor out of it.
      onchange: () => this.env.redraw(),
    });
    field.value = this.draftAddress ?? localConnection(this.env.listing())?.connection.server_url ?? "";
    return field;
  }

  /** One button for each server the person is likely to run: it fills the address and asks the server at once. */
  private presets(): HTMLElement {
    return h(
      "div",
      { class: "ai-presets" },
      h("span", { class: "muted" }, `${this.env.t("aiServerPresets")} `),
      ...PRESETS.map((preset) =>
        h(
          "button",
          {
            type: "button",
            class: "quiet",
            "data-address": preset.address,
            onclick: () => {
              this.draftAddress = preset.address;
              this.env.check(preset.address);
            },
          },
          preset.name,
        ),
      ),
    );
  }

  private checkButton(): HTMLElement {
    const t = this.env.t;
    const asking = this.asked.phase === "asking";
    const button = h(
      "button",
      {
        id: "ai-server-check",
        type: "button",
        disabled: asking || this.address() === "",
        onclick: () => this.env.check(this.address()),
      },
      t(asking ? "aiServerChecking" : "aiServerCheck"),
    );
    this.checkButtonDrawn = button;
    return button;
  }

  /** What the server said, in the words of what a person does about it. */
  private statusLines(): Child[] {
    const t = this.env.t;
    if (this.asked.phase === "asking") return [h("p", { class: "muted", role: "status" }, t("aiServerAsking"))];
    if (this.asked.phase === "refused") {
      return [h("p", { class: "banner banner-warn", role: "status" }, this.asked.message ?? this.asked.kind)];
    }
    const said = this.status();
    if (said === null) return [h("p", { class: "muted" }, t("aiServerNotChecked"))];
    switch (said.state) {
      case "listed":
        return [
          h(
            "p",
            { class: "banner banner-ok", role: "status" },
            said.models.length === 0
              ? t("aiServerListedNone")
              : t("aiServerListed", { count: String(said.models.length) }),
          ),
        ];
      case "unreachable":
        return [this.problem("aiServerUnreachable", said.reason)];
      case "certificate":
        return [this.problem("aiServerCertificate", said.reason)];
      case "needs-key":
        return [this.problem("aiServerNeedsKey", said.reason)];
      case "not-a-model-list":
        return [this.problem("aiServerNotAModelList", said.reason)];
    }
  }

  private problem(words: Key, reason: string): HTMLElement {
    return h(
      "div",
      { class: "banner banner-warn", role: "status" },
      h("p", {}, this.env.t(words)),
      h("p", { class: "muted" }, reason),
    );
  }

  /** The model is typed, with the ones the server listed offered; one that is not listed is still typed. */
  private modelField(): HTMLElement {
    const t = this.env.t;
    const said = this.status();
    const listed = said?.state === "listed" ? said.models : [];
    const field = h("input", {
      id: "ai-server-model",
      type: "text",
      maxlength: "200",
      spellcheck: "false",
      autocomplete: "off",
      list: "ai-server-models",
      "aria-label": t("aiModelId"),
      placeholder: t("aiServerModelPlaceholder"),
      oninput: (event) => {
        this.draftModel = (event.target as HTMLInputElement).value;
        this.refreshSave();
      },
    });
    field.value = this.draftModel ?? localConnection(this.env.listing())?.connection.model ?? "";
    const options = h(
      "datalist",
      { id: "ai-server-models" },
      ...listed.map((model) => h("option", { value: model })),
    );
    return h("div", {}, field, options);
  }

  /** Where the specification goes, as loudly as it deserves. */
  private warnings(): Child[] {
    const t = this.env.t;
    const said = this.status();
    if (said === null) return [];
    if (said.reach === "network" && !said.tls) return [h("p", { class: "banner banner-warn" }, t("aiServerPlainNetwork"))];
    if (said.reach === "network") return [h("p", { class: "muted" }, t("aiServerSecureNetwork"))];
    return [h("p", { class: "muted" }, t("aiServerThisComputer"))];
  }

  private saveControls(): HTMLElement | null {
    const t = this.env.t;
    if (!this.env.described.writes_settings) return null;
    const save = h(
      "button",
      {
        id: "ai-save",
        type: "button",
        disabled: this.env.busy() || !this.canSave(),
        onclick: () => this.env.save(),
      },
      t(this.env.busy() ? "aiSaving" : "aiSave"),
    );
    this.saveButton = save;
    return h("div", { class: "choices" }, save);
  }

  /** Typing changes whether what is typed is enough to ask about, and to keep. */
  private refreshSave(): void {
    if (this.saveButton !== null) this.saveButton.disabled = this.env.busy() || !this.canSave();
    if (this.checkButtonDrawn !== null) {
      this.checkButtonDrawn.disabled = this.asked.phase === "asking" || this.address() === "";
    }
  }
}
