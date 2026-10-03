// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The screen: the works on one side, the selected work's text and history on the
// other. It decides nothing about the text. Every rule about what a save is lives
// in the core and the editor model; this file asks, shows the answer, and offers
// the person the choices a refusal leaves.

import { apiOver, type Api } from "./api";
import {
  conflictRevisions,
  ContractError,
  SUPPORTED_COMMAND_SET_VERSION,
  type Described,
  type HistoryEntry,
  type Listing,
  type SourceText,
  type Work,
} from "./contract";
import { h, type Child } from "./dom";
import {
  conflicted,
  edit,
  failed,
  isDirty,
  keepMine,
  open,
  saveRequest,
  saved,
  saving,
  takeTheirs,
  type EditorModel,
} from "./editor_model";
import { initialLocale, languageName, LOCALES, translate, type Key, type Locale } from "./i18n";
import { CommandFailure, TRANSPORT, UNAUTHORIZED, type Transport } from "./ipc";
import { tokenFromPaste, type Credentials } from "./token";

const LOCALE_KEY = "sce.locale";

export interface Environment {
  readonly transport: Transport;
  /** Present when the server wants a token the person can be asked for. */
  readonly credentials?: Credentials | undefined;
  readonly storage: Pick<Storage, "getItem" | "setItem"> | null;
  readonly browserLanguage: string | undefined;
}

export class App {
  private readonly api: Api;
  private locale: Locale;

  /** Set when nothing can work: the screen shows it and stops. */
  private fatal: string | null = null;
  private described: Described | null = null;
  private listing: Listing | null = null;
  private selected: Work | null = null;
  private editor: EditorModel | null = null;
  private entries: HistoryEntry[] = [];
  /** An older revision being looked at, read-only. */
  private viewing: SourceText | null = null;
  private notice: string | null = null;
  private loading = true;
  /**
   * Which editor the screen is showing, counted. Every answer that arrives for a
   * request is applied only if this is still the number it was asked under, so a
   * reply for a work (or an opening of it) the person has since left cannot touch
   * the one they are in now. A rule over the editor, not over a pair of works.
   */
  private session = 0;
  /** The newest request to open a work; an older one that answers later is dropped. */
  private opening = 0;
  /** The newest request to look at an older revision. */
  private looking = 0;
  /** A work the person asked for while the editor held text that is not saved. */
  private pendingSwitch: Work | null = null;
  /** What is typed in the new-work field, kept across redraws. */
  private draftTitle = "";
  /** The server refused for want of a token, and the person can supply one. */
  private needsToken = false;
  /** A token has been supplied since, so a further refusal means it was wrong. */
  private triedToken = false;

  constructor(
    private readonly root: HTMLElement,
    private readonly env: Environment,
  ) {
    this.api = apiOver(env.transport);
    this.locale = initialLocale(readKept(env.storage, LOCALE_KEY), env.browserLanguage);
  }

  async start(): Promise<void> {
    this.render();
    await this.guard(async () => {
      this.described = await this.api.describe();
      if (this.described.command_set_version !== SUPPORTED_COMMAND_SET_VERSION) {
        this.fatal = this.t("versionMismatch", {
          screen: String(SUPPORTED_COMMAND_SET_VERSION),
          core: String(this.described.command_set_version),
        });
        return;
      }
      this.listing = await this.api.listWorks();
      this.needsToken = false;
      this.triedToken = false;
    });
    this.loading = false;
    this.render();
  }

  private async signIn(pasted: string): Promise<void> {
    const token = tokenFromPaste(pasted);
    if (token === "" || this.env.credentials === undefined) return;
    this.env.credentials.save(token);
    this.triedToken = true;
    this.needsToken = false;
    this.fatal = null;
    this.loading = true;
    await this.start();
  }

  // ---- actions ----------------------------------------------------------

  /**
   * The person asked for `work`. If the editor holds text that is not saved, or a
   * save is still in flight, nothing is replaced: they are asked what to do with
   * it first.
   */
  private async openWork(work: Work): Promise<void> {
    const editor = this.editor;
    if (editor !== null && (isDirty(editor) || editor.phase === "saving")) {
      this.pendingSwitch = work;
      this.render();
      return;
    }
    await this.select(work);
  }

