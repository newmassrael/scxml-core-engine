// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The screen: the works on one side, the selected work's text and history on the
// other. It decides nothing about the text. Every rule about what a save is lives
// in the core and the editor model; this file asks, shows the answer, and offers
// the person the choices a refusal leaves.

import {
  acceptRefused,
  accepting,
  gate,
  isUnsettled,
  tally,
  type AcceptancePanel,
  type AcceptanceState,
  type Shown,
  type Withheld,
} from "./acceptance_model";
import { apiOver, type Api } from "./api";
import { answerState, type AnswerState } from "./answer_states";
import { comparePages, previousOf, type ChangePanel } from "./change_model";
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
  isOpenRequest,
  type AdapterListing,
  type BundleRead,
  type Described,
  type GenerationRequest,
  type HistoryEntry,
  type Listing,
  type ReadAcceptance,
  type RequestHead,
  type RequirementOutcome,
  type RequirementsReport,
  type SourceText,
  type Unresolved,
  type Work,
  type WorkHeads,
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
import { findQuote, groundOf, linesNaming, quotesOf, statesOf } from "./grounding_model";
import {
  connectedNames,
  controlsOf,
  isConnected,
  pressKey,
  savingEndsARequest,
  statusOf,
  type Status,
} from "./generation_model";
import { movedParts, sameHeads, sameRequest, type WorkOnScreen } from "./heads_model";
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
import { nextDelay, WATCH_MS, type Ticker } from "./watch";

const LOCALE_KEY = "sce.locale";
const ZOOM_KEY = "sce.zoom";

/** SCE's words for a requirement, each with the sentence that says what it means. A word SCE adds later is shown as spelled. */
const OUTCOME_SENTENCES: Record<string, Key> = {
  implemented: "outcomeImplemented",
  "scenario-passed": "outcomeScenarioPassed",
  missing: "outcomeMissing",
  unresolved: "outcomeUnresolved",
  dangling: "outcomeDangling",
  contradicted: "outcomeContradicted",
  "scenario-failed": "outcomeScenarioFailed",
  "needs-scenario": "outcomeNeedsScenario",
  delegated: "outcomeDelegated",
  "out-of-scope": "outcomeOutOfScope",
  "system-level": "outcomeSystemLevel",
};

/** The words for where an answer stands. */
const ANSWER_STATE_WORDS: Record<AnswerState, Key> = {
  unsaved: "answerStateUnsaved",
  saved: "answerStateSaved",
  writing: "answerStateWriting",
  "in-model": "answerStateInModel",
  ignored: "answerStateIgnored",
};

/** The sentence that says why the accept button is not offered; `accepting` is said by the button itself. */
const WITHHELD_WORDS: Record<Exclude<Withheld, "accepting">, Key> = {
  unsaved: "withheldUnsaved",
  "not-measured": "withheldNotMeasured",
  behind: "withheldBehind",
  unread: "withheldUnread",
  differs: "withheldDiffers",
  already: "withheldAlready",
};

