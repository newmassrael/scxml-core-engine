#!/usr/bin/env python3
"""Apply SCE integration patches to doomgeneric source files."""

import os
import sys
import re

def patch_file(filepath, patches):
    """Apply patches to a file. Each patch is (pattern, replacement)."""
    with open(filepath, 'r') as f:
        content = f.read()

    for pattern, replacement in patches:
        new_content = re.sub(pattern, replacement, content, count=1, flags=re.DOTALL)
        if new_content == content:
            print(f"    WARNING: Pattern not matched in {os.path.basename(filepath)}: {pattern[:50]}...")
        content = new_content

    with open(filepath, 'w') as f:
        f.write(content)
    print(f"  Patched: {os.path.basename(filepath)}")

def main():
    if len(sys.argv) != 2:
        print("Usage: apply_patches.py <doomgeneric_dir>")
        sys.exit(1)

    dg_dir = os.path.join(sys.argv[1], "doomgeneric")

    # g_game.c - game state hooks
    patch_file(os.path.join(dg_dir, "g_game.c"), [
        # Add include after doomstat.h
        (r'(#include "doomstat\.h")',
         r'\1\n\n// SCE State Machine Integration\n#include "sce_doom_hooks.h"'),
        # G_DoCompleted
        (r'(void G_DoCompleted \(void\)\s*\{\s*\n\s*int\s+i;)',
         r'\1\n    SCE_GameCompleted();'),
        # G_DoLoadGame
        (r'(void G_DoLoadGame \(void\)\s*\{\s*\n)',
         r'\1    SCE_GameLoadGame();\n'),
        # G_DoNewGame
        (r'(void G_DoNewGame \(void\)\s*\{\s*\n)',
         r'\1    SCE_GameNewGame();\n'),
        # Player spawn complete - when playerstate becomes PST_LIVE
        (r'(p->playerstate = PST_LIVE;)',
         r'\1\n    // SCE: Player spawn complete\n    SCE_PlayerSpawnComplete();'),
        # Player respawn - when playerstate becomes PST_REBORN (dead player wants to respawn)
        (r'(players\[i\]\.playerstate = PST_REBORN;)',
         r'\1\n        // SCE: Player starting respawn\n        SCE_PlayerRespawn();'),
        # G_DoWorldDone - when going from intermission to next level
        (r'(void G_DoWorldDone \(void\)\s*\{\s*\n)',
         r'\1    // SCE: Moving to next level after intermission\n    SCE_GameWorldDone();\n'),
    ])

    # f_finale.c - finale hooks
    patch_file(os.path.join(dg_dir, "f_finale.c"), [
        # Add include at top after includes
        (r'(#include "doomstat\.h")',
         r'\1\n\n// SCE State Machine Integration\n#include "sce_doom_hooks.h"'),
        # F_StartFinale - when finale begins
        (r'(void F_StartFinale \(void\)\s*\{\s*\n)',
         r'\1    // SCE: Finale sequence starting\n    SCE_GameFinale();\n'),
    ])

    # p_mobj.c - spawn/remove hooks
    patch_file(os.path.join(dg_dir, "p_mobj.c"), [
        # Add include after doomstat.h
        (r'(#include "doomstat\.h")',
         r'\1\n\n// SCE State Machine Integration\n#include "sce_doom_hooks.h"'),
        # After P_AddThinker in P_SpawnMobj
        (r'(P_AddThinker \(&mobj->thinker\);)',
         r'\1\n\n    // SCE: Track monster spawn\n    SCE_EnemySpawned(mobj);'),
        # After P_UnsetThingPosition in P_RemoveMobj
        (r'(P_UnsetThingPosition \(mobj\);)',
         r'\1\n\n    // SCE: Track monster removal\n    SCE_EnemyRemoved(mobj);'),
    ])

    # p_inter.c - kill hooks
    patch_file(os.path.join(dg_dir, "p_inter.c"), [
        # Add include after doomstat.h
        (r'(#include "doomstat\.h")',
         r'\1\n\n// SCE State Machine Integration\n#include "sce_doom_hooks.h"'),
        # Track monster killed by player (after source->player->killcount++)
        (r'(if \(target->flags & MF_COUNTKILL\)\n\s*source->player->killcount\+\+;)',
         r'\1\n\n\t// SCE: Track monster killed by player\n\tSCE_EnemyKilled(target);'),
        # Track monster killed by environment/other monsters (after players[0].killcount++)
        (r'(players\[0\]\.killcount\+\+;)',
         r'\1\n\n\t// SCE: Track monster killed by environment/other monsters\n\tSCE_EnemyKilled(target);'),
        # After player death state
        (r'(target->player->playerstate = PST_DEAD;)',
         r'\1\n        // SCE: Track player killed\n        SCE_PlayerKilled();'),
    ])

    # p_enemy.c - AI state hooks
    patch_file(os.path.join(dg_dir, "p_enemy.c"), [
        # Add include after doomstat.h
        (r'(#include "doomstat\.h")',
         r'\1\n\n// SCE State Machine Integration\n#include "sce_doom_hooks.h"'),
        # In A_Look, before seestate transition
        (r'(\n\s*P_SetMobjState \(actor, actor->info->seestate\);)',
         r'\n\n    // SCE: Monster is now alert\n    SCE_EnemyAlert(actor);\1'),
        # In A_Chase, after the int delta declaration
        (r'(void A_Chase \(mobj_t\*\s*actor\)\s*\{\s*\n\s*int\s+delta;)',
         r'\1\n\n    // SCE: Monster is chasing\n    SCE_EnemyChasing(actor);'),
        # In A_Pain, at function start
        (r'(void A_Pain \(mobj_t\* actor\)\s*\{\s*\n)',
         r'\1    // SCE: Monster in pain\n    SCE_EnemyPain(actor);\n\n'),
        # In A_FaceTarget, after clearing AMBUSH flag
        (r'(actor->flags &= ~MF_AMBUSH;\s*\n)',
         r'\1\n    // SCE: Monster is attacking\n    SCE_EnemyAttacking(actor);\n'),
    ])

    # p_pspr.c - weapon state hooks (CORRECTED LOCATIONS)
    patch_file(os.path.join(dg_dir, "p_pspr.c"), [
        # Add include after doomstat.h
        (r'(#include "doomstat\.h")',
         r'\1\n\n// SCE State Machine Integration\n#include "sce_doom_hooks.h"'),

        # P_FireWeapon - after P_NoiseAlert (weapon firing)
        (r'(P_NoiseAlert \(player->mo, player->mo\);)',
         r'\1\n    // SCE: Weapon fired\n    SCE_WeaponFire();'),

        # A_WeaponReady - when starting to lower weapon for switch
        # This is when pendingweapon is set and downstate is activated
        (r'(// check for change\s*\n\s*//  if player is dead, put the weapon away\s*\n\s*if \(player->pendingweapon != wp_nochange \|\| !player->health\)\s*\n\s*\{\s*\n\s*// change weapon)',
         r'\1\n        // SCE: Starting weapon switch\n        if (player->pendingweapon != wp_nochange) SCE_WeaponSwitch();'),

        # A_Lower - before P_BringUpWeapon (lowering complete)
        (r'(player->readyweapon = player->pendingweapon;\s*\n\s*\n\s*P_BringUpWeapon \(player\);)',
         r'player->readyweapon = player->pendingweapon;\n\n    // SCE: Weapon lowering complete\n    SCE_WeaponLowerComplete();\n\n    P_BringUpWeapon (player);'),

        # A_Raise - after weapon fully raised (psp->sy = WEAPONTOP)
        (r'(psp->sy = WEAPONTOP;\s*\n\s*\n\s*// The weapon has been raised all the way)',
         r'psp->sy = WEAPONTOP;\n\n    // SCE: Weapon raising complete\n    SCE_WeaponRaiseComplete();\n\n    // The weapon has been raised all the way'),

        # A_ReFire - when NOT refiring (firing complete)
        (r'(else\s*\n\s*\{\s*\n\s*player->refire = 0;\s*\n\s*P_CheckAmmo \(player\);)',
         r'else\n    {\n        // SCE: Firing complete (not refiring)\n        SCE_WeaponFireComplete();\n        player->refire = 0;\n        P_CheckAmmo (player);'),
    ])

    # st_stuff.c - god mode cheat hook
    patch_file(os.path.join(dg_dir, "st_stuff.c"), [
        # Add include after doomstat.h
        (r'(#include "doomstat\.h")',
         r'\1\n\n// SCE State Machine Integration\n#include "sce_doom_hooks.h"'),
        # After god mode ON message
        (r"(plyr->message = DEH_String\(STSTR_DQDON\);)",
         r'\1\n\t  // SCE: God mode activated\n\t  SCE_PlayerGodModeOn();'),
        # Replace else without braces to else with braces and god mode OFF
        (r"(\n\telse\s*\n\s*plyr->message = DEH_String\(STSTR_DQDOFF\);)",
         r'\n\telse {\n\t  plyr->message = DEH_String(STSTR_DQDOFF);\n\t  // SCE: God mode deactivated\n\t  SCE_PlayerGodModeOff();\n\t}'),
    ])

    # doomgeneric_emscripten.c - init hook
    patch_file(os.path.join(dg_dir, "doomgeneric_emscripten.c"), [
        # Add include after emscripten.h
        (r'(#include <emscripten\.h>)',
         r'\1\n\n// SCE State Machine Integration\n#include "sce_doom_hooks.h"'),
        # After texture creation in DG_Init
        (r'(texture = SDL_CreateTexture\([^;]+;)',
         r'\1\n\n  // Initialize SCE state machines\n  SCE_Init();'),
    ])

    print("SCE integration patches applied successfully")

if __name__ == "__main__":
    main()