  /** Load `work` into the editor. Of several requests in flight, only the newest is applied. */
  private async select(work: Work): Promise<void> {
    const ticket = ++this.opening;
    this.pendingSwitch = null;
    this.notice = null;
    try {
      const [source, entries] = await Promise.all([
        this.api.readSource(work.id),
        this.api.history(work.id),
      ]);
      if (ticket !== this.opening) return;
      this.session += 1;
      this.selected = work;
      this.editor = open(work.id, source);
      this.entries = entries;
      this.viewing = null;
    } catch (error) {
      if (ticket !== this.opening) return;
      this.report(error);
    }
    this.render();
  }

  private async create(title: string): Promise<void> {
    await this.guard(async () => {
      const work = await this.api.createWork(title);
      this.draftTitle = "";
      this.listing = await this.api.listWorks();
      await this.openWork(work);
    });
    this.render();
  }

  private async saveAndSwitch(): Promise<void> {
    const target = this.pendingSwitch;
    await this.save();
    // Only a save that took, with nothing typed since, lets the switch go on.
    const editor = this.editor;
    if (target !== null && editor !== null && editor.phase === "idle" && !isDirty(editor)) {
      await this.select(target);
    }
  }

  private cancelSwitch(): void {
    this.pendingSwitch = null;
    this.render();
  }

  private async save(): Promise<void> {
    const editor = this.editor;
    const request = editor === null ? null : saveRequest(editor);
    if (editor === null || request === null) return;
    const session = this.session;
    this.editor = saving(editor);
    this.notice = null;
    this.render();

    let outcome;
    try {
      outcome = await this.api.saveSource(request.id, request.text, request.base);
    } catch (error) {
      // The editor the save was made from may be gone; if so it is not this
      // answer's to touch.
      if (session === this.session && this.editor !== null) {
        this.editor =
          !this.askForToken(error) && error instanceof CommandFailure && error.kind === "conflict"
            ? conflicted(this.editor, conflictRevisions(error.detail).current)
            : failed(this.editor, this.explain(error));
      }
      this.render();
      return;
    }

    // The core has the text. From here on the save has succeeded whatever else
    // goes wrong, so the editor's base moves at once: a retry built on the old
    // base would conflict with this very save.
    if (session === this.session && this.editor !== null) {
      this.editor = saved(this.editor, request.text, outcome);
    }
    this.render();

    // The history is a separate read, and its failure is its own: it says the
    // list could not be read, not that the text was not saved.
    try {
      const entries = await this.api.history(request.id);
      if (session === this.session) this.entries = entries;
    } catch (error) {
      // Not fatal: the text is saved, and only the list is stale.
      if (session === this.session && !this.askForToken(error)) {
        this.notice = this.explain(error);
      }
    }
    this.render();
  }

  private async takeTheirs(): Promise<void> {
    const editor = this.editor;
    if (editor === null) return;
    const session = this.session;
    this.notice = null;
    try {
      const [source, entries] = await Promise.all([
        this.api.readSource(editor.workId),
        this.api.history(editor.workId),
      ]);
      // Only the editor that asked, still in the conflict it asked about.
      if (session !== this.session || this.editor === null || this.editor.phase !== "conflict") return;
      this.editor = takeTheirs(this.editor, source);
      this.entries = entries;
    } catch (error) {
      if (session !== this.session) return;
      this.report(error);
    }
    this.render();
  }

  private async keepMine(): Promise<void> {
    if (this.editor === null) return;
    this.editor = keepMine(this.editor);
    await this.save();
  }

  private async view(entry: HistoryEntry): Promise<void> {
    const work = this.selected;
    if (work === null) return;
    const session = this.session;
    const ticket = ++this.looking;
    this.notice = null;
    try {
      const source = await this.api.readSource(work.id, entry.revision);
      if (session !== this.session || ticket !== this.looking) return;
      this.viewing = source;
    } catch (error) {
      if (session !== this.session || ticket !== this.looking) return;
      this.report(error);
    }
    this.render();
  }

  private async restore(entry: HistoryEntry): Promise<void> {
    const work = this.selected;
    const asked = this.editor;
    if (work === null || asked === null || isDirty(asked) || asked.phase !== "idle") return;
    const session = this.session;
    this.notice = null;
    try {
      const source = await this.api.readSource(work.id, entry.revision);
      if (session !== this.session) return;
      const editor = this.editor;
      if (source === null || editor === null) return;
      // Loading replaces what is in the editor, so it is done only to the text it
      // was asked about: anything typed or saved while the answer was on its way
      // is the person's, and is not overwritten.
      if (editor.text !== asked.text || editor.base !== asked.base || editor.phase !== "idle") {
        this.notice = this.t("restoreSkipped");
      } else {
        this.editor = edit(editor, source.text);
        this.viewing = null;
      }
    } catch (error) {
      if (session !== this.session) return;
      this.report(error);
    }
    this.render();
  }

