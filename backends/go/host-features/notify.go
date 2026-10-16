// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package hostfeatures

import "strconv"

// notification is one notification the host holds. What was posted is held as it
// was given: the host shows text as written.
type notification struct{ id, title, body string }

// notifications is the platform's tray, as two lists.
type notifications struct {
	permission   string
	userDecision string
	// waiting are kept to be shown at their instant, in the order posted.
	waiting []notification
	// shown are handed to the platform, in the order posted.
	shown []notification
}

func (n *notifications) ids(list []notification) []string {
	out := []string{}
	for _, item := range list {
		out = append(out, item.id)
	}
	return out
}

// withdraw removes what id names from both lists; whether it named anything.
func (n *notifications) withdraw(id string) bool {
	removed := false
	keep := func(list []notification) []notification {
		kept := list[:0]
		for _, item := range list {
			if item.id == id {
				removed = true
			} else {
				kept = append(kept, item)
			}
		}
		return kept
	}
	n.waiting = keep(n.waiting)
	n.shown = keep(n.shown)
	return removed
}

func (h *ReferenceHost) notifyPermission() Outcome {
	return record(map[string]any{"state": h.notify.permission})
}

func (h *ReferenceHost) notifyRequestPermission() Outcome {
	if h.notify.permission == "undetermined" {
		// A host with no user to ask answers denied.
		if h.notify.userDecision == "granted" {
			h.notify.permission = "granted"
		} else {
			h.notify.permission = "denied"
		}
	}
	return h.notifyPermission()
}

func (h *ReferenceHost) notifyPost(id, title, body, atMs string) Outcome {
	if h.notify.permission != "granted" {
		return record(map[string]any{"status": "denied"})
	}
	at, err := strconv.ParseInt(atMs, 10, 64)
	if err != nil {
		at = 0
	}
	reading, ok := h.clock.Read()
	if !ok {
		return failure("sce-host-notify-post", "unavailable")
	}
	h.notify.withdraw(id)
	item := notification{id: id, title: title, body: body}
	if at > reading.UTCMs {
		h.notify.waiting = append(h.notify.waiting, item)
		return record(map[string]any{"status": "scheduled"})
	}
	h.notify.shown = append(h.notify.shown, item)
	return record(map[string]any{"status": "posted"})
}

func (h *ReferenceHost) notifyCancel(id string) Outcome {
	return record(map[string]any{"existed": h.notify.withdraw(id)})
}
