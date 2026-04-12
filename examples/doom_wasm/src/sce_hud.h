// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

/**
 * @file sce_hud.h
 * @brief In-game HUD for SCE systems (combo counter, berserk overlay)
 *
 * Renders SCE state machine UI directly to DOOM's framebuffer
 * using the engine's own rendering pipeline (V_DrawPatch, M_WriteText, palette).
 */

#ifndef SCE_HUD_H
#define SCE_HUD_H

#include "doomtype.h"

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Initialize SCE HUD (call after HU_Init so hu_font is loaded)
 */
void SCE_HUD_Init(void);

/**
 * Draw SCE HUD elements on top of the 3D view.
 * Call from HU_Drawer() after message/chat drawing.
 */
void SCE_HUD_Drawer(void);

/**
 * Update combo display state (called from sce_wrapper callbacks)
 */
void SCE_HUD_SetCombo(int count, boolean active);

/**
 * Update combo timer state (called each tic from sce_wrapper)
 * @param remaining_ms  Remaining time in ms (-1 to clear)
 * @param total_ms      Total timer duration in ms
 */
void SCE_HUD_SetComboTimer(double remaining_ms, double total_ms);

/**
 * Update berserk overlay state (called from sce_wrapper callbacks)
 * @param intensity  Berserk intensity level (0 = off)
 * @param active     Whether berserk mode is currently active
 */
void SCE_HUD_SetBerserk(int intensity, boolean active);

/**
 * Apply berserk palette tint via DOOM's palette system.
 * Call from ST_doPaletteStuff() to override the palette when SCE berserk is active.
 * @return palette index to use (0 = no override, 1-8 = red palette)
 */
int SCE_HUD_GetBerserkPalette(void);

#ifdef __cplusplus
}
#endif

#endif /* SCE_HUD_H */
