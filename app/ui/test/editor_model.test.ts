// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import { describe, expect, it } from "vitest";

import type { Saved, SourceText } from "../src/contract";
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
} from "../src/editor_model";

const A = "a".repeat(64);
const B = "b".repeat(64);
const C = "c".repeat(64);
const source = (revision: string, text: string): SourceText => ({ revision, text });
const outcome = (revision: string, parent: string | null): Saved => ({ outcome: "saved", revision, parent });

describe("a work with no text yet", () => {
  it("opens empty and clean, with nothing to save until something is typed", () => {
    const model = open("w", null);
    expect(model).toMatchObject({ base: null, text: "", saved: "", phase: "idle" });
    expect(isDirty(model)).toBe(false);
    expect(saveRequest(model)).toBeNull();
  });

  it("saves its first text with no base", () => {
    const model = edit(open("w", null), "first");
    expect(saveRequest(model)).toEqual({ id: "w", text: "first", base: null });
  });
});

describe("a work with text", () => {
  it("names the revision it was opened from as the base of the next save", () => {
    const model = edit(open("w", source(A, "one")), "one and more");
    expect(saveRequest(model)).toEqual({ id: "w", text: "one and more", base: A });
  });

  it("is clean again when the text is typed back to what was saved", () => {
    const model = edit(edit(open("w", source(A, "one")), "two"), "one");
    expect(isDirty(model)).toBe(false);
    expect(saveRequest(model)).toBeNull();
  });
});

describe("a save in flight", () => {
  it("blocks a second save of the same buffer", () => {
    const model = saving(edit(open("w", source(A, "one")), "two"));
    expect(model.phase).toBe("saving");
    expect(saveRequest(model)).toBeNull();
  });

  it("leaves what was typed meanwhile in the buffer, unsaved, once the first save lands", () => {
    let model = saving(edit(open("w", source(A, "one")), "two"));
    model = edit(model, "two and three");
    model = saved(model, "two", outcome(B, A));
    expect(model).toMatchObject({ base: B, saved: "two", text: "two and three", phase: "idle" });
    expect(saveRequest(model)).toEqual({ id: "w", text: "two and three", base: B });
  });

  it("is clean after its own save when nothing was typed meanwhile", () => {
    const model = saved(saving(edit(open("w", source(A, "one")), "two")), "two", outcome(B, A));
    expect(isDirty(model)).toBe(false);
  });
});

describe("a conflict", () => {
  const conflict = () => conflicted(saving(edit(open("w", source(A, "one")), "mine")), B);

  it("keeps the person's text and blocks saving until they choose", () => {
    const model = conflict();
    expect(model).toMatchObject({ phase: "conflict", current: B, text: "mine", base: A });
    expect(saveRequest(model)).toBeNull();
  });

  it("lets the person keep theirs: the next save is built on the revision that turned up", () => {
    const model = keepMine(conflict());
    expect(model).toMatchObject({ phase: "idle", base: B, current: null, text: "mine" });
    expect(saveRequest(model)).toEqual({ id: "w", text: "mine", base: B });
  });

  it("lets the person take the other text, which discards their own", () => {
    const model = takeTheirs(conflict(), source(B, "theirs"));
    expect(model).toMatchObject({ phase: "idle", base: B, text: "theirs", saved: "theirs" });
    expect(isDirty(model)).toBe(false);
  });

  it("is not left by keepMine when there is no conflict", () => {
    const model = edit(open("w", source(A, "one")), "two");
    expect(keepMine(model)).toBe(model);
  });

  it("can happen again: a second concurrent save conflicts on the new base", () => {
    let model = keepMine(conflict());
    model = conflicted(saving(model), C);
    expect(model).toMatchObject({ phase: "conflict", base: B, current: C });
  });
});

describe("a failure", () => {
  it("keeps the text and allows a retry", () => {
    const model = failed(saving(edit(open("w", source(A, "one")), "two")), "the disk is full");
    expect(model).toMatchObject({ phase: "failed", failure: "the disk is full", text: "two" });
    expect(saveRequest(model)).toEqual({ id: "w", text: "two", base: A });
  });

  it("clears the failure when the retry starts", () => {
    const model = saving(failed(edit(open("w", source(A, "one")), "two"), "x"));
    expect(model).toMatchObject({ phase: "saving", failure: null });
  });
});
