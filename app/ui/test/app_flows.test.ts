// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// @vitest-environment jsdom

// The screen's own order of events: what happens when an answer arrives after the
// person has moved on, when text is not saved, and when a save succeeds and the
// read after it does not.
//
// The pure editor model is tested elsewhere and cannot see any of this: these
// failures live in the gaps between a request and its answer, which only the real
// `App`, driven through its buttons over a core whose answers can be held back,
// has. They were found by an external review of the first version, each by a
// sequence a person can perform, and each is a rule over the editor and not over a
// pair of works, so the cases below vary the sequence and not only the names.

import { beforeEach, describe, expect, it } from "vitest";

import { App } from "../src/app";
import { CommandFailure, UNAUTHORIZED, type Args, type Transport } from "../src/ipc";
import type { Ticker } from "../src/watch";

const hex = (n: number): string => n.toString(16).padStart(64, "0");

interface Revision {
  revision: string;
  text: string;
}

interface Gate {
  name: string;
  match: (args: Args) => boolean;
  opened: Promise<void>;
  used: boolean;
}

/** A core in memory, whose answers the test can hold back and let go of. */
class FakeCore implements Transport {
  readonly calls: Array<{ name: string; args: Args }> = [];
  private readonly works = new Map<string, { title: string; revisions: Revision[] }>();
  private readonly revisionOf = new Map<string, string>();
  private readonly gates: Gate[] = [];
  private readonly failures: Array<{ name: string; error: CommandFailure }> = [];
  private readonly models = new Map<
    string,
    { text: string; writtenFor: string | null; others: ReadonlyArray<{ name: string; text: string }> }
  >();
  private readonly answersOf = new Map<
    string,
    { revision: string; entries: Record<string, { answer: string; answered_at: string }> }
  >();
  private answerSaves = 0;
  private readonly lists = new Map<string, { revision: string; writtenFor: string | null; ids: string[] }>();
  private readonly acceptances = new Map<
    string,
    { revision: string; basis: Record<string, string>; channel: string; open: string[] }
  >();
  private listSaves = 0;
  private acceptSaves = 0;

  /**
   * The requirement list an authoring client saved for a work, and the text revision it
   * says it read. A later call is a new revision of the list.
   */
  setRequirements(id: string, writtenFor: string | null, ids: string[] = ["R1", "R2"]): void {
    this.listSaves += 1;
    this.lists.set(id, { revision: this.revision(`list:${this.listSaves}`), writtenFor, ids });
  }

  /** What the owner accepted, as the core keeps it: the work as it stands now, stated on `channel`. */
  setAcceptance(id: string, channel = "direct"): void {
    const basis = this.basisOf(id);
    if (basis === null) throw new Error(`${id} has no text, model and list to accept`);
    this.acceptSaves += 1;
    this.acceptances.set(id, {
      revision: this.revision(`acceptance:${this.acceptSaves}`),
      basis,
      channel,
      open: ["1 question(s) the specification leaves open (open-guard)"],
    });
  }

  /** The revisions of everything an acceptance is about, or `null` while one of them is missing. */
  private basisOf(id: string): Record<string, string> | null {
    const source = this.works.get(id)?.revisions.at(-1)?.revision;
    const model = this.models.get(id);
    const list = this.lists.get(id);
    if (source === undefined || model === undefined || list === undefined) return null;
    const answers = this.answersOf.get(id)?.revision;
    return {
      source,
      model: this.revision(`model:${model.text}`),
      requirements: list.revision,
      ...(answers === undefined ? {} : { answers }),
    };
  }

  /** What an acceptance lapsed over, in the product's one sentence. */
  private lapseOf(then: Record<string, string>, now: Record<string, string>): string | null {
    const files: Array<[string, string]> = [
      ["design/model.scxml", "model"],
      ["spec/requirements.manifest.json", "requirements"],
      ["spec/source.txt", "source"],
      ["spec/answers.json", "answers"],
    ];
    const moved = files.filter(([, key]) => then[key] !== now[key]).map(([path]) => `${path} moved`);
    return moved.length === 0 ? null : moved.join("; ");
  }

  /** The owner's answers as saved, from another entrance or an earlier session. */
  setAnswers(id: string, entries: Record<string, string>): void {
    this.answerSaves += 1;
    this.answersOf.set(id, {
      revision: this.revision(`answers:${this.answerSaves}`),
      entries: Object.fromEntries(
        Object.entries(entries).map(([question, answer]) => [question, { answer, answered_at: "2026-10-03T09:00:00Z" }]),
      ),
    });
  }

  /** What the core holds of the owner's answers, as words by question. */
  answersHeld(id: string): Record<string, string> {
    return Object.fromEntries(Object.entries(this.answersOf.get(id)?.entries ?? {}).map(([q, e]) => [q, e.answer]));
  }

  addWork(id: string, title: string, texts: string[]): void {
    this.works.set(id, { title, revisions: texts.map((text) => ({ revision: this.revision(text), text })) });
  }

  /** A text saved from another entrance: a new head, as `save_source` makes one. */
  saveElsewhere(id: string, text: string): void {
    this.works.get(id)?.revisions.push({ revision: this.revision(text), text });
  }

  /**
   * The work's model, and the text revision its writer says it read. `others` makes it
   * a model of several documents: `text` is then the entry `door.scxml`, and each of
   * `others` is a document it imports.
   */
  setModel(
    id: string,
    text: string,
    writtenFor: string | null,
    others: ReadonlyArray<{ name: string; text: string }> = [],
  ): void {
    this.models.set(id, { text, writtenFor, others });
  }

  /** The SVG this core draws for a model, so a test can look for it on the screen. */
  figureSvg(text: string): string {
    return `<svg xmlns="http://www.w3.org/2000/svg" width="40pt" height="20pt"><text x="0" y="10">picture of ${text}</text></svg>`;
  }

  revision(text: string): string {
    if (!this.revisionOf.has(text)) this.revisionOf.set(text, hex(this.revisionOf.size + 1));
    return this.revisionOf.get(text) as string;
  }

  headText(id: string): string | undefined {
    return this.works.get(id)?.revisions.at(-1)?.text;
  }

  /** Hold the next call of `name` that `match`es until `release` is called. */
  hold(name: string, match: (args: Args) => boolean = () => true): { release: () => void } {
    let release = (): void => undefined;
    const opened = new Promise<void>((resolve) => {
      release = resolve;
    });
    this.gates.push({ name, match, opened, used: false });
    return { release: () => release() };
  }

  /** Make the next call of `name` fail. */
  failNext(name: string, error: CommandFailure): void {
    this.failures.push({ name, error });
  }

  callsOf(name: string): Array<Args> {
    return this.calls.filter((c) => c.name === name).map((c) => c.args);
  }

  async call(name: string, args: Args = {}): Promise<unknown> {
    this.calls.push({ name, args });
    const failure = this.failures.findIndex((f) => f.name === name);
    if (failure >= 0) throw this.failures.splice(failure, 1)[0]?.error;
    const gate = this.gates.find((g) => !g.used && g.name === name && g.match(args));
    if (gate !== undefined) {
      gate.used = true;
      await gate.opened;
    }
    return this.answer(name, args);
  }

