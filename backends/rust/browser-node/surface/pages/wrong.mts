// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A page that uses the surface of `static_optional` wrong (docs/adr/0117, D7): it must not
// compile. Each line marked as wrong is one use a page could get wrong, and TypeScript must
// report an error on it and on no other line, so a declaration that let one through, or that
// refused this page for another reason, fails the check (`../check.mjs`). Each wrong line
// differs from a line of `right.mts` in the one place it names.

import { bind, type MachinePort, type StateId } from "./static_optional.mjs";

declare const port: MachinePort;
const surface = bind(port);

void surface.raise({ name: "due.sett", data: { id: 1, due: null, title: null, status: null } }); // wrong: an event the machine does not take
void surface.raise({ name: "error.execution" }); // wrong: an event the platform raises, not a page
void surface.raise({ name: "due.set" }); // wrong: an event that carries data, raised without it
void surface.raise({ name: "due.clear", data: {} }); // wrong: an event that carries none, raised with some
void surface.raise({ name: "due.set", data: { id: "1", due: null, title: null, status: null } }); // wrong: a number written as text
void surface.raise({ name: "due.set", data: { id: 1, title: null, status: null } }); // wrong: a field the payload requires, left out
void surface.raise({ name: "due.set", data: { id: 1, due: null, title: null, status: null, extra: 1 } }); // wrong: a field the payload does not have
void surface.raise({ name: "due.set", data: { id: 1, due: null, title: null, status: "closed" } }); // wrong: a value the enum does not declare
void surface.raise({ name: "due.set", data: { id: 1, due: 5, title: null, status: null } }); // wrong: a 64-bit integer as a number, not the text of its digits
void surface.raise({ name: "due.sent", data: { id: 1, due: null, title: null, status: null } }); // wrong: the payload of another event
surface.subscribe((snapshot) => { const days: number = snapshot.variables.days; void days; }); // wrong: a 64-bit integer read as a number
surface.subscribe((snapshot) => { void snapshot.variables.huge; }); // wrong: a variable the machine keeps to itself
const waiting: StateId = "waiting"; // wrong: a state the machine does not have
void waiting;
