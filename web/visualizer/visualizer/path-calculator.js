// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * Path Calculator - Handles path computation, collision detection, boundary points
 */

class PathCalculator {
    constructor(visualizer) {
        this.visualizer = visualizer;

        // Label positioning constants
        this.COORDINATE_TOLERANCE = 1.0;      // Pixel tolerance for straight line detection
        this.LABEL_OFFSET_VERTICAL = 8;       // Vertical offset for bent paths
        this.LABEL_OFFSET_HORIZONTAL = 10;    // Horizontal offset for bent paths
        this.MIN_LABEL_DISTANCE = 20;         // Minimum distance from state edges

        // Self-loop path constants
        this.BOUNDARY_DIRECTION_OFFSET = 100; // Offset for boundary point direction calculation
    }

    /**
     * What one arrow covered by a label costs the placement stage, in the
     * same units as overlap area.
     *
     * ⚠ A weight, and an unvalidated one. It is set so that covering a
     * single arrow outranks a small overlap with another label but not a
     * large one, which is a judgement about what a reader minds more, not a
     * measurement. It is named rather than inlined so it can be argued with.
     */
    static get CROSSED_LINE_COST() { return 400; }

    /**
     * The lines this label carries, from the one place that decides them.
     *
     * Split from the markup below so the same list reaches the box
     * reservation in `label-metrics.js`. A second list built here would let
     * the drawn label and the space ELK reserved for it disagree about how
     * many lines there are.
     */
    getTransitionLabelLines(transition) {
        return buildLinkLabelLines(
            transition,
            (action) =>
                (typeof ActionFormatter !== 'undefined')
                    ? ActionFormatter.formatAction(action)
                    : { main: 'action', details: [] },
            // A link the reader has opened shows every alternative; the rest
            // show the cap and a count of what is behind it.
            transition.labelExpanded === true
        );
    }

    getTransitionLabelText(transition) {
        // Generate hierarchical HTML structure for better readability.
        // W3C SCXML 3.12 / 3.12.1 / 3.7 / 5.9.1: which lines exist is
        // decided by getTransitionLabelLines; this only dresses them.
        const parts = this.getTransitionLabelLines(transition).map(line => {
            switch (line.kind) {
                case 'eventless':
                    return `<div class="label-eventless">${line.text}</div>`;
                case 'event':
                    return line.wildcard
                        ? `<div class="label-event label-wildcard">${line.head}`
                            + `<span class="wildcard-hint">${line.hint}</span></div>`
                        : `<div class="label-event">${line.text}</div>`;
                case 'condition':
                    return `<div class="label-condition">${line.text}</div>`;
                case 'action':
                    return `<div class="label-action">${line.text}</div>`;
                case 'always':
                    return `<div class="label-always">${line.text}</div>`;
                case 'separator':
                    return `<div class="label-separator"></div>`;
                case 'more':
                    // Announces what the cap is hiding. Clickable, and the
                    // count is of TRANSITIONS: a reader told "+9 more" must
                    // not find eleven when they open it.
                    return `<div class="label-more" role="button" tabindex="0">${line.text}</div>`;
                default:
                    throw new Error(`path-calculator: unknown label line kind ${line.kind}`);
            }
        });

        // Build label classes with color variant
        let labelClasses = 'transition-label';
        if (transition.colorIndex !== null && transition.colorIndex !== undefined) {
            labelClasses += ` label-color-${transition.colorIndex}`;
        }

        // EVERY transition this arrow stands for, space separated, so a
        // lookup for one of them finds the merged arrow. The matchers use
        // `~=`, which is the attribute selector for a space-separated list;
        // an arrow standing for a single transition is the one-element case
        // and behaves exactly as the old exact match did.
        const ids = transition.transitionIds && transition.transitionIds.length
            ? transition.transitionIds
            : (transition.transitionId ? [transition.transitionId] : []);
        const transitionId = ids.length ? `data-transition-id="${ids.join(' ')}"` : '';

        return `<div class="${labelClasses}" ${transitionId}>${parts.join('')}</div>`;
    }

    /**
     * Helper: Check if two coordinates are close enough to be considered a straight line
     * @param {number} coord1 - First coordinate
     * @param {number} coord2 - Second coordinate
     * @returns {boolean} True if coordinates are within tolerance
     */
    _isStraightPath(coord1, coord2) {
        return Math.abs(coord1 - coord2) < this.COORDINATE_TOLERANCE;
    }

    /**
     * Helper: Calculate midpoint between two values with optional offset
     * @param {number} start - Start value
     * @param {number} end - End value
     * @param {number} offset - Optional offset to apply (default: 0)
     * @returns {number} Calculated midpoint
     */
    _calculateMidpoint(start, end, offset = 0) {
        if (start == null || end == null) {
            console.error('[MIDPOINT] Invalid coordinates:', { start, end });
            return 0;
        }
        return (start + end) / 2 + offset;
    }

    /**
     * Helper: Create label position object with logging
     * @param {number} x - X coordinate
     * @param {number} y - Y coordinate
     * @param {string} pathType - Description of path type for logging
     * @param {string} transitionId - Transition identifier
     * @returns {{x: number, y: number}} Position object
     */
    _createLabelPosition(x, y, pathType, transitionId) {
        const pos = { x, y };
        logger.debug(`[LABEL POS] ${transitionId}: ${pathType} → position(${x.toFixed(1)}, ${y.toFixed(1)})`);
        return pos;
    }

    /**
     * Helper: Ensure minimum distance from state edge
     * @param {number} position - Calculated position
     * @param {number} stateEdge - State edge position
     * @returns {number} Adjusted position
     */
    _ensureMinimumDistance(position, stateEdge) {
        const distance = Math.abs(position - stateEdge);
        if (distance < this.MIN_LABEL_DISTANCE) {
            const direction = position > stateEdge ? 1 : -1;
            return stateEdge + (this.MIN_LABEL_DISTANCE * direction);
        }
        return position;
    }

    /**
     * Is ELK's label coordinate in the same frame as the states it labels?
     *
     * The same containment hazard as [`elkSectionReachesItsNodes`]: a label
     * on a cross-hierarchy edge can come back in an ancestor's frame, and a
     * label read in the wrong frame lands in open space — where it is worse
     * than the midpoint it replaced, because it names a transition nowhere
     * near it.
     *
     * Asked as "does it sit within the span of the two states it belongs
     * to, with room to spare", which a correctly-framed label always does
     * and a mis-framed one does not.
     */
    elkLabelIsInFrame(link) {
        const source = this.visualizer.nodes.find(n => n.id === (link.visualSource || link.source));
        const target = this.visualizer.nodes.find(n => n.id === (link.visualTarget || link.target));
        if (!source || !target
            || !Number.isFinite(source.x) || !Number.isFinite(target.x)) {
            return false;
        }
        const margin = 200;
        const minX = Math.min(source.x - source.width / 2, target.x - target.width / 2) - margin;
        const maxX = Math.max(source.x + source.width / 2, target.x + target.width / 2) + margin;
        const minY = Math.min(source.y - source.height / 2, target.y - target.height / 2) - margin;
        const maxY = Math.max(source.y + source.height / 2, target.y + target.height / 2) + margin;
        const { x, y } = link.elkLabel;
        return x >= minX && x <= maxX && y >= minY && y <= maxY;
    }