  private answer(name: string, args: Args): unknown {
    const work = typeof args["id"] === "string" ? this.works.get(args["id"]) : undefined;
    switch (name) {
      case "describe":
        return { command_set_version: 11, commands: [], root: "/fake/works" };
      case "read_work_heads": {
        if (work === undefined) throw new CommandFailure("not-found", "work `absent`");
        const id = String(args["id"]);
        const model = this.models.get(id);
        const list = this.lists.get(id);
        return {
          source: work?.revisions.at(-1)?.revision ?? null,
          model: model === undefined ? null : { revision: this.revision(`model:${model.text}`), written_for: model.writtenFor },
          answers: this.answersOf.get(id)?.revision ?? null,
          requirements: list === undefined ? null : { revision: list.revision, written_for: list.writtenFor },
          acceptance: this.acceptances.get(id)?.revision ?? null,
          bundle: null,
          request: null,
        };
      }
      case "read_requirements": {
        const list = typeof args["id"] === "string" ? this.lists.get(args["id"]) : undefined;
        const head = work?.revisions.at(-1)?.revision ?? null;
        return {
          requirements:
            list === undefined
              ? null
              : { revision: list.revision, written_for: list.writtenFor, manifest: "{}", sidecar: null },
          source_head: head,
          standing: list === undefined ? null : standingOf(list.writtenFor, head),
        };
      }
      case "requirements_report": {
        const id = String(args["id"]);
        const basis = this.basisOf(id);
        const model = this.models.get(id);
        const list = this.lists.get(id);
        if (basis === null || model === undefined || list === undefined) {
          throw new CommandFailure("not-found", "a model or a requirement list of this work (none was saved)");
        }
        const head = work?.revisions.at(-1)?.revision ?? null;
        return {
          basis,
          source_head: head,
          model_standing: standingOf(model.writtenFor, head),
          requirements_standing: standingOf(list.writtenFor, head),
          generator: "fake-sce 0",
          denominator: "synthesized",
          outcomes: list.ids.map((requirement, i) => ({
            id: requirement,
            outcome:
              model.text.includes("MISSING") && i === 0
                ? "missing"
                : model.text.includes("SCENARIO") && i === 1
                  ? "needs-scenario"
                  : model.text.includes("DANGLING") && i === 1
                    ? "dangling"
                    : model.text.includes("WAIVED") && i === 1
                      ? "waived"
                      : "implemented",
            section: `S${i + 1}`,
            node_paths: model.text.includes("MISSING") && i === 0 ? [] : [`states.s${i}`],
          })),
          page: `ACCEPTANCE REPORT\n  ${list.ids.length} requirements\n`,
          page_refusal: null,
        };
      }
      case "read_acceptance": {
        const id = String(args["id"]);
        const held = this.acceptances.get(id);
        const now = this.basisOf(id);
        if (held === undefined || now === null) return { acceptance: null, standing: "none", lapse: null };
        const lapse = this.lapseOf(held.basis, now);
        return {
          acceptance: {
            revision: held.revision,
            accepted_at: "2026-10-03T09:00:10Z",
            channel: held.channel,
            basis: held.basis,
            open: held.open,
          },
          standing: lapse === null ? "holds" : "lapsed",
          lapse,
          now,
        };
      }
      case "accept": {
        const id = String(args["id"]);
        const now = this.basisOf(id);
        const model = this.models.get(id);
        const list = this.lists.get(id);
        if (now === null || model === undefined || list === undefined) {
          throw new CommandFailure("not-found", "a model or a requirement list of this work (none was saved)");
        }
        const expected = args["expect"] as Record<string, string>;
        // The core compares each revision, and the owner's answers are none (`null`, or absent) until given.
        const moved = Object.keys({ ...now, ...expected }).filter((key) => (now[key] ?? null) !== (expected[key] ?? null));
        if (moved.length > 0) {
          throw new CommandFailure(
            "moved",
            `${moved.join(", ")} changed after you were shown it, so nothing was accepted; read it again`,
            { moved, current: now },
          );
        }
        const head = work?.revisions.at(-1)?.revision ?? null;
        if (model.writtenFor !== head || list.writtenFor !== head) {
          throw new CommandFailure("not-current", "the model was not written for the text as it is now");
        }
        this.acceptSaves += 1;
        const revision = this.revision(`acceptance:${this.acceptSaves}`);
        this.acceptances.set(id, { revision, basis: now, channel: "direct", open: [] });
        return { outcome: "saved", revision, parent: null };
      }
      case "read_answers": {
        const held = typeof args["id"] === "string" ? this.answersOf.get(args["id"]) : undefined;
        return { answers: held ?? null };
      }
      case "save_answers": {
        const id = String(args["id"]);
        const held = this.answersOf.get(id);
        if ((args["base"] ?? null) !== (held?.revision ?? null)) {
          throw new CommandFailure("conflict", "the answers moved", { base: args["base"] ?? null, current: held?.revision ?? null });
        }
        const wanted = args["answers"] as Record<string, string>;
        const entries: Record<string, { answer: string; answered_at: string }> = {};
        for (const [question, words] of Object.entries(wanted)) {
          const before = held?.entries[question];
          entries[question] =
            before !== undefined && before.answer === words
              ? before
              : { answer: words, answered_at: "2026-10-03T09:00:30Z" };
        }
        this.answerSaves += 1;
        const revision = this.revision(`answers:${this.answerSaves}`);
        this.answersOf.set(id, { revision, entries });
        return { outcome: "saved", revision, parent: held?.revision ?? null };
      }
      case "review": {
        const model = typeof args["id"] === "string" ? this.models.get(args["id"]) : undefined;
        if (model === undefined) throw new CommandFailure("not-found", "no model");
        const head = work?.revisions.at(-1)?.revision ?? null;
        const revision = this.revision(`model:${model.text}`);
        const standing =
          model.writtenFor === null ? "unstated" : model.writtenFor === head ? "current" : "behind";
        const base = { model: { revision, written_for: model.writtenFor }, source_head: head, standing, generator: "fake-sce 0" };
        if (model.text.includes("REFUSE")) {
          return {
            ...base,
            check: {
              verdict: "refused",
              kind: null,
              open: [],
              unresolved: [],
              records: [
                { code: "validation/invalid-reference", message: "no such state 'nowhere'", stage: "validation", line: 3 },
              ],
            },
            page: null,
            page_refusal: null,
          };
        }
        const refused = model.text.includes("NOPAGE");
        return {
          ...base,
          check: {
            verdict: "accepted",
            kind: "statechart",
            open: ["1 question(s) the specification leaves open (open-guard)"],
            unresolved: [
              { id: "open-guard", node_path: "states.closed.transitions[0]", line: 3, reason: "Which cards open the door?" },
              { id: "close-delay", node_path: "states.opened.transitions[0]", line: 4, reason: null },
            ],
            records: [],
          },
          // Indented, with a run of spaces and a final newline: the screen keeps them.
          page: refused ? null : `machine door (lexicon: ${String(args["lexicon"] ?? "-")})\n  state closed:\n    on open   -> opened\n`,
          page_refusal: refused ? { code: "cli/pseudo-unsupported", message: "the page does not abbreviate this" } : null,
        };
      }
      case "remove_work": {
        if (work === undefined) throw new CommandFailure("not-found", "no such work");
        this.works.delete(String(args["id"]));
        return { removed: { id: String(args["id"]), title: work.title, created_at: "2026-10-03T09:00:00Z" } };
      }
      case "read_model":
      case "figures": {
        const model = typeof args["id"] === "string" ? this.models.get(args["id"]) : undefined;
        const head = work?.revisions.at(-1)?.revision ?? null;
        if (model === undefined) {
          if (name === "figures") throw new CommandFailure("not-found", "no model");
          return { model: null, source_head: head, standing: null };
        }
        const revision = this.revision(`model:${model.text}`);
        // The core's rule, in one place: for the text as it is, for an earlier one, or unsaid.
        const standing =
          model.writtenFor === null ? "unstated" : model.writtenFor === head ? "current" : "behind";
        if (name === "read_model") {
          const entry = model.others.length > 0 ? "door.scxml" : "model.scxml";
          return {
            model: {
              revision,
              written_for: model.writtenFor,
              text: model.text,
              entry,
              documents: [{ name: entry, text: model.text }, ...model.others],
            },
            source_head: head,
            standing,
          };
        }
        return {
          model: { revision, written_for: model.writtenFor },
          source_head: head,
          standing,
          generator: "fake-sce 0",
          sheets: [
            { name: "picture.svg", svg: this.figureSvg(model.text) },
            { name: "fields-1.svg", svg: this.figureSvg("the fields") },
          ],
        };
      }
      case "list_works":
        return {
          works: [...this.works].map(([id, w]) => ({ id, title: w.title, created_at: "2026-10-03T09:00:00Z" })),
          unreadable: [],
        };
      case "create_work": {
        const id = String(args["title"]).toLowerCase().replace(/[^a-z0-9]+/g, "-");
        this.works.set(id, { title: String(args["title"]), revisions: [] });
        return { id, title: String(args["title"]), created_at: "2026-10-03T09:00:00Z" };
      }
      case "read_source": {
        const revisions = work?.revisions ?? [];
        const wanted = args["revision"];
        const found = wanted === undefined ? revisions.at(-1) : revisions.find((r) => r.revision === wanted);
        return { source: found === undefined ? null : { revision: found.revision, text: found.text } };
      }
      case "history":
        return {
          entries: (work?.revisions ?? []).map((r, i, all) => ({
            revision: r.revision,
            parent: i === 0 ? null : (all[i - 1]?.revision ?? null),
            saved_at: "2026-10-03T09:00:00Z",
          })),
        };
      case "save_source": {
        if (work === undefined) throw new CommandFailure("not-found", "no such work");
        const head = work.revisions.at(-1)?.revision ?? null;
        if ((args["base"] ?? null) !== head) {
          throw new CommandFailure("conflict", "the base is not current", { base: args["base"] ?? null, current: head });
        }
        const text = String(args["text"]);
        const revision = this.revision(text);
        if (revision === head) return { outcome: "unchanged", revision };
        work.revisions.push({ revision, text });
        return { outcome: "saved", revision, parent: head };
      }
      default:
        throw new CommandFailure("unknown-command", name);
    }
  }
}

/** The core's rule, in one place: written for the text as it is, for an earlier one, or unsaid. */
function standingOf(writtenFor: string | null, head: string | null): "current" | "behind" | "unstated" {
  return writtenFor === null ? "unstated" : writtenFor === head ? "current" : "behind";
}

const settle = async (): Promise<void> => {
  for (let i = 0; i < 4; i += 1) await new Promise((resolve) => setTimeout(resolve, 0));
};

let root: HTMLElement;
let core: FakeCore;
let app: App;

async function start(): Promise<void> {
  document.body.innerHTML = '<div id="app"></div>';
  root = document.getElementById("app") as HTMLElement;
  core = new FakeCore();
  core.addWork("alpha", "Alpha", ["alpha one", "alpha two"]);
  core.addWork("beta", "Beta", ["beta one"]);
  app = new App(root, { transport: core, storage: null, browserLanguage: "en" });
  await app.start();
  await settle();
}

const buttons = (label: string): HTMLButtonElement[] =>
  [...root.querySelectorAll("button")].filter((b) => b.textContent === label);

async function click(label: string, index = 0): Promise<void> {
  const button = buttons(label)[index];
  if (button === undefined) throw new Error(`no button "${label}" (${index}) in: ${root.textContent}`);
  button.click();
  await settle();
}

const editor = (): HTMLTextAreaElement => root.querySelector("#source") as HTMLTextAreaElement;
const heading = (): string => root.querySelector("main h2")?.textContent ?? "";
const status = (): string => root.querySelector("#status")?.textContent ?? "";

async function type(text: string): Promise<void> {
  editor().value = text;
  editor().dispatchEvent(new Event("input", { bubbles: true }));
  await settle();
}

beforeEach(start);