  private chooseLocale(locale: Locale): void {
    this.locale = locale;
    try {
      this.env.storage?.setItem(LOCALE_KEY, locale);
    } catch {
      // The choice holds for this session only.
    }
    this.render();
  }

  // ---- errors -----------------------------------------------------------

  /** Run `work`; whatever it throws becomes the notice, or the fatal text if nothing can work. */
  private async guard(work: () => Promise<void>): Promise<void> {
    this.notice = null;
    try {
      await work();
    } catch (error) {
      this.report(error);
    }
  }

  /** Show a failure: the sign-in form, the fatal text, or the notice, whichever it calls for. */
  private report(error: unknown): void {
    const text = this.explain(error);
    if (this.askForToken(error)) {
      // The sign-in form is the whole answer; nothing else is shown with it.
    } else if (
      error instanceof CommandFailure &&
      (error.kind === TRANSPORT || error.kind === UNAUTHORIZED)
    ) {
      this.fatal = text;
    } else {
      this.notice = text;
    }
  }

  /** Whether `error` is the server wanting a token the person can type in; if so, the sign-in form is shown next. */
  private askForToken(error: unknown): boolean {
    const wanted =
      error instanceof CommandFailure &&
      error.kind === UNAUTHORIZED &&
      this.env.credentials !== undefined;
    if (wanted) this.needsToken = true;
    return wanted;
  }

  private explain(error: unknown): string {
    if (error instanceof CommandFailure) {
      if (error.kind === UNAUTHORIZED) return this.t("unauthorized");
      if (error.kind === TRANSPORT) return `${this.t("unreachable")} ${error.message}`;
      return error.message;
    }
    if (error instanceof ContractError) return this.t("contractBroken", { detail: error.message });
    return String(error);
  }

  private t(key: Key, values?: Record<string, string>): string {
    return translate(this.locale, key, values);
  }

  // ---- drawing ----------------------------------------------------------

  private render(): void {
    const keep = this.captureEditorFocus();
    this.root.ownerDocument.documentElement.lang = this.locale;
    this.root.replaceChildren(
      h(
        "div",
        { class: "shell" },
        this.header(),
        this.needsToken
          ? this.tokenForm()
          : this.fatal !== null
            ? h("p", { class: "banner banner-error", role: "alert" }, this.fatal)
            : this.loading
              ? h("p", { class: "muted" }, this.t("loading"))
              : this.body(),
      ),
    );
    this.restoreEditorFocus(keep);
  }

  /** Asked for when the address carried no token (a link handler may have cut it off). */
  private tokenForm(): HTMLElement {
    const field = h("input", {
      type: "password",
      name: "token",
      autocomplete: "off",
      autocapitalize: "off",
      autocorrect: "off",
      spellcheck: "false",
      required: true,
      "aria-label": this.t("tokenLabel"),
      placeholder: this.t("tokenLabel"),
    });
    return h(
      "form",
      {
        class: "token-form",
        onsubmit: (event) => {
          event.preventDefault();
          void this.signIn(field.value);
        },
      },
      h("h2", {}, this.t("tokenTitle")),
      h("p", {}, this.t("tokenBody")),
      this.triedToken
        ? h("p", { class: "banner banner-error", role: "alert" }, this.t("tokenRefused"))
        : null,
      field,
      " ",
      h("button", { type: "submit" }, this.t("tokenConnect")),
    );
  }

  private header(): HTMLElement {
    const picker = h(
      "select",
      {
        "aria-label": this.t("language"),
        onchange: (event) => this.chooseLocale((event.target as HTMLSelectElement).value as Locale),
      },
      ...LOCALES.map((locale) =>
        h("option", { value: locale, selected: locale === this.locale }, languageName(locale)),
      ),
    );
    return h(
      "header",
      { class: "top" },
      h("h1", {}, this.t("appTitle")),
      this.described === null
        ? null
        : h("span", { class: "muted root", title: this.t("worksFolder") }, this.described.root),
      picker,
    );
  }