    /**
     * Where a label goes, answered for the drawing as a whole.
     *
     * ⭐ This READS the decision [`placeTransitionLabels`] made; it does not
     * make one. Label placement is a STAGE, and a stage has one output that
     * every reader shares. `seedTransitionLabelPosition` below is the
     * starting guess that stage refines — kept reachable here so a label
     * still lands somewhere sensible if the stage has not run yet, and so a
     * label follows its line while the reader is still dragging.
     *
     * ⚠ It used to be the other way round: this function COMPUTED, per
     * label, from one line. That is why five transitions leaving one state
     * put five labels in one place — each was placed correctly with respect
     * to its own line, and nothing looked at the other four.
     */
    getTransitionLabelPosition(transition) {
        const custom = this.visualizer.getCustomLabelPosition(transition);
        if (custom) {
            return custom;
        }
        if (transition.labelAnchor) {
            const pos = this.anchorToPoint(transition, transition.labelAnchor);
            if (pos) {
                return pos;
            }
        }
        return this.seedTransitionLabelPosition(transition);
    }

    /**
     * Turn a place ON THE LINE into a place on the canvas, using the line as
     * it is drawn right now.
     *
     * ⭐ This is why the stage stores an anchor rather than a coordinate. A
     * coordinate is a fact about geometry at the moment it was computed, and
     * the geometry moves afterwards for reasons the stage cannot see — the
     * drag-end timer clearing `isDragging` makes every edge touching the
     * dragged state eligible for its ELK route again, and the lines change
     * shape 50ms after the last placement pass. Labels placed as coordinates
     * were left up to 469px from the arrows they name, each of them correct
     * about a line that no longer existed.
     *
     * An anchor cannot go stale that way: it says "two fifths along, on the
     * left", and whatever the line does, the label is still there.
     */
    anchorToPoint(link, anchor) {
        if (!anchor || !Number.isFinite(anchor.t)) {
            return null;
        }
        let d;
        try { d = this.getLinkPath(link); } catch (e) { return null; }
        if (!d || /NaN|undefined/.test(d)) {
            return null;
        }
        const pts = PathCalculator.pathPolyline(d);
        if (pts.length < 2) {
            return null;
        }
        const total = PathCalculator.polylineLength(pts);
        const p = PathCalculator.pointAlong(pts, anchor.t);
        if (!p) {
            return null;
        }
        const dir = PathCalculator.directionAt(pts, total * anchor.t);
        return {
            x: p.x + (-dir.y) * (anchor.side || 0) * (anchor.gap || 0),
            y: p.y + dir.x * (anchor.side || 0) * (anchor.gap || 0),
        };
    }

    seedTransitionLabelPosition(transition) {
        const transitionId = `${transition.source}→${transition.target}`;
        logger.debug(`[LABEL POS] Calculating position for ${transitionId}`);

        // Check for custom position first (user-dragged)
        const customPos = this.visualizer.getCustomLabelPosition(transition);
        if (customPos) {
            logger.debug(`[LABEL POS] ${transitionId}: Using custom position (${customPos.x.toFixed(1)}, ${customPos.y.toFixed(1)})`);
            return customPos;
        }

        // Then ELK's, if it placed this label and its route still stands.
        //
        // ELK positions a label against every other label and every other
        // edge; the midpoint rule below can only see the one line it is
        // given. That is why label-on-label and label-on-state collisions
        // were unmoved by seven different routings — nothing downstream of
        // the midpoint could move them. A user drag still wins, because a
        // reader who has placed a label has said where they want it.
        if (transition.elkLabel
            && Number.isFinite(transition.elkLabel.x)
            && Number.isFinite(transition.elkLabel.y)
            && this.elkLabelIsInFrame(transition)) {
            logger.debug(`[LABEL POS] ${transitionId}: Using ELK position`);
            return this._createLabelPosition(
                transition.elkLabel.x, transition.elkLabel.y, 'elk', transitionId);
        }

        // Use routing information to get actual path coordinates
        if (transition.routing && transition.routing.sourcePoint && transition.routing.targetPoint) {
            const start = transition.routing.sourcePoint;
            const end = transition.routing.targetPoint;
            const sourceEdge = transition.routing.sourceEdge;
            const targetEdge = transition.routing.targetEdge;

            const sx = start.x;
            const sy = start.y;
            const tx = end.x;
            const ty = end.y;

            logger.debug(`[LABEL POS] ${transitionId}: Has routing - source(${sx}, ${sy}) [${sourceEdge}] → target(${tx}, ${ty}) [${targetEdge}]`);

            const MIN_SEGMENT = PATH_CONSTANTS.MIN_SEGMENT_LENGTH;
            const sourceIsVertical = (sourceEdge === 'top' || sourceEdge === 'bottom');
            const targetIsVertical = (targetEdge === 'top' || targetEdge === 'bottom');

            // Calculate the middle segment of the path based on edge types
            if (sourceIsVertical && targetIsVertical) {
                // Both vertical edges: VHV path (vertical-horizontal-vertical)

                if (this._isStraightPath(sx, tx)) {
                    // Straight vertical transition: place label at visual midpoint
                    const midY = this._calculateMidpoint(sy, ty);
                    const adjustedY = this._ensureMinimumDistance(midY, sy);
                    return this._createLabelPosition(sx, adjustedY, 'Straight vertical', transitionId);
                } else {
                    // Bent VHV path: place label on horizontal segment
                    const cornerY = sourceEdge === 'top'
                        ? sy - MIN_SEGMENT
                        : sy + MIN_SEGMENT;
                    const labelX = this._calculateMidpoint(sx, tx);
                    const rawLabelY = cornerY - this.LABEL_OFFSET_VERTICAL;
                    const labelY = this._ensureMinimumDistance(rawLabelY, sy);
                    return this._createLabelPosition(labelX, labelY, 'Bent VHV path', transitionId);
                }
            } else if (!sourceIsVertical && !targetIsVertical) {
                // Both horizontal edges: HVH path (horizontal-vertical-horizontal)

                if (this._isStraightPath(sy, ty)) {
                    // Straight horizontal transition: place label at visual midpoint
                    const midX = this._calculateMidpoint(sx, tx);
                    const adjustedX = this._ensureMinimumDistance(midX, sx);
                    return this._createLabelPosition(adjustedX, sy, 'Straight horizontal', transitionId);
                } else {
                    // Bent HVH path: place label on vertical segment
                    const cornerX = sourceEdge === 'right'
                        ? sx + MIN_SEGMENT
                        : sx - MIN_SEGMENT;
                    const rawLabelX = cornerX + this.LABEL_OFFSET_HORIZONTAL;
                    const labelX = this._ensureMinimumDistance(rawLabelX, sx);
                    const labelY = this._calculateMidpoint(sy, ty);
                    return this._createLabelPosition(labelX, labelY, 'Bent HVH path', transitionId);
                }
            } else if (sourceIsVertical && !targetIsVertical) {
                // Source vertical, target horizontal: V→H path
                const cornerY = sourceEdge === 'top'
                    ? sy - MIN_SEGMENT
                    : sy + MIN_SEGMENT;
                const labelX = this._calculateMidpoint(sx, tx);
                const rawLabelY = cornerY - this.LABEL_OFFSET_VERTICAL;
                const labelY = this._ensureMinimumDistance(rawLabelY, sy);
                return this._createLabelPosition(labelX, labelY, 'V→H path', transitionId);
            } else {
                // Source horizontal, target vertical: H→V path
                const cornerX = sourceEdge === 'right'
                    ? sx + MIN_SEGMENT
                    : sx - MIN_SEGMENT;
                const rawLabelX = cornerX + this.LABEL_OFFSET_HORIZONTAL;
                const labelX = this._ensureMinimumDistance(rawLabelX, sx);
                const labelY = this._calculateMidpoint(sy, ty);
                return this._createLabelPosition(labelX, labelY, 'H→V path', transitionId);
            }
        }

        // Fallback to simple midpoint (no routing information)
        logger.debug(`[LABEL POS] ${transitionId}: No routing info - using fallback midpoint`);
        const sourceNode = this.visualizer.nodes.find(n => n.id === transition.source);
        const targetNode = this.visualizer.nodes.find(n => n.id === transition.target);

        if (!sourceNode || !targetNode) {
            logger.debug(`[LABEL POS] ${transitionId}: ERROR - Source or target node not found`);
            return { x: 0, y: 0 };
        }

        const labelX = this._calculateMidpoint(sourceNode.x, targetNode.x);
        const labelY = this._calculateMidpoint(sourceNode.y, targetNode.y, -this.LABEL_OFFSET_VERTICAL);
        return this._createLabelPosition(labelX, labelY, 'Fallback midpoint', transitionId);
    }