describe("an answer that arrives after the person has moved on", () => {
  it("does not put one work's older text into another work's editor", async () => {
    await click("Alpha");
    expect(editor().value).toBe("alpha two");
    // History is newest first, so the older revision is the second "Load into editor".
    const older = core.revision("alpha one");
    const held = core.hold("read_source", (a) => a["revision"] === older);
    await click("Load into editor", 1);

    await click("Beta");
    expect(heading()).toBe("Beta");
    await type("beta edited");
    held.release();
    await settle();

    expect(heading()).toBe("Beta");
    expect(editor().value).toBe("beta edited");
    await click("Save");
    expect(core.headText("beta")).toBe("beta edited");
    expect(core.headText("alpha")).toBe("alpha two");
  });

  it("does not load an older text over what was typed while it was being read", async () => {
    await click("Alpha");
    const older = core.revision("alpha one");
    const held = core.hold("read_source", (a) => a["revision"] === older);
    await click("Load into editor", 1);
    await type("typed meanwhile");
    held.release();
    await settle();

    expect(editor().value).toBe("typed meanwhile");
    expect(root.textContent).toContain("was not loaded");
  });

  it("shows the work last asked for when the first one answers last", async () => {
    const slow = core.hold("read_source", (a) => a["id"] === "alpha");
    await click("Alpha");
    await click("Beta");
    expect(heading()).toBe("Beta");
    slow.release();
    await settle();

    expect(heading()).toBe("Beta");
    expect(editor().value).toBe("beta one");
  });

  it("does not show an older revision of a work the person has left", async () => {
    await click("Alpha");
    const older = core.revision("alpha one");
    const held = core.hold("read_source", (a) => a["revision"] === older);
    await click("View", 1);
    await click("Beta");
    held.release();
    await settle();

    expect(heading()).toBe("Beta");
    expect(root.textContent).not.toContain("Viewing revision");
  });
});

describe("text that is not saved", () => {
  it("is not replaced by opening another work, and the person is asked", async () => {
    await click("Alpha");
    await type("alpha changed");
    await click("Beta");

    expect(editor().value).toBe("alpha changed");
    expect(root.textContent).toContain("not saved");
    expect(core.callsOf("read_source").filter((a) => a["id"] === "beta")).toHaveLength(0);
    expect(app.hasUnsavedChanges()).toBe(true);
  });

  it("stays when the person chooses to stay", async () => {
    await click("Alpha");
    await type("alpha changed");
    await click("Beta");
    await click("Stay here");

    expect(root.textContent).not.toContain("not saved");
    expect(editor().value).toBe("alpha changed");
  });

  it("is discarded only when the person says so", async () => {
    await click("Alpha");
    await type("alpha changed");
    await click("Beta");
    await click("Discard my changes and open it");

    expect(heading()).toBe("Beta");
    expect(core.headText("alpha")).toBe("alpha two");
  });

  it("is saved first when the person asks, and the other work then opens", async () => {
    await click("Alpha");
    await type("alpha changed");
    await click("Beta");
    await click("Save, then open it");

    expect(core.headText("alpha")).toBe("alpha changed");
    expect(heading()).toBe("Beta");
    expect(editor().value).toBe("beta one");
  });

  it("keeps the person waiting on the choice when the save before opening does not take", async () => {
    await click("Alpha");
    await type("alpha changed");
    // Another writer got there first, so this save is refused as a conflict.
    core.addWork("alpha", "Alpha", ["alpha one", "alpha two", "alpha elsewhere"]);
    await click("Beta");
    await click("Save, then open it");

    expect(heading()).toBe("Alpha");
    expect(editor().value).toBe("alpha changed");
    expect(root.textContent).toContain("not saved");
  });

  it("is protected for the same work too: opening it again does not reload over it", async () => {
    await click("Alpha");
    await type("alpha changed");
    await click("Alpha");

    expect(editor().value).toBe("alpha changed");
    expect(root.textContent).toContain("not saved");
  });

  it("is protected from what is typed while the other work is still being read", async () => {
    await click("Alpha");
    const slow = core.hold("read_source", (a) => a["id"] === "beta");
    await click("Beta");
    // Nothing was unsaved when Beta was asked for, so the read is on its way.
    await type("typed while waiting");
    slow.release();
    await settle();

    expect(heading()).toBe("Alpha");
    expect(editor().value).toBe("typed while waiting");
    expect(root.textContent).toContain("not saved");
    expect(app.hasUnsavedChanges()).toBe(true);

    // The choice is the same one as before the read: the text is kept unless the person lets it go.
    await click("Save, then open it");
    expect(core.headText("alpha")).toBe("typed while waiting");
    expect(heading()).toBe("Beta");
  });

  it("is not lost to the other work arriving, when the person then lets it go", async () => {
    await click("Alpha");
    const slow = core.hold("read_source", (a) => a["id"] === "beta");
    await click("Beta");
    await type("typed while waiting");
    slow.release();
    await settle();
    await click("Discard my changes and open it");

    expect(heading()).toBe("Beta");
    expect(editor().value).toBe("beta one");
    expect(core.headText("alpha")).toBe("alpha two");
  });

  it("does not hold the other work back when nothing was typed while it was read", async () => {
    await click("Alpha");
    const slow = core.hold("read_source", (a) => a["id"] === "beta");
    await click("Beta");
    slow.release();
    await settle();

    expect(heading()).toBe("Beta");
    expect(root.textContent).not.toContain("not saved");
  });

  it("is not unsaved once it is saved", async () => {
    await click("Alpha");
    expect(app.hasUnsavedChanges()).toBe(false);
    await type("alpha changed");
    expect(app.hasUnsavedChanges()).toBe(true);
    await click("Save");
    expect(app.hasUnsavedChanges()).toBe(false);
  });
});

describe("a save that took, and a read after it that did not", () => {
  it("is still a saved text, and the next save does not conflict with it", async () => {
    await click("Alpha");
    await type("alpha three");
    core.failNext("history", new CommandFailure("io", "the history could not be read"));
    await click("Save");

    expect(core.headText("alpha")).toBe("alpha three");
    expect(status()).toBe("Saved");
    expect(root.textContent).not.toContain("The save failed");
    expect(root.textContent).toContain("the history could not be read");

    await type("alpha four");
    await click("Save");
    expect(core.headText("alpha")).toBe("alpha four");
    expect(root.textContent).not.toContain("changed while you were editing");
  });

  it("is reported as failed when the save itself fails", async () => {
    await click("Alpha");
    await type("alpha three");
    core.failNext("save_source", new CommandFailure("io", "the disk is full"));
    await click("Save");

    expect(root.textContent).toContain("The save failed");
    expect(core.headText("alpha")).toBe("alpha two");
    expect(editor().value).toBe("alpha three");
  });
});

describe("the new-work field", () => {
  it("keeps what is typed in it across a redraw", async () => {
    const field = root.querySelector('input[name="title"]') as HTMLInputElement;
    field.value = "Gamma";
    field.dispatchEvent(new Event("input", { bubbles: true }));
    await click("Alpha");

    expect((root.querySelector('input[name="title"]') as HTMLInputElement).value).toBe("Gamma");
  });
});

// ---- the model, as SCE draws it -------------------------------------------

const images = (): HTMLImageElement[] => [...root.querySelectorAll<HTMLImageElement>(".sheet img")];
const modelText = (): string => root.querySelector(".model")?.textContent ?? "";
const decoded = (image: HTMLImageElement): string =>
  decodeURIComponent(image.src.replace("data:image/svg+xml;charset=utf-8,", ""));

