// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/**
 * What a transition label SAYS, and how big the box holding it is.
 *
 * ## Why the box is computed rather than measured
 *
 * A layout engine cannot keep a label clear of an edge it was never told
 * about. ELK reserves space only for labels declared in its input, so the
 * size has to be known BEFORE layout runs — and the obvious way to know it,
 * rendering the label and reading `getBBox()`, has two defects that are not
 * obvious:
 *
 *   1. It needs a browser. Every check of the layout would then need one
 *      too, and the layout is the only thing here a test can actually diff.
 *   2. ⚠ It is not deterministic even WITH a browser. `visualizer.css` names
 *      a font STACK (`Monaco, Menlo, Ubuntu Mono, Consolas, monospace`) and
 *      embeds no webfont, so the same label measures differently on macOS,
 *      Windows and a CI container. A golden layout taken on one machine
 *      would be wrong on the next.
 *
 * So the direction is reversed: this module DECIDES the box, the layout
 * reserves exactly it, and the renderer sizes the label container to exactly
 * it. Producer and consumer cannot disagree, because there is one number and
 * both read it.
 *
 * ## Why an estimate is sound here, and where it would not be
 *
 * The label is monospace (`--font-mono`) with `white-space: nowrap` on every
 * line, so its width is a character count and its height is a line count —
 * no wrapping to predict, no proportional advances to sum. The only unknown
 * is the advance ratio of whichever font in the stack the viewer has, and
 * those are known and clustered: Monaco and Liberation Mono 0.600, Menlo and
 * DejaVu Sans Mono 0.602, Consolas 0.550. [`ADVANCE_RATIO`] sits above all of
 * them, so the box is never too small — it is at worst slightly generous, and
 * a generous box costs whitespace where a tight one costs a collision.
 *
 * ⚠ That soundness argument depends on the label staying monospace and
 * `nowrap`. If a label ever wraps, or takes a proportional font, this file
 * stops being an estimate and becomes a guess.
 */

/**
 * Character advance as a fraction of font size, for the monospace stack.
 *
 * Above every candidate in the stack (the measured ratios are in the header)
 * so the reserved box is an upper bound rather than an average. Deliberately
 * NOT tuned to one font: the viewer's font is not knowable here, and a box
 * that fits the narrowest one collides on the widest.
 */
const ADVANCE_RATIO = 0.62;

/** CSS `line-height` on `.transition-label`. */
const LINE_HEIGHT_RATIO = 1.3;

/**
 * The box `.transition-label` adds around its lines: `padding: 3px 5px` plus
 * a 1px border on each side.
 *
 * ⚠ These mirror `visualizer.css` and nothing makes them follow it. The pair
 * is asserted by the label-metrics probe, which reads the stylesheet — a
 * mirrored constant that drifts is how a reserved box silently stops matching
 * the thing it reserves for.
 */
const LABEL_PADDING_X = 5;
const LABEL_PADDING_Y = 3;
const LABEL_BORDER = 1;

/**
 * Per-line geometry, by the kind of line. `font` is the CSS `font-size`,
 * `indent` the horizontal space the line's own rules add, `extraY` its own
 * vertical padding, and `marginBottom` the gap to the next line.
 */
const LINE_STYLES = {
    eventless: { font: 11, indent: 3 + 3, extraY: 1 + 1, marginBottom: 1 },
    event: { font: 12, indent: 0, extraY: 0, marginBottom: 1 },
    condition: { font: 11, indent: 0, extraY: 1 + 1, marginBottom: 1 },
    action: { font: 11, indent: 10, extraY: 0, marginBottom: 1 },
    always: { font: 11, indent: 0, extraY: 0, marginBottom: 0 },
};

/**
 * How many monospace cells a code point occupies.
 *
 * Two for the ranges a monospace font draws double-width (CJK, Hangul, the
 * fullwidth forms) and for pictographs, which no monospace font has a cell
 * for and which fall back to an emoji face roughly two cells wide. Zero for
 * combining marks, which advance nothing.
 *
 * ⚠ Deliberately coarse. It decides a reservation, not a rendering, and the
 * cost of being one cell generous is whitespace.
 */
function cellsForCodePoint(cp) {
    // Combining marks advance nothing.
    if ((cp >= 0x0300 && cp <= 0x036f) || (cp >= 0x20d0 && cp <= 0x20ff)) {
        return 0;
    }
    const wide =
        (cp >= 0x1100 && cp <= 0x115f) ||     // Hangul Jamo
        (cp >= 0x2e80 && cp <= 0xa4cf) ||     // CJK radicals .. Yi
        (cp >= 0xac00 && cp <= 0xd7a3) ||     // Hangul syllables
        (cp >= 0xf900 && cp <= 0xfaff) ||     // CJK compatibility ideographs
        (cp >= 0xfe30 && cp <= 0xfe6f) ||     // CJK compatibility forms
        (cp >= 0xff00 && cp <= 0xff60) ||     // Fullwidth forms
        (cp >= 0xffe0 && cp <= 0xffe6) ||
        (cp >= 0x1f300 && cp <= 0x1faff) ||   // Pictographs
        (cp >= 0x2600 && cp <= 0x27bf);       // Misc symbols and dingbats
    return wide ? 2 : 1;
}