    // ---------------------------------------------------------------- stage

    /**
     * THE label placement stage: place every label, once, knowing the rest.
     *
     * ⭐ This exists because label placement had TWO implementations that
     * could not agree. On load ELK placed each label against every other
     * label and every other edge; after a gesture ELK's placement was thrown
     * out with its route, and what took over was a per-label midpoint rule
     * that can see exactly one line. So the drawing was readable until the
     * reader touched it, and then five `check` labels leaving one state
     * stacked into one spot — each of them correct about its own line.
     *
     * A stage has one output. Every reader goes through
     * [`getTransitionLabelPosition`], which returns what this wrote.
     *
     * ⚠ Two things are never moved, and both are decisions somebody already
     * made: a label the reader dragged by hand, and — while a drag is in
     * progress — every label, because a label that jumps to a new spot on
     * each pointer move is worse than one that is merely crowded. Under the
     * pointer, labels FOLLOW their lines; when the pointer is released, this
     * runs and places them.
     *
     * ⚠⚠ Edge label placement is NP-hard (Kakoulis and Tollis), and the
     * published overlap-removal work names our exact case as the one it does
     * not finish: labels on several edges between the same pair of nodes.
     * So this is a heuristic and says so. What makes it the right KIND of
     * heuristic is that it is global and deterministic — the same geometry
     * places labels the same way, and a label is only ever moved along the
     * line it names, so it never stops pointing at its own transition.
     */
    placeTransitionLabels(links, live = false) {
        const transitions = (links || []).filter(l => l && l.linkType === 'transition');
        if (!transitions.length) {
            return;
        }

        // Under the pointer, follow the line. See the note above.
        //
        // ⚠ The caller says whether this is a live frame; this does NOT ask
        // `isDragging`. It did, and the labels were never placed at all: the
        // drag-end handler clears `isDragging` inside a `setTimeout` — to
        // dodge a hover race that has nothing to do with labels — so at the
        // moment the gesture settles and the drawing is derived again, the
        // flag still says a drag is in progress. Every run took this branch,
        // dropped the positions, and nothing ran afterwards to replace them.
        // Measured in a browser: five labels stacked into one spot, ten
        // overlapping pairs, while the headless probe read zero.
        if (live) {
            // ⭐ Nothing to drop. An anchor says "two fifths along, on the
            // left", so a label follows its line for free while the reader
            // drags — and re-running the avoidance search on every pointer
            // move would make labels jump about, which is worse than being
            // briefly crowded. The search runs when the gesture settles.
            return;
        }

        // Deterministic order, so the same drawing places the same way.
        const ordered = [...transitions].sort((a, b) => String(a.id).localeCompare(String(b.id)));

        // Every drawn line, once, so a candidate can be asked whether it
        // covers somebody else's arrow. Built here rather than per candidate
        // because `getLinkPath` is not free and the geometry does not move
        // while this runs.
        this._drawnSegments = [];
        for (const link of ordered) {
            let d;
            try { d = this.getLinkPath(link); } catch (e) { continue; }
            if (!d || /NaN|undefined/.test(d)) continue;
            const pts = PathCalculator.pathPolyline(d);
            const segs = [];
            for (let i = 1; i < pts.length; i++) {
                segs.push({ a: pts[i - 1], b: pts[i] });
            }
            if (segs.length) this._drawnSegments.push({ id: link.id, segs });
        }

        const taken = [];
        for (const node of this.visualizer.nodes) {
            if (!Number.isFinite(node.x) || !Number.isFinite(node.y)) continue;
            if (node.children && node.children.length) continue;   // a container is not an obstacle
            taken.push({
                x1: node.x - node.width / 2, y1: node.y - node.height / 2,
                x2: node.x + node.width / 2, y2: node.y + node.height / 2,
            });
        }

        // A label the reader placed is fixed, and claims its area first.
        const pinned = [];
        const free = [];
        for (const link of ordered) {
            (this.visualizer.getCustomLabelPosition(link) ? pinned : free).push(link);
        }
        for (const link of pinned) {
            // A pinned label has no anchor: the reader chose a place on the
            // canvas, not a place on the line, and `getTransitionLabelPosition`
            // returns their choice before it looks at anchors at all.
            link.labelAnchor = null;
            const rect = this._labelRect(link, this.visualizer.getCustomLabelPosition(link));
            if (rect) taken.push(rect);
        }

        for (const link of free) {
            const chosen = this._chooseLabelSpot(link, taken);
            link.labelAnchor = chosen.pos.anchor || null;
            if (chosen.rect) taken.push(chosen.rect);
        }

        PathCalculator.syncLabelsToOriginals(transitions);
        this._drawnSegments = null;
    }

    /**
     * Carry the stage's decision back to the links it was computed for.
     *
     * ⚠ `getVisibleLinks` says "always recalculate" and returns a fresh
     * `{...link}` COPY on every call, so a decision written on one call's
     * copies is invisible to the next. `routing` has been synced back for
     * this reason since before this stage existed; label anchors need the
     * same treatment, and without it the placement is silently discarded
     * between the pass that computes it and the pass that draws it.
     *
     * ⭐ It also makes the decision inspectable. A copy nobody outside the
     * function can reach is a decision nobody can check, and twice in one
     * round a measurement read a second set of copies and reported that a
     * fix had done nothing.
     */
    static syncLabelsToOriginals(links) {
        for (const link of links) {
            if (!link.originalLink) continue;
            if (link.labelAnchor) {
                link.originalLink.labelAnchor = link.labelAnchor;
            } else {
                delete link.originalLink.labelAnchor;
            }
            if (link.labelPlacement) {
                link.originalLink.labelPlacement = link.labelPlacement;
            } else {
                delete link.originalLink.labelPlacement;
            }
        }
    }

    /** The reserved box for `link`, centred on `pos`. */
    _labelRect(link, pos) {
        const box = this.visualizer.layoutManager.labelBoxForLink(link);
        if (!box || !pos || !Number.isFinite(pos.x) || !Number.isFinite(pos.y)) {
            return null;
        }
        return {
            x1: pos.x - box.width / 2, y1: pos.y - box.height / 2,
            x2: pos.x + box.width / 2, y2: pos.y + box.height / 2,
        };
    }

