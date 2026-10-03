// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The screen: the works on one side, the selected work's text and history on the
// other. It decides nothing about the text. Every rule about what a save is lives
// in the core and the editor model; this file asks, shows the answer, and offers
// the person the choices a refusal leaves.

import { apiOver, type Api } from "./api";
import {
  answersConflicted,
  answersFailed,
  answersRequest,
  answersSaved,
  answersSaving,
  editAnswer,
  isAnswersDirty,
  openAnswers,
  wordsOf,
  type AnswersModel,
} from "./answers_model";
import {
  conflictRevisions,
  ContractError,
  SUPPORTED_COMMAND_SET_VERSION,
  type Described,
  type HistoryEntry,
  type Listing,
  type SourceText,
  type Unresolved,
  type Work,
} from "./contract";
import type { Desktop } from "./desktop";
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
import {
  drawFailureOf,
  sheetName,
  svgAddress,
  zoomFrom,
  ZOOMS,
  type ModelPanel,
  type ModelRead,
  type ReviewPanel,
} from "./model_view";
import { tokenFromPaste, type Credentials } from "./token";

const LOCALE_KEY = "sce.locale";
const ZOOM_KEY = "sce.zoom";

export interface Environment {
  readonly transport: Transport;
  /** Present when the server wants a token the person can be asked for. */
  readonly credentials?: Credentials | undefined;
  /** Present inside a desktop window: told what is unsaved, and asked to close it. */
  readonly desktop?: Desktop | undefined;
  readonly storage: Pick<Storage, "getItem" | "setItem"> | null;
  readonly browserLanguage: string | undefined;
}