  private body(): HTMLElement {
    return h(
      "div",
      { class: "layout" },
      this.sidebar(),
      h(
        "main",
        { class: "work" },
        this.switchBanner(),
        this.notice === null ? null : h("p", { class: "banner banner-error", role: "alert" }, this.notice),
        this.selected === null || this.editor === null
          ? h("p", { class: "muted" }, this.t("pickAWork"))
          : this.workPane(this.selected, this.editor),
      ),
    );
  }

  /** Shown while a work the person asked for waits on what to do with unsaved text. */
  private switchBanner(): HTMLElement | null {
    const target = this.pendingSwitch;
    if (target === null) return null;
    const saving = this.editor?.phase === "saving";
    return h(
      "section",
      { class: "banner banner-warn", role: "alert" },
      h("strong", {}, this.t("switchTitle")),
      h("p", {}, this.t("switchBody", { title: target.title })),
      h(
        "div",
        { class: "choices" },
        h(
          "button",
          { type: "button", disabled: saving, onclick: () => void this.saveAndSwitch() },
          this.t("saveAndSwitch"),
        ),
        h(
          "button",
          { type: "button", disabled: saving, onclick: () => void this.select(target) },
          this.t("discardAndSwitch"),
        ),
        h("button", { type: "button", onclick: () => this.cancelSwitch() }, this.t("cancelSwitch")),
      ),
    );
  }

  /** Whether the editor holds text the core has not been given. */
  hasUnsavedChanges(): boolean {
    return this.editor !== null && (isDirty(this.editor) || this.editor.phase === "saving");
  }

  private sidebar(): HTMLElement {
    const listing = this.listing;
    const titleInput = h("input", {
      type: "text",
      name: "title",
      maxlength: "200",
      required: true,
      "aria-label": this.t("newWorkTitle"),
      placeholder: this.t("newWorkTitle"),
      oninput: (event) => {
        this.draftTitle = (event.target as HTMLInputElement).value;
      },
    });
    titleInput.value = this.draftTitle;
    const form = h(
      "form",
      {
        class: "new-work",
        onsubmit: (event) => {
          event.preventDefault();
          const title = titleInput.value.trim();
          if (title !== "") void this.create(title);
        },
      },
      titleInput,
      h("button", { type: "submit" }, this.t("newWorkCreate")),
    );
    return h(
      "nav",
      { class: "works", "aria-label": this.t("works") },
      h("h2", {}, this.t("works")),
      listing === null || listing.works.length === 0
        ? h("p", { class: "muted" }, this.t("noWorks"))
        : h(
            "ul",
            {},
            ...listing.works.map((work) =>
              h(
                "li",
                {},
                h(
                  "button",
                  {
                    class: "work-link",
                    type: "button",
                    "aria-current": this.selected?.id === work.id ? "true" : undefined,
                    onclick: () => void this.openWork(work),
                  },
                  work.title,
                ),
              ),
            ),
          ),
      listing !== null && listing.unreadable.length > 0
        ? h(
            "details",
            { class: "unreadable" },
            h("summary", {}, `${this.t("unreadableHeading")} (${listing.unreadable.length})`),
            h("ul", {}, ...listing.unreadable.map((u) => h("li", {}, `${u.id}: ${u.reason}`))),
          )
        : null,
      h("h3", {}, this.t("newWork")),
      form,
    );
  }

  private workPane(work: Work, editor: EditorModel): HTMLElement {
    const viewing = this.viewing;
    const textarea = h("textarea", {
      id: "source",
      class: "source",
      spellcheck: "false",
      "aria-label": this.t("editorLabel"),
      readonly: viewing !== null,
      oninput: (event) => {
        if (this.editor === null) return;
        this.editor = edit(this.editor, (event.target as HTMLTextAreaElement).value);
        this.refreshChrome();
      },
      onkeydown: (event) => {
        const key = event as KeyboardEvent;
        if ((key.ctrlKey || key.metaKey) && key.key.toLowerCase() === "s") {
          key.preventDefault();
          void this.save();
        }
      },
    });
    textarea.value = viewing === null ? editor.text : viewing.text;

    return h(
      "div",
      { class: "pane" },
      h("h2", {}, work.title),
      viewing !== null
        ? h(
            "p",
            { class: "banner" },
            this.t("viewingOld", { rev: viewing.revision.slice(0, 12) }),
            " ",
            h("button", { type: "button", onclick: () => this.back() }, this.t("backToCurrent")),
          )
        : null,
      editor.base === null && viewing === null ? h("p", { class: "muted" }, this.t("noTextYet")) : null,
      this.conflictBanner(editor),
      editor.phase === "failed" ? this.failureBanner(editor) : null,
      textarea,
      h(
        "div",
        { class: "bar" },
        h("button", { id: "save", type: "button", onclick: () => void this.save() }, this.t("save")),
        h("span", { id: "status", class: "status", role: "status", "aria-live": "polite" }),
        h(
          "span",
          { class: "muted rev" },
          `${this.t("revision")}: ${editor.base === null ? this.t("noRevision") : editor.base.slice(0, 12)}`,
        ),
      ),
      this.historyPanel(editor),
    );
  }