describe("the model panel", () => {
  it("says there is no model when the work has none, and draws nothing", async () => {
    await click("Alpha");
    expect(modelText()).toContain("No model yet");
    expect(images()).toHaveLength(0);
    expect(core.callsOf("figures")).toHaveLength(0);
  });

  it("shows the sheets SCE drew, in its order, as images the model's text cannot script", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
    await click("Alpha");

    expect(images()).toHaveLength(2);
    expect([...root.querySelectorAll(".sheet figcaption")].map((c) => c.textContent)).toEqual([
      "picture.svg",
      "fields-1.svg",
    ]);
    expect(decoded(images()[0] as HTMLImageElement)).toBe(core.figureSvg("<scxml/>"));
    expect(images()[0]?.getAttribute("alt")).toBe("picture");
    expect(modelText()).toContain("written for the text as it is now");
    expect(modelText()).toContain("Drawn by fake-sce 0");
    // The model's own text is there to read, folded.
    expect(root.querySelector(".scxml")?.textContent).toBe("<scxml/>");
  });

  it("shows a model of one document as one text, as before", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
    await click("Alpha");

    const folded = root.querySelector(".model-scxml");
    expect(folded?.querySelector("summary")?.textContent).toBe("The model's SCXML");
    expect(folded?.querySelectorAll("pre.scxml")).toHaveLength(1);
    expect(folded?.querySelector("h5")).toBeNull();
  });

  it("shows a model of several documents under the file name each is imported by, the entry named", async () => {
    core.setModel("alpha", "<scxml>entry</scxml>", core.revision("alpha two"), [
      { name: "close.scxml", text: "<event-schema>close</event-schema>" },
      { name: "open.scxml", text: "<event-schema>open</event-schema>" },
    ]);
    await click("Alpha");

    const folded = root.querySelector(".model-scxml");
    expect(folded?.querySelector("summary")?.textContent).toBe("The model's SCXML: 3 documents");
    expect([...(folded?.querySelectorAll("h5.document-name code") ?? [])].map((c) => c.textContent)).toEqual([
      "door.scxml",
      "close.scxml",
      "open.scxml",
    ]);
    expect([...(folded?.querySelectorAll("pre.scxml") ?? [])].map((p) => p.textContent)).toEqual([
      "<scxml>entry</scxml>",
      "<event-schema>close</event-schema>",
      "<event-schema>open</event-schema>",
    ]);
    // Only the entry says so.
    expect(folded?.querySelectorAll("h5.document-name")[0]?.textContent).toContain("the document SCE is asked about");
    expect(folded?.querySelectorAll("h5.document-name")[1]?.textContent).not.toContain("asked about");
    // The review and the figures are of the same model.
    expect(core.callsOf("review")).toHaveLength(1);
    expect(images()).toHaveLength(2);
  });

  it("never turns the drawing into elements of the page", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
    // Whatever the product wrote, even markup that would run in a page, is only an image here.
    const hostile = '<svg xmlns="http://www.w3.org/2000/svg"><script>window.__ran = true</script></svg>';
    core.figureSvg = () => hostile;
    await click("Alpha");

    expect(root.querySelector("script")).toBeNull();
    expect(root.querySelector(".sheet svg")).toBeNull();
    expect((window as unknown as Record<string, unknown>)["__ran"]).toBeUndefined();
    expect(images()).toHaveLength(2);
  });

  it("says a model written for an earlier text is behind, with both revisions", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha one"));
    await click("Alpha");

    const banner = root.querySelector(".model .banner-warn")?.textContent ?? "";
    expect(banner).toContain("earlier text");
    expect(banner).toContain(core.revision("alpha one").slice(0, 12));
    expect(banner).toContain(core.revision("alpha two").slice(0, 12));
    expect(images()).toHaveLength(2);
  });

  it("says when nothing records which text the model was written for", async () => {
    core.setModel("alpha", "<scxml/>", null);
    await click("Alpha");
    expect(modelText()).toContain("Nothing records which text");
  });

  it("shows SCE's refusal in the product's words, with the model still readable", async () => {
    core.setModel("alpha", "<scxml>big</scxml>", core.revision("alpha two"));
    core.failNext(
      "figures",
      new CommandFailure(
        "sce-refused",
        "SCE refused the model (cli/diagram-does-not-fit): the figure needs 925 x 125 pt",
        { code: "cli/diagram-does-not-fit" },
      ),
    );
    await click("Alpha");

    expect(images()).toHaveLength(0);
    const refusal = root.querySelector(".model .banner-error")?.textContent ?? "";
    expect(refusal).toContain("SCE did not draw this model");
    expect(refusal).toContain("the figure needs 925 x 125 pt");
    expect(refusal).toContain("cli/diagram-does-not-fit");
    expect(root.querySelector(".scxml")?.textContent).toBe("<scxml>big</scxml>");
    // The refusal is the model's, not the text's: the editor is untouched.
    expect(editor().value).toBe("alpha two");
    expect(root.querySelector("main > .banner-error")).toBeNull();
  });

  it("does not hold the editor back while SCE draws", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
    const held = core.hold("figures");
    await click("Alpha");

    expect(editor().value).toBe("alpha two");
    expect(modelText()).toContain("SCE is drawing the model");
    expect(images()).toHaveLength(0);

    held.release();
    await settle();
    expect(images()).toHaveLength(2);
    expect(modelText()).not.toContain("SCE is drawing");
  });

  it("does not put one work's drawing under another work", async () => {
    core.setModel("alpha", "<scxml>alpha</scxml>", core.revision("alpha two"));
    const held = core.hold("figures");
    await click("Alpha");
    await click("Beta");
    held.release();
    await settle();

    expect(heading()).toBe("Beta");
    expect(images()).toHaveLength(0);
    expect(modelText()).toContain("No model yet");
  });

  it("moves where the model stands when the text is saved, without drawing it again", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
    await click("Alpha");
    expect(core.callsOf("figures")).toHaveLength(1);
    expect(modelText()).toContain("written for the text as it is now");

    await type("alpha three");
    await click("Save");

    expect(modelText()).toContain("earlier text");
    expect(modelText()).toContain(core.revision("alpha two").slice(0, 12));
    expect(core.callsOf("figures")).toHaveLength(1);
    expect(images()).toHaveLength(2);
  });

  it("draws again when asked to read again", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
    await click("Alpha");
    core.setModel("alpha", "<scxml>changed</scxml>", core.revision("alpha two"));
    await click("Read again");

    expect(core.callsOf("figures")).toHaveLength(2);
    expect(root.querySelector(".scxml")?.textContent).toBe("<scxml>changed</scxml>");
    expect(decoded(images()[0] as HTMLImageElement)).toBe(core.figureSvg("<scxml>changed</scxml>"));
  });

  it("asks SCE to draw in the language the screen is in, and again when the language changes", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
    await click("Alpha");
    expect(core.callsOf("figures")[0]?.["lexicon"]).toBe("en");

    const picker = root.querySelector("header select") as HTMLSelectElement;
    picker.value = "ko";
    picker.dispatchEvent(new Event("change", { bubbles: true }));
    await settle();

    expect(core.callsOf("figures")).toHaveLength(2);
    expect(core.callsOf("figures")[1]?.["lexicon"]).toBe("ko");
    expect(images()).toHaveLength(2);
  });

  it("shows the drawings larger than SCE set them, in a size the person can change and that is kept", async () => {
    const kept = new Map<string, string>();
    const storage = {
      getItem: (key: string): string | null => kept.get(key) ?? null,
      setItem: (key: string, value: string): void => void kept.set(key, value),
    };
    const open = async (): Promise<void> => {
      document.body.innerHTML = '<div id="app"></div>';
      root = document.getElementById("app") as HTMLElement;
      core = new FakeCore();
      core.addWork("alpha", "Alpha", ["alpha one", "alpha two"]);
      core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
      app = new App(root, { transport: core, storage, browserLanguage: "en" });
      await app.start();
      await settle();
      await click("Alpha");
    };
    const zoom = (): string => root.querySelector<HTMLElement>(".sheets")?.getAttribute("style") ?? "";

    await open();
    expect(zoom()).toContain("--sheet-zoom: 1.5");
    expect(buttons("150%")[0]?.getAttribute("aria-pressed")).toBe("true");

    await click("200%");
    expect(zoom()).toContain("--sheet-zoom: 2");
    expect(buttons("200%")[0]?.getAttribute("aria-pressed")).toBe("true");
    expect(buttons("150%")[0]?.getAttribute("aria-pressed")).toBe("false");
    expect(kept.get("sce.zoom")).toBe("2");
    expect(core.callsOf("figures")).toHaveLength(1);

    // A new screen starts at the size the person chose.
    await open();
    expect(zoom()).toContain("--sheet-zoom: 2");
  });

  it("reports a model that cannot be read as the panel's own message, not as an empty one", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
    core.failNext("read_model", new CommandFailure("corrupt", "the model file is damaged"));
    await click("Alpha");

    expect(modelText()).toContain("the model file is damaged");
    expect(modelText()).not.toContain("No model yet");
    expect(editor().value).toBe("alpha two");
  });
});

// ---- what SCE says of the model -------------------------------------------

const reviewText = (): string => root.querySelector(".review")?.textContent ?? "";
const pseudo = (): HTMLElement | null => root.querySelector(".review pre.pseudo");

describe("what SCE says of the model", () => {
  it("shows the verdict, what the model leaves open, and the page exactly as SCE wrote it", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
    await click("Alpha");

    expect(reviewText()).toContain("SCE accepted the model as a statechart.");
    expect(reviewText()).toContain("1 question(s) the specification leaves open (open-guard)");
    expect(reviewText()).toContain("open-guard (line 3)");
    // Every space and line break of the page is the product's.
    expect(pseudo()?.textContent).toBe("machine door (lexicon: en)\n  state closed:\n    on open   -> opened\n");
    // The page is text, never markup.
    expect(root.querySelector(".review pre.pseudo *")).toBeNull();
  });

  it("says once that a passed check does not say the model agrees with the text", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
    await click("Alpha");

    expect(reviewText()).toContain("does not say the model agrees with your text");
    expect(reviewText()).not.toContain("proved");
  });

  it("asks SCE for the page in the language the screen is in, and again when it changes", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
    await click("Alpha");
    expect(core.callsOf("review")[0]?.["lexicon"]).toBe("en");

    const picker = root.querySelector("header select") as HTMLSelectElement;
    picker.value = "ko";
    picker.dispatchEvent(new Event("change", { bubbles: true }));
    await settle();

    expect(core.callsOf("review")).toHaveLength(2);
    expect(core.callsOf("review")[1]?.["lexicon"]).toBe("ko");
    expect(pseudo()?.textContent).toContain("(lexicon: ko)");
  });

  it("shows every record SCE wrote for a model it refuses, and no page", async () => {
    core.setModel("alpha", "<scxml>REFUSE</scxml>", core.revision("alpha two"));
    await click("Alpha");

    expect(reviewText()).toContain("SCE refused the model");
    expect(reviewText()).toContain("no such state 'nowhere'");
    expect(reviewText()).toContain("validation/invalid-reference");
    expect(reviewText()).toContain("line 3");
    expect(pseudo()).toBeNull();
    // A refused model is not told it passed.
    expect(reviewText()).not.toContain("does not say the model agrees");
  });

  it("keeps the verdict and says why when SCE accepts a model and will not write its page", async () => {
    core.setModel("alpha", "<scxml>NOPAGE</scxml>", core.revision("alpha two"));
    await click("Alpha");

    expect(reviewText()).toContain("SCE accepted the model");
    expect(reviewText()).toContain("did not write its pseudocode page");
    expect(reviewText()).toContain("cli/pseudo-unsupported");
    expect(pseudo()).toBeNull();
  });

  it("says SCE could not answer, in the product's words, and still draws the figures", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
    core.failNext("review", new CommandFailure("sce-timeout", "the SCE generator did not finish in 30 s and was stopped"));
    await click("Alpha");

    expect(reviewText()).toContain("SCE could not read the model");
    expect(reviewText()).toContain("did not finish in 30 s");
    expect(pseudo()).toBeNull();
    expect(images()).toHaveLength(2);
  });

  it("does not hold the page back for a slow drawing, nor the drawing for a slow page", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
    const drawing = core.hold("figures");
    await click("Alpha");
    expect(pseudo()?.textContent).toContain("machine door");
    expect(images()).toHaveLength(0);
    drawing.release();
    await settle();
    expect(images()).toHaveLength(2);

    await click("Beta");
    core.setModel("beta", "<scxml/>", core.revision("beta one"));
    const page = core.hold("review");
    await click("Alpha");
    expect(images()).toHaveLength(2);
    expect(reviewText()).toContain("Reading what SCE says");
    page.release();
    await settle();
    expect(pseudo()).not.toBeNull();
  });

  it("does not put one work's page under another work", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
    const page = core.hold("review", (args) => args["id"] === "alpha");
    await click("Alpha");
    await click("Beta");
    page.release();
    await settle();

    expect(heading()).toBe("Beta");
    expect(root.querySelector(".review")).toBeNull();
    expect(pseudo()).toBeNull();
  });

  it("is not asked for a work that has no model, and is gone with a work that is removed", async () => {
    await click("Alpha");
    expect(core.callsOf("review")).toHaveLength(0);
    expect(root.querySelector(".review")).toBeNull();

    core.setModel("beta", "<scxml/>", core.revision("beta one"));
    await click("Beta");
    expect(pseudo()).not.toBeNull();
    await click("Remove this work");
    await click("Remove");
    expect(root.querySelector(".review")).toBeNull();
  });

  it("is not asked again when only the text is saved: the model is the same", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
    await click("Alpha");
    expect(core.callsOf("review")).toHaveLength(1);

    await type("alpha changed");
    await click("Save");
    expect(core.callsOf("review")).toHaveLength(1);
    expect(core.callsOf("figures")).toHaveLength(1);
    // Where the model stands has moved, and the screen says so.
    expect(modelText()).toContain("earlier text");
    expect(pseudo()).not.toBeNull();
  });
});