export interface Environment {
  readonly transport: Transport;
  /** Present when the server wants a token the person can be asked for. */
  readonly credentials?: Credentials | undefined;
  /** Present inside a desktop window: told what is unsaved, and asked to close it. */
  readonly desktop?: Desktop | undefined;
  readonly storage: Pick<Storage, "getItem" | "setItem"> | null;
  readonly browserLanguage: string | undefined;
  /**
   * What wakes the screen to ask whether the selected work moved under it (a save from
   * another window, an authoring client's next model). Absent: it asks only when the
   * person does something.
   */
  readonly ticker?: Ticker | undefined;
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
  /** How the model shown differs from the one it replaced, and the newest request for it. */
  private change: ChangePanel | null = null;
  private changeTicket = 0;
  /** The owner's answers to the model's open questions, and what is typed into them. */
  private answers: AnswersModel | null = null;
  /** Why the answers could not be read, when they could not. */
  private answersUnreadable: string | null = null;
  /** The newest request for the answers; an older one that answers later is dropped. */
  private answersTicket = 0;
  /** The requirement list, SCE's measure of the design against it, and what the owner accepted. */
  private acceptance: AcceptancePanel | null = null;
  /** The newest request for the acceptance; an older one that answers later is dropped. */
  private acceptanceTicket = 0;
  /** A work the person asked for while the editor held text that is not saved. */
  private pendingSwitch: Work | null = null;
  /** The window was asked to close while something was not saved, and the person has not yet said what to do. */
  private closing = false;
  /** What the desktop shell was last told is unsaved, so it is told only what changed. */
  private reportedUnsaved: boolean | null = null;
  /** What is typed in the new-work field, kept across redraws. */
  private draftTitle = "";
  /** Stops the questions about whether the selected work moved, and the one waiting to be asked. */
  private stopWatching: (() => void) | null = null;
  /**
   * What the core said when the screen last read the work again because it had moved. A
   * question that gets the same answer is not acted on twice: a read that is slow, or one
   * that failed, is not started over every few seconds.
   */
  private reactedTo: WorkHeads | null = null;
  /** Where the work's latest request stands, as the core last said, and what was read of it. */
  private requestHead: RequestHead | null = null;
  private requestDetail: GenerationRequest | null = null;
  /** Which AI adapters are there, as last read. */
  private adapters: AdapterListing | null = null;
  /** A request being made or called off now: a second press would be a second request. */
  private requestBusy: "making" | "cancelling" | null = null;
  /** Why the last attempt to ask was not made, in words; and whether a request that is open may be replaced. */
  private requestNotice: string | null = null;
  private offerReplace = false;
  /** The person pressed save while a request was open, and has not yet said what to do. */
  private guarding = false;
  /**
   * The bundle that is the model shown, and the answers it was made about. `bundleKey` is the
   * revision last read (`null`: the work has none, `undefined`: not asked yet), so that a bundle
   * is read when it changes and not at every question.
   */
  private bundle: BundleRead | null = null;
  private bundleAnswers: Readonly<Record<string, string>> | null = null;
  private bundleKey: string | null | undefined = undefined;
  /** The questions the model shown asks, for the state of each answer as it is typed. */
  private askedNow: ReadonlySet<string> = new Set();
  /** A requirement the person marked in the pseudocode: the lines that name the states it is carried by are lit. */
  private marked: { readonly requirement: string; readonly states: readonly string[] } | null = null;
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
    // The questions about the work on screen stopped when the server asked for the token.
    if (this.selected !== null && !this.needsToken && this.fatal === null) this.watch();
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
    // What the screen held when the read began. `openWork` asked about unsaved text at
    // that moment only, and a read takes time: whatever is typed after it is not covered
    // by that question, so it is asked again when the answer arrives.
    const editorAtAsk = this.editor;
    const answersAtAsk = this.answers;
    try {
      const [source, entries] = await Promise.all([
        this.api.readSource(work.id),
        this.api.history(work.id),
      ]);
      if (ticket !== this.opening) return;
      if (this.typedSince(editorAtAsk, answersAtAsk)) {
        this.pendingSwitch = work;
        this.render();
        return;
      }
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
      this.changeTicket += 1;
      this.change = null;
      this.answersTicket += 1;
      this.answers = null;
      this.answersUnreadable = null;
      this.acceptanceTicket += 1;
      this.acceptance = null;
      this.reactedTo = null;
      this.requestHead = null;
      this.requestDetail = null;
      this.requestBusy = null;
      this.requestNotice = null;
      this.offerReplace = false;
      this.guarding = false;
      this.bundle = null;
      this.bundleAnswers = null;
      this.bundleKey = undefined;
      this.askedNow = new Set();
      this.marked = null;
      opened = true;
    } catch (error) {
      if (ticket !== this.opening) return;
      this.report(error);
    }
    this.render();
    if (opened) {
      void this.loadModel(work.id, true);
      void this.loadAnswers(work.id);
      void this.loadRequest(work.id, this.session);
      this.watch();
    }
  }

  /** Where the work's request stands, and which AIs are there, when the work opens. */
  private async loadRequest(id: string, session: number): Promise<void> {
    try {
      const heads = await this.api.readWorkHeads(id);
      if (session !== this.session) return;
      await this.noteRequest(id, session, heads);
    } catch (error) {
      if (session === this.session) this.askForToken(error);
    }
  }

  /**
   * Take what the core says of the latest request and of the adapters, and draw it again if
   * either moved. Asked on every question about the work, so a request an executor took, or
   * let go of, or finished, is on the screen without the person pressing anything.
   */
  private async noteRequest(id: string, session: number, heads: WorkHeads): Promise<void> {
    const head = heads.request;
    let changed = !sameRequest(this.requestHead, head);
    this.requestHead = head;
    if (head === null) {
      this.requestDetail = null;
    } else if (changed || this.requestDetail?.id !== head.id) {
      // Who holds it, why it failed, and which answers it was made about are in the request
      // and not in the heads.
      if (isOpenRequest(head.state) || head.state === "failed") {
        try {
          const read = await this.api.readRequest(id, head.id);
          if (session !== this.session) return;
          this.requestDetail = read;
          changed = true;
        } catch (error) {
          if (session !== this.session) return;
          this.askForToken(error);
        }
      }
    }
    try {
      const listing = await this.api.readAdapterStatus();
      if (session !== this.session) return;
      if (adaptersKey(this.adapters) !== adaptersKey(listing)) changed = true;
      this.adapters = listing;
    } catch (error) {
      if (session !== this.session) return;
      this.askForToken(error);
    }
    if (await this.noteBundle(id, session, heads.bundle)) changed = true;
    if (session !== this.session) return;
    if (changed) this.render();
  }

  /**
   * What the model shown was made from, when a request made it: the bundle, and the answers it
   * was about. Read when the bundle the core names is not the one held; a model that no request
   * made has none, and an answer is then only saved. Answers whether anything changed.
   */
  private async noteBundle(id: string, session: number, revision: string | null): Promise<boolean> {
    if (this.bundleKey === revision) return false;
    if (revision === null) {
      const had = this.bundle !== null;
      this.bundleKey = null;
      this.bundle = null;
      this.bundleAnswers = null;
      return had;
    }
    try {
      const read = await this.api.readBundle(id);
      if (session !== this.session) return false;
      let answers: Record<string, string> = {};
      if (read !== null && read.bundle.answers !== null) {
        const made = await this.api.readAnswers(id, read.bundle.answers);
        if (session !== this.session) return false;
        answers = Object.fromEntries(Object.entries(made?.entries ?? {}).map(([q, e]) => [q, e.answer]));
      }
      this.bundle = read;
      this.bundleAnswers = read === null ? null : answers;
      this.bundleKey = revision;
      return true;
    } catch (error) {
      // Asked again at the next question: the answers read as saved until then.
      if (session === this.session) this.askForToken(error);
      return false;
    }
  }

  /**
   * The person pressed the button: the text and the answers they typed are saved first, and a
   * request is made about what is then saved. Nothing is asked of the AI that the person has
   * not seen: the request names the text and the answers THIS screen shows, and the core
   * refuses it (`moved`) when the work is no longer at them.
   */
  private async generate(replace: boolean): Promise<void> {
    const work = this.selected;
    if (work === null || this.requestBusy !== null) return;
    const session = this.session;
    this.requestBusy = "making";
    this.requestNotice = null;
    this.offerReplace = false;
    this.render();
    try {
      // The button saves what is typed (§ conflicts first: a save that did not take is
      // the person's to resolve, and nothing is asked in the meantime).
      if (this.editor !== null && isDirty(this.editor)) await this.save(true);
      if (this.answers !== null && isAnswersDirty(this.answers)) await this.saveAnswers(true);
      if (session !== this.session) return;
      const editor = this.editor;
      if (editor === null || editor.base === null || editor.phase !== "idle" || isDirty(editor)) return;
      if (this.answers !== null && (this.answers.phase !== "idle" || isAnswersDirty(this.answers))) return;
      const registered = await this.api.requestGeneration(
        work.id,
        pressKey(),
        { source: editor.base, answers: this.answers?.base ?? null },
        replace,
      );
      if (session !== this.session) return;
      this.requestHead = {
        id: registered.request.id,
        state: registered.request.state,
        attempt: registered.request.attempt,
      };
      this.requestDetail = registered.request;
    } catch (error) {
      if (session !== this.session) return;
      if (this.askForToken(error)) return;
      if (error instanceof CommandFailure && error.kind === "moved") {
        this.requestNotice = this.t("generationMoved");
        void this.lookAgain(work, session).catch(() => undefined);
      } else if (error instanceof CommandFailure && error.kind === "active-request") {
        this.requestNotice = this.t("generationActive");
        this.offerReplace = true;
      } else {
        this.requestNotice = this.t("generationRefused", { detail: this.explain(error) });
      }
    } finally {
      if (session === this.session) {
        this.requestBusy = null;
        this.render();
      }
    }
  }

  /** The person called the open request off. Whoever holds it is told at its next word. */
  private async cancelGeneration(): Promise<void> {
    const work = this.selected;
    const head = this.requestHead;
    if (work === null || head === null || this.requestBusy !== null) return;
    const session = this.session;
    this.requestBusy = "cancelling";
    this.requestNotice = null;
    this.render();
    try {
      const cancelled = await this.api.cancelRequest(work.id, head.id);
      if (session !== this.session) return;
      this.requestHead = { id: cancelled.id, state: cancelled.state, attempt: cancelled.attempt };
      this.requestDetail = cancelled;
    } catch (error) {
      if (session !== this.session) return;
      if (this.askForToken(error)) return;
      this.requestNotice = this.t("generationRefused", { detail: this.explain(error) });
      // What it says now is on screen at the next question; a request that ended meanwhile is not one to cancel.
      void this.lookAgain(work, session).catch(() => undefined);
    } finally {
      if (session === this.session) {
        this.requestBusy = null;
        this.render();
      }
    }
  }

  /**
   * A save of the text or the answers while a request is open ends it, in the core, in the
   * same step. The person is told first, because what the request writes would not be
   * published: they save and let it end, or leave it. Answers whether the save was held.
   */
  private holdSaveForTheRequest(): boolean {
    if (!savingEndsARequest(this.requestHead)) return false;
    this.guarding = true;
    this.render();
    return true;
  }

  /** What the person chose at the guard: saved, and the request ended with it. */
  private async saveAndEndTheRequest(): Promise<void> {
    this.guarding = false;
    const target = this.pendingSwitch;
    await this.save(true);
    await this.saveAnswers(true);
    if (target !== null && !this.hasUnsavedChanges()) {
      await this.select(target);
    } else if (this.closing && !this.hasUnsavedChanges()) {
      await this.env.desktop?.close();
    }
  }

  /** A save went through: a request that was open was ended by it, as the core does. */
  private noteSaveEndedTheRequest(): void {
    const head = this.requestHead;
    if (head !== null && savingEndsARequest(head)) this.requestHead = { ...head, state: "superseded" };
  }

  // ---- the work moving under the screen ------------------------------------

  /**
   * Ask the core every few seconds whether the selected work moved under what is shown,
   * and read again only what did. An authoring client's model, a save from another window
   * and an acceptance made elsewhere appear without the person pressing anything, and
   * what they are typing is not touched.
   *
   * One question at a time: the next is scheduled when the last, and what it led to, is
   * done. A question that fails is asked again later (twice as late each time, up to a
   * limit), and one the server answers with a refusal for want of a token stops the
   * questions until the person signs in.
   */
  private watch(): void {
    this.stopWatching?.();
    this.stopWatching = null;
    const ticker = this.env.ticker;
    if (ticker === undefined) return;
    const session = this.session;
    let cancelled = false;
    let cancelTimer: (() => void) | null = null;
    let delay = WATCH_MS;
    const schedule = (): void => {
      cancelTimer = ticker.after(delay, () => void ask());
    };
    const ask = async (): Promise<void> => {
      const work = this.selected;
      if (cancelled || session !== this.session || work === null) return;
      try {
        await this.lookAgain(work, session);
        delay = WATCH_MS;
      } catch (error) {
        if (cancelled || session !== this.session) return;
        // The sign-in form is drawn, and the questions start again when the person has signed in.
        if (this.askForToken(error)) return;
        if (error instanceof CommandFailure && error.kind === "not-found") {
          // The work was taken away from another window: there is nothing to ask about.
          this.notice = this.explain(error);
          this.render();
          return;
        }
        delay = nextDelay(delay);
      }
      if (!cancelled && session === this.session) schedule();
    };
    this.stopWatching = () => {
      cancelled = true;
      cancelTimer?.();
    };
    schedule();
  }

  /** Ask the core where the work stands and read again what is shown differently. */
  private async lookAgain(work: Work, session: number): Promise<void> {
    const heads = await this.api.readWorkHeads(work.id);
    if (session !== this.session) return;
    // The request and the adapters are noted whether or not a chain moved: an executor taking
    // a request, or letting go of it, moves no chain.
    await this.noteRequest(work.id, session, heads);
    if (session !== this.session) return;
    const moved = movedParts(this.onScreen(), heads);
    if (moved.length === 0 || sameHeads(this.reactedTo, heads)) return;
    this.reactedTo = heads;
    if (moved.includes("source")) await this.refreshSource(work.id, session);
    const answersRead = moved.includes("answers") && (await this.refreshAnswers(work.id, session));
    if (session !== this.session) return;
    // A model that moved, or whose text moved under it (`movedParts` says so), is read
    // again, and the acceptance with it. Otherwise a list, an acceptance or answers that
    // moved are the acceptance's to read again.
    if (moved.includes("model")) {
      await this.loadModel(work.id, false);
    } else if (moved.includes("requirements") || moved.includes("acceptance") || answersRead) {
      await this.loadAcceptance(work.id);
    }
  }

  /**
   * What is on screen of each part of the work, for comparing with the core's heads. A
   * part is left out (`undefined`) while it is being read or saved, holds what the person
   * typed, or could not be read: it is not read again over their head.
   */
  private onScreen(): WorkOnScreen {
    const editor = this.editor;
    const answers = this.answers;
    const model = this.model;
    const panel = this.acceptance;
    const list = panel !== null && panel.phase === "read" && !panel.state.accepting ? panel.state : null;
    return {
      source: editor === null || editor.phase !== "idle" || isDirty(editor) ? undefined : editor.base,
      model:
        model === null || model.phase === "reading" || model.phase === "failed"
          ? undefined
          : model.phase === "none"
            ? null
            : {
                head: { revision: model.read.model.revision, written_for: model.read.model.written_for },
                sourceHead: model.read.sourceHead,
              },
      answers: answers === null || answers.phase !== "idle" || isAnswersDirty(answers) ? undefined : answers.base,
      requirements:
        panel === null
          ? undefined
          : panel.phase === "no-list"
            ? null
            : list === null
              ? undefined
              : list.list.requirements === null
                ? null
                : {
                    head: {
                      revision: list.list.requirements.revision,
                      written_for: list.list.requirements.written_for,
                    },
                    sourceHead: list.list.source_head,
                  },
      acceptance:
        panel === null
          ? undefined
          : panel.phase === "no-list"
            ? null
            : list === null
              ? undefined
              : (list.acceptance.acceptance?.revision ?? null),
    };
  }

  /**
   * Another entrance saved the text. When the editor holds nothing of the person's it is
   * shown as it now is; typed text is not replaced, and the conflict a save would meet is
   * the person's to resolve. Answers whether the text was read again.
   */
  private async refreshSource(id: string, session: number): Promise<boolean> {
    const asked = this.editor;
    if (asked === null || asked.phase !== "idle" || isDirty(asked)) return false;
    try {
      const [source, entries] = await Promise.all([this.api.readSource(id), this.api.history(id)]);
      if (session !== this.session) return false;
      const editor = this.editor;
      // Loading replaces what is in the editor, so it is done only to the text it was
      // asked about: anything typed or saved while the answer was on its way is the person's.
      if (editor === null || editor.phase !== "idle" || editor.text !== asked.text || editor.base !== asked.base) {
        return false;
      }
      this.editor = open(id, source);
      this.entries = entries;
      this.render();
      return true;
    } catch (error) {
      if (session === this.session) this.askForToken(error);
      return false;
    }
  }

  /** The owner's answers were saved from another entrance and none are typed here: they are shown. */
  private async refreshAnswers(id: string, session: number): Promise<boolean> {
    const asked = this.answers;
    if (asked === null || asked.phase !== "idle" || isAnswersDirty(asked)) return false;
    try {
      const read = await this.api.readAnswers(id);
      if (session !== this.session) return false;
      const answers = this.answers;
      if (answers === null || answers.phase !== "idle" || isAnswersDirty(answers) || answers.base !== asked.base) {
        return false;
      }
      this.answers = openAnswers(read);
      this.render();
      return true;
    } catch (error) {
      if (session === this.session) this.askForToken(error);
      return false;
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
  private async saveAnswers(confirmed = false): Promise<void> {
    const model = this.answers;
    const work = this.selected;
    const request = model === null ? null : answersRequest(model);
    if (model === null || work === null || request === null) return;
    if (!confirmed && this.holdSaveForTheRequest()) return;
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

    this.noteSaveEndedTheRequest();
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
    // The owner's answers are part of what an acceptance is of, so what they now say
    // moves whether it holds.
    if (session === this.session) void this.loadAcceptance(work.id);
  }

  /** The save button and its words follow every keystroke, without redrawing the field being typed in. */
  private refreshAnswersChrome(): void {
    this.refreshAcceptChrome();
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
    this.refreshAnswerStates();
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
  private async loadModel(id: string, redraw: boolean, alsoAcceptance = true): Promise<void> {
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
        this.acceptanceTicket += 1;
        this.acceptance = null;
        this.render();
        return;
      }
      const known: ModelRead = { model: read.model, standing: read.standing, sourceHead: read.source_head };
      // What was accepted is of the model as it is now, and the text may have moved
      // under it: asked again whenever the model is read, drawn again or not.
      if (alsoAcceptance) void this.loadAcceptance(id);
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
      // What this model changed from the one before it is read beside the review.
      void this.loadChange(id, revision, review.page);
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

  /**
   * How the model shown differs from the one it replaced, as lines of their pseudocode pages.
   * The earlier model is the one before this in the core's model history; SCE writes its page
   * as it wrote this one's. Only the newest request, for the work that asked, is applied.
   */
  private async loadChange(id: string, revision: string, page: string | null): Promise<void> {
    const session = this.session;
    const ticket = ++this.changeTicket;
    const current = (): boolean => session === this.session && ticket === this.changeTicket;
    this.change = { phase: "reading" };
    try {
      const previous = previousOf(await this.api.modelHistory(id), revision);
      if (!current()) return;
      if (previous === null) {
        this.change = { phase: "none" };
      } else {
        const before = await this.api.review(id, previous, this.locale);
        if (!current()) return;
        this.change = comparePages(before.page, page);
      }
    } catch (error) {
      if (!current()) return;
      if (this.askForToken(error)) return;
      this.change = { phase: "unavailable", reason: this.explain(error) };
    }
    this.render();
  }

  /** What changed from the model before, in the pseudocode; nothing for a first model. */
  private changeSection(): HTMLElement | null {
    const change = this.change;
    if (change === null || change.phase === "none") return null;
    switch (change.phase) {
      case "reading":
        return h("p", { class: "muted change", "data-change": "reading" }, this.t("changeReading"));
      case "unchanged":
        return h("p", { class: "muted change", "data-change": "unchanged" }, this.t("changeNone"));
      case "unavailable":
        return h(
          "p",
          { class: "muted change", "data-change": "unavailable" },
          change.reason === "before" || change.reason === "after"
            ? this.t(change.reason === "before" ? "changeNoPageBefore" : "changeNoPageAfter")
            : this.t("changeFailed", { detail: change.reason }),
        );
      case "changed":
        return h(
          "details",
          { class: "change", "data-change": "changed", open: true },
          h(
            "summary",
            {},
            this.t("changeTitle", { added: String(change.added), removed: String(change.removed) }),
          ),
          h("p", { class: "muted" }, this.t("changeNote")),
          h(
            "pre",
            { class: "pseudo diff" },
            ...change.hunks.flatMap((hunk, index) => [
              index === 0 ? null : h("span", { class: "gap" }, "...\n"),
              ...hunk.map((line) =>
                h(
                  "span",
                  { class: line.kind },
                  `${line.kind === "added" ? "+ " : line.kind === "removed" ? "- " : "  "}${line.text}\n`,
                ),
              ),
            ]),
          ),
        );
    }
  }

  /**
   * Read the requirement list, SCE's measure of the design against it, and whether the
   * owner's acceptance still holds. SCE not measuring does not hide an acceptance the
   * owner made, and a work with no list says so instead of showing an empty table.
   * Only the newest request, for the editor that asked, is applied.
   */
  private async loadAcceptance(id: string): Promise<void> {
    const session = this.session;
    const ticket = ++this.acceptanceTicket;
    const current = (): boolean => session === this.session && ticket === this.acceptanceTicket;
    // A panel already on screen stays until the new answer replaces it: a reread after
    // an accept or a save does not flash a "reading" over what the person is looking at.
    if (this.acceptance === null) this.acceptance = { phase: "reading" };
    try {
      const list = await this.api.readRequirements(id);
      if (!current()) return;
      if (list.requirements === null) {
        this.acceptance = { phase: "no-list" };
        this.render();
        return;
      }
      const [acceptance, measured] = await Promise.all([this.api.readAcceptance(id), this.measure(id)]);
      if (!current()) return;
      this.acceptance = {
        phase: "read",
        state: {
          list,
          acceptance,
          report: measured.report,
          measureFailure: measured.failure,
          accepting: false,
          refusal: null,
        },
      };
    } catch (error) {
      if (!current()) return;
      // A token wanted is the sign-in form's to answer; anything else is the panel's own message.
      if (this.askForToken(error)) return;
      this.acceptance = { phase: "failed", message: this.explain(error) };
    }
    this.render();
  }

  /**
   * SCE's measure of the design against the list. SCE saying no, or nothing, is a state
   * of the panel (`failure`); anything else (the server unreachable, a wrong token, an
   * answer in a shape this screen does not know) is thrown for the caller to report.
   */
  private async measure(id: string): Promise<{ report: RequirementsReport | null; failure: string | null }> {
    try {
      return { report: await this.api.requirementsReport(id), failure: null };
    } catch (error) {
      const failure = drawFailureOf(error);
      if (failure === null) throw error;
      return { report: null, failure: failure.message };
    }
  }

  /**
   * The owner pressed accept on the page they were shown. What they were shown (the
   * revisions in its `basis`) is what is sent, and the core accepts it only if all of
   * it is still what is saved. Whatever the answer, what is there NOW is read and
   * shown, so a refusal comes with the page it refers to and the next press is made
   * knowing what changed.
   */
  private async accept(): Promise<void> {
    const panel = this.acceptance;
    const work = this.selected;
    if (panel === null || panel.phase !== "read" || work === null) return;
    const report = panel.state.report;
    if (report === null || gate(panel.state, this.hasUnsavedChanges(), this.shown()) !== null) return;
    const session = this.session;
    this.acceptance = { phase: "read", state: accepting(panel.state) };
    this.render();
    let refusal: string | null = null;
    try {
      await this.api.accept(work.id, report.basis);
    } catch (error) {
      if (session !== this.session) return;
      if (this.askForToken(error)) return;
      refusal = this.explain(error);
    }
    await this.loadAcceptance(work.id);
    if (session !== this.session || refusal === null) return;
    const now = this.acceptance;
    if (now !== null && now.phase === "read") {
      this.acceptance = { phase: "read", state: acceptRefused(now.state, refusal) };
      this.render();
    }
    // The core named what it holds now. The page was read again above; the design the
    // screen draws is read again here (the page is not measured a second time), so that
    // what the owner is looking at is what the next press would accept: `gate` holds the
    // button until it is.
    void this.loadModel(work.id, false, false);
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
      this.acceptanceTicket += 1;
      this.looking += 1;
      this.review = null;
      this.answers = null;
      this.answersUnreadable = null;
      this.acceptance = null;
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

  private async save(confirmed = false): Promise<void> {
    const editor = this.editor;
    const request = editor === null ? null : saveRequest(editor);
    if (editor === null || request === null) return;
    if (!confirmed && this.holdSaveForTheRequest()) return;
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
      // An unchanged save moved nothing, so it ended nothing.
      if (outcome.outcome === "saved") this.noteSaveEndedTheRequest();
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

  /**
   * Whether `error` is the server wanting a token the person can type in; if so, the
   * sign-in form is drawn now. It is drawn here and not left to the caller because the
   * reads that open a work answer in no fixed order: a refusal that came after the last
   * redraw and only set a flag left the screen as it was, with no way to sign in.
   */
  private askForToken(error: unknown): boolean {
    const wanted =
      error instanceof CommandFailure &&
      error.kind === UNAUTHORIZED &&
      this.env.credentials !== undefined;
    if (wanted) {
      this.needsToken = true;
      this.render();
    }
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

  /** Whether a save of the text or of the answers, or an acceptance, is on its way: a work cannot be removed under it. */
  private somethingIsSaving(): boolean {
    const acceptInFlight = this.acceptance?.phase === "read" && this.acceptance.state.accepting;
    return this.editor?.phase === "saving" || this.answers?.phase === "saving" || acceptInFlight;
  }

  /**
   * Whether the text or the answers were changed after `editor` and `answers` were
   * taken, and now hold something not given to the core. Changed and then put back
   * to what is saved holds nothing; and what was already unsaved when they were taken
   * is the person's to have chosen to give up, which is not asked twice.
   */
  private typedSince(editor: EditorModel | null, answers: AnswersModel | null): boolean {
    return (this.editor !== editor || this.answers !== answers) && this.hasUnsavedChanges();
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
        h("div", { class: "model-column" }, this.generationSection(), this.modelPanel(work)),
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

  /**
   * Asking for a model, and where the request stands. The sentence is the core's state in the
   * person's words, and only what the core has said: no percentage is made up for an AI that
   * is working, and a request nobody is there to take is not shown as running.
   */
  private generationSection(): HTMLElement {
    const status = statusOf(this.requestHead, this.requestDetail, this.adapters);
    const controls = controlsOf(status, this.requestBusy !== null);
    const text = this.editor !== null && this.editor.base !== null;
    const modelHere = this.model !== null && (this.model.phase === "drawn" || this.model.phase === "not-drawn" || this.model.phase === "drawing");
    const label = controls.replaces
      ? this.t("generateReplace")
      : status.kind === "idle" && !modelHere
        ? this.t("generateFirst")
        : this.t("generateAgain");
    const busyWords =
      this.requestBusy === "making"
        ? this.t("generateRegistering")
        : this.requestBusy === "cancelling"
          ? this.t("generateCancelling")
          : null;
    return h(
      "section",
      { class: "generation", "aria-label": this.t("generationTitle") },
      h(
        "div",
        { class: "model-head" },
        h("h3", {}, this.t("generationTitle")),
        controls.canGenerate
          ? h(
              "button",
              {
                id: "generate",
                type: "button",
                disabled: !text,
                onclick: () => void this.generate(controls.replaces),
              },
              label,
            )
          : null,
        controls.canCancel
          ? h(
              "button",
              { id: "cancel-request", type: "button", class: "quiet", onclick: () => void this.cancelGeneration() },
              this.t("generateCancel"),
            )
          : null,
      ),
      this.guarding ? this.guardBanner() : null,
      h(
        "p",
        { id: "generation-status", class: "status", role: "status", "aria-live": "polite" },
        busyWords ?? this.generationWords(status, modelHere),
      ),
      this.requestNotice === null
        ? null
        : h(
            "p",
            { class: "banner banner-warn", role: "alert" },
            this.requestNotice,
            this.offerReplace
              ? h(
                  "button",
                  { id: "replace-request", type: "button", onclick: () => void this.generate(true) },
                  this.t("generateReplace"),
                )
              : null,
          ),
      this.aiLine(status),
    );
  }

  /** The sentence for where the request stands. A request that finished says nothing a model panel does not. */
  private generationWords(status: Status, modelHere: boolean): string {
    switch (status.kind) {
      case "idle":
        return modelHere ? "" : this.t("generationIdle");
      case "queued":
        return this.t(status.connected ? "generationQueued" : "generationQueuedNoAi");
      case "running":
        return status.holder === null
          ? this.t("generationRunningUnnamed", { attempt: String(status.attempt) })
          : this.t("generationRunning", { attempt: String(status.attempt), holder: status.holder });
      case "interrupted":
        return this.t("generationInterrupted");
      case "failed":
        return status.reason === null
          ? this.t("generationFailedUnsaid")
          : this.t("generationFailed", { reason: status.reason });
      case "cancelled":
        return this.t("generationCancelled");
      case "superseded":
        return this.t("generationSuperseded");
      case "completed":
        return this.t("generationCompleted");
    }
  }

  /** Whether an AI is there, said when it is not already said by the state of the request. */
  private aiLine(status: Status): HTMLElement | null {
    if (status.kind === "queued" || status.kind === "running") return null;
    if (this.adapters === null) return null;
    return h(
      "p",
      { id: "generation-ai", class: "muted" },
      isConnected(this.adapters)
        ? this.t("generationAiHere", { names: connectedNames(this.adapters).join(", ") })
        : this.t("generationNoAi"),
    );
  }

  /** Asked when a save would end the request that is open: the person says whether it goes ahead. */
  private guardBanner(): HTMLElement {
    return h(
      "section",
      { class: "banner banner-warn", role: "alert" },
      h("strong", {}, this.t("guardTitle")),
      h("p", {}, this.t("guardBody")),
      h(
        "div",
        { class: "choices" },
        h(
          "button",
          { id: "guard-save", type: "button", onclick: () => void this.saveAndEndTheRequest() },
          this.t("guardSave"),
        ),
        h(
          "button",
          {
            id: "guard-leave",
            type: "button",
            onclick: () => {
              this.guarding = false;
              this.pendingSwitch = null;
              this.closing = false;
              this.render();
            },
          },
          this.t("guardLeave"),
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
      read === null ? null : this.acceptanceSection(),
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
            this.pseudoPage(page),
          ),
      this.changeSection(),
      pageRefusal === null
        ? null
        : h(
            "p",
            { class: "banner banner-warn", role: "alert" },
            this.t("reviewPageRefused", { detail: `${pageRefusal.message} (${pageRefusal.code})` }),
          ),
    );
  }

  /**
   * Accepting the design: what SCE finds of each requirement, the page the owner reads
   * before deciding, and whether what they accepted before still holds. Every
   * classification is SCE's; the screen arranges it, tells the owner what the design
   * leaves open, and offers the button. A design with a gap is the owner's to accept.
   */
  private acceptanceSection(): HTMLElement | null {
    const panel = this.acceptance;
    if (panel === null) return null;
    const heading = h("h4", {}, this.t("acceptTitle"));
    switch (panel.phase) {
      case "reading":
        return h("section", { class: "acceptance" }, heading, h("p", { class: "muted" }, this.t("acceptReading")));
      case "no-list":
        return h("section", { class: "acceptance" }, heading, h("p", { class: "muted" }, this.t("acceptNoList")));
      case "failed":
        return h(
          "section",
          { class: "acceptance" },
          heading,
          h("p", { class: "banner banner-error", role: "alert" }, this.t("acceptFailed", { detail: panel.message })),
        );
      case "read":
        return this.acceptanceBody(heading, panel.state);
    }
  }

  private acceptanceBody(heading: HTMLElement, state: AcceptanceState): HTMLElement {
    const report = state.report;
    const standing = state.list.standing;
    return h(
      "section",
      { class: "acceptance" },
      heading,
      standing === "behind"
        ? h("p", { class: "banner banner-warn", role: "status" }, this.t("requirementsBehind"))
        : standing === "unstated"
          ? h("p", { class: "banner", role: "status" }, this.t("requirementsUnstated"))
          : null,
      this.acceptedBanner(state.acceptance),
      report === null
        ? h(
            "p",
            { class: "banner banner-warn", role: "alert" },
            this.t("measureFailed", { detail: state.measureFailure ?? "" }),
          )
        : this.measureBlock(report),
      state.refusal === null
        ? null
        : h("p", { class: "banner banner-error", role: "alert" }, this.t("acceptRefused", { detail: state.refusal })),
      this.acceptBar(state),
    );
  }

  /** Whether the owner has accepted, whether it still holds, and what SCE listed as open when they did. */
  private acceptedBanner(read: ReadAcceptance): HTMLElement {
    const held = read.acceptance;
    if (held === null) return h("p", { class: "muted" }, this.t("acceptedNone"));
    const time = formatTime(held.accepted_at, this.locale);
    return h(
      "div",
      { class: "accepted" },
      h(
        "p",
        { class: read.standing === "holds" ? "banner banner-ok" : "banner banner-warn", role: "status" },
        read.standing === "holds"
          ? this.t("acceptedHolds", { time })
          : this.t("acceptedLapsed", { time, lapse: read.lapse ?? "" }),
      ),
      h("p", { class: "muted" }, this.channelSentence(held.channel)),
      held.open.length === 0
        ? null
        : h(
            "details",
            { class: "accepted-open" },
            h("summary", {}, this.t("acceptedOpenTitle")),
            h("ul", {}, ...held.open.map((sentence) => h("li", {}, sentence))),
          ),
    );
  }

  /** Which surface the acceptance was stated on, as the record says it: not every acceptance is the owner's own press. */
  private channelSentence(channel: string): string {
    if (channel === "direct") return this.t("channelDirect");
    if (channel === "relayed") return this.t("channelRelayed");
    return this.t("channelUnknown", { channel });
  }

  /** What SCE finds of the requirements: counts, the table, and the page it writes for the owner. */
  private measureBlock(report: RequirementsReport): HTMLElement {
    const gloss = (word: string): string => {
      const sentence = OUTCOME_SENTENCES[word];
      return sentence === undefined ? word : this.t(sentence);
    };
    const counted = tally(report.outcomes);
    const quotes = this.grounding()?.quotes ?? {};
    return h(
      "div",
      { class: "measure" },
      h(
        "p",
        {},
        report.denominator === null
          ? this.t("requirementsCountUnstated", { count: String(report.outcomes.length) })
          : this.t("requirementsCount", {
              count: String(report.outcomes.length),
              denominator: report.denominator,
            }),
      ),
      h(
        "ul",
        { class: "tally" },
        ...counted.map(([word, count]) =>
          h(
            "li",
            { class: isUnsettled(word) ? "unsettled" : undefined },
            h("span", { class: "count" }, String(count)),
            " ",
            gloss(word),
          ),
        ),
      ),
      counted.some(([word]) => isUnsettled(word)) ? h("p", { class: "muted" }, this.t("acceptGapsHint")) : null,
      h(
        "details",
        { class: "requirements" },
        h("summary", {}, this.t("requirementsTable")),
        h(
          "table",
          {},
          h(
            "thead",
            {},
            h(
              "tr",
              {},
              h("th", {}, this.t("requirementId")),
              h("th", {}, this.t("requirementOutcome")),
              h("th", {}, this.t("requirementSection")),
              h("th", {}, this.t("requirementCarried")),
              h("th", {}, this.t("requirementGo")),
            ),
          ),
          h(
            "tbody",
            {},
            ...report.outcomes.map((o) => {
              const quote = quotes[o.id];
              return h(
                "tr",
                { "data-requirement": o.id },
                h("td", {}, h("code", {}, o.id)),
                h("td", {}, o.outcome),
                h("td", {}, o.section ?? ""),
                h("td", {}, o.node_paths.length === 0 ? this.t("requirementNowhere") : o.node_paths.join(", ")),
                h(
                  "td",
                  { class: "goto" },
                  quote === undefined
                    ? null
                    : h(
                        "button",
                        { type: "button", class: "quiet show-text", onclick: () => this.showInText(quote) },
                        this.t("groundShow"),
                      ),
                  o.node_paths.length === 0
                    ? null
                    : h(
                        "button",
                        { type: "button", class: "quiet mark-lines", onclick: () => this.markRequirement(o) },
                        this.marked?.requirement === o.id ? this.t("markClear") : this.t("markInPage"),
                      ),
                ),
              );
            }),
          ),
        ),
      ),
      report.page === null
        ? null
        : h(
            "details",
            { class: "page" },
            h("summary", {}, this.t("acceptPageTitle")),
            h("pre", { class: "pseudo" }, report.page),
          ),
      report.page_refusal === null
        ? null
        : h(
            "p",
            { class: "banner banner-warn", role: "alert" },
            this.t("acceptPageRefused", {
              detail: `${report.page_refusal.message} (${report.page_refusal.code})`,
            }),
          ),
    );
  }

  /**
   * What the owner is told before they press: the matters SCE lists as left open
   * (accepting closes none of them; the requirements it leaves unsettled are marked in
   * its count above), then the button. Whether the button is offered is `gate`'s, and
   * follows what is typed (`refreshAcceptChrome`).
   */
  private acceptBar(state: AcceptanceState): HTMLElement {
    const review = this.review;
    const open = review !== null && review.phase === "read" ? review.review.check.open.length : null;
    // "Again" is said only where pressing is possible and means something: after a lapse.
    // An acceptance that holds is withheld, and says so, under the same words as a first one.
    const lapsed = state.acceptance.standing === "lapsed";
    return h(
      "div",
      { class: "accept-bar" },
      open === null || open === 0
        ? null
        : h("ul", { class: "gaps" }, h("li", {}, this.t("acceptGapOpen", { count: String(open) }))),
      h("p", { class: "muted" }, this.t("acceptNote")),
      h(
        "div",
        { class: "bar" },
        h(
          "button",
          { id: "accept", type: "button", onclick: () => void this.accept() },
          state.accepting ? this.t("acceptBusy") : this.t(lapsed ? "acceptAgainButton" : "acceptButton"),
        ),
        h("span", { id: "accept-note", class: "status", role: "status", "aria-live": "polite" }),
      ),
    );
  }

  /** Why the accept button is not offered now, in words; follows every keystroke without a redraw. */
  private refreshAcceptChrome(): void {
    const panel = this.acceptance;
    const button = this.root.querySelector<HTMLButtonElement>("#accept");
    const note = this.root.querySelector<HTMLElement>("#accept-note");
    if (panel === null || panel.phase !== "read" || button === null || note === null) return;
    const withheld = gate(panel.state, this.hasUnsavedChanges(), this.shown());
    button.disabled = withheld !== null;
    note.textContent = withheld === null || withheld === "accepting" ? "" : this.t(WITHHELD_WORDS[withheld]);
  }

  /**
   * The revisions the screen is showing, for `gate` to hold against the report. The text
   * is the editor's base (what it was read or last saved as), the design is the model
   * panel's, and the answers are the answers editor's. A part that is not on the screen
   * (still being read, or it could not be read) is `undefined`, which `gate` withholds the
   * button for: it is not the same as a part that matches, and not the same as `null`,
   * which is a part that is on the screen and has nothing saved.
   */
  private shown(): Shown {
    const model = this.model;
    const shownModel =
      model !== null && (model.phase === "drawing" || model.phase === "drawn" || model.phase === "not-drawn")
        ? model.read.model.revision
        : undefined;
    return {
      source: this.editor === null ? undefined : this.editor.base,
      model: shownModel,
      answers: this.answers === null ? undefined : this.answers.base,
    };
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
    this.askedNow = askedIds;
    const orphans = Object.keys(model.saved)
      .filter((id) => !askedIds.has(id))
      .sort();
    if (asked.length === 0 && orphans.length === 0) return null;

    const field = (id: string, label: Child[], nodePath?: string): HTMLElement => {
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
        nodePath === undefined ? null : this.groundBlock(nodePath),
        input,
        entry === undefined
          ? null
          : h(
              "span",
              { class: "muted answered-at" },
              this.t("answerAt", { time: formatTime(entry.answered_at, this.locale) }),
            ),
        h("p", { class: "muted answer-state", "data-answer-state": id }, this.answerStateWords(id)),
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
        ], u.node_path),
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
        this.regenerateButton(),
        h("span", { id: "answers-status", class: "status", role: "status", "aria-live": "polite" }),
      ),
    );
  }

  /**
   * What ties a question or a place of the design to a sentence of the text: the product's own
   * statement of where each requirement is carried, and the sentence the list quotes for it. Only
   * when the design was measured against a list; otherwise there is nothing to tie it to.
   */
  private grounding(): { outcomes: readonly RequirementOutcome[]; quotes: Readonly<Record<string, string>> } | null {
    const panel = this.acceptance;
    if (panel === null || panel.phase !== "read") return null;
    const report = panel.state.report;
    if (report === null) return null;
    return { outcomes: report.outcomes, quotes: quotesOf(panel.state.list.requirements?.sidecar ?? null) };
  }

  /** The sentence of the text a question is about, with the way to it. Nothing when none can be told. */
  private groundBlock(nodePath: string): HTMLElement | null {
    const data = this.grounding();
    if (data === null) return null;
    const ground = groundOf(nodePath, data.outcomes, data.quotes);
    if (ground === null) return null;
    return h(
      "div",
      { class: "ground", "data-ground": ground.requirement },
      h("span", { class: "muted" }, this.t("groundRelated", { id: ground.requirement })),
      ground.quote === null ? null : h("blockquote", {}, ground.quote),
      ground.quote === null
        ? null
        : h(
            "button",
            { type: "button", class: "quiet", onclick: () => this.showInText(ground.quote as string) },
            this.t("groundShow"),
          ),
    );
  }

  /** Select the sentence in the editor and bring it into view; say so when the text no longer holds it. */
  private showInText(quote: string): void {
    const editor = this.editor;
    const textarea = this.root.querySelector<HTMLTextAreaElement>("#source");
    if (editor === null || textarea === null || this.viewing !== null) return;
    const place = findQuote(quote, editor.text);
    if (place === null) {
      this.notice = this.t("groundNotInText");
      this.render();
      return;
    }
    this.notice = null;
    textarea.focus();
    textarea.setSelectionRange(place.start, place.end);
    // The sentence is brought into view: a few lines above it, so that it has its own context.
    const line = editor.text.slice(0, place.start).split("\n").length - 1;
    const lineHeight = Number.parseFloat(this.root.ownerDocument.defaultView?.getComputedStyle(textarea).lineHeight ?? "") || 18;
    textarea.scrollTop = Math.max(0, (line - 2) * lineHeight);
  }

  /** Light the lines of the pseudocode that name the states a requirement is carried by; the same press clears it. */
  private markRequirement(outcome: RequirementOutcome): void {
    this.marked =
      this.marked?.requirement === outcome.id
        ? null
        : { requirement: outcome.id, states: statesOf(outcome.node_paths) };
    this.render();
  }

  /**
   * The pseudocode page, with the lines that name the marked requirement's states lit. The page
   * is the product's rendering and carries no address of what a line is for, so a state's name is
   * what ties a line to a place; the screen says that is what it did.
   */
  private pseudoPage(page: string): HTMLElement {
    const marked = this.marked;
    if (marked === null) return h("pre", { class: "pseudo" }, page);
    const lit = new Set(linesNaming(page, marked.states));
    const lines = page.split("\n");
    return h(
      "div",
      {},
      h(
        "p",
        { class: "muted marked", "data-marked": marked.requirement },
        marked.states.length === 0
          ? this.t("markNoStates", { id: marked.requirement })
          : lit.size === 0
            ? this.t("markNoLines", { states: marked.states.join(", ") })
            : this.t("markedLines", { id: marked.requirement, states: marked.states.join(", ") }),
        " ",
        h(
          "button",
          {
            type: "button",
            class: "quiet",
            onclick: () => {
              this.marked = null;
              this.render();
            },
          },
          this.t("markClear"),
        ),
      ),
      h(
        "pre",
        { class: "pseudo" },
        ...lines.map((line, index) =>
          h("span", { class: lit.has(index) ? "lit" : undefined }, index === lines.length - 1 ? line : `${line}\n`),
        ),
      ),
    );
  }

  /**
   * The answer's state in words, or nothing for a question nobody answered. Where it stands is
   * what the core records: typed, saved, being written into a model, or in the model shown.
   */
  private answerStateWords(id: string): string {
    const model = this.answers;
    if (model === null) return "";
    const state = answerState(id, {
      typed: wordsOf(model, id),
      saved: model.saved[id]?.answer ?? null,
      asked: this.askedNow.has(id),
      madeFrom: this.bundleAnswers === null ? null : { answers: this.bundleAnswers },
      head: this.requestHead,
      detail: this.requestDetail,
      answersNow: model.base,
    });
    return state === null ? "" : this.t(ANSWER_STATE_WORDS[state]);
  }

  /** Each answer's state follows every keystroke, like the save button, without redrawing the field. */
  private refreshAnswerStates(): void {
    for (const line of this.root.querySelectorAll<HTMLElement>("[data-answer-state]")) {
      line.textContent = this.answerStateWords(line.dataset["answerState"] ?? "");
    }
  }

  /**
   * "Generate again from these answers": the answers typed are saved first and a request is made
   * about what is then saved, which is what generating does. Offered when a request can be made.
   */
  private regenerateButton(): HTMLElement | null {
    const controls = controlsOf(
      statusOf(this.requestHead, this.requestDetail, this.adapters),
      this.requestBusy !== null,
    );
    if (!controls.canGenerate) return null;
    return h(
      "button",
      { id: "regenerate", type: "button", onclick: () => void this.generate(controls.replaces) },
      this.t("answersRegenerate"),
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
    this.refreshAcceptChrome();
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

/** What is said of the adapters, as one comparable thing: who is there and what each can do. */
function adaptersKey(listing: AdapterListing | null): string {
  return JSON.stringify(
    listing === null ? null : listing.adapters.map((a) => [a.name, a.live, a.capabilities.join(",")]),
  );
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