    /**
     * The first spot along this link's own line that hits nothing, or the
     * least-bad one if every spot hits something.
     *
     * ⚠ Candidates come from the path the renderer will DRAW, parsed from
     * the same string, rather than from the routing that produced it. There
     * are two producers of routes and the point of this stage is not to
     * acquire a third opinion about where the line went.
     */
    _chooseLabelSpot(link, taken) {
        const seed = this.seedTransitionLabelPosition(link);
        const spots = this._spotsAlongPath(link);

        // ⚠ Spots on the drawn line FIRST, and the seed only when the line
        // yields none. The seed was tried first at one point and it is the
        // wrong thing to prefer: it is ELK's label position, which is only
        // meaningful while ELK's route is the route being drawn. After a
        // gesture the line has been re-routed and that position has not
        // moved, so a collision-free seed is collision-free out in open
        // space — measured in a browser at 100 to 341 units from the line
        // the label names. Zero overlaps and every label adrift is not an
        // improvement on some overlaps and every label attached.
        //
        // ⭐ Nothing is lost by demoting it. ELK's placement was valuable
        // because it avoided the other labels; this stage does that itself
        // now, and does it against the geometry actually on screen.
        const candidates = spots.length ? spots : [{ x: seed.x, y: seed.y, anchor: null }];

        // ⚠ The stage says why, the way `_createLabelPosition` does. A
        // placement that reports only a coordinate cannot be told apart
        // from one that never ran, and for one round it was not: the
        // numbers sat still and the reason was invisible.
        let best = null;
        let tried = 0;
        for (const pos of candidates) {
            const rect = this._labelRect(link, pos);
            if (!rect) continue;
            tried++;
            const cost = taken.reduce((sum, t) => sum + PathCalculator.overlapArea(rect, t), 0)
                + this._linesCrossing(rect, link) * PathCalculator.CROSSED_LINE_COST;
            if (cost === 0) {
                link.labelPlacement = { reason: 'clear', tried, spots: spots.length, residual: 0 };
                return { pos, rect };
            }
            if (!best || cost < best.cost) {
                best = { pos, rect, cost };
            }
        }
        if (best) {
            link.labelPlacement = {
                reason: 'least-bad', tried, spots: spots.length, residual: Math.round(best.cost),
            };
            return best;
        }
        link.labelPlacement = { reason: 'no-box', tried, spots: spots.length, residual: null };
        return { pos: seed, rect: this._labelRect(link, seed) };
    }

    /**
     * Points along the drawn path, nearest the middle first.
     *
     * ⚠ Stepped by LENGTH, not by fraction of the path. Fractions were the
     * first attempt and they do not separate anything: a tenth of a path is
     * a few pixels on a short edge and hundreds on a long one, so five
     * labels leaving one state moved 12 and 20 pixels apart while their
     * boxes were over a hundred wide, and all three pairs still overlapped.
     * A step in pixels is a step a label box can be compared against.
     *
     * ⭐ Nearest-first is what keeps the result sane without a rule about
     * direction: the search takes the closest spot to the middle that is
     * clear, so a label on a vertical trunk slides up or down and one on a
     * horizontal run slides sideways, with nothing here having to know
     * which it is on.
     */
    _spotsAlongPath(link) {
        let d;
        try { d = this.getLinkPath(link); } catch (e) { return []; }
        if (!d || /NaN|undefined/.test(d)) {
            return [];
        }
        const pts = PathCalculator.pathPolyline(d);
        if (pts.length < 2) {
            return [];
        }
        const STEP = 8;
        const MAX_SPOTS = 120;
        const total = PathCalculator.polylineLength(pts);
        if (!(total > 0)) {
            return [];
        }

        // ⚠ BESIDE the line, not on it. Putting the label centre on the path
        // attached it perfectly and made it unreadable: on a trunk shared by
        // five transitions a label sitting on the line covers the other four,
        // and the census went from 2 label-on-edge overlaps to 21. A label
        // belongs next to its line — which is what the offsets the old
        // per-label rule used were for, and what ELK does.
        const box = this.visualizer.layoutManager.labelBoxForLink(link);
        const gap = box ? (box.height / 2 + this.MIN_LABEL_DISTANCE / 2) : this.LABEL_OFFSET_VERTICAL;

        const half = Math.min(Math.floor(total / (2 * STEP)), MAX_SPOTS / 2);
        const spots = [];
        for (let k = 0; k <= half; k++) {
            for (const along of (k === 0 ? [0] : [-1, 1])) {
                const at = total / 2 + along * k * STEP;
                if (at < 0 || at > total) continue;
                const t = at / total;
                const p = PathCalculator.pointAlong(pts, t);
                if (!p) continue;
                const dir = PathCalculator.directionAt(pts, at);
                // Perpendicular to the local direction, both sides, nearer
                // side first so the choice stays deterministic.
                //
                // ⚠ Each candidate carries the ANCHOR it came from, not just
                // the point. The anchor is what gets stored, so the label
                // stays on the line when the line is re-derived.
                for (const side of [1, -1]) {
                    spots.push({
                        x: p.x + (-dir.y) * side * gap,
                        y: p.y + dir.x * side * gap,
                        anchor: { t, side, gap },
                    });
                }
            }
        }
        return spots;
    }

    /** Unit direction of the polyline at arc length `at`. */
    static directionAt(pts, at) {
        let acc = 0;
        for (let i = 1; i < pts.length; i++) {
            const dx = pts[i].x - pts[i - 1].x;
            const dy = pts[i].y - pts[i - 1].y;
            const len = Math.hypot(dx, dy);
            if (len === 0) continue;
            if (acc + len >= at || i === pts.length - 1) {
                return { x: dx / len, y: dy / len };
            }
            acc += len;
        }
        return { x: 1, y: 0 };
    }

    /**
     * How many OTHER transitions' lines pass through this rectangle.
     *
     * ⚠ Counted because the obstacle set without it is wrong in the case
     * that matters: labels and states alone leave a label free to sit on
     * top of the four other arrows sharing its trunk, which is precisely
     * where five transitions out of one state put them.
     */
    _linesCrossing(rect, link) {
        const segs = this._drawnSegments;
        if (!segs) {
            return 0;
        }
        let n = 0;
        for (const entry of segs) {
            if (entry.id === link.id) continue;
            for (const s of entry.segs) {
                if (PathCalculator.segmentHitsRect(s, rect)) { n++; break; }
            }
        }
        return n;
    }

    static segmentHitsRect(s, r) {
        // Trivial accept: an endpoint inside.
        const inside = (p) => p.x >= r.x1 && p.x <= r.x2 && p.y >= r.y1 && p.y <= r.y2;
        if (inside(s.a) || inside(s.b)) {
            return true;
        }
        // Liang-Barsky clip of the segment against the rectangle.
        let t0 = 0;
        let t1 = 1;
        const dx = s.b.x - s.a.x;
        const dy = s.b.y - s.a.y;
        const tests = [
            { p: -dx, q: s.a.x - r.x1 },
            { p: dx, q: r.x2 - s.a.x },
            { p: -dy, q: s.a.y - r.y1 },
            { p: dy, q: r.y2 - s.a.y },
        ];
        for (const { p, q } of tests) {
            if (p === 0) {
                if (q < 0) return false;
                continue;
            }
            const t = q / p;
            if (p < 0) {
                if (t > t1) return false;
                if (t > t0) t0 = t;
            } else {
                if (t < t0) return false;
                if (t < t1) t1 = t;
            }
        }
        return true;
    }

    static polylineLength(pts) {
        let total = 0;
        for (let i = 1; i < pts.length; i++) {
            total += Math.hypot(pts[i].x - pts[i - 1].x, pts[i].y - pts[i - 1].y);
        }
        return total;
    }

    static pathPolyline(d) {
        const n = (d.match(/-?\d+(?:\.\d+)?/g) || []).map(Number);
        const pts = [];
        for (let i = 0; i + 1 < n.length; i += 2) {
            pts.push({ x: n[i], y: n[i + 1] });
        }
        return pts;
    }

    /** The point `t` of the way along a polyline, by length. */
    static pointAlong(pts, t) {
        let total = 0;
        const runs = [];
        for (let i = 1; i < pts.length; i++) {
            const len = Math.hypot(pts[i].x - pts[i - 1].x, pts[i].y - pts[i - 1].y);
            runs.push(len);
            total += len;
        }
        if (!(total > 0)) {
            return pts[0] ? { x: pts[0].x, y: pts[0].y } : null;
        }
        let want = total * t;
        for (let i = 0; i < runs.length; i++) {
            if (want <= runs[i] || i === runs.length - 1) {
                const f = runs[i] > 0 ? want / runs[i] : 0;
                return {
                    x: pts[i].x + (pts[i + 1].x - pts[i].x) * f,
                    y: pts[i].y + (pts[i + 1].y - pts[i].y) * f,
                };
            }
            want -= runs[i];
        }
        return null;
    }