// ---- the owner's answers --------------------------------------------------

const fields = (): HTMLTextAreaElement[] => [...root.querySelectorAll<HTMLTextAreaElement>("textarea[data-qid]")];
const fieldOf = (id: string): HTMLTextAreaElement => {
  const found = fields().find((f) => f.dataset["qid"] === id);
  if (found === undefined) throw new Error(`no answer field for ${id} in: ${root.textContent}`);
  return found;
};
const saveAnswersButton = (): HTMLButtonElement => root.querySelector("#save-answers") as HTMLButtonElement;
const answersStatus = (): string => root.querySelector("#answers-status")?.textContent ?? "";

async function answer(id: string, text: string): Promise<void> {
  const field = fieldOf(id);
  field.value = text;
  field.dispatchEvent(new Event("input", { bubbles: true }));
  await settle();
}

describe("the owner's answers", () => {
  beforeEach(() => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
  });

  it("are asked for, one field to each question the model marks, in the model's own words", async () => {
    await click("Alpha");

    expect(fields().map((f) => f.dataset["qid"])).toEqual(["open-guard", "close-delay"]);
    expect(root.querySelector(".answers")?.textContent).toContain("Which cards open the door?");
    expect(root.querySelector(".answers")?.textContent).toContain("The model gave no wording for this question.");
    expect(root.querySelector(".answers")?.textContent).toContain("Nothing here changes the model.");
    expect(saveAnswersButton().disabled).toBe(true);
  });

  it("are saved as typed, and the screen then shows what the core holds", async () => {
    await click("Alpha");
    await answer("open-guard", "Any card on the list opens it.");
    expect(saveAnswersButton().disabled).toBe(false);
    expect(answersStatus()).toBe("Unsaved answers");
    expect(app.hasUnsavedChanges()).toBe(true);

    await click("Save answers");

    expect(core.callsOf("save_answers")).toEqual([
      { id: "alpha", answers: { "open-guard": "Any card on the list opens it." }, base: null },
    ]);
    expect(core.answersHeld("alpha")).toEqual({ "open-guard": "Any card on the list opens it." });
    expect(answersStatus()).toBe("Answers saved");
    expect(fieldOf("open-guard").value).toBe("Any card on the list opens it.");
    expect(root.querySelector(".answered-at")?.textContent).toContain("Said");
    expect(saveAnswersButton().disabled).toBe(true);
    expect(app.hasUnsavedChanges()).toBe(false);
  });

  it("are shown when they were given before, and carried when another question is answered", async () => {
    core.setAnswers("alpha", { "open-guard": "Any card on the list opens it." });
    await click("Alpha");
    expect(fieldOf("open-guard").value).toBe("Any card on the list opens it.");

    await answer("close-delay", "Ten seconds.");
    await click("Save answers");

    const sent = core.callsOf("save_answers")[0];
    expect(sent?.["answers"]).toEqual({
      "close-delay": "Ten seconds.",
      "open-guard": "Any card on the list opens it.",
    });
    expect(sent?.["base"]).toMatch(/^[0-9a-f]{64}$/);
  });

  it("are taken back when their field is cleared", async () => {
    core.setAnswers("alpha", { "open-guard": "Any card.", "close-delay": "Ten seconds." });
    await click("Alpha");
    await answer("close-delay", "");
    await click("Save answers");

    expect(core.answersHeld("alpha")).toEqual({ "open-guard": "Any card." });
    expect(fieldOf("close-delay").value).toBe("");
  });

  it("are not offered for saving when nothing differs from what is saved", async () => {
    core.setAnswers("alpha", { "open-guard": "Any card." });
    await click("Alpha");
    await answer("open-guard", "Any card, always.");
    expect(saveAnswersButton().disabled).toBe(false);
    await answer("open-guard", "Any card.");
    expect(saveAnswersButton().disabled).toBe(true);
    expect(app.hasUnsavedChanges()).toBe(false);
  });

  it("keep what was typed, and say so, when they were saved elsewhere meanwhile", async () => {
    core.setAnswers("alpha", { "open-guard": "yes" });
    await click("Alpha");
    await answer("open-guard", "mine");
    // Another entrance saves on top while the person is typing.
    core.setAnswers("alpha", { "open-guard": "theirs" });
    await click("Save answers");

    expect(root.querySelector(".answers")?.textContent).toContain("saved elsewhere while you were typing");
    expect(fieldOf("open-guard").value).toBe("mine");
    expect(core.answersHeld("alpha")).toEqual({ "open-guard": "theirs" });

    // Saved again, on top of the revision that turned up.
    await click("Save answers");
    expect(core.answersHeld("alpha")).toEqual({ "open-guard": "mine" });
    expect(root.querySelector(".answers")?.textContent).not.toContain("saved elsewhere");
  });

  it("say why they were not saved, and keep what was typed", async () => {
    await click("Alpha");
    await answer("open-guard", "Any card.");
    core.failNext("save_answers", new CommandFailure("io", "the disk is full"));
    await click("Save answers");

    expect(root.querySelector(".answers")?.textContent).toContain("The answers were not saved: the disk is full");
    expect(fieldOf("open-guard").value).toBe("Any card.");
    expect(app.hasUnsavedChanges()).toBe(true);
  });

  it("to questions the model no longer asks are kept apart, and can be taken back", async () => {
    core.setAnswers("alpha", { "old-question": "It stays the same." });
    await click("Alpha");

    expect(root.querySelector(".orphans")?.textContent).toContain("Answers to questions this model does not ask");
    expect(fieldOf("old-question").value).toBe("It stays the same.");
    await answer("old-question", "");
    await click("Save answers");
    expect(core.answersHeld("alpha")).toEqual({});
  });

  it("are asked about before another work replaces them, and saved when the person says so", async () => {
    await click("Alpha");
    await answer("open-guard", "Any card.");
    await click("Beta");

    expect(root.textContent).toContain("not saved");
    expect(heading()).toBe("Alpha");
    expect(core.callsOf("read_source").filter((a) => a["id"] === "beta")).toHaveLength(0);

    await click("Save, then open it");
    expect(core.answersHeld("alpha")).toEqual({ "open-guard": "Any card." });
    expect(heading()).toBe("Beta");
  });

  it("are protected from what is typed while the other work is still being read", async () => {
    await click("Alpha");
    const slow = core.hold("read_source", (a) => a["id"] === "beta");
    await click("Beta");
    await answer("open-guard", "Any card.");
    slow.release();
    await settle();

    expect(heading()).toBe("Alpha");
    expect(fieldOf("open-guard").value).toBe("Any card.");
    expect(root.textContent).toContain("not saved");

    await click("Save, then open it");
    expect(core.answersHeld("alpha")).toEqual({ "open-guard": "Any card." });
    expect(heading()).toBe("Beta");
  });

  it("stay under the cursor when SCE's drawing arrives and the screen is redrawn", async () => {
    const drawing = core.hold("figures");
    await click("Alpha");
    const field = fieldOf("open-guard");
    field.focus();
    field.value = "Any car";
    field.dispatchEvent(new Event("input", { bubbles: true }));
    field.setSelectionRange(3, 5);

    drawing.release();
    await settle();

    const after = fieldOf("open-guard");
    expect(document.activeElement).toBe(after);
    expect(after.value).toBe("Any car");
    expect([after.selectionStart, after.selectionEnd]).toEqual([3, 5]);
  });

  it("that cannot be read are said so in words, and the model's review still shows", async () => {
    core.failNext("read_answers", new CommandFailure("corrupt", "the saved answers are not what the store wrote"));
    await click("Alpha");

    expect(reviewText()).toContain("Your answers could not be read");
    expect(reviewText()).toContain("not what the store wrote");
    expect(pseudo()).not.toBeNull();
    expect(fields()).toHaveLength(0);
  });

  it("of one work are not put under another when they arrive late", async () => {
    core.setAnswers("alpha", { "open-guard": "Any card." });
    const held = core.hold("read_answers", (args) => args["id"] === "alpha");
    await click("Alpha");
    await click("Beta");
    held.release();
    await settle();

    expect(heading()).toBe("Beta");
    expect(fields()).toHaveLength(0);
    expect(app.hasUnsavedChanges()).toBe(false);
  });
});

// ---- a desktop window being closed ----------------------------------------

/** What the shell of a desktop window hears from the screen, and is told to do. */
class FakeDesktop {
  readonly reports: boolean[] = [];
  closed = 0;
  unsaved(unsaved: boolean): void {
    this.reports.push(unsaved);
  }
  async close(): Promise<void> {
    this.closed += 1;
  }
}

