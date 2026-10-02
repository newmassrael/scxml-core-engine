// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

import { describe, expect, it } from "vitest";

import { allStrings, initialLocale, LOCALES, translate } from "../src/i18n";

const placeholders = (text: string) => [...text.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();

describe("the screen's words", () => {
  it("have no empty string in any language", () => {
    for (const [locale, key, text] of allStrings()) {
      expect(text.trim(), `${locale}.${key}`).not.toBe("");
    }
  });

  it("use the same placeholders in every language", () => {
    const byKey = new Map<string, string[][]>();
    for (const [, key, text] of allStrings()) {
      byKey.set(key, [...(byKey.get(key) ?? []), placeholders(text).map(String)]);
    }
    for (const [key, sets] of byKey) {
      expect(sets.length, key).toBe(LOCALES.length);
      for (const set of sets) expect(set, key).toEqual(sets[0]);
    }
  });

  it("fill placeholders and leave an unfilled one visible", () => {
    expect(translate("en", "viewingOld", { rev: "abc" })).toBe("Viewing revision abc. This is read-only.");
    expect(translate("en", "viewingOld")).toContain("{rev}");
  });

  it("start in the saved language, else the browser's, else English", () => {
    expect(initialLocale("ko", "en-US")).toBe("ko");
    expect(initialLocale(null, "ko-KR")).toBe("ko");
    expect(initialLocale(null, "KO")).toBe("ko");
    expect(initialLocale(null, "fr-FR")).toBe("en");
    expect(initialLocale("zz", undefined)).toBe("en");
  });
});