    static overlapArea(a, b) {
        const w = Math.min(a.x2, b.x2) - Math.max(a.x1, b.x1);
        const h = Math.min(a.y2, b.y2) - Math.max(a.y1, b.y1);
        return (w > 0 && h > 0) ? w * h : 0;
    }

    getNodeSide(node, toX, toY) {
        const cx = node.x || 0;
        const cy = node.y || 0;
        const dx = toX - cx;
        const dy = toY - cy;

        // Use angle to determine side
        const angle = Math.atan2(dy, dx) * 180 / Math.PI;

        // -45 to 45: right, 45 to 135: bottom, 135 to 180 or -180 to -135: left, -135 to -45: top
        if (angle >= -45 && angle < 45) return 'right';
        if (angle >= 45 && angle < 135) return 'bottom';
        if (angle >= 135 || angle < -135) return 'left';
        return 'top';
    }

    getNodeBoundaryPoint(node, fromX, fromY, link, isSource, connections) {
        const cx = node.x || 0;
        const cy = node.y || 0;

        // Direction vector from node center to target
        const dx = fromX - cx;
        const dy = fromY - cy;
        const distance = Math.sqrt(dx * dx + dy * dy);

        if (distance === 0) return { x: cx, y: cy };

        // Normalize direction
        const ndx = dx / distance;
        const ndy = dy / distance;

        // Get node dimensions based on type
        if (node.type === 'initial-pseudo') {
            // Circle with radius 10
            return {
                x: cx + ndx * 10,
                y: cy + ndy * 10
            };
        } else if (node.type === 'history') {
            // Circle with radius 20
            return {
                x: cx + ndx * 20,
                y: cy + ndy * 20
            };
        } else if (node.type === 'atomic' || node.type === 'final') {
            // Rectangle with smart snapping (use actual node dimensions)
            const halfWidth = (node.width || 60) / 2;
            const halfHeight = (node.height || 40) / 2;

            // Determine which side this connection is on
            const side = this.visualizer.getNodeSide(node, fromX, fromY);

            // Get snap position on this side
            let snapX = cx, snapY = cy;

            if (connections && connections[node.id]) {
                const sideConnections = connections[node.id][side];
                const index = sideConnections.findIndex(c =>
                    c.link.id === link.id && c.isSource === isSource
                );

                if (index >= 0 && sideConnections.length > 0) {
                    const count = sideConnections.length;
                    const position = (index + 1) / (count + 1); // Divide side into (count+1) segments

                    if (side === 'top') {
                        snapX = cx - halfWidth + (halfWidth * 2 * position);
                        snapY = cy - halfHeight;
                    } else if (side === 'bottom') {
                        snapX = cx - halfWidth + (halfWidth * 2 * position);
                        snapY = cy + halfHeight;
                    } else if (side === 'left') {
                        snapX = cx - halfWidth;
                        snapY = cy - halfHeight + (halfHeight * 2 * position);
                    } else if (side === 'right') {
                        snapX = cx + halfWidth;
                        snapY = cy - halfHeight + (halfHeight * 2 * position);
                    }

                    // Snap point is exactly on boundary
                    return { x: snapX, y: snapY };
                }
            }

            // Fallback: no snapping, use angle-based intersection
            const tx = Math.abs(ndx) > 0 ? halfWidth / Math.abs(ndx) : Infinity;
            const ty = Math.abs(ndy) > 0 ? halfHeight / Math.abs(ndy) : Infinity;
            const t = Math.min(tx, ty);

            return {
                x: cx + ndx * t,
                y: cy + ndy * t
            };
        } else if (SCXMLVisualizer.isCompoundOrParallel(node)) {
            // Use node's width and height
            const halfWidth = (node.width || 60) / 2;
            const halfHeight = (node.height || 40) / 2;

            const tx = Math.abs(ndx) > 0 ? halfWidth / Math.abs(ndx) : Infinity;
            const ty = Math.abs(ndy) > 0 ? halfHeight / Math.abs(ndy) : Infinity;
            const t = Math.min(tx, ty);

            return {
                x: cx + ndx * t,
                y: cy + ndy * t
            };
        }

        // Default: return center
        return { x: cx, y: cy };
    }

    getOrthogonalIncomingDirection(start, end) {
        const dx = Math.abs(end.x - start.x);
        const dy = Math.abs(end.y - start.y);

        // Already aligned - direct line
        if (dx < 1) {
            // Vertical line
            return end.y > start.y ? 'from-top' : 'from-bottom';
        }
        if (dy < 1) {
            // Horizontal line
            return end.x > start.x ? 'from-left' : 'from-right';
        }

        // Z-shaped path with midpoint
        const midY = (start.y + end.y) / 2;

        // Last segment is vertical: (end.x, midY) → (end.x, end.y)
        return end.y > midY ? 'from-top' : 'from-bottom';
    }

    getOrthogonalOutgoingDirection(start, end) {
        const dx = Math.abs(end.x - start.x);
        const dy = Math.abs(end.y - start.y);

        // Already aligned - direct line
        if (dx < 1) {
            // Vertical line
            return end.y > start.y ? 'to-bottom' : 'to-top';
        }
        if (dy < 1) {
            // Horizontal line
            return end.x > start.x ? 'to-right' : 'to-left';
        }

        // Z-shaped path with midpoint
        const midY = (start.y + end.y) / 2;

        // First segment is vertical: (start.x, start.y) → (start.x, midY)
        return midY > start.y ? 'to-bottom' : 'to-top';
    }