describe("a desktop window asked to close", () => {
  let desktop: FakeDesktop;

  beforeEach(async () => {
    desktop = new FakeDesktop();
    document.body.innerHTML = '<div id="app"></div>';
    root = document.getElementById("app") as HTMLElement;
    core = new FakeCore();
    core.addWork("alpha", "Alpha", ["alpha one", "alpha two"]);
    core.addWork("beta", "Beta", ["beta one"]);
    core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
    app = new App(root, { transport: core, storage: null, browserLanguage: "en", desktop });
    await app.start();
    await settle();
  });

  it("is told what is unsaved when it changes, and only then", async () => {
    await click("Alpha");
    expect(desktop.reports.at(-1)).toBe(false);
    const before = desktop.reports.length;

    await type("alpha changed");
    expect(desktop.reports.at(-1)).toBe(true);
    await type("alpha changed again");
    expect(desktop.reports.length).toBe(before + 1);

    await click("Save");
    expect(desktop.reports.at(-1)).toBe(false);
  });

  it("is told about typed answers the same way", async () => {
    await click("Alpha");
    await answer("open-guard", "Any card.");
    expect(desktop.reports.at(-1)).toBe(true);
    await click("Save answers");
    expect(desktop.reports.at(-1)).toBe(false);
  });

  it("closes at once when nothing is unsaved after all", async () => {
    await click("Alpha");
    app.askToClose();
    await settle();

    expect(desktop.closed).toBe(1);
    expect(root.textContent).not.toContain("Closing the window would lose");
  });

  it("asks first when the editor holds text the core has not been given, and stays when the person says so", async () => {
    await click("Alpha");
    await type("alpha changed");
    app.askToClose();
    await settle();

    expect(root.textContent).toContain("Closing the window would lose");
    expect(desktop.closed).toBe(0);
    await click("Stay here");
    expect(root.textContent).not.toContain("Closing the window would lose");
    expect(desktop.closed).toBe(0);
    expect(editor().value).toBe("alpha changed");
  });

  it("says again that something is unsaved while the question is open: that is how the shell knows it is answered", async () => {
    await click("Alpha");
    await type("alpha changed");
    const before = desktop.reports.length;
    app.askToClose();
    await settle();

    expect(desktop.reports.length).toBeGreaterThan(before);
    expect(desktop.reports.at(-1)).toBe(true);
  });

  it("closes without saving only when the person says to discard", async () => {
    await click("Alpha");
    await type("alpha changed");
    app.askToClose();
    await settle();
    await click("Discard my changes and close");

    expect(desktop.closed).toBe(1);
    expect(core.headText("alpha")).toBe("alpha two");
  });

  it("saves the text and the answers first when the person says to, and then closes", async () => {
    await click("Alpha");
    await type("alpha changed");
    await answer("open-guard", "Any card.");
    app.askToClose();
    await settle();
    await click("Save, then close");

    expect(core.headText("alpha")).toBe("alpha changed");
    expect(core.answersHeld("alpha")).toEqual({ "open-guard": "Any card." });
    expect(desktop.closed).toBe(1);
  });

  it("stays open when the save before closing does not take", async () => {
    await click("Alpha");
    await type("alpha changed");
    core.failNext("save_source", new CommandFailure("io", "the disk is full"));
    app.askToClose();
    await settle();
    await click("Save, then close");

    expect(desktop.closed).toBe(0);
    expect(root.textContent).toContain("the disk is full");
    expect(app.hasUnsavedChanges()).toBe(true);
  });

  it("is not closed under a save that is still on its way", async () => {
    await click("Alpha");
    await type("alpha changed");
    const held = core.hold("save_source");
    app.askToClose();
    await settle();
    void click("Save, then close");
    await settle();
    expect(desktop.closed).toBe(0);
    expect(buttons("Discard my changes and close")[0]?.disabled).toBe(true);

    held.release();
    await settle();
    expect(desktop.closed).toBe(1);
  });
});

// ---- removing a work ------------------------------------------------------

const workLinks = (): string[] => [...root.querySelectorAll(".work-link")].map((b) => b.textContent ?? "");

describe("removing a work", () => {
  it("asks first, says what stays, and removes nothing until the person says yes", async () => {
    await click("Alpha");
    await click("Remove this work");

    expect(root.textContent).toContain('Remove "Alpha" from the list?');
    expect(root.textContent).toContain("removed.json");
    expect(core.callsOf("remove_work")).toHaveLength(0);

    await click("Keep the work");
    expect(root.textContent).not.toContain('Remove "Alpha" from the list?');
    expect(core.callsOf("remove_work")).toHaveLength(0);
    expect(workLinks()).toEqual(["Alpha", "Beta"]);
  });

  it("takes the work out of the list and the screen, and tells the person", async () => {
    await click("Alpha");
    await click("Remove this work");
    await click("Remove");

    expect(core.callsOf("remove_work")).toEqual([{ id: "alpha" }]);
    expect(workLinks()).toEqual(["Beta"]);
    expect(editor()).toBeNull();
    expect(root.textContent).toContain("Pick a work");
    expect(root.textContent).toContain('"Alpha" was removed from the list.');

    // The work beside it is as it was.
    await click("Beta");
    expect(editor().value).toBe("beta one");
    expect(root.textContent).not.toContain("was removed from the list");
  });

  it("says that text not yet saved goes with the work, and only then", async () => {
    await click("Alpha");
    await click("Remove this work");
    expect(root.textContent).not.toContain("not saved is lost");
    await click("Keep the work");

    await type("alpha changed");
    await click("Remove this work");
    expect(root.textContent).toContain("not saved is lost");
  });

  it("is not offered while a save is on its way", async () => {
    await click("Alpha");
    const held = core.hold("save_source");
    await type("alpha changed");
    await click("Save");

    const remove = buttons("Remove this work")[0];
    expect(remove?.disabled).toBe(true);
    held.release();
    await settle();
    expect(buttons("Remove this work")[0]?.disabled).toBe(false);
  });

  it("does not let a drawing that was still on its way land under the work that replaced it", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha two"));
    const drawing = core.hold("figures");
    await click("Alpha");
    await click("Remove this work");
    await click("Remove");
    drawing.release();
    await settle();

    expect(images()).toHaveLength(0);
    expect(root.querySelector(".model")).toBeNull();
    expect(root.textContent).toContain("Pick a work");
  });

  it("keeps the work on screen, with the core's words, when the removal is refused", async () => {
    await click("Alpha");
    core.failNext("remove_work", new CommandFailure("busy", "another save held the work for 30000 ms"));
    await click("Remove this work");
    await click("Remove");

    expect(root.textContent).toContain("another save held the work");
    expect(heading()).toBe("Alpha");
    expect(workLinks()).toEqual(["Alpha", "Beta"]);
    expect(root.textContent).not.toContain("was removed from the list");
  });
});

// ---- accepting the design ---------------------------------------------------

const acceptanceText = (): string => root.querySelector(".acceptance")?.textContent ?? "";
const acceptButton = (): HTMLButtonElement => root.querySelector("#accept") as HTMLButtonElement;
const acceptNote = (): string => root.querySelector("#accept-note")?.textContent ?? "";
const requirementRows = (): string[][] =>
  [...root.querySelectorAll(".acceptance tbody tr")].map((row) =>
    [...row.querySelectorAll("td")].map((cell) => cell.textContent ?? ""),
  );

/** The revision of the text a work holds now: what a model or a list says it was written for. */
const headOf = (id: string): string => core.revision(core.headText(id) as string);