  private back(): void {
    this.viewing = null;
    this.render();
  }

  private conflictBanner(editor: EditorModel): HTMLElement | null {
    if (editor.phase !== "conflict") return null;
    return h(
      "section",
      { class: "banner banner-warn", role: "alert" },
      h("strong", {}, this.t("conflictTitle")),
      h(
        "p",
        {},
        this.t("conflictBody", { current: editor.current === null ? "?" : editor.current.slice(0, 12) }),
      ),
      h(
        "div",
        { class: "choices" },
        h("button", { type: "button", onclick: () => void this.takeTheirs() }, this.t("conflictTakeTheirs")),
        h("button", { type: "button", onclick: () => void this.keepMine() }, this.t("conflictKeepMine")),
      ),
      h("p", { class: "muted" }, this.t("conflictKeepMineHint")),
    );
  }

  private failureBanner(editor: EditorModel): HTMLElement {
    return h(
      "section",
      { class: "banner banner-error", role: "alert" },
      h("strong", {}, this.t("failureTitle")),
      h("p", {}, editor.failure ?? ""),
      h("button", { type: "button", onclick: () => void this.save() }, this.t("retry")),
    );
  }

  private historyPanel(editor: EditorModel): HTMLElement {
    const canRestore = !isDirty(editor);
    const rows: Child[] = this.entries
      .slice()
      .reverse()
      .map((entry) =>
        h(
          "li",
          {},
          h("code", {}, entry.revision.slice(0, 12)),
          " ",
          h("time", { datetime: entry.saved_at }, formatTime(entry.saved_at, this.locale)),
          " ",
          h("button", { type: "button", onclick: () => void this.view(entry) }, this.t("historyView")),
          " ",
          h(
            "button",
            {
              type: "button",
              disabled: !canRestore,
              title: canRestore ? undefined : this.t("restoreNeedsSave"),
              onclick: () => void this.restore(entry),
            },
            this.t("historyRestore"),
          ),
        ),
      );
    return h(
      "section",
      { class: "history" },
      h("h3", {}, this.t("history")),
      rows.length === 0 ? h("p", { class: "muted" }, this.t("noHistory")) : h("ul", {}, ...rows),
    );
  }

  /** The parts that follow every keystroke, without redrawing the editor under the cursor. */
  private refreshChrome(): void {
    const editor = this.editor;
    const status = this.root.querySelector<HTMLElement>("#status");
    const button = this.root.querySelector<HTMLButtonElement>("#save");
    if (editor === null || status === null || button === null) return;
    button.disabled = saveRequest(editor) === null;
    status.textContent =
      editor.phase === "saving"
        ? this.t("saving")
        : isDirty(editor)
          ? this.t("unsaved")
          : editor.base === null
            ? ""
            : this.t("saved");
  }

  private captureEditorFocus(): { focused: boolean; start: number; end: number } | null {
    const textarea = this.root.querySelector<HTMLTextAreaElement>("#source");
    if (textarea === null) return null;
    return {
      focused: this.root.ownerDocument.activeElement === textarea,
      start: textarea.selectionStart,
      end: textarea.selectionEnd,
    };
  }

  private restoreEditorFocus(keep: { focused: boolean; start: number; end: number } | null): void {
    this.refreshChrome();
    const textarea = this.root.querySelector<HTMLTextAreaElement>("#source");
    if (textarea === null || keep === null || !keep.focused) return;
    textarea.focus();
    textarea.setSelectionRange(keep.start, keep.end);
  }
}

function readKept(storage: Environment["storage"], key: string): string | null {
  try {
    return storage?.getItem(key) ?? null;
  } catch {
    return null;
  }
}

function formatTime(timestamp: string, locale: Locale): string {
  const date = new Date(timestamp);
  return Number.isNaN(date.getTime()) ? timestamp : date.toLocaleString(locale);
}
