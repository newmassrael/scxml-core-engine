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
import { CommandFailure, type Args, type Transport } from "../src/ipc";

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
  private readonly models = new Map<string, { text: string; writtenFor: string | null }>();
  private readonly answersOf = new Map<
    string,
    { revision: string; entries: Record<string, { answer: string; answered_at: string }> }
  >();
  private answerSaves = 0;

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

  /** The work's model, and the text revision its writer says it read. */
  setModel(id: string, text: string, writtenFor: string | null): void {
    this.models.set(id, { text, writtenFor });
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
        return { command_set_version: 5, commands: [], root: "/fake/works" };
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
          return {
            model: { revision, written_for: model.writtenFor, text: model.text },
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