    getOrthogonalBoundaryPoint(node, direction, link = null, isSource = true, connections = null) {
        const cx = node.x || 0;
        const cy = node.y || 0;

        if (node.type === 'atomic' || node.type === 'final') {
            const halfWidth = PATH_CONSTANTS.INITIAL_NODE_HALF_WIDTH;
            const halfHeight = 20;

            // **PRIORITY: Use routing if available (don't let direction override it)**
            if (link && link.routing) {
                const optPoint = isSource ? link.routing.sourcePoint : link.routing.targetPoint;

                if (optPoint) {
                    // Use the optimized snap point directly, no fallback needed
                    return { x: optPoint.x, y: optPoint.y };
                }
            }

            // Map direction to side
            let side = null;
            if (direction === 'from-top' || direction === 'to-top') {
                side = 'top';
            } else if (direction === 'from-bottom' || direction === 'to-bottom') {
                side = 'bottom';
            } else if (direction === 'from-left' || direction === 'to-left') {
                side = 'left';
            } else if (direction === 'from-right' || direction === 'to-right') {
                side = 'right';
            }

            // Smart snapping: use layout optimizer to calculate optimal snap position
            if (side && link) {
                const snapResult = this.visualizer.layoutOptimizer.calculateSnapPosition(
                    node.id,
                    side,
                    link.id,
                    direction
                );

                if (snapResult) {
                    return { x: snapResult.x, y: snapResult.y };
                }

                // If blocked by initial transition, try alternative edges
                if (this.visualizer.layoutOptimizer.hasInitialTransitionOnEdge(node.id, side)) {
                    logger.debug(`[FALLBACK] ${node.id} ${side}: blocked, trying alternative edge`);

                    // Try alternative edges based on original direction
                    const alternatives = [];
                    if (side === 'top' || side === 'bottom') {
                        alternatives.push('left', 'right', side === 'top' ? 'bottom' : 'top');
                    } else {
                        alternatives.push('top', 'bottom', side === 'left' ? 'right' : 'left');
                    }

                    // Try each alternative
                    for (const altSide of alternatives) {
                        if (!this.visualizer.layoutOptimizer.hasInitialTransitionOnEdge(node.id, altSide)) {
                            logger.debug(`[FALLBACK] ${node.id}: using ${altSide} instead of ${side}`);

                            // Calculate proper snap position for alternative edge
                            const altDirection = isSource ? `to-${altSide}` : `from-${altSide}`;
                            const altSnapResult = this.visualizer.layoutOptimizer.calculateSnapPosition(
                                node.id,
                                altSide,
                                link.id,
                                altDirection
                            );

                            // **DO NOT MODIFY routing - it's read-only!**
                            // Return the alternative snap position without modifying routing

                            if (altSnapResult) {
                                return { x: altSnapResult.x, y: altSnapResult.y };
                            }

                            // Fallback to edge center if snap calculation fails
                            if (altSide === 'top') {
                                return { x: cx, y: cy - halfHeight };
                            } else if (altSide === 'bottom') {
                                return { x: cx, y: cy + halfHeight };
                            } else if (altSide === 'left') {
                                return { x: cx - halfWidth, y: cy };
                            } else if (altSide === 'right') {
                                return { x: cx + halfWidth, y: cy };
                            }
                        }
                    }
                }
            }

            // Fallback: no smart snapping, use edge center
            if (direction === 'from-top' || direction === 'to-top') {
                return { x: cx, y: cy - halfHeight };
            } else if (direction === 'from-bottom' || direction === 'to-bottom') {
                return { x: cx, y: cy + halfHeight };
            } else if (direction === 'from-left' || direction === 'to-left') {
                return { x: cx - halfWidth, y: cy };
            } else if (direction === 'from-right' || direction === 'to-right') {
                return { x: cx + halfWidth, y: cy };
            }
        } else if (node.type === 'initial-pseudo') {
            const radius = 10;
            if (direction === 'from-top' || direction === 'to-top') {
                return { x: cx, y: cy - radius };
            } else if (direction === 'from-bottom' || direction === 'to-bottom') {
                return { x: cx, y: cy + radius };
            } else if (direction === 'from-left' || direction === 'to-left') {
                return { x: cx - radius, y: cy };
            } else if (direction === 'from-right' || direction === 'to-right') {
                return { x: cx + radius, y: cy };
            }
        } else if (node.type === 'history') {
            const radius = 20;
            if (direction === 'from-top' || direction === 'to-top') {
                return { x: cx, y: cy - radius };
            } else if (direction === 'from-bottom' || direction === 'to-bottom') {
                return { x: cx, y: cy + radius };
            } else if (direction === 'from-left' || direction === 'to-left') {
                return { x: cx - radius, y: cy };
            } else if (direction === 'from-right' || direction === 'to-right') {
                return { x: cx + radius, y: cy };
            }
        }

        // Default: return center
        return { x: cx, y: cy };
    }

    getNodeBounds(node) {
        const cx = node.x || 0;
        const cy = node.y || 0;

        if (node.type === 'atomic' || node.type === 'final') {
            // Use actual node dimensions (not hardcoded 60x40)
            const halfWidth = (node.width || 60) / 2;
            const halfHeight = (node.height || 40) / 2;
            return {
                left: cx - halfWidth,
                right: cx + halfWidth,
                top: cy - halfHeight,
                bottom: cy + halfHeight
            };
        } else if (node.type === 'initial-pseudo') {
            return {
                left: cx - 10,
                right: cx + 10,
                top: cy - 10,
                bottom: cy + 10
            };
        } else if (node.type === 'history') {
            return {
                left: cx - 20,
                right: cx + 20,
                top: cy - 20,
                bottom: cy + 20
            };
        } else if (SCXMLVisualizer.isCompoundOrParallel(node)) {
            const halfWidth = (node.width || 60) / 2;
            const halfHeight = (node.height || 40) / 2;
            return {
                left: cx - halfWidth,
                right: cx + halfWidth,
                top: cy - halfHeight,
                bottom: cy + halfHeight
            };
        }

        return { left: cx, right: cx, top: cy, bottom: cy };
    }

    horizontalLineIntersectsNode(y, xStart, xEnd, node) {
        const bounds = this.visualizer.getNodeBounds(node);

        // Check if y is within node's vertical range
        if (y < bounds.top || y > bounds.bottom) {
            return false;
        }

        // Check if horizontal line segment overlaps with node's horizontal range
        const lineLeft = Math.min(xStart, xEnd);
        const lineRight = Math.max(xStart, xEnd);

        return !(lineRight < bounds.left || lineLeft > bounds.right);
    }

    verticalLineIntersectsNode(x, yStart, yEnd, node) {
        const bounds = this.visualizer.getNodeBounds(node);

        // Check if x is within node's horizontal bounds
        if (x < bounds.left || x > bounds.right) {
            return false;
        }

        // Check if vertical segment overlaps node's vertical bounds
        const lineTop = Math.min(yStart, yEnd);
        const lineBottom = Math.max(yStart, yEnd);

        return !(lineBottom < bounds.top || lineTop > bounds.bottom);
    }

    getObstacleNodes(sourceId, targetId) {
        return this.visualizer.nodes.filter(node => 
            node.id !== sourceId && 
            node.id !== targetId &&
            node.type !== 'initial'  // Initial state markers are small, ignore them
        );
    }

    findCollisionFreeY(sx, sy, tx, ty, obstacles) {
        const candidates = [];
        const margin = 15;

        // Strategy 1: Try midpoint
        const midY = (sy + ty) / 2;
        candidates.push(midY);

        // Strategy 2: Try routing above all obstacles
        const maxTop = Math.max(
            ...obstacles.map(node => this.visualizer.getNodeBounds(node).top),
            sy, ty
        );
        candidates.push(maxTop - margin);

        // Strategy 3: Try routing below all obstacles
        const minBottom = Math.min(
            ...obstacles.map(node => this.visualizer.getNodeBounds(node).bottom),
            sy, ty
        );
        candidates.push(minBottom + margin);

        // Strategy 4: Try routing at source height
        candidates.push(sy);

        // Strategy 5: Try routing at target height
        candidates.push(ty);

        // Test each candidate and find first that doesn't collide
        for (const candidateY of candidates) {
            let hasCollision = false;

            // Check horizontal segment collision
            for (const obstacle of obstacles) {
                if (this.visualizer.horizontalLineIntersectsNode(candidateY, sx, tx, obstacle)) {
                    hasCollision = true;
                    break;
                }
            }

            if (!hasCollision) {
                // Also verify vertical segments don't collide
                const verticalCollision = obstacles.some(obstacle =>
                    this.visualizer.verticalLineIntersectsNode(sx, sy, candidateY, obstacle) ||
                    this.visualizer.verticalLineIntersectsNode(tx, candidateY, ty, obstacle)
                );

                if (!verticalCollision) {
                    return candidateY;
                }
            }
        }

        // Fallback: return midpoint (better than nothing)
        return midY;
    }

    calculateLinkDirections(sourceNode, targetNode, link) {
        // **PRIORITY: If routing exists, calculate midY and store in routing**
        // Don't run collision avoidance again - optimizer already calculated optimal path
        if (link.routing && link.routing.sourceEdge && link.routing.targetEdge) {
            const sy = link.routing.sourcePoint.y;
            const ty = link.routing.targetPoint.y;
            const midY = (sy + ty) / 2;

            // Store midY in routing for z-path collision avoidance
            link.routing.midY = midY;
            return;
        }

        // **FALLBACK: routing should always exist after optimizer runs for transition/initial links**
        // Containment and delegation links don't have routing (hierarchical structure, not routing path)
        // Skip warning if either node lacks coordinates (expected when ELK doesn't layout nested hierarchies)
        const hasCoords = sourceNode && targetNode &&
                         sourceNode.x !== undefined && sourceNode.y !== undefined &&
                         targetNode.x !== undefined && targetNode.y !== undefined;

        if (link.linkType !== 'containment' && link.linkType !== 'delegation' && hasCoords) {
            logger.warn(`[CALC DIR WARNING] ${link.source}→${link.target}: No routing found! Optimizer should have run first.`);
        }
    }

