// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * @file sce_hud.c
 * @brief In-game HUD for SCE systems (combo counter, berserk overlay)
 *
 * Renders combo counter text and berserk screen tint directly into
 * DOOM's framebuffer using the engine's patch/font rendering system.
 *
 * All state is read directly from SCE APIs each frame (Pull pattern)
 * to stay perfectly synchronized with the state machines.
 * No duplicate state is cached — no Push callbacks needed.
 */

#include "sce_hud.h"
#include "sce_doom_hooks.h"
#include "sce_wrapper.h"

#include "doomdef.h"
#include "hu_stuff.h"
#include "i_video.h"
#include "v_video.h"
#include "i_swap.h"
#include "d_loop.h"

#include <stdio.h>
#include <string.h>

/* M_WriteText / M_StringWidth are defined in m_menu.c but not exported in m_menu.h */
extern void M_WriteText(int x, int y, char *string);
extern int M_StringWidth(char *string);

/* External: hu_font loaded by HU_Init */
extern patch_t *hu_font[HU_FONTSIZE];

/* Palette color indices for drawing primitives */
#define COLOR_ORANGE   215
#define COLOR_RED      176
#define COLOR_BLACK    0
#define COLOR_GRAY     96

/* Pulse periods in tics (35 tics/sec) */
#define PULSE_SLOW     35    /* 1.0s period - berserk intensity 1-3 */
#define PULSE_MID      21    /* 0.6s period - berserk intensity 4-6 */
#define PULSE_FAST     10    /* 0.3s period - berserk intensity 7+ */
#define COMBO_PULSE_LEN 10   /* 0.3s combo hit pulse */

/* Track when last combo kill happened for pulse animation */
static int hud_last_combo_count = 0;
static int hud_combo_pulse_tic = 0;

/* ============================================
 * Interface functions (Push callbacks are now no-ops)
 * ============================================ */

void SCE_HUD_Init(void) {
    hud_last_combo_count = 0;
    hud_combo_pulse_tic = 0;
}

void SCE_HUD_SetCombo(int count, boolean active) {
    (void)active;
    /* Track kill moments for combo pulse animation */
    if (count > hud_last_combo_count && count > 0) {
        hud_combo_pulse_tic = gametic;
    }
    hud_last_combo_count = count;
    if (count == 0) {
        hud_combo_pulse_tic = 0;
        hud_last_combo_count = 0;
    }
}

void SCE_HUD_SetComboTimer(double remaining_ms, double total_ms) {
    /* No-op: timer is now read via sce_combo_timer_get() each frame */
    (void)remaining_ms;
    (void)total_ms;
}

void SCE_HUD_SetBerserk(int intensity, boolean active) {
    (void)intensity;
    (void)active;
}

int SCE_HUD_GetBerserkPalette(void) {
    if (!SCE_BerserkIsActive())
        return 0;

    int intensity = SCE_BerserkGetIntensity();
    if (intensity <= 0) return 0;

    /* Determine pulse period based on intensity */
    int period;
    if (intensity <= 3) period = PULSE_SLOW;
    else if (intensity <= 6) period = PULSE_MID;
    else period = PULSE_FAST;

    /* Pulsing: alternate between two palette levels using gametic */
    int phase = gametic % period;
    boolean pulse_high = (phase < period / 2);

    if (intensity <= 3) return pulse_high ? 2 : 1;
    if (intensity <= 6) return pulse_high ? 3 : 2;
    return pulse_high ? 4 : 3;
}

/* ============================================
 * Drawing Helpers
 * ============================================ */

static void SCE_HUD_DrawTextCentered(int center_x, int y, const char *text) {
    int w = M_StringWidth((char *)text);
    int x = center_x - w / 2;
    if (x < 0) x = 0;
    M_WriteText(x, y, (char *)text);
}

static void SCE_HUD_DrawBar(int x, int y, int w, int h, int fill_w, int bg_color, int fill_color) {
    if (x < 0) x = 0;
    if (y < 0) y = 0;
    if (x + w > SCREENWIDTH) w = SCREENWIDTH - x;
    if (y + h > SCREENHEIGHT) h = SCREENHEIGHT - y;
    if (w <= 0 || h <= 0) return;

    V_DrawFilledBox(x, y, w, h, bg_color);
    if (fill_w > w) fill_w = w;
    if (fill_w > 0) {
        V_DrawFilledBox(x, y, fill_w, h, fill_color);
    }
    V_DrawBox(x, y, w, h, COLOR_BLACK);
}

/* ============================================
 * Main HUD Drawer — 100% Pull from SCE APIs
 * ============================================ */

void SCE_HUD_Drawer(void) {
    char text[64];
    int center_x = SCREENWIDTH / 2;
    boolean show_anything = false;

    /* Read all state directly from SCE APIs each frame */
    int combo_count = SCE_ComboGetCount();
    boolean combo_active = SCE_ComboIsActive();
    boolean berserk_active = SCE_BerserkIsActive();
    int berserk_intensity = SCE_BerserkGetIntensity();

    double timer_remaining = -1;
    double timer_total = 0;
    sce_combo_timer_get(&timer_remaining, &timer_total);
    boolean timer_active = (timer_remaining > 0 && timer_total > 0);

    /* Match WASM behavior: only show at count > 1 */
    boolean show_hud = (combo_count > 1 && (combo_active || timer_active));

    /* Combo hit pulse: check if within pulse window */
    boolean in_combo_pulse = (hud_combo_pulse_tic > 0 &&
                              (gametic - hud_combo_pulse_tic) < COMBO_PULSE_LEN);

    /* === Combo Counter Display === */
    if (show_hud) {
        if (berserk_active) {
            int y_pos = 20;

            /* Berserk text pulse: toggle visibility based on gametic */
            int period;
            if (berserk_intensity <= 3) period = PULSE_SLOW;
            else if (berserk_intensity <= 6) period = PULSE_MID;
            else period = PULSE_FAST;

            boolean text_visible = ((gametic % period) < (period * 7 / 10));

            if (text_visible) {
                snprintf(text, sizeof(text), "BERSERK! X%d", combo_count);
                SCE_HUD_DrawTextCentered(center_x, y_pos, text);
            }
        } else {
            int y_pos = 24;

            /* Combo pulse: flash on kill */
            if (!in_combo_pulse || (gametic % 2 == 0)) {
                snprintf(text, sizeof(text), "%dX COMBO!", combo_count);
                SCE_HUD_DrawTextCentered(center_x, y_pos, text);
            }
        }
        show_anything = true;
    }

    /* === Timer Bar === */
    if (timer_active && show_hud) {
        int bar_w = 80;
        int bar_h = 4;
        int bar_x = center_x - bar_w / 2;
        int bar_y = berserk_active ? 42 : 40;

        double ratio = timer_remaining / timer_total;
        if (ratio > 1.0) ratio = 1.0;
        if (ratio < 0.0) ratio = 0.0;
        int fill_w = (int)(bar_w * ratio);

        int fill_color = berserk_active ? COLOR_RED : COLOR_ORANGE;
        SCE_HUD_DrawBar(bar_x, bar_y, bar_w, bar_h, fill_w, COLOR_GRAY, fill_color);
        show_anything = true;
    }

    if (show_anything) {
        V_MarkRect(SCREENWIDTH / 2 - 50, 18, 100, 46);
    }
}