describe("accepting the design", () => {
  beforeEach(() => {
    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    core.setRequirements("alpha", headOf("alpha"));
  });

  it("says there is no requirement list when the work has none, and offers nothing to accept", async () => {
    core.setModel("beta", "<scxml/>", headOf("beta"));
    await click("Beta");

    expect(acceptanceText()).toContain("No requirement list yet");
    expect(root.querySelector("#accept")).toBeNull();
    expect(core.callsOf("requirements_report")).toHaveLength(0);
    expect(core.callsOf("read_acceptance")).toHaveLength(0);
  });

  it("is not asked for a work that has no model", async () => {
    await click("Beta");

    expect(root.querySelector(".acceptance")).toBeNull();
    expect(core.callsOf("read_requirements")).toHaveLength(0);
  });

  it("shows SCE's count, each requirement in SCE's word, and the page exactly as SCE wrote it", async () => {
    core.setModel("alpha", "<scxml>MISSING SCENARIO</scxml>", headOf("alpha"));
    await click("Alpha");

    expect(acceptanceText()).toContain("SCE measured the design against 2 requirements (synthesized).");
    expect([...root.querySelectorAll(".acceptance .tally li")].map((li) => li.textContent)).toEqual([
      "1 missing: nothing in the design carries it",
      "1 needs-scenario: only a test can settle it",
    ]);
    expect(requirementRows()).toEqual([
      ["R1", "missing", "S1", "nowhere"],
      ["R2", "needs-scenario", "S2", "states.s1"],
    ]);
    expect(root.querySelector(".acceptance .page pre")?.textContent).toBe("ACCEPTANCE REPORT\n  2 requirements\n");
  });

  it("marks the requirements SCE finds unsettled, says what each of SCE's words means, and shows a new one as spelled", async () => {
    core.setModel("alpha", "<scxml>MISSING DANGLING</scxml>", headOf("alpha"));
    await click("Alpha");
    const lines = (selector: string): Array<string | null> =>
      [...root.querySelectorAll(selector)].map((li) => li.textContent);
    expect(lines(".acceptance .tally li")).toEqual([
      "1 missing: nothing in the design carries it",
      "1 dangling: the design cites it and the list has no such requirement",
    ]);
    expect(lines(".acceptance .tally li.unsettled")).toHaveLength(2);
    expect(acceptanceText()).toContain("The marked lines are requirements the design leaves unsettled.");

    // A word a later SCE writes is shown as it was spelled, and is not guessed to be a gap.
    core.setModel("alpha", "<scxml>WAIVED</scxml>", headOf("alpha"));
    await click("Read again");
    expect(lines(".acceptance .tally li")).toEqual(["1 implemented: a node of the design carries it", "1 waived"]);
    expect(lines(".acceptance .tally li.unsettled")).toEqual([]);
    expect(acceptanceText()).not.toContain("leaves unsettled");
  });

  it("tells the owner of the gaps before they accept, and still lets them accept", async () => {
    core.setModel("alpha", "<scxml>MISSING</scxml>", headOf("alpha"));
    await click("Alpha");
    expect([...root.querySelectorAll(".acceptance .tally li.unsettled")].map((li) => li.textContent)).toEqual([
      "1 missing: nothing in the design carries it",
    ]);
    expect([...root.querySelectorAll(".acceptance .gaps li")].map((li) => li.textContent)).toEqual([
      "Matters SCE lists as left open: 1",
    ]);
    expect(acceptanceText()).toContain("Nothing has been accepted yet.");
    expect(acceptButton().disabled).toBe(false);

    await click("Accept this design");

    const [sent] = core.callsOf("accept");
    expect(core.callsOf("accept")).toHaveLength(1);
    expect(sent?.["id"]).toBe("alpha");
    const expectation = sent?.["expect"] as Record<string, string | null>;
    // Exactly what the page showed, revision for revision; the owner had answered nothing.
    expect(Object.keys(expectation).sort()).toEqual(["answers", "model", "requirements", "source"]);
    expect(expectation["answers"]).toBeNull();
    expect(expectation["source"]).toBe(headOf("alpha"));
    expect(expectation["model"]).toBe(core.revision("model:<scxml>MISSING</scxml>"));
    expect(acceptanceText()).toContain("It holds");
    expect(acceptanceText()).toContain("Accepted here, in this application.");
    expect(acceptButton().disabled).toBe(true);
    expect(acceptNote()).toBe("This design is accepted as it is.");
  });

  it("accepts only what it showed: what moved meanwhile is not accepted, and what is there now is shown", async () => {
    await click("Alpha");
    // Another entrance saves a different design after this screen showed the page.
    core.setModel("alpha", "<scxml><!-- another --></scxml>", headOf("alpha"));
    await click("Accept this design");

    expect(core.callsOf("accept")).toHaveLength(1);
    expect(acceptanceText()).toContain("Nothing was accepted: model changed after you were shown it");
    expect(acceptanceText()).toContain("Nothing has been accepted yet.");
    // What is there now was read, so the next press is made knowing it.
    expect(core.callsOf("requirements_report")).toHaveLength(2);
    await click("Accept this design");

    const second = core.callsOf("accept")[1]?.["expect"] as Record<string, string>;
    expect(second["model"]).toBe(core.revision("model:<scxml><!-- another --></scxml>"));
    expect(acceptanceText()).toContain("It holds");
    expect(acceptanceText()).not.toContain("Nothing was accepted");
  });

  it("is withheld while the saved answers are still being read, and offered when they are shown", async () => {
    core.setAnswers("alpha", { "open-guard": "Only listed cards." });
    const slow = core.hold("read_answers");
    await click("Alpha");

    // The page was measured with the answers, and the owner has not been shown them yet.
    expect(acceptButton().disabled).toBe(true);
    expect(acceptNote()).toContain("still being read");
    acceptButton().click();
    await settle();
    expect(core.callsOf("accept")).toHaveLength(0);

    slow.release();
    await settle();
    expect(fieldOf("open-guard").value).toBe("Only listed cards.");
    expect(acceptButton().disabled).toBe(false);
    expect(acceptNote()).toBe("");
  });

  it("stays withheld when the saved answers cannot be read, and says so", async () => {
    core.setAnswers("alpha", { "open-guard": "Only listed cards." });
    core.failNext("read_answers", new CommandFailure("corrupt", "the saved answers are not what the store wrote"));
    await click("Alpha");

    expect(acceptButton().disabled).toBe(true);
    expect(acceptNote()).toContain("could not be read");
  });

  it("is withheld when the answers read are not the ones the page measured", async () => {
    core.setAnswers("alpha", { "open-guard": "Only listed cards." });
    await click("Alpha");
    expect(acceptButton().disabled).toBe(false);

    // Answers saved from another entrance after the page was measured; this screen read them before.
    core.setAnswers("alpha", { "open-guard": "Any card." });
    await click("Read again");
    expect(acceptButton().disabled).toBe(true);
    expect(acceptNote()).toContain("not the one the design was measured against");
  });

  it("accepts nothing when the text moved on in another entrance after the page was shown", async () => {
    await click("Alpha");
    // The text moves on in another entrance; this screen has not read it yet.
    core.addWork("alpha", "Alpha", ["alpha one", "alpha two", "alpha three"]);
    await click("Accept this design");

    expect(acceptanceText()).toContain("Nothing was accepted: source changed after you were shown it");
    expect(core.callsOf("accept")).toHaveLength(1);
  });

  it("is withheld while text or answers are typed and not saved, with the reason, and offered again when they are not", async () => {
    await click("Alpha");
    expect(acceptButton().disabled).toBe(false);
    expect(acceptNote()).toBe("");

    await type("alpha changed");
    expect(acceptButton().disabled).toBe(true);
    expect(acceptNote()).toBe("Save the text and your answers first: what is accepted is what is saved.");
    await type("alpha two");
    expect(acceptButton().disabled).toBe(false);
    expect(acceptNote()).toBe("");

    await answer("open-guard", "Any card on the list.");
    expect(acceptButton().disabled).toBe(true);
    expect(acceptNote()).toContain("Save the text and your answers first");
    await answer("open-guard", "");
    expect(acceptButton().disabled).toBe(false);
    expect(core.callsOf("accept")).toHaveLength(0);
  });

  it("is withheld for a design or a list written for an earlier text, and says so", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha one"));
    await click("Alpha");

    expect(acceptButton().disabled).toBe(true);
    expect(acceptNote()).toContain("written for an earlier text");
    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    core.setRequirements("alpha", core.revision("alpha one"));
    await click("Read again");
    expect(acceptanceText()).toContain("The requirement list was written for an earlier text");
    expect(acceptButton().disabled).toBe(true);
  });

  it("is withheld while the text on screen is not the one the design was measured against", async () => {
    await click("Alpha");
    expect(acceptButton().disabled).toBe(false);

    // Another entrance saves a new text, and the authoring client writes the design and
    // the list for it. This screen reads the model again and not the text.
    await core.call("save_source", { id: "alpha", text: "alpha three", base: headOf("alpha") });
    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    core.setRequirements("alpha", headOf("alpha"));
    await click("Read again");

    expect(editor().value).toBe("alpha two");
    expect(acceptButton().disabled).toBe(true);
    expect(acceptNote()).toContain("not the one the design was measured against");
    acceptButton().click();
    await settle();
    expect(core.callsOf("accept")).toHaveLength(0);

    // Reading the work again shows the text the design is about, and the owner may accept.
    await click("Alpha");
    expect(editor().value).toBe("alpha three");
    expect(acceptButton().disabled).toBe(false);
    expect(acceptNote()).toBe("");
  });

  it("does not ask the owner to read the text again when the screen already shows it", async () => {
    await click("Alpha");
    await click("Read again");

    expect(acceptButton().disabled).toBe(false);
    expect(acceptNote()).toBe("");
  });

  it("says an acceptance lapsed, in SCE's sentence, and offers to accept the design as it is now", async () => {
    core.setAcceptance("alpha");
    core.setModel("alpha", "<scxml><!-- edited --></scxml>", headOf("alpha"));
    await click("Alpha");

    expect(acceptanceText()).toContain("no longer holds. SCE says: design/model.scxml moved");
    expect(acceptanceText()).toContain("What SCE listed as left open when it was accepted");
    expect(acceptButton().textContent).toBe("Accept the design as it is now");
    expect(acceptButton().disabled).toBe(false);
  });

  it("says when an acceptance was relayed by an authoring client and not made in this application", async () => {
    core.setAcceptance("alpha", "relayed");
    await click("Alpha");

    expect(acceptanceText()).toContain("It holds");
    expect(acceptanceText()).toContain("Relayed by an authoring client: it was not accepted in this application.");
  });

  it("says SCE did not measure the design, keeps what was accepted, and offers nothing to accept", async () => {
    core.setAcceptance("alpha");
    core.failNext("requirements_report", new CommandFailure("sce-timeout", "SCE did not answer within 30 s"));
    await click("Alpha");

    expect(acceptanceText()).toContain("SCE did not measure the design against the list: SCE did not answer within 30 s");
    expect(acceptanceText()).toContain("It holds");
    expect(root.querySelector(".acceptance table")).toBeNull();
    expect(acceptButton().disabled).toBe(true);
    expect(acceptNote()).toContain("SCE did not measure the design");
  });

  it("says in words when the acceptance cannot be read, and the review still shows", async () => {
    core.failNext("read_acceptance", new CommandFailure("sce-failed", "the product crashed"));
    await click("Alpha");

    expect(acceptanceText()).toContain("could not be read: the product crashed");
    expect(pseudo()).not.toBeNull();
  });

  it("is read again when the answers are saved, which are part of what is accepted", async () => {
    await click("Alpha");
    await click("Accept this design");
    expect(acceptanceText()).toContain("It holds");
    const before = core.callsOf("read_acceptance").length;

    await answer("open-guard", "Any card on the list.");
    await click("Save answers");

    expect(core.callsOf("read_acceptance")).toHaveLength(before + 1);
    expect(acceptanceText()).toContain("no longer holds. SCE says: spec/answers.json moved");
    expect(acceptButton().textContent).toBe("Accept the design as it is now");
  });

  it("is not put under another work when it arrives late, and is gone with a work that is removed", async () => {
    core.setRequirements("alpha", headOf("alpha"), ["A1", "A2"]);
    core.setModel("beta", "<scxml/>", headOf("beta"));
    core.setRequirements("beta", headOf("beta"), ["B1"]);
    const slow = core.hold("read_requirements", (a) => a["id"] === "alpha");
    await click("Alpha");
    await click("Beta");
    slow.release();
    await settle();

    expect(requirementRows().map((row) => row[0])).toEqual(["B1"]);
    await click("Remove this work");
    await click("Remove");
    expect(root.querySelector(".acceptance")).toBeNull();
  });

  it("is not accepted twice while the first is on its way, and a work is not removed under it", async () => {
    await click("Alpha");
    const held = core.hold("accept");
    buttons("Accept this design")[0]?.click();
    await settle();

    expect(acceptButton().textContent).toBe("Accepting...");
    expect(acceptButton().disabled).toBe(true);
    expect(buttons("Remove this work")[0]?.disabled).toBe(true);
    held.release();
    await settle();

    expect(core.callsOf("accept")).toHaveLength(1);
    expect(acceptanceText()).toContain("It holds");
  });
});

