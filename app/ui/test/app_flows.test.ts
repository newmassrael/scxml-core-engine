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

  addWork(id: string, title: string, texts: string[]): void {
    this.works.set(id, { title, revisions: texts.map((text) => ({ revision: this.revision(text), text })) });
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
        return { command_set_version: 1, commands: [], root: "/fake/works" };
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