    createOrthogonalPath(sourceNode, targetNode, link, connections) {
        // **W3C SCXML 5.9.2: Self-loop transitions (targetless/internal)**
        // When source === target, create fake routing to use existing orthogonal path logic
        if (sourceNode.id === targetNode.id && !link.routing) {
            const cx = sourceNode.x || 0;
            const cy = sourceNode.y || 0;

            // Use getNodeBoundaryPoint to get accurate snap point coordinates
            // Self-loop: right edge → bottom edge
            const sourcePoint = this.getNodeBoundaryPoint(
                sourceNode,
                cx + this.BOUNDARY_DIRECTION_OFFSET,  // Right direction
                cy,                                    // Same y level
                link,
                true,                                  // isSource
                connections
            );

            const targetPoint = this.getNodeBoundaryPoint(
                sourceNode,                            // Same node (self-loop)
                cx,                                    // Center x
                cy + this.BOUNDARY_DIRECTION_OFFSET,  // Bottom direction
                link,
                false,                                 // isTarget
                connections
            );

            // Create fake routing for self-loop
            // This makes the self-loop compatible with existing orthogonal path logic
            link.routing = {
                sourcePoint: sourcePoint,
                targetPoint: targetPoint,
                sourceEdge: 'right',
                targetEdge: 'bottom'
            };

            // Fall through to use existing orthogonal path logic below
        }

        // **OPTIMIZED SNAP POINTS: Use routing if available**
        if (link.routing) {
            const start = link.routing.sourcePoint;
            const end = link.routing.targetPoint;
            const sourceEdge = link.routing.sourceEdge;
            const targetEdge = link.routing.targetEdge;

            const sx = start.x;
            const sy = start.y;
            const tx = end.x;
            const ty = end.y;

            // Debug mode: log path coordinates
            if (this.visualizer.debugMode) {
                logger.debug(`[PATH DEBUG] ${link.source}→${link.target}: source=(${sx.toFixed(1)}, ${sy.toFixed(1)}), target=(${tx.toFixed(1)}, ${ty.toFixed(1)})`);
            }

            const dx = Math.abs(tx - sx);
            const dy = Math.abs(ty - sy);

            // Check if direct line (horizontal or vertical alignment)
            if (dx < 1 || dy < 1) {
                // Direct line
                return `M ${sx} ${sy} L ${tx} ${ty}`;
            }

            // Create orthogonal path based on edge directions with minimum segment lengths
            const sourceIsVertical = (sourceEdge === 'top' || sourceEdge === 'bottom');
            const targetIsVertical = (targetEdge === 'top' || targetEdge === 'bottom');
            const MIN_SEGMENT = PATH_CONSTANTS.MIN_SEGMENT_LENGTH;  // Minimum horizontal/vertical segment length

            if (sourceIsVertical && targetIsVertical) {
                // Both vertical edges: vertical-horizontal-vertical (5 points)
                // Ensure minimum vertical segments from source and target
                let y1;
                if (sourceEdge === 'top') {
                    y1 = sy - MIN_SEGMENT;
                } else { // bottom
                    y1 = sy + MIN_SEGMENT;
                }

                let y2;
                if (targetEdge === 'top') {
                    y2 = ty - MIN_SEGMENT;
                } else { // bottom
                    y2 = ty + MIN_SEGMENT;
                }

                // Path: start → vertical MIN_SEGMENT → horizontal to target x → vertical MIN_SEGMENT → end
                return `M ${sx} ${sy} L ${sx} ${y1} L ${tx} ${y1} L ${tx} ${y2} L ${tx} ${ty}`;
            } else if (!sourceIsVertical && !targetIsVertical) {
                // Both horizontal edges: horizontal-vertical-horizontal (5 points)
                // Ensure minimum horizontal segments from source and target
                let x1;
                if (sourceEdge === 'right') {
                    x1 = sx + MIN_SEGMENT;
                } else { // left
                    x1 = sx - MIN_SEGMENT;
                }

                let x2;
                if (targetEdge === 'right') {
                    x2 = tx + MIN_SEGMENT;
                } else { // left
                    x2 = tx - MIN_SEGMENT;
                }

                // Path: start → horizontal MIN_SEGMENT → vertical to target y → horizontal MIN_SEGMENT → end
                const path = `M ${sx} ${sy} L ${x1} ${sy} L ${x1} ${ty} L ${x2} ${ty} L ${tx} ${ty}`;
                if (this.visualizer.debugMode) {
                    logger.debug(`[PATH GEN] ${link.source}→${link.target} ${sourceEdge}→${targetEdge}: M(${sx.toFixed(1)},${sy.toFixed(1)}) →H(${x1.toFixed(1)},${sy.toFixed(1)}) →V(${x1.toFixed(1)},${ty.toFixed(1)}) →H(${x2.toFixed(1)},${ty.toFixed(1)}) →H(${tx.toFixed(1)},${ty.toFixed(1)})`);
                }
                return path;
            } else if (sourceIsVertical && !targetIsVertical) {
                // Source vertical, target horizontal: vertical-then-horizontal (5 points)
                // Ensure minimum segments
                let y1;
                if (sourceEdge === 'top') {
                    y1 = sy - MIN_SEGMENT;
                } else { // bottom
                    y1 = sy + MIN_SEGMENT;
                }

                let x2;
                if (targetEdge === 'right') {
                    x2 = tx + MIN_SEGMENT;
                } else { // left
                    x2 = tx - MIN_SEGMENT;
                }

                // Path: start → vertical MIN_SEGMENT → horizontal to x2 → horizontal MIN_SEGMENT to end
                return `M ${sx} ${sy} L ${sx} ${y1} L ${x2} ${y1} L ${x2} ${ty} L ${tx} ${ty}`;
            } else {
                // Source horizontal, target vertical: horizontal-then-vertical (5 points)
                // Ensure minimum segments
                let x1;
                if (sourceEdge === 'right') {
                    x1 = sx + MIN_SEGMENT;
                } else { // left
                    x1 = sx - MIN_SEGMENT;
                }

                let y2;
                if (targetEdge === 'top') {
                    y2 = ty - MIN_SEGMENT;
                } else { // bottom
                    y2 = ty + MIN_SEGMENT;
                }

                // Path: start → horizontal MIN_SEGMENT → vertical to y2 → vertical MIN_SEGMENT to end
                const path = `M ${sx} ${sy} L ${x1} ${sy} L ${x1} ${y2} L ${tx} ${y2} L ${tx} ${ty}`;
                if (this.visualizer.debugMode) {
                    logger.debug(`[PATH GEN] ${link.source}→${link.target} ${sourceEdge}→${targetEdge}: M(${sx.toFixed(1)},${sy.toFixed(1)}) →H(${x1.toFixed(1)},${sy.toFixed(1)}) →V(${x1.toFixed(1)},${y2.toFixed(1)}) →H(${tx.toFixed(1)},${y2.toFixed(1)}) →V(${tx.toFixed(1)},${ty.toFixed(1)})`);
                }
                return path;
            }
        }

        // **FALLBACK: routing should always exist after optimizer runs for transition/initial links**
        // Containment and delegation links don't have routing (hierarchical structure, not routing path)
        // Skip warning if either node lacks coordinates (expected when ELK doesn't layout nested hierarchies)
        const hasCoords = sourceNode && targetNode &&
                         sourceNode.x !== undefined && sourceNode.y !== undefined &&
                         targetNode.x !== undefined && targetNode.y !== undefined;

        if (link.linkType !== 'containment' && link.linkType !== 'delegation' && hasCoords) {
            logger.warn(`[PATH WARNING] ${link.source}→${link.target}: No routing found! Falling back to node centers.`);
        }

        // Draw direct line as emergency fallback
        const sx = sourceNode.x || 0;
        const sy = sourceNode.y || 0;
        const tx = targetNode.x || 0;
        const ty = targetNode.y || 0;
        return `M ${sx} ${sy} L ${tx} ${ty}`;
    }