export class App {
  private readonly api: Api;
  private locale: Locale;
  /** How large the drawings are shown, as a multiple of the size SCE set them at. */
  private zoom: number;

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
  /** Something that went well and that the person should hear of, apart from a failure. */
  private info: string | null = null;
  /** The person pressed "remove" for the selected work and has not yet said yes or no. */
  private removing = false;
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
  /** The model of the selected work, and how far its drawing has got. */
  private model: ModelPanel | null = null;
  /** The newest request to read and draw the model; an older one that answers later is dropped. */
  private modelTicket = 0;
  /** What SCE says of the model (its check and pseudocode page), apart from its drawing. */
  private review: ReviewPanel | null = null;
  /** The newest request for the review; an older one that answers later is dropped. */
  private reviewTicket = 0;
  /** The owner's answers to the model's open questions, and what is typed into them. */
  private answers: AnswersModel | null = null;
  /** Why the answers could not be read, when they could not. */
  private answersUnreadable: string | null = null;
  /** The newest request for the answers; an older one that answers later is dropped. */
  private answersTicket = 0;
  /** A work the person asked for while the editor held text that is not saved. */
  private pendingSwitch: Work | null = null;
  /** The window was asked to close while something was not saved, and the person has not yet said what to do. */
  private closing = false;
  /** What the desktop shell was last told is unsaved, so it is told only what changed. */
  private reportedUnsaved: boolean | null = null;
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
    this.zoom = zoomFrom(readKept(env.storage, ZOOM_KEY));
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
    if (this.hasUnsavedChanges()) {
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
    this.info = null;
    this.removing = false;
    let opened = false;
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
      // The model is a separate read, and a slow drawing of it must not hold the
      // text back: the editor is shown now and the model fills in when SCE has drawn it.
      this.modelTicket += 1;
      this.model = { phase: "reading" };
      this.reviewTicket += 1;
      this.review = null;
      this.answersTicket += 1;
      this.answers = null;
      this.answersUnreadable = null;
      opened = true;
    } catch (error) {
      if (ticket !== this.opening) return;
      this.report(error);
    }
    this.render();
    if (opened) {
      void this.loadModel(work.id, true);
      void this.loadAnswers(work.id);
    }
  }

  /**
   * Read the owner's answers to the model's open questions. They are the work's, not
   * the model's: they are read when the work opens and kept across models, because
   * a question the next draft still asks is answered already.
   */
  private async loadAnswers(id: string): Promise<void> {
    const session = this.session;
    const ticket = ++this.answersTicket;
    const current = (): boolean => session === this.session && ticket === this.answersTicket;
    try {
      const read = await this.api.readAnswers(id);
      if (!current()) return;
      this.answers = openAnswers(read);
      this.answersUnreadable = null;
    } catch (error) {
      if (!current()) return;
      if (this.askForToken(error)) return;
      this.answersUnreadable = this.explain(error);
    }
    this.render();
  }

  /** The person typed into the field of question `id`. Nothing is redrawn under their cursor. */
  private typeAnswer(id: string, text: string): void {
    if (this.answers === null) return;
    this.answers = editAnswer(this.answers, id, text);
    this.refreshAnswersChrome();
  }

  /** Save the answers as the owner now has them, on top of the revision they were read as. */
  private async saveAnswers(): Promise<void> {
    const model = this.answers;
    const work = this.selected;
    const request = model === null ? null : answersRequest(model);
    if (model === null || work === null || request === null) return;
    const session = this.session;
    this.answers = answersSaving(model);
    this.render();
    try {
      await this.api.saveAnswers(work.id, request.answers, request.base);
    } catch (error) {
      if (session !== this.session || this.answers === null) return;
      if (this.askForToken(error)) return;
      if (error instanceof CommandFailure && error.kind === "conflict") {
        // What is saved now is loaded and what was typed is kept: the person looks,
        // and saves again on top of it.
        try {
          const current = await this.api.readAnswers(work.id);
          if (session !== this.session || this.answers === null) return;
          this.answers = answersConflicted(this.answers, current);
        } catch (again) {
          if (session !== this.session || this.answers === null) return;
          this.answers = answersFailed(this.answers, this.explain(again));
        }
      } else {
        this.answers = answersFailed(this.answers, this.explain(error));
      }
      this.render();
      return;
    }

    // The core has the answers. What it holds is what is shown, because it stamps
    // each answer and the stamp is its to say; if that read fails the save still
    // took, and the panel says the answers cannot be read rather than that it failed.
    try {
      const read = await this.api.readAnswers(work.id);
      if (session !== this.session || this.answers === null) return;
      this.answers = answersSaved(this.answers, read, request.answers);
    } catch (error) {
      if (session !== this.session) return;
      if (this.askForToken(error)) return;
      this.answers = null;
      this.answersUnreadable = this.explain(error);
    }
    this.render();
  }

  /** The save button and its words follow every keystroke, without redrawing the field being typed in. */
  private refreshAnswersChrome(): void {
    const model = this.answers;
    const button = this.root.querySelector<HTMLButtonElement>("#save-answers");
    const status = this.root.querySelector<HTMLElement>("#answers-status");
    if (model === null || button === null || status === null) return;
    const dirty = isAnswersDirty(model);
    button.disabled = !dirty || model.phase === "saving";
    status.textContent =
      model.phase === "saving"
        ? this.t("saving")
        : dirty
          ? this.t("answersUnsaved")
          : model.base === null
            ? ""
            : this.t("answersSaved");
    this.reportUnsaved();
  }

  /**
   * Read the work's model, and have SCE draw it.
   *
   * Only the newest request, for the work and the editor that asked, is applied.
   * `redraw` is false after a save of the text, which moves where the model
   * stands but not the model: its sheets are kept, and SCE is not run again for
   * an answer it has already given.
   */
  private async loadModel(id: string, redraw: boolean): Promise<void> {
    const session = this.session;
    const ticket = ++this.modelTicket;
    const current = (): boolean => session === this.session && ticket === this.modelTicket;
    const prior = this.model;
    try {
      const read = await this.api.readModel(id);
      if (!current()) return;
      if (read.model === null || read.standing === null) {
        this.model = { phase: "none" };
        this.reviewTicket += 1;
        this.review = null;
        this.render();
        return;
      }
      const known: ModelRead = { model: read.model, standing: read.standing, sourceHead: read.source_head };
      if (!redraw && prior?.phase === "drawn" && prior.read.model.revision === known.model.revision) {
        this.model = { phase: "drawn", read: known, figures: prior.figures };
        this.render();
        return;
      }
      this.model = { phase: "drawing", read: known };
      // SCE's check and page are asked for beside the drawing and answer in their
      // own time: a model that is slow to draw does not hold the page back.
      void this.loadReview(id, known.model.revision);
      this.render();
      try {
        // SCE draws in the language the screen is in: its page vocabularies are
        // named as the screen's languages are (`en`, `ko`).
        const figures = await this.api.figures(id, known.model.revision, this.locale);
        if (!current()) return;
        this.model = { phase: "drawn", read: known, figures };
      } catch (error) {
        if (!current()) return;
        const failure = drawFailureOf(error);
        if (failure === null) throw error;
        this.model = { phase: "not-drawn", read: known, failure };
      }
    } catch (error) {
      if (!current()) return;
      // A token wanted is the sign-in form's to answer; anything else is the
      // panel's own message, so it does not look like the work has no model.
      if (!this.askForToken(error)) this.model = { phase: "failed", message: this.explain(error) };
    }
    this.render();
  }

  /**
   * Ask SCE what it says of the model: its check and its pseudocode page, in the
   * language the screen is in. Only the newest request, for the editor that asked,
   * is applied.
   */
  private async loadReview(id: string, revision: string): Promise<void> {
    const session = this.session;
    const ticket = ++this.reviewTicket;
    const current = (): boolean => session === this.session && ticket === this.reviewTicket;
    this.review = { phase: "reading" };
    try {
      const review = await this.api.review(id, revision, this.locale);
      if (!current()) return;
      this.review = { phase: "read", review };
    } catch (error) {
      if (!current()) return;
      // A token wanted is the sign-in form's to answer; anything else is the
      // panel's own message, so it does not look like SCE had nothing to say.
      if (this.askForToken(error)) return;
      const failure = drawFailureOf(error);
      this.review = { phase: "failed", message: failure === null ? this.explain(error) : failure.message };
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

  /** The person said yes to removing the selected work. */
  private async remove(): Promise<void> {
    const work = this.selected;
    const editor = this.editor;
    // A save still on its way would write into a work the person is told is gone.
    if (work === null || editor === null || this.somethingIsSaving()) return;
    this.removing = false;
    await this.guard(async () => {
      const removed = await this.api.removeWork(work.id);
      // Whatever was asked for the removed work and has not answered is dropped:
      // the editor it would have landed in is gone.
      this.session += 1;
      this.opening += 1;
      this.modelTicket += 1;
      this.reviewTicket += 1;
      this.answersTicket += 1;
      this.looking += 1;
      this.review = null;
      this.answers = null;
      this.answersUnreadable = null;
      this.selected = null;
      this.editor = null;
      this.entries = [];
      this.viewing = null;
      this.model = null;
      this.pendingSwitch = null;
      this.info = this.t("removedNotice", { title: removed.title });
      this.listing = await this.api.listWorks();
    });
    this.render();
  }

  private async saveAndSwitch(): Promise<void> {
    const target = this.pendingSwitch;
    await this.save();
    await this.saveAnswers();
    // Only saves that took, with nothing typed since, let the switch go on.
    if (target !== null && !this.hasUnsavedChanges()) {
      await this.select(target);
    }
  }

  private cancelSwitch(): void {
    this.pendingSwitch = null;
    this.closing = false;
    this.render();
  }

  /**
   * The desktop shell was asked to close the window and held the close because this
   * screen said it holds something unsaved. With nothing unsaved after all it closes
   * at once; otherwise the person is asked, and the shell is told again that the
   * question is open, which is how it knows this screen is answering.
   */
  askToClose(): void {
    if (!this.hasUnsavedChanges()) {
      void this.env.desktop?.close();
      return;
    }
    this.closing = true;
    this.render();
    this.reportUnsaved(true);
  }

  private async saveAndClose(): Promise<void> {
    await this.save();
    await this.saveAnswers();
    // Only saves that took, with nothing typed since, let the window go.
    if (this.closing && !this.hasUnsavedChanges()) await this.env.desktop?.close();
  }

  /**
   * Tell the desktop shell whether this screen holds anything unsaved, when that has
   * changed (`force` says it again: the shell counts that as an answer).
   */
  private reportUnsaved(force = false): void {
    const desktop = this.env.desktop;
    if (desktop === undefined) return;
    const unsaved = this.hasUnsavedChanges();
    if (!force && unsaved === this.reportedUnsaved) return;
    this.reportedUnsaved = unsaved;
    desktop.unsaved(unsaved);
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
    // The text moved, so where the model stands may have: asked again, and not
    // drawn again unless the model itself changed.
    if (session === this.session) void this.loadModel(request.id, false);
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
    // What SCE drew was drawn in the other language: it draws again in this one.
    const model = this.model;
    if (this.selected !== null && model !== null && (model.phase === "drawn" || model.phase === "not-drawn")) {
      void this.loadModel(this.selected.id, true);
    }
  }

  private chooseZoom(zoom: number): void {
    this.zoom = zoom;
    try {
      this.env.storage?.setItem(ZOOM_KEY, String(zoom));
    } catch {
      // The choice holds for this session only.
    }
    this.render();
  }

  // ---- errors -----------------------------------------------------------

  /** Run `work`; whatever it throws becomes the notice, or the fatal text if nothing can work. */
  private async guard(work: () => Promise<void>): Promise<void> {
    this.notice = null;
    this.info = null;
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
    const keepAnswer = this.captureAnswerFocus();
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
    this.refreshAnswersChrome();
    this.restoreAnswerFocus(keepAnswer);
    // Whatever moved what is unsaved (a save, a removal, another work) is told to
    // the desktop shell here too: the two chrome refreshes above return early when
    // there is no editor or no answers on screen.
    this.reportUnsaved();
  }

  /**
   * The answer field the person is typing in and where their cursor is, so a redraw
   * that is not theirs (SCE's page arriving, a drawing finishing) does not take the
   * field from under them.
   */
  private captureAnswerFocus(): { qid: string; start: number; end: number } | null {
    const active = this.root.ownerDocument.activeElement;
    if (!(active instanceof HTMLTextAreaElement) || !this.root.contains(active)) return null;
    const qid = active.dataset["qid"];
    return qid === undefined ? null : { qid, start: active.selectionStart, end: active.selectionEnd };
  }

  private restoreAnswerFocus(keep: { qid: string; start: number; end: number } | null): void {
    if (keep === null) return;
    const field = [...this.root.querySelectorAll<HTMLTextAreaElement>("textarea[data-qid]")].find(
      (f) => f.dataset["qid"] === keep.qid,
    );
    if (field === undefined) return;
    field.focus();
    field.setSelectionRange(keep.start, keep.end);
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
        this.info === null ? null : h("p", { class: "banner banner-ok", role: "status" }, this.info),
        this.selected === null || this.editor === null
          ? h("p", { class: "muted" }, this.t("pickAWork"))
          : this.workPane(this.selected, this.editor),
      ),
    );
  }

  /**
   * Shown while the person waits on what to do with what is not saved: before another
   * work replaces it (`pendingSwitch`), or before the window closes over it (`closing`).
   */
  private switchBanner(): HTMLElement | null {
    const target = this.pendingSwitch;
    if (target === null && !this.closing) return null;
    const saving = this.somethingIsSaving();
    const closing = target === null;
    return h(
      "section",
      { class: "banner banner-warn", role: "alert" },
      h("strong", {}, this.t(closing ? "closeTitle" : "switchTitle")),
      h("p", {}, closing ? this.t("closeBody") : this.t("switchBody", { title: target.title })),
      h(
        "div",
        { class: "choices" },
        h(
          "button",
          {
            type: "button",
            disabled: saving,
            onclick: () => void (closing ? this.saveAndClose() : this.saveAndSwitch()),
          },
          this.t(closing ? "saveAndClose" : "saveAndSwitch"),
        ),
        h(
          "button",
          {
            type: "button",
            disabled: saving,
            onclick: () => void (closing ? this.env.desktop?.close() : this.select(target)),
          },
          this.t(closing ? "discardAndClose" : "discardAndSwitch"),
        ),
        h("button", { type: "button", onclick: () => this.cancelSwitch() }, this.t("cancelSwitch")),
      ),
    );
  }

  /** Whether a save of the text or of the answers is on its way: a work cannot be removed under it. */
  private somethingIsSaving(): boolean {
    return this.editor?.phase === "saving" || this.answers?.phase === "saving";
  }

  /** Whether the editor holds text, or the answers hold words, the core has not been given. */
  hasUnsavedChanges(): boolean {
    const text = this.editor !== null && (isDirty(this.editor) || this.editor.phase === "saving");
    const answers = this.answers !== null && (isAnswersDirty(this.answers) || this.answers.phase === "saving");
    return text || answers;
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
      h(
        "div",
        { class: "pane-head" },
        h("h2", {}, work.title),
        h(
          "button",
          {
            type: "button",
            class: "quiet",
            disabled: this.somethingIsSaving(),
            onclick: () => {
              this.removing = true;
              this.render();
            },
          },
          this.t("workRemove"),
        ),
      ),
      this.removing ? this.removeBanner(work) : null,
      // Two columns where the screen is wide enough: the text on one side and what
      // SCE says of its model on the other, because the page is read AGAINST the
      // text. Narrow, they stack in the same order.
      h(
        "div",
        { class: "columns" },
        h(
          "div",
          { class: "text-column" },
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
        ),
        h("div", { class: "model-column" }, this.modelPanel(work)),
      ),
    );
  }

  /** Asked before a work leaves the list; said in terms of what stays and how to bring it back. */
  private removeBanner(work: Work): HTMLElement {
    return h(
      "section",
      { class: "banner banner-warn", role: "alert" },
      h("strong", {}, this.t("removeTitle", { title: work.title })),
      h("p", {}, this.t("removeBody")),
      this.hasUnsavedChanges() ? h("p", {}, this.t("removeBodyUnsaved")) : null,
      h(
        "div",
        { class: "choices" },
        h(
          "button",
          { type: "button", disabled: this.somethingIsSaving(), onclick: () => void this.remove() },
          this.t("removeConfirm"),
        ),
        h(
          "button",
          {
            type: "button",
            onclick: () => {
              this.removing = false;
              this.render();
            },
          },
          this.t("removeCancel"),
        ),
      ),
    );
  }

  /** The model SCE draws of this work, where it stands to the text, and the model itself as text. */
  private modelPanel(work: Work): HTMLElement {
    const model = this.model;
    const busy = model === null || model.phase === "reading" || model.phase === "drawing";
    const read =
      model !== null && (model.phase === "drawing" || model.phase === "drawn" || model.phase === "not-drawn")
        ? model.read
        : null;
    return h(
      "section",
      { class: "model", "aria-label": this.t("modelTitle") },
      h(
        "div",
        { class: "model-head" },
        h("h3", {}, this.t("modelTitle")),
        h(
          "button",
          { type: "button", disabled: busy, onclick: () => void this.loadModel(work.id, true) },
          this.t("modelRead"),
        ),
        model !== null && model.phase === "drawn" ? this.zoomControl() : null,
        read === null
          ? null
          : h("span", { class: "muted rev" }, `${this.t("modelRevision")}: ${read.model.revision.slice(0, 12)}`),
      ),
      read === null ? null : this.standingBanner(read),
      read === null ? null : this.reviewSection(),
      this.modelBody(model),
      read === null ? null : this.scxmlOf(read),
    );
  }

  /**
   * The model's SCXML, folded. A model of several documents shows each under the file
   * name its imports know it by, the entry first and named as such: the person reads
   * what an import points at, not only the document that holds it.
   */
  private scxmlOf(read: ModelRead): HTMLElement {
    const documents = read.model.documents;
    if (documents.length <= 1) {
      return h(
        "details",
        { class: "model-scxml" },
        h("summary", {}, this.t("modelScxml")),
        h("pre", { class: "scxml" }, read.model.text),
      );
    }
    return h(
      "details",
      { class: "model-scxml" },
      h("summary", {}, this.t("modelScxmlSet", { count: String(documents.length) })),
      ...documents.flatMap((document) => [
        h(
          "h5",
          { class: "document-name" },
          h("code", {}, document.name),
          document.name === read.model.entry ? ` ${this.t("modelEntry")}` : null,
        ),
        h("pre", { class: "scxml" }, document.text),
      ]),
    );
  }

  /**
   * What SCE says of the model: its verdict, what the model leaves open, and the
   * pseudocode page the person reads against their own text. Every word of it is
   * the product's; the screen only arranges it. Said once, where it can be read:
   * a passed check does not say the model agrees with the text.
   */
  private reviewSection(): HTMLElement | null {
    const review = this.review;
    if (review === null) return null;
    const heading = h("h4", {}, this.t("reviewTitle"));
    if (review.phase === "reading") {
      return h("section", { class: "review" }, heading, h("p", { class: "muted" }, this.t("reviewReading")));
    }
    if (review.phase === "failed") {
      return h(
        "section",
        { class: "review" },
        heading,
        h("p", { class: "banner banner-error", role: "alert" }, this.t("reviewFailed", { detail: review.message })),
      );
    }
    const { check, page, page_refusal: pageRefusal } = review.review;
    const accepted = check.verdict === "accepted";
    return h(
      "section",
      { class: "review" },
      heading,
      h(
        "p",
        { class: accepted ? "banner banner-ok" : "banner banner-error", role: "status" },
        !accepted
          ? this.t("reviewRefused")
          : check.kind === null
            ? this.t("reviewAcceptedUnknownKind")
            : this.t("reviewAccepted", { kind: check.kind }),
      ),
      // Said right under the verdict, before anything else is read: a passed check
      // is the product's verdict on the document and not agreement with the text.
      accepted ? h("p", { class: "muted" }, this.t("reviewNote")) : null,
      accepted ? this.openMatters(check.open, check.unresolved.length) : null,
      accepted ? this.questions(check.unresolved) : null,
      accepted ? null : this.records(check.records),
      page === null
        ? null
        : h(
            "details",
            { class: "page", open: true },
            h("summary", {}, this.t("reviewPageTitle")),
            h("pre", { class: "pseudo" }, page),
          ),
      pageRefusal === null
        ? null
        : h(
            "p",
            { class: "banner banner-warn", role: "alert" },
            this.t("reviewPageRefused", { detail: `${pageRefusal.message} (${pageRefusal.code})` }),
          ),
    );
  }

  /** What an accepted model leaves to a person, in SCE's own sentences. */
  private openMatters(open: readonly string[], questions: number): HTMLElement | null {
    if (open.length === 0) {
      // Nothing in SCE's words, and no question to answer either: say so once.
      return questions === 0 ? h("p", { class: "muted" }, this.t("reviewNothingOpen")) : null;
    }
    return h(
      "div",
      { class: "open-matters" },
      h("h5", {}, this.t("reviewOpenTitle")),
      h("ul", {}, ...open.map((sentence) => h("li", {}, sentence))),
    );
  }

  /**
   * The questions the model marks as not decided, each with a field for the owner's
   * answer, and the answers to questions the model no longer asks. The answers go
   * to the authoring client the next time it reads the work (`works_read`), which
   * applies them; nothing here writes into the model.
   */
  private questions(unresolved: readonly Unresolved[]): HTMLElement | null {
    const model = this.answers;
    const asked = [...new Map(unresolved.map((u) => [u.id, u])).values()];
    if (model === null) {
      return asked.length === 0 && this.answersUnreadable === null
        ? null
        : h(
            "p",
            { class: this.answersUnreadable === null ? "muted" : "banner banner-error" },
            this.answersUnreadable === null
              ? this.t("answersReading")
              : this.t("answersFailed", { detail: this.answersUnreadable }),
          );
    }
    const askedIds = new Set(asked.map((u) => u.id));
    const orphans = Object.keys(model.saved)
      .filter((id) => !askedIds.has(id))
      .sort();
    if (asked.length === 0 && orphans.length === 0) return null;

    const field = (id: string, label: Child[]): HTMLElement => {
      const input = h("textarea", {
        class: "answer-input",
        "data-qid": id,
        rows: "2",
        spellcheck: "false",
        "aria-label": this.t("answerLabel", { id }),
        oninput: (event) => this.typeAnswer(id, (event.target as HTMLTextAreaElement).value),
      });
      input.value = wordsOf(model, id);
      const entry = model.saved[id];
      return h(
        "div",
        { class: "question" },
        h("div", { class: "question-text" }, ...label),
        input,
        entry === undefined
          ? null
          : h(
              "span",
              { class: "muted answered-at" },
              this.t("answerAt", { time: formatTime(entry.answered_at, this.locale) }),
            ),
      );
    };

    return h(
      "section",
      { class: "answers" },
      h("h5", {}, this.t("answersTitle")),
      h("p", { class: "muted" }, this.t("answersHint")),
      ...asked.map((u) =>
        field(u.id, [
          h("strong", {}, u.reason ?? this.t("answerNoWording")),
          " ",
          h(
            "span",
            { class: "muted" },
            u.line === null
              ? this.t("reviewUnresolvedNoLine", { id: u.id })
              : this.t("reviewUnresolvedAt", { id: u.id, line: String(u.line) }),
          ),
        ]),
      ),
      orphans.length === 0
        ? null
        : h(
            "div",
            { class: "orphans" },
            h("h5", {}, this.t("answersOrphanTitle")),
            h("p", { class: "muted" }, this.t("answersOrphanNote")),
            ...orphans.map((id) => field(id, [h("code", {}, id)])),
          ),
      model.phase === "conflict" ? h("p", { class: "banner banner-warn", role: "alert" }, this.t("answersConflict")) : null,
      model.phase === "failed"
        ? h("p", { class: "banner banner-error", role: "alert" }, this.t("answersSaveFailed", { detail: model.failure ?? "" }))
        : null,
      h(
        "div",
        { class: "bar" },
        h("button", { id: "save-answers", type: "button", onclick: () => void this.saveAnswers() }, this.t("answersSave")),
        h("span", { id: "answers-status", class: "status", role: "status", "aria-live": "polite" }),
      ),
    );
  }

  /** Every record SCE wrote about a model it refused, with its code and where it found it. */
  private records(records: readonly { code: string; message: string; line: number | null }[]): HTMLElement {
    return h(
      "div",
      { class: "records" },
      h("h5", {}, this.t("reviewRecordsTitle")),
      h(
        "ul",
        {},
        ...records.map((r) =>
          h(
            "li",
            {},
            r.message,
            " ",
            h("code", {}, r.code),
            r.line === null ? null : ` ${this.t("reviewRecordLine", { line: String(r.line) })}`,
          ),
        ),
      ),
    );
  }

  /** The sizes a drawing can be shown at; the one in use is pressed. */
  private zoomControl(): HTMLElement {
    return h(
      "span",
      { class: "zoom", role: "group", "aria-label": this.t("modelZoom") },
      this.t("modelZoom"),
      ...ZOOMS.map((zoom) =>
        h(
          "button",
          {
            type: "button",
            "aria-pressed": String(zoom === this.zoom),
            onclick: () => this.chooseZoom(zoom),
          },
          `${Math.round(zoom * 100)}%`,
        ),
      ),
    );
  }

  /** Whether the model was written for the text on screen, in the words the core's `standing` carries. */
  private standingBanner(read: ModelRead): HTMLElement {
    const short = (revision: string | null): string => (revision === null ? "?" : revision.slice(0, 12));
    if (read.standing === "current") {
      return h("p", { class: "banner banner-ok", role: "status" }, this.t("modelCurrent"));
    }
    if (read.standing === "behind") {
      const written = short(read.model.written_for);
      return h(
        "p",
        { class: "banner banner-warn", role: "status" },
        read.sourceHead === null
          ? this.t("modelBehindUnknown", { written })
          : this.t("modelBehind", { written, now: short(read.sourceHead) }),
      );
    }
    return h("p", { class: "banner", role: "status" }, this.t("modelUnstated"));
  }

  private modelBody(model: ModelPanel | null): HTMLElement | null {
    if (model === null || model.phase === "reading") {
      return h("p", { class: "muted" }, this.t("modelReading"));
    }
    switch (model.phase) {
      case "none":
        return h("p", { class: "muted" }, this.t("modelNone"));
      case "drawing":
        return h("p", { class: "muted" }, this.t("modelDrawing"));
      case "failed":
        return h("p", { class: "banner banner-error", role: "alert" }, model.message);
      case "not-drawn":
        return h(
          "section",
          { class: "banner banner-error", role: "alert" },
          h("strong", {}, this.t("modelNotDrawn")),
          h("p", {}, model.failure.message),
          model.failure.code === null ? null : h("p", { class: "muted" }, model.failure.code),
          h("p", { class: "muted" }, this.t("modelNotDrawnHint")),
        );
      case "drawn":
        return h(
          "div",
          {},
          h(
            "div",
            { class: "sheets", style: `--sheet-zoom: ${this.zoom}` },
            ...model.figures.sheets.map((sheet) =>
              h(
                "figure",
                { class: "sheet" },
                h("img", { src: svgAddress(sheet.svg), alt: sheetName(sheet.name) }),
                h("figcaption", {}, sheet.name),
              ),
            ),
          ),
          model.figures.generator === null
            ? null
            : h("p", { class: "muted" }, this.t("modelGenerator", { generator: model.figures.generator })),
        );
    }
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
    this.reportUnsaved();
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
