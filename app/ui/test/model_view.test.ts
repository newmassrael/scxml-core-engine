// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The pure facts of the model panel: what counts as SCE not drawing a model, and
// how a drawing becomes something an image can load.

import { describe, expect, it } from "vitest";

import { CommandFailure } from "../src/ipc";
import { DEFAULT_ZOOM, drawFailureOf, sheetName, svgAddress, zoomFrom, ZOOMS } from "../src/model_view";

describe("a failure to draw", () => {
  it("is SCE's refusal, with the product's code when it gave one", () => {
    const refused = new CommandFailure("sce-refused", "SCE refused the model (x): too big", {
      code: "cli/diagram-does-not-fit",
    });
    expect(drawFailureOf(refused)).toEqual({
      kind: "sce-refused",
      message: "SCE refused the model (x): too big",
      code: "cli/diagram-does-not-fit",
    });
  });

  it("is each of the other ways SCE does not draw: absent, too slow, or failing unexplained", () => {
    for (const kind of ["sce-unavailable", "sce-timeout", "sce-failed"]) {
      expect(drawFailureOf(new CommandFailure(kind, "no"))?.kind).toBe(kind);
    }
    // Without a code in its detail, there is no code to show.
    expect(drawFailureOf(new CommandFailure("sce-timeout", "slow", { seconds: 30 }))?.code).toBeNull();
    expect(drawFailureOf(new CommandFailure("sce-refused", "no", null))?.code).toBeNull();
  });

  it("is not any other failure, which the screen reports the way it reports those everywhere", () => {
    for (const kind of ["transport", "unauthorized", "not-found", "corrupt", "conflict"]) {
      expect(drawFailureOf(new CommandFailure(kind, "no")), kind).toBeNull();
    }
    expect(drawFailureOf(new Error("sce-refused"))).toBeNull();
    expect(drawFailureOf("sce-refused")).toBeNull();
  });
});

describe("a drawing as an address", () => {
  it("is the SVG, percent-encoded, and reads back as the same text", () => {
    const svg = '<svg xmlns="http://www.w3.org/2000/svg"><text>Ω ≥ 100% & <b></text></svg>\n';
    const address = svgAddress(svg);
    expect(address.startsWith("data:image/svg+xml;charset=utf-8,")).toBe(true);
    expect(address).not.toMatch(/[<>"\s]/);
    expect(decodeURIComponent(address.slice(address.indexOf(",") + 1))).toBe(svg);
  });
});

describe("the size a drawing is shown at", () => {
  it("is one of the sizes the screen offers, and the default is among them and larger than SCE set it", () => {
    expect(ZOOMS).toContain(DEFAULT_ZOOM);
    expect(DEFAULT_ZOOM).toBeGreaterThan(1);
    for (const zoom of ZOOMS) expect(zoomFrom(String(zoom))).toBe(zoom);
  });

  it("falls back to the default for anything kept that is not one of them", () => {
    for (const kept of [null, "", "abc", "0", "-2", "1.7", "NaN", "Infinity", "1e1"]) {
      expect(zoomFrom(kept), String(kept)).toBe(DEFAULT_ZOOM);
    }
  });
});

describe("a sheet's name", () => {
  it("is the file the product wrote, without its extension", () => {
    expect(sheetName("fields-1.svg")).toBe("fields-1");
    expect(sheetName("layout.SVG")).toBe("layout");
    expect(sheetName("no-extension")).toBe("no-extension");
  });
});