    /**
     * Does this ELK route actually join the two states it belongs to?
     *
     * ⚠ ELK expresses an edge's coordinates relative to the node that
     * CONTAINS the edge, and with `hierarchyHandling: INCLUDE_CHILDREN` it
     * may hoist a cross-hierarchy edge to the root while leaving its
     * coordinates in the frame of the lowest common ancestor. Read as
     * absolute, such a route lands a long way from both endpoints —
     * measured on `ancestor_entry_is_not_default_entry`, two edges inside a
     * nested `<parallel>` drew 653-769px away from the states they connect,
     * as lines floating in open space.
     *
     * ⭐ So this ASKS rather than models. Reproducing ELK's containment
     * rules here would put a second copy of them in this file, free to
     * disagree with the version ELK actually used; checking that the route
     * arrives where it must cannot disagree with anything. A route that
     * fails falls back to the orthogonal path, which is what every edge
     * used before ELK's routing was kept at all.
     */
    elkSectionReachesItsNodes(section, sourceNode, targetNode) {
        if (!section || !section.startPoint || !section.endPoint) {
            return false;
        }
        const near = (point, node) => {
            if (!node || !Number.isFinite(node.x) || !Number.isFinite(node.y)) {
                return false;
            }
            // Generous: the point must be on or near the box, not exactly on
            // its border, because ELK attaches at a port inset from the edge
            // and the renderer rounds.
            const slack = this.MIN_LABEL_DISTANCE;
            const halfW = (node.width || 0) / 2 + slack;
            const halfH = (node.height || 0) / 2 + slack;
            return Math.abs(point.x - node.x) <= halfW && Math.abs(point.y - node.y) <= halfH;
        };
        return near(section.startPoint, sourceNode) && near(section.endPoint, targetNode);
    }

    /**
     * Does this link draw ELK's route, or the orthogonal fallback?
     *
     * ⭐ One place answers, because the census asks it too. Measured before
     * the frame correction landed, `ancestor_entry_is_not_default_entry`
     * drew EVERY edge with the fallback — four routes computed by ELK, four
     * rejected, zero used — and no number in the census said so, because
     * the census restated the branch's condition instead of asking it. A
     * copy of a condition is free to agree with the original right up until
     * the moment it matters.
     */
    usesELKRoute(link, sourceNode, targetNode) {
        if (!this.elkRouteIsApplicable(link, sourceNode, targetNode)) {
            return false;
        }
        const src = sourceNode || this.visualizer.nodes.find(n => n.id === (link.visualSource || link.source));
        const tgt = targetNode || this.visualizer.nodes.find(n => n.id === (link.visualTarget || link.target));
        return this.elkSectionReachesItsNodes(link.elkSections[0], src, tgt);
    }

    /**
     * Could this link draw ELK's route at all, before asking whether the
     * route it got is usable?
     *
     * ⚠ The two questions are separated so a count of dropped routes means
     * one thing. A self-loop is NOT a dropped route: a targetless or
     * internal transition (W3C SCXML 5.9.2) is drawn by the dedicated shape
     * in `createOrthogonalPath` — right edge out, bottom edge back — and
     * ELK's generic loop was never going to be used for it. Counting those
     * alongside routes the drawing threw away would leave the census with a
     * floor it can never reach, and a number that cannot reach zero stops
     * being read.
     */
    elkRouteIsApplicable(link, sourceNode, targetNode) {
        const src = sourceNode || this.visualizer.nodes.find(n => n.id === (link.visualSource || link.source));
        const tgt = targetNode || this.visualizer.nodes.find(n => n.id === (link.visualTarget || link.target));
        if (!src || !tgt || src.id === tgt.id) {
            return false;
        }
        if (src.isDragging || tgt.isDragging) {
            return false;
        }
        return !!(link.elkSections && link.elkSections.length > 0);
    }

    getLinkPath(link) {
        logger.debug(`[GET LINK PATH] Called for ${link.source}→${link.target}`);
        // Get source and target nodes (use visual redirect if available)
        const visualSourceId = link.visualSource || link.source;
        const visualTargetId = link.visualTarget || link.target;

        const sourceNode = this.visualizer.nodes.find(n => n.id === visualSourceId);
        const targetNode = this.visualizer.nodes.find(n => n.id === visualTargetId);

        if (!sourceNode || !targetNode) {
            logger.debug(`[GET LINK PATH] Source or target node not found (visualSource='${visualSourceId}', visualTarget='${visualTargetId}')`);
            return 'M 0 0';
        }

        // **TWO-PASS: No need for analyzeLinkConnections(), optimizer uses link.routing**
        const connections = null;

        // If either node is being dragged, use dynamic orthogonal path recalculation
        if (sourceNode.isDragging || targetNode.isDragging) {
            // Create ORTHOGONAL path with direction-aware boundary snapping
            return this.visualizer.createOrthogonalPath(sourceNode, targetNode, link, connections);
        }

        // Use ELK edge routing only if available (only during initial ELK layout)
        if (this.usesELKRoute(link, sourceNode, targetNode)) {
            const section = link.elkSections[0];

            // Calculate boundary points for start and end
            let startPoint, endPoint;

            if (section.bendPoints && section.bendPoints.length > 0) {
                // ELK's own attachment points, not a recomputation of them.
                //
                // ⚠ This branch used to call `getNodeBoundaryPoint` for each
                // end, and that returned NaN: the branch had been
                // unreachable since `applyELKLayout` began deleting
                // `elkSections` in the same function that wrote them, so
                // nothing exercised it and it rotted where no test could
                // see. `M NaN NaN` is what a browser silently declines to
                // draw.
                //
                // Recomputing was also the wrong thing to do. ELK chose
                // where each edge meets each node as part of routing it —
                // that is what keeps two edges entering one side apart —
                // and `section.startPoint` / `endPoint` are that choice.
                // Deriving a different point from the first bend puts the
                // line's end somewhere ELK did not plan for.
                startPoint = section.startPoint;
                endPoint = section.endPoint;

                let path = `M ${startPoint.x} ${startPoint.y}`;
                section.bendPoints.forEach(point => {
                    path += ` L ${point.x} ${point.y}`;
                });
                path += ` L ${endPoint.x} ${endPoint.y}`;

                return path;
            } else if (section.startPoint && section.endPoint) {
                // A straight run ELK routed: still its points, still not ours.
                return `M ${section.startPoint.x} ${section.startPoint.y}`
                    + ` L ${section.endPoint.x} ${section.endPoint.y}`;
            } else {
                // No bend points, direct line with boundary calculation
                const sx = sourceNode.x || 0;
                const sy = sourceNode.y || 0;
                const tx = targetNode.x || 0;
                const ty = targetNode.y || 0;

                startPoint = this.visualizer.getNodeBoundaryPoint(sourceNode, tx, ty, link, true, connections);
                endPoint = this.visualizer.getNodeBoundaryPoint(targetNode, sx, sy, link, false, connections);

                return `M ${startPoint.x} ${startPoint.y} L ${endPoint.x} ${endPoint.y}`;
            }
        }

        // Fallback to orthogonal path (after ELK routing is invalidated)
        // This ensures all paths use routing for consistent routing
        return this.visualizer.createOrthogonalPath(sourceNode, targetNode, link, connections);
    }
}