/** A timer the test holds: a question waits until it is let go, so what follows it can be looked at. */
class ManualTicker implements Ticker {
  private readonly waiting: Array<{ run: () => void; cancelled: boolean; ms: number }> = [];

  after(ms: number, run: () => void): () => void {
    const entry = { run, cancelled: false, ms };
    this.waiting.push(entry);
    return () => {
      entry.cancelled = true;
    };
  }

  /** The waits asked for and neither let go nor cancelled, in the order they were asked. */
  get pending(): number[] {
    return this.waiting.filter((w) => !w.cancelled).map((w) => w.ms);
  }

  /** Let the next wait end, and let the question it starts and what follows from it finish. */
  async fire(): Promise<void> {
    const next = this.waiting.find((w) => !w.cancelled);
    if (next === undefined) throw new Error("no question is waiting to be asked");
    next.cancelled = true;
    next.run();
    for (let i = 0; i < 3; i += 1) await settle();
  }
}

describe("a token the server asks for in the middle of a read", () => {
  beforeEach(async () => {
    document.body.innerHTML = '<div id="app"></div>';
    root = document.getElementById("app") as HTMLElement;
    core = new FakeCore();
    core.addWork("alpha", "Alpha", ["alpha one", "alpha two"]);
    core.addWork("beta", "Beta", ["beta one"]);
    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    core.setRequirements("alpha", headOf("alpha"));
    app = new App(root, {
      transport: core,
      storage: null,
      browserLanguage: "en",
      credentials: { token: () => null, save: () => undefined },
    });
    await app.start();
    await settle();
  });

  // Each of these reads is started beside the others when a work opens, and each is
  // one the server can refuse first. Whichever it is, the person is shown the form that
  // asks for the token: a refusal that only sets a flag leaves the screen as it was.
  for (const name of ["read_answers", "review", "read_requirements", "read_model", "figures"]) {
    it(`draws the sign-in form when \`${name}\` is the read that is refused`, async () => {
      core.failNext(name, new CommandFailure(UNAUTHORIZED, "no token"));

      await click("Alpha");
      await settle();

      expect(root.querySelector("form.token-form"), name).not.toBeNull();
    });
  }

  it("draws it though nothing else is left to draw, the refused read being the last to answer", async () => {
    // A work with no model has nothing to draw once the model panel says so, and the
    // answers are read beside it: the redraw that the model panel's answer brings comes
    // first, so the refusal that follows has to draw the form itself.
    core.failNext("read_answers", new CommandFailure(UNAUTHORIZED, "no token"));

    await click("Beta");
    await settle();

    expect(root.querySelector("form.token-form"), root.textContent ?? "").not.toBeNull();
  });
});

describe("a work that moves under the screen", () => {
  let ticker: ManualTicker;

  beforeEach(async () => {
    ticker = new ManualTicker();
    document.body.innerHTML = '<div id="app"></div>';
    root = document.getElementById("app") as HTMLElement;
    core = new FakeCore();
    core.addWork("alpha", "Alpha", ["alpha one", "alpha two"]);
    core.addWork("beta", "Beta", ["beta one"]);
    app = new App(root, { transport: core, storage: null, browserLanguage: "en", ticker });
    await app.start();
    await settle();
  });

  it("shows a model an authoring client saved after the work was opened, without being asked", async () => {
    await click("Alpha");
    expect(modelText()).toContain("No model yet");

    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    await ticker.fire();

    expect(images()).toHaveLength(2);
    expect(modelText()).toContain("written for the text as it is now");
  });

  it("keeps what the person typed when the work moves under them", async () => {
    await click("Alpha");
    await type("alpha edited");

    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    await ticker.fire();

    expect(images()).toHaveLength(2);
    expect(editor().value).toBe("alpha edited");
    expect(status()).toBe("Unsaved changes");
  });

  it("shows a text another entrance saved when the editor holds nothing of the person's", async () => {
    await click("Alpha");
    expect(editor().value).toBe("alpha two");

    core.saveElsewhere("alpha", "alpha three");
    await ticker.fire();

    expect(editor().value).toBe("alpha three");
  });

  it("does not replace typed text with a text saved elsewhere, and says where the model now stands", async () => {
    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    await click("Alpha");
    expect(modelText()).toContain("written for the text as it is now");

    await type("mine");
    core.saveElsewhere("alpha", "theirs");
    await ticker.fire();

    expect(editor().value).toBe("mine");
    expect(root.querySelector(".model .banner-warn")?.textContent ?? "").toContain("earlier text");
  });

  it("reads a model kept for a later text again, and does not draw it again", async () => {
    core.setModel("alpha", "<scxml/>", core.revision("alpha one"));
    await click("Alpha");
    expect(root.querySelector(".model .banner-warn")?.textContent ?? "").toContain("earlier text");

    // The authoring client read the new text and kept the model: the same revision, written for another text.
    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    await ticker.fire();

    expect(root.querySelector(".model .banner-warn")).toBeNull();
    expect(modelText()).toContain("written for the text as it is now");
    expect(core.callsOf("figures")).toHaveLength(1);
  });

  it("shows an acceptance made elsewhere", async () => {
    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    core.setRequirements("alpha", headOf("alpha"));
    await click("Alpha");
    expect(acceptanceText()).toContain("Nothing has been accepted yet.");
    // The page always says what an acceptance holds for; only an acceptance says that it does.
    expect(acceptanceText()).not.toContain("It holds: the text");

    core.setAcceptance("alpha");
    await ticker.fire();

    expect(acceptanceText()).toContain("It holds: the text, the list, the design and your answers are as they were.");
  });

  it("shows answers saved from another window when none are typed here", async () => {
    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    await click("Alpha");
    expect(fieldOf("open-guard").value).toBe("");

    core.setAnswers("alpha", { "open-guard": "Any card on the list opens it." });
    await ticker.fire();

    expect(fieldOf("open-guard").value).toBe("Any card on the list opens it.");
  });

  it("does not replace answers being typed with answers saved from another window", async () => {
    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    await click("Alpha");
    await answer("open-guard", "Typed here.");

    core.setAnswers("alpha", { "open-guard": "Saved there." });
    await ticker.fire();

    expect(fieldOf("open-guard").value).toBe("Typed here.");
    expect(core.callsOf("read_answers")).toHaveLength(1);
  });

  it("reads nothing again while the work stays as it is shown", async () => {
    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    core.setRequirements("alpha", headOf("alpha"));
    await click("Alpha");
    const read = (name: string): number => core.callsOf(name).length;
    const before = [read("read_model"), read("read_source"), read("read_answers"), read("requirements_report")];

    await ticker.fire();
    await ticker.fire();

    expect(core.callsOf("read_work_heads")).toHaveLength(2);
    expect([read("read_model"), read("read_source"), read("read_answers"), read("requirements_report")]).toEqual(before);
  });

  it("does not take its own save for a change from elsewhere", async () => {
    await click("Alpha");
    await type("alpha edited");
    await click("Save");
    const reads = core.callsOf("read_source").length;

    await ticker.fire();

    expect(core.callsOf("read_source")).toHaveLength(reads);
    expect(editor().value).toBe("alpha edited");
  });

  it("asks again only when the read it started is done, and then reads nothing more", async () => {
    core.setModel("alpha", "<scxml/>", headOf("alpha"));
    await click("Alpha");
    const slow = core.hold("read_model");
    core.setModel("alpha", "<scxml>next</scxml>", headOf("alpha"));
    await ticker.fire();

    // The read is on its way: the next question is not scheduled, so it cannot start the read over.
    expect(core.callsOf("read_model")).toHaveLength(2);
    expect(ticker.pending).toEqual([]);
    slow.release();
    await settle();
    await settle();
    expect(ticker.pending).toEqual([2000]);
    await ticker.fire();

    // The read answered and the screen shows it: nothing more is read.
    expect(core.callsOf("read_model")).toHaveLength(2);
    expect(root.querySelector(".scxml")?.textContent).toBe("<scxml>next</scxml>");
  });

  it("waits twice as long after the core fails to answer, and the usual time again once it does", async () => {
    await click("Alpha");
    expect(ticker.pending).toEqual([2000]);

    core.failNext("read_work_heads", new CommandFailure("io", "the works folder could not be read"));
    await ticker.fire();
    expect(ticker.pending).toEqual([4000]);

    await ticker.fire();
    expect(ticker.pending).toEqual([2000]);
  });

  it("stops asking about a work the person left", async () => {
    await click("Alpha");
    await click("Beta");
    expect(ticker.pending).toEqual([2000]);

    await ticker.fire();

    expect(core.callsOf("read_work_heads").map((a) => a["id"])).toEqual(["beta"]);
  });

  it("stops asking when the server wants a token, and asks again once the person has signed in", async () => {
    const held: { token: string | null } = { token: null };
    app = new App(root, {
      transport: core,
      storage: null,
      browserLanguage: "en",
      ticker,
      credentials: { token: () => held.token, save: (token) => (held.token = token) },
    });
    await app.start();
    await settle();
    await click("Alpha");
    core.failNext("read_work_heads", new CommandFailure(UNAUTHORIZED, "no token"));

    await ticker.fire();

    expect(root.querySelector("form.token-form")).not.toBeNull();
    expect(ticker.pending).toEqual([]);

    const field = root.querySelector('input[name="token"]') as HTMLInputElement;
    field.value = "0123456789abcdef";
    field.form?.dispatchEvent(new Event("submit", { cancelable: true, bubbles: true }));
    await settle();
    await settle();

    expect(held.token).toBe("0123456789abcdef");
    expect(root.querySelector("form.token-form")).toBeNull();
    expect(ticker.pending).toEqual([2000]);
  });

  it("says so when the work was taken away from another window, and stops asking", async () => {
    await click("Alpha");
    await core.call("remove_work", { id: "alpha" });

    await ticker.fire();

    expect(root.textContent).toContain("work `absent`");
    expect(ticker.pending).toEqual([]);
  });
});