/** Width of one line of monospace text, in CSS pixels. */
function textWidth(text, fontSize) {
    let cells = 0;
    for (const ch of String(text)) {
        cells += cellsForCodePoint(ch.codePointAt(0));
    }
    return cells * ADVANCE_RATIO * fontSize;
}

/**
 * What the label says, as lines — the ONE place that decides it.
 *
 * ⭐ Both consumers come through here: the renderer turns these into the
 * label's HTML, and [`measureTransitionLabel`] turns them into its box. A
 * second list built anywhere else would let the drawn label and the reserved
 * space disagree about how many lines there are, which is the disagreement
 * this whole file exists to make impossible.
 *
 * Returns `[{kind, text, ...}]` in the order they are drawn. `kind` indexes
 * [`LINE_STYLES`], so a kind added here without a style is a crash rather
 * than a silently unmeasured line.
 */
function buildTransitionLabelLines(transition, formatAction) {
    const lines = [];

    // W3C SCXML 3.12: an eventless transition is announced before anything
    // else, and takes the place of the event name rather than joining it.
    if (transition.eventless === true) {
        lines.push({ kind: 'eventless', text: '⚡ eventless' });
    } else if (transition.event) {
        // W3C SCXML 5.9.1: the wildcard is spelled out, since a bare `*`
        // reads as a missing value rather than as "matches all".
        if (transition.event === '*') {
            // `head` and `hint` are the same string split where the drawn
            // label styles it differently. `text` stays whole, because the
            // box has to hold both parts whatever markup separates them.
            lines.push({
                kind: 'event',
                text: '★ * (wildcard)',
                wildcard: true,
                head: '★ * ',
                hint: '(wildcard)',
            });
        } else {
            lines.push({ kind: 'event', text: transition.event });
        }
    }

    // W3C SCXML 3.12.1: the guard.
    if (transition.cond) {
        lines.push({ kind: 'condition', text: `\u{1f50d} [${transition.cond}]` });
    }

    // W3C SCXML 3.7: the executable content, one line each.
    if (transition.actions && transition.actions.length > 0) {
        for (const action of transition.actions) {
            const formatted = formatAction ? formatAction(action) : { main: 'action' };
            lines.push({ kind: 'action', text: `↳ ${formatted.main}`, action });
        }
    }

    // A transition with no event, no guard and no actions still has to say
    // something, or its arrow reads as unlabelled rather than unconditional.
    if (lines.length === 0) {
        lines.push({ kind: 'always', text: '(always)' });
    }

    return lines;
}

/**
 * The box the label needs, in CSS pixels.
 *
 * The number ELK is given and the number the renderer sizes the container
 * to. Integral, because a fractional reservation buys nothing and makes two
 * runs on different platforms differ in the last place.
 */
function measureTransitionLabel(lines) {
    let widest = 0;
    let height = 0;

    lines.forEach((line, index) => {
        const style = LINE_STYLES[line.kind];
        if (!style) {
            throw new Error(`label-metrics: no style for line kind ${line.kind}`);
        }
        widest = Math.max(widest, textWidth(line.text, style.font) + style.indent);
        height += style.font * LINE_HEIGHT_RATIO + style.extraY;
        if (index < lines.length - 1) {
            height += style.marginBottom;
        }
    });

    return {
        width: Math.ceil(widest + 2 * LABEL_PADDING_X + 2 * LABEL_BORDER),
        height: Math.ceil(height + 2 * LABEL_PADDING_Y + 2 * LABEL_BORDER),
    };
}

/**
 * Lines and box in one call, for a caller holding a transition.
 *
 * ⚠ Returns a box for EVERY transition, including one whose only line is
 * `(always)`. An unlabelled edge is not a thing this diagram draws, so a
 * zero box would reserve nothing for something that is nonetheless painted.
 */
function transitionLabelBox(transition, formatAction) {
    const lines = buildTransitionLabelLines(transition, formatAction);
    return { lines, ...measureTransitionLabel(lines) };
}

if (typeof module !== 'undefined' && module.exports) {
    module.exports = {
        ADVANCE_RATIO,
        LINE_HEIGHT_RATIO,
        LABEL_PADDING_X,
        LABEL_PADDING_Y,
        LABEL_BORDER,
        LINE_STYLES,
        cellsForCodePoint,
        textWidth,
        buildTransitionLabelLines,
        measureTransitionLabel,
        transitionLabelBox,
    };
}
