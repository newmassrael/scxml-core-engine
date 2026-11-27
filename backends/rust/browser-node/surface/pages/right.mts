// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A page that uses the surface of `static_optional` right (docs/adr/0117, D7): it must
// compile, with every check TypeScript makes under `strict`. Its twin, `wrong.mts`, must not.

import {
  bind,
  EVENTS,
  MACHINE,
  SurfaceRefused,
  type Event,
  type EventName,
  type MachinePort,
  type Snapshot,
  type StateId,
} from "./static_optional.mjs";

const port: MachinePort = {
  raise: (name: EventName, data: string | null) => {
    void name;
    void data;
  },
  subscribe: (listener: (snapshot: string) => void) => {
    void listener;
    return () => {};
  },
};
const surface = bind(port);

const raised: Promise<void>[] = [
  surface.raise({ name: "due.set", data: { id: 7, due: "1700000000000", title: "soon", status: "in_progress" } }),
  surface.raise({ name: "due.set", data: { id: 0, due: null, title: null, status: null } }),
  surface.raise({ name: "due.sent", data: { due: null, title: "seen", status: "done" } }),
  surface.raise({ name: "due.clear" }),
  surface.raise({ name: "due.read" }),
  surface.raise({ name: "due.fallback" }),
  surface.raise({ name: "relay" }),
  surface.raise({ name: "finish" }),
];

const held: Event = { name: "relay" };
const names: readonly EventName[] = EVENTS;
const machine: "static_optional" = MACHINE;

const stop = surface.subscribe((snapshot: Snapshot) => {
  const where: readonly StateId[] = snapshot.configuration;
  const tracking: StateId = "tracking";
  const due: string | null = snapshot.variables.due;
  const status: "open" | "in_progress" | "done" | null = snapshot.variables.status;
  const plain: "open" | "in_progress" | "done" = snapshot.variables.plain;
  const id: number = snapshot.variables.item.id;
  const ids: number[] = snapshot.variables.items.map((item) => item.id);
  const days: string = snapshot.variables.days;
  const late: boolean = snapshot.variables.late;
  const truncated: boolean = snapshot.truncated;
  void [where, tracking, due, status, plain, id, ids, days, late, truncated];
});
stop();

try {
  void surface.raise(held);
} catch (error) {
  if (error instanceof SurfaceRefused) {
    const at: string = error.path;
    const which: string | null = error.event;
    void [at, which];
  }
}

void [raised, names, machine];
