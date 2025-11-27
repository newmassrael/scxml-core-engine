/**
 * @file sce_secret_hint.c
 * @brief Secret hint system implementation with BFS pathfinding
 *
 * Uses BFS through DOOM's sector adjacency graph to find
 * the shortest path to the nearest unrevealed secret sector.
 */

#include "sce_secret_hint.h"
#include "doomstat.h"
#include "doomdata.h"
#include "r_state.h"
#include "r_main.h"
#include "p_local.h"
#include "p_mobj.h"
#include "info.h"
#include "tables.h"
#include "m_fixed.h"
#include <string.h>
#include <stdlib.h>

/* Debug logging - define DEBUG_SECRET to enable verbose output */
#ifdef DEBUG_SECRET
#define SECRET_DEBUG(...) printf(__VA_ARGS__)
#else
#define SECRET_DEBUG(...) ((void)0)
#endif

/* BFS queue structure */
typedef struct {
    int sector;
    int parent;
} bfs_node_t;

/* Module state */
static boolean s_enabled = true;
static secret_path_t s_current_path;
static boolean s_path_active = false;

/* Spawned hint sprites (plasma balls) */
static mobj_t *s_hint_sprites[SECRET_MAX_ARROWS];
static int s_num_sprites = 0;

/* Forward declarations */
static boolean IsDoorSpecial(int special);
static boolean IsLiftSpecial(int special);
static boolean IsImpassableWall(line_t *line);
static void BuildSectorAdjacency(void);
static void GetSectorCenter(int sector_idx, fixed_t *out_x, fixed_t *out_y);

/* Line-of-sight checking using P_PathTraverse */
static boolean s_sight_blocked = false;
static line_t *s_blocking_line = NULL;  /* The line that blocked sight */
static fixed_t s_block_frac = FRACUNIT; /* Fraction along path where block occurred */

/**
 * Callback for P_PathTraverse - checks if line blocks sight.
 * Returns true to continue traversal, false to stop (blocked).
 */
static boolean PTR_SightCheck(intercept_t *in) {
    line_t *line = in->d.line;

    /* Two-sided line - check if there's an opening */
    if (line->backsector) {
        P_LineOpening(line);
        if (openrange <= 0) {
            /* No vertical opening - blocked */
            s_sight_blocked = true;
            s_blocking_line = line;
            s_block_frac = in->frac;
            return false;
        }
        /* Has opening, continue checking */
        return true;
    }

    /* One-sided line (solid wall) - blocked */
    s_sight_blocked = true;
    s_blocking_line = line;
    s_block_frac = in->frac;
    return false;
}

/**
 * Check if a straight line from (x1,y1) to (x2,y2) crosses any walls.
 * @return true if line is clear (no walls), false if blocked
 */
static boolean CheckLineOfSight(fixed_t x1, fixed_t y1, fixed_t x2, fixed_t y2) {
    s_sight_blocked = false;
    s_blocking_line = NULL;
    s_block_frac = FRACUNIT;
    P_PathTraverse(x1, y1, x2, y2, PT_ADDLINES, PTR_SightCheck);
    return !s_sight_blocked;
}

/**
 * Get the blocking line from the last CheckLineOfSight call.
 * @return The line that blocked, or NULL if path was clear
 */
static line_t* GetBlockingLine(void) {
    return s_blocking_line;
}

/**
 * Find a waypoint to go around a blocking wall.
 * Tests both vertices of the wall with offsets in both directions.
 * Chooses the waypoint that is reachable from start AND can reach destination.
 */
static void FindWaypointAroundWall(line_t *wall,
                                    fixed_t start_x, fixed_t start_y,
                                    fixed_t dest_x, fixed_t dest_y,
                                    fixed_t *out_x, fixed_t *out_y) {
    fixed_t v1_x = wall->v1->x;
    fixed_t v1_y = wall->v1->y;
    fixed_t v2_x = wall->v2->x;
    fixed_t v2_y = wall->v2->y;

    /* Wall direction and normal vectors */
    fixed_t wall_dx = v2_x - v1_x;
    fixed_t wall_dy = v2_y - v1_y;
    fixed_t wall_len = P_AproxDistance(wall_dx, wall_dy);

    /* Normal vector (perpendicular to wall) */
    fixed_t nx = 0, ny = 0;
    /* Tangent vector (along wall) */
    fixed_t tx = 0, ty = 0;
    if (wall_len > 0) {
        nx = FixedDiv(-wall_dy, wall_len);
        ny = FixedDiv(wall_dx, wall_len);
        tx = FixedDiv(wall_dx, wall_len);
        ty = FixedDiv(wall_dy, wall_len);
    }

    fixed_t offset = 48 * FRACUNIT;  /* 48 units offset from wall */

    /* Determine which side of the wall the destination is on */
    /* Using cross product: (dest - v1) x (v2 - v1) */
    fixed_t dest_side = FixedMul(dest_x - v1_x, wall_dy) - FixedMul(dest_y - v1_y, wall_dx);
    int dest_sign = (dest_side > 0) ? 1 : -1;

    /* Test 4 candidates: each vertex, offset toward destination side AND along wall */
    struct {
        fixed_t x, y;
        boolean reachable_from_start;
        boolean can_reach_dest;
        fixed_t total_dist;
    } candidates[4];
    int num_candidates = 0;

    /* For each vertex, create waypoint that goes around the wall end toward destination */
    /* v1: offset perpendicular (toward dest) and tangent (away from wall center) */
    /* v2: offset perpendicular (toward dest) and tangent (away from wall center) */

    /* Candidate 0: v1, offset toward dest side and outward along wall */
    candidates[0].x = v1_x + FixedMul(nx, offset * dest_sign) - FixedMul(tx, offset);
    candidates[0].y = v1_y + FixedMul(ny, offset * dest_sign) - FixedMul(ty, offset);

    /* Candidate 1: v2, offset toward dest side and outward along wall */
    candidates[1].x = v2_x + FixedMul(nx, offset * dest_sign) + FixedMul(tx, offset);
    candidates[1].y = v2_y + FixedMul(ny, offset * dest_sign) + FixedMul(ty, offset);

    /* Candidate 2: v1, opposite side (in case dest_sign is wrong due to geometry) */
    candidates[2].x = v1_x + FixedMul(nx, offset * (-dest_sign)) - FixedMul(tx, offset);
    candidates[2].y = v1_y + FixedMul(ny, offset * (-dest_sign)) - FixedMul(ty, offset);

    /* Candidate 3: v2, opposite side */
    candidates[3].x = v2_x + FixedMul(nx, offset * (-dest_sign)) + FixedMul(tx, offset);
    candidates[3].y = v2_y + FixedMul(ny, offset * (-dest_sign)) + FixedMul(ty, offset);

    num_candidates = 4;

    /* Check reachability for all candidates */
    for (int i = 0; i < num_candidates; i++) {
        candidates[i].reachable_from_start = CheckLineOfSight(start_x, start_y,
                                                               candidates[i].x, candidates[i].y);
        candidates[i].can_reach_dest = CheckLineOfSight(candidates[i].x, candidates[i].y,
                                                         dest_x, dest_y);
        candidates[i].total_dist =
            P_AproxDistance(start_x - candidates[i].x, start_y - candidates[i].y) +
            P_AproxDistance(candidates[i].x - dest_x, candidates[i].y - dest_y);
    }

    /* Debug: log all candidates */
    SECRET_DEBUG("[SECRET] FindWaypoint: dest_sign=%d, dest=(%d,%d)\n",
           dest_sign, dest_x >> FRACBITS, dest_y >> FRACBITS);
    for (int i = 0; i < num_candidates; i++) {
        SECRET_DEBUG("[SECRET]   Candidate %d: (%d,%d) reach_start=%d reach_dest=%d\n",
               i, candidates[i].x >> FRACBITS, candidates[i].y >> FRACBITS,
               candidates[i].reachable_from_start, candidates[i].can_reach_dest);
    }

    /* Priority 1: Reachable from start AND can reach dest (prefer dest_sign side first) */
    int best = -1;
    fixed_t best_dist = INT_MAX;
    for (int i = 0; i < 2; i++) {  /* Check dest_sign side first (candidates 0,1) */
        if (candidates[i].reachable_from_start && candidates[i].can_reach_dest) {
            if (candidates[i].total_dist < best_dist) {
                best = i;
                best_dist = candidates[i].total_dist;
            }
        }
    }
    if (best < 0) {
        for (int i = 2; i < 4; i++) {  /* Then check opposite side */
            if (candidates[i].reachable_from_start && candidates[i].can_reach_dest) {
                if (candidates[i].total_dist < best_dist) {
                    best = i;
                    best_dist = candidates[i].total_dist;
                }
            }
        }
    }

    /* Priority 2: Can reach dest (even if not directly reachable from start) */
    if (best < 0) {
        best_dist = INT_MAX;
        for (int i = 0; i < num_candidates; i++) {
            if (candidates[i].can_reach_dest) {
                if (candidates[i].total_dist < best_dist) {
                    best = i;
                    best_dist = candidates[i].total_dist;
                }
            }
        }
    }

    /* Priority 3: At least reachable from start */
    if (best < 0) {
        best_dist = INT_MAX;
        for (int i = 0; i < num_candidates; i++) {
            if (candidates[i].reachable_from_start) {
                if (candidates[i].total_dist < best_dist) {
                    best = i;
                    best_dist = candidates[i].total_dist;
                }
            }
        }
    }

    /* Priority 4: Any candidate (fallback) */
    if (best < 0) {
        best_dist = INT_MAX;
        for (int i = 0; i < num_candidates; i++) {
            if (candidates[i].total_dist < best_dist) {
                best = i;
                best_dist = candidates[i].total_dist;
            }
        }
    }

    if (best >= 0) {
        *out_x = candidates[best].x;
        *out_y = candidates[best].y;
    } else {
        /* Ultimate fallback: wall midpoint */
        *out_x = (v1_x + v2_x) / 2;
        *out_y = (v1_y + v2_y) / 2;
    }
}

/* Sector adjacency cache (built per level) */
static int *s_adjacency_list = NULL;    /* Flat array of adjacent sector indices */
static int *s_adjacency_offset = NULL;  /* Start offset for each sector */
static int *s_adjacency_count = NULL;   /* Count of adjacent sectors for each */
static int s_adjacency_total = 0;
static boolean s_adjacency_dirty = true; /* Rebuild when doors/lifts change */

/* Track blocked sector connections (1-sided walls blocking BFS paths) */
#define MAX_BLOCKED_CONNECTIONS 32
typedef struct {
    int sector_a;
    int sector_b;
    int blocking_line;  /* The 1-sided wall line index */
} blocked_connection_t;
static blocked_connection_t s_blocked_connections[MAX_BLOCKED_CONNECTIONS];
static int s_num_blocked_connections = 0;

/**
 * Check if a sector connection is blocked by a known 1-sided wall.
 */
static boolean IsConnectionBlocked(int sector_a, int sector_b) {
    int i;
    for (i = 0; i < s_num_blocked_connections; i++) {
        if ((s_blocked_connections[i].sector_a == sector_a &&
             s_blocked_connections[i].sector_b == sector_b) ||
            (s_blocked_connections[i].sector_a == sector_b &&
             s_blocked_connections[i].sector_b == sector_a)) {
            return true;
        }
    }
    return false;
}

/**
 * Add a blocked connection (1-sided wall blocks travel between sectors).
 */
static void AddBlockedConnection(int sector_a, int sector_b, int blocking_line) {
    int i;

    /* Check if already blocked */
    for (i = 0; i < s_num_blocked_connections; i++) {
        if ((s_blocked_connections[i].sector_a == sector_a &&
             s_blocked_connections[i].sector_b == sector_b) ||
            (s_blocked_connections[i].sector_a == sector_b &&
             s_blocked_connections[i].sector_b == sector_a)) {
            return;  /* Already tracked */
        }
    }

    if (s_num_blocked_connections < MAX_BLOCKED_CONNECTIONS) {
        s_blocked_connections[s_num_blocked_connections].sector_a = sector_a;
        s_blocked_connections[s_num_blocked_connections].sector_b = sector_b;
        s_blocked_connections[s_num_blocked_connections].blocking_line = blocking_line;
        s_num_blocked_connections++;
        SECRET_DEBUG("[SECRET] Added blocked connection: sector %d <-> %d (line %d), total=%d\n",
               sector_a, sector_b, blocking_line, s_num_blocked_connections);
    }
}

/**
 * Clear all blocked connections (on level load or target change).
 */
static void ClearBlockedConnections(void) {
    if (s_num_blocked_connections > 0) {
        SECRET_DEBUG("[SECRET] Clearing %d blocked connections\n", s_num_blocked_connections);
    }
    s_num_blocked_connections = 0;
}

/* Track last player sector to avoid unnecessary path recalculation */
static int s_last_player_sector = -1;

/* Track last player position to avoid unnecessary arrow regeneration */
static fixed_t s_last_player_x = 0;
static fixed_t s_last_player_y = 0;
#define PLAYER_MOVE_THRESHOLD (16 * FRACUNIT)  /* Update arrows when moved 16+ units */

/* Target storage for selection system */
static target_info_t s_targets[TARGET_TYPE_COUNT][SECRET_MAX_TARGETS];
static int s_target_counts[TARGET_TYPE_COUNT] = {0};
static target_type_t s_current_type = TARGET_SECRET;
static int s_current_index = 0;

/* Original secret sector tracking (preserves info after discovery) */
static int s_original_secret_sectors[SECRET_MAX_TARGETS];  /* Sector indices that were originally secrets */
static int s_original_secret_count = 0;

/* Static name buffers for target display */
static char s_target_names[SECRET_MAX_TARGETS][32];

/**
 * Get center point of a sector using bounding box method.
 * More reliable than vertex averaging for complex sector shapes.
 */
static void GetSectorCenter(int sector_idx, fixed_t *out_x, fixed_t *out_y) {
    int i;
    fixed_t min_x = INT_MAX, max_x = INT_MIN;
    fixed_t min_y = INT_MAX, max_y = INT_MIN;
    boolean found = false;

    /* Find bounding box by scanning all lines that reference this sector */
    for (i = 0; i < numlines; i++) {
        line_t *line = &lines[i];

        /* Check if this line belongs to our sector */
        if ((line->frontsector && (line->frontsector - sectors) == sector_idx) ||
            (line->backsector && (line->backsector - sectors) == sector_idx)) {

            /* Include both vertices of the line */
            if (line->v1->x < min_x) min_x = line->v1->x;
            if (line->v1->x > max_x) max_x = line->v1->x;
            if (line->v1->y < min_y) min_y = line->v1->y;
            if (line->v1->y > max_y) max_y = line->v1->y;

            if (line->v2->x < min_x) min_x = line->v2->x;
            if (line->v2->x > max_x) max_x = line->v2->x;
            if (line->v2->y < min_y) min_y = line->v2->y;
            if (line->v2->y > max_y) max_y = line->v2->y;

            found = true;
        }
    }

    if (found) {
        *out_x = (min_x + max_x) / 2;
        *out_y = (min_y + max_y) / 2;
    } else {
        *out_x = 0;
        *out_y = 0;
    }
}

/**
 * Calculate angle from point (x1,y1) to (x2,y2)
 */
static angle_t CalcAngle(fixed_t x1, fixed_t y1, fixed_t x2, fixed_t y2) {
    fixed_t dx = x2 - x1;
    fixed_t dy = y2 - y1;
    return R_PointToAngle2(x1, y1, x2, y2);
}

/**
 * Check if player can walk through a two-sided linedef.
 * Considers blocking flags, step height, and opening height.
 *
 * DOOM passability rules:
 * 1. One-sided lines (no backsector) = impassable wall
 * 2. ML_BLOCKING flag = impassable (unless door/lift special)
 * 3. Opening < 56 units = too small for player
 * 4. Step height > 24 units = too high to climb
 *
 * Note: Lines with door/lift specials are always passable (can be opened).
 */
static boolean CanPassLine(line_t *line) {
    fixed_t front_floor, front_ceil;
    fixed_t back_floor, back_ceil;
    fixed_t low_ceil, high_floor;
    fixed_t opening;
    fixed_t floor_diff;

    /* One-sided line = wall */
    if (!line->frontsector || !line->backsector) {
        return false;
    }

    /* Check blocking flag - but doors with specials can still be passed */
    if (line->flags & ML_BLOCKING) {
        /* If it has a door/lift special, it can be opened */
        if (!IsDoorSpecial(line->special) && !IsLiftSpecial(line->special)) {
            return false;
        }
    }

    /* If line has door/lift special, always consider it passable */
    /* The door might be closed now but can be opened */
    if (IsDoorSpecial(line->special) || IsLiftSpecial(line->special)) {
        return true;
    }

    front_floor = line->frontsector->floorheight;
    front_ceil = line->frontsector->ceilingheight;
    back_floor = line->backsector->floorheight;
    back_ceil = line->backsector->ceilingheight;

    /* Calculate opening: gap between higher floor and lower ceiling */
    high_floor = (front_floor > back_floor) ? front_floor : back_floor;
    low_ceil = (front_ceil < back_ceil) ? front_ceil : back_ceil;
    opening = low_ceil - high_floor;

    /* Player needs at least 56 units of vertical space */
    if (opening < 56 * FRACUNIT) {
        return false;
    }

    /*
     * Step height check for BFS pathfinding:
     * Since BFS doesn't track direction, require BOTH directions passable.
     * Player can step up max 24 units. Use absolute floor difference.
     */
    floor_diff = back_floor - front_floor;
    if (floor_diff < 0) floor_diff = -floor_diff;  /* abs() */

    if (floor_diff > 24 * FRACUNIT) {
        return false;  /* Too much height difference for BFS */
    }

    return true;
}

/**
 * Build sector adjacency graph from linedef data.
 * Only includes connections the player can actually walk through.
 */
static void BuildSectorAdjacency(void) {
    int i;
    int *temp_counts;
    int offset;

    /* Free previous adjacency data */
    if (s_adjacency_list) {
        free(s_adjacency_list);
        s_adjacency_list = NULL;
    }
    if (s_adjacency_offset) {
        free(s_adjacency_offset);
        s_adjacency_offset = NULL;
    }
    if (s_adjacency_count) {
        free(s_adjacency_count);
        s_adjacency_count = NULL;
    }

    /* Allocate count arrays */
    s_adjacency_count = (int*)calloc(numsectors, sizeof(int));
    s_adjacency_offset = (int*)malloc(numsectors * sizeof(int));
    temp_counts = (int*)calloc(numsectors, sizeof(int));

    if (!s_adjacency_count || !s_adjacency_offset || !temp_counts) {
        /* Clean up partial allocations on failure */
        if (s_adjacency_count) { free(s_adjacency_count); s_adjacency_count = NULL; }
        if (s_adjacency_offset) { free(s_adjacency_offset); s_adjacency_offset = NULL; }
        if (temp_counts) { free(temp_counts); }
        return;
    }

    /* First pass: count walkable adjacent sectors */
    for (i = 0; i < numlines; i++) {
        line_t *line = &lines[i];

        /* Check if player can walk through this line */
        if (line->frontsector && line->backsector &&
            line->frontsector != line->backsector &&
            CanPassLine(line)) {
            int front_idx = line->frontsector - sectors;
            int back_idx = line->backsector - sectors;

            s_adjacency_count[front_idx]++;
            s_adjacency_count[back_idx]++;
        }
    }

    /* Calculate total size and offsets */
    s_adjacency_total = 0;
    for (i = 0; i < numsectors; i++) {
        s_adjacency_offset[i] = s_adjacency_total;
        s_adjacency_total += s_adjacency_count[i];
    }

    /* Allocate flat adjacency list */
    s_adjacency_list = (int*)malloc(s_adjacency_total * sizeof(int));
    if (!s_adjacency_list) {
        /* Clean up all allocations on failure */
        free(temp_counts);
        free(s_adjacency_count); s_adjacency_count = NULL;
        free(s_adjacency_offset); s_adjacency_offset = NULL;
        return;
    }

    /* Initialize with -1 (invalid) */
    for (i = 0; i < s_adjacency_total; i++) {
        s_adjacency_list[i] = -1;
    }

    /* Second pass: fill adjacency list with walkable connections only */
    for (i = 0; i < numlines; i++) {
        line_t *line = &lines[i];

        if (line->frontsector && line->backsector &&
            line->frontsector != line->backsector &&
            CanPassLine(line)) {
            int front_idx = line->frontsector - sectors;
            int back_idx = line->backsector - sectors;

            /* Add back to front's list */
            offset = s_adjacency_offset[front_idx] + temp_counts[front_idx];
            s_adjacency_list[offset] = back_idx;
            temp_counts[front_idx]++;

            /* Add front to back's list */
            offset = s_adjacency_offset[back_idx] + temp_counts[back_idx];
            s_adjacency_list[offset] = front_idx;
            temp_counts[back_idx]++;
        }
    }

    free(temp_counts);
    s_adjacency_dirty = false;  /* Mark as up-to-date */
}

/**
 * Conditionally rebuild adjacency if dirty.
 * More efficient than rebuilding every frame.
 */
static void EnsureAdjacencyValid(void) {
    if (s_adjacency_dirty) {
        BuildSectorAdjacency();
    }
}

/**
 * Mark adjacency as needing rebuild (call when doors/lifts change).
 * Also triggers path recalculation on next update.
 */
void Secret_InvalidateAdjacency(void) {
    s_adjacency_dirty = true;
    /* Force path recalculation by resetting last player sector */
    s_last_player_sector = -1;
}

/**
 * Get the sector containing a point.
 * Uses subsector lookup.
 */
static int GetSectorAt(fixed_t x, fixed_t y) {
    subsector_t *ss = R_PointInSubsector(x, y);
    if (ss && ss->sector) {
        return ss->sector - sectors;
    }
    return -1;
}

/**
 * Find the linedef that connects two adjacent sectors.
 * Returns linedef index, or -1 if not found.
 */
static int FindConnectingLine(int sector1, int sector2) {
    int i;
    for (i = 0; i < numlines; i++) {
        line_t *line = &lines[i];
        if (line->frontsector && line->backsector) {
            int front_idx = line->frontsector - sectors;
            int back_idx = line->backsector - sectors;
            if ((front_idx == sector1 && back_idx == sector2) ||
                (front_idx == sector2 && back_idx == sector1)) {
                return i;
            }
        }
    }
    return -1;
}

/**
 * Get center of a linedef.
 */
static void GetLineMidpoint(int line_idx, fixed_t *out_x, fixed_t *out_y) {
    line_t *line = &lines[line_idx];
    *out_x = (line->v1->x + line->v2->x) / 2;
    *out_y = (line->v1->y + line->v2->y) / 2;
}

/**
 * Get a waypoint on the connecting linedef, pushed into the destination sector.
 * This ensures the waypoint is clearly inside walkable space, not on the boundary.
 *
 * @param line_idx The connecting linedef
 * @param dest_sector The destination sector (we push the point toward this sector)
 * @param out_x Output X coordinate
 * @param out_y Output Y coordinate
 */
static void GetPortalWaypoint(int line_idx, int dest_sector, fixed_t *out_x, fixed_t *out_y) {
    line_t *line = &lines[line_idx];
    fixed_t mid_x = (line->v1->x + line->v2->x) / 2;
    fixed_t mid_y = (line->v1->y + line->v2->y) / 2;

    /* Calculate line normal (perpendicular to the linedef) */
    fixed_t dx = line->v2->x - line->v1->x;
    fixed_t dy = line->v2->y - line->v1->y;

    /* Normal vectors (both directions) */
    fixed_t nx1 = -dy;  /* Perpendicular direction 1 */
    fixed_t ny1 = dx;
    fixed_t nx2 = dy;   /* Perpendicular direction 2 */
    fixed_t ny2 = -dx;

    /* Normalize and scale by offset (32 units = player radius) */
    fixed_t len = P_AproxDistance(dx, dy);
    if (len == 0) {
        *out_x = mid_x;
        *out_y = mid_y;
        return;
    }

    #define PORTAL_OFFSET (48 * FRACUNIT)  /* Push 48 units into sector */

    fixed_t offset_x1 = FixedDiv(FixedMul(nx1, PORTAL_OFFSET), len);
    fixed_t offset_y1 = FixedDiv(FixedMul(ny1, PORTAL_OFFSET), len);
    fixed_t offset_x2 = FixedDiv(FixedMul(nx2, PORTAL_OFFSET), len);
    fixed_t offset_y2 = FixedDiv(FixedMul(ny2, PORTAL_OFFSET), len);

    /* Try both directions, pick the one that lands in dest_sector */
    fixed_t test_x1 = mid_x + offset_x1;
    fixed_t test_y1 = mid_y + offset_y1;
    fixed_t test_x2 = mid_x + offset_x2;
    fixed_t test_y2 = mid_y + offset_y2;

    int sector1 = GetSectorAt(test_x1, test_y1);
    int sector2 = GetSectorAt(test_x2, test_y2);

    if (sector1 == dest_sector) {
        *out_x = test_x1;
        *out_y = test_y1;
    } else if (sector2 == dest_sector) {
        *out_x = test_x2;
        *out_y = test_y2;
    } else {
        /* Fallback to linedef center */
        *out_x = mid_x;
        *out_y = mid_y;
    }
}

/**
 * Check if sector is a secret that hasn't been discovered.
 * Secret sectors have special == 9.
 */
static boolean IsUndiscoveredSecret(int sector_idx) {
    if (sector_idx < 0 || sector_idx >= numsectors) {
        return false;
    }
    return sectors[sector_idx].special == 9;
}

/**
 * BFS to find shortest path from start_sector to target_sector.
 * If target is unreachable, returns path to the closest reachable sector.
 */
static boolean BFSFindPath(int start_sector, int target_sector, secret_path_t *out_path) {
    bfs_node_t *queue;
    boolean *visited;
    int queue_head = 0;
    int queue_tail = 0;
    int i, current, adj_idx, neighbor;
    int found_target = -1;

    /* Track closest reachable sector to target */
    int closest_idx = 0;
    fixed_t closest_dist = INT_MAX;
    fixed_t target_x, target_y;

    if (start_sector < 0 || target_sector < 0 || !s_adjacency_list) {
        return false;
    }

    /* Get target sector center for distance calculation */
    GetSectorCenter(target_sector, &target_x, &target_y);

    queue = (bfs_node_t*)malloc(numsectors * sizeof(bfs_node_t));
    visited = (boolean*)calloc(numsectors, sizeof(boolean));

    if (!queue || !visited) {
        if (queue) free(queue);
        if (visited) free(visited);
        return false;
    }

    queue[queue_tail].sector = start_sector;
    queue[queue_tail].parent = -1;
    queue_tail++;
    visited[start_sector] = true;

    while (queue_head < queue_tail) {
        current = queue[queue_head].sector;

        /* Check if this is the closest sector to target so far */
        fixed_t curr_x, curr_y;
        GetSectorCenter(current, &curr_x, &curr_y);
        fixed_t dist = P_AproxDistance(curr_x - target_x, curr_y - target_y);
        if (dist < closest_dist) {
            closest_dist = dist;
            closest_idx = queue_head;
        }

        if (current == target_sector) {
            found_target = queue_head;
            break;
        }

        for (i = 0; i < s_adjacency_count[current]; i++) {
            adj_idx = s_adjacency_offset[current] + i;
            neighbor = s_adjacency_list[adj_idx];

            if (neighbor >= 0 && neighbor < numsectors && !visited[neighbor]) {
                /* Skip connections blocked by 1-sided walls */
                if (IsConnectionBlocked(current, neighbor)) {
                    continue;
                }
                visited[neighbor] = true;
                queue[queue_tail].sector = neighbor;
                queue[queue_tail].parent = queue_head;
                queue_tail++;
            }
        }

        queue_head++;
    }

    /* Use found target if available, otherwise use closest reachable sector */
    int result_idx = (found_target >= 0) ? found_target : closest_idx;

    if (result_idx >= 0 && queue[result_idx].sector != start_sector) {
        int path_idx = 0;
        int trace = result_idx;
        int temp_path[SECRET_MAX_PATH_LENGTH];

        while (trace >= 0 && path_idx < SECRET_MAX_PATH_LENGTH) {
            temp_path[path_idx++] = queue[trace].sector;
            trace = queue[trace].parent;
        }

        out_path->path_length = path_idx;
        out_path->target_sector = (found_target >= 0) ? target_sector : queue[result_idx].sector;
        for (i = 0; i < path_idx; i++) {
            out_path->path[i] = temp_path[path_idx - 1 - i];
        }
        out_path->valid = true;

        free(queue);
        free(visited);
        return true;
    }

    free(queue);
    free(visited);
    return false;
}

/**
 * BFS to find shortest path from start_sector to nearest secret.
 * If no secret is reachable, returns path to the furthest explored sector
 * (closest to any secret sector by straight-line distance).
 */
static boolean BFSFindSecret(int start_sector, secret_path_t *out_path) {
    bfs_node_t *queue;
    boolean *visited;
    int queue_head = 0;
    int queue_tail = 0;
    int i, current, adj_idx, neighbor;
    int found_secret = -1;

    /* Track the sector closest to any secret (for partial path) */
    int closest_to_secret_idx = 0;
    fixed_t closest_to_secret_dist = INT_MAX;

    if (start_sector < 0 || !s_adjacency_list) {
        return false;
    }

    /* Allocate BFS structures */
    queue = (bfs_node_t*)malloc(numsectors * sizeof(bfs_node_t));
    visited = (boolean*)calloc(numsectors, sizeof(boolean));

    if (!queue || !visited) {
        if (queue) free(queue);
        if (visited) free(visited);
        return false;
    }

    /* Initialize BFS from start sector */
    queue[queue_tail].sector = start_sector;
    queue[queue_tail].parent = -1;
    queue_tail++;
    visited[start_sector] = true;

    /* BFS loop */
    while (queue_head < queue_tail) {
        current = queue[queue_head].sector;

        /* Check if current sector is an undiscovered secret */
        if (IsUndiscoveredSecret(current)) {
            found_secret = queue_head;
            break;
        }

        /* Check distance to all secret sectors - track closest reachable point */
        fixed_t curr_x, curr_y;
        GetSectorCenter(current, &curr_x, &curr_y);
        for (i = 0; i < numsectors; i++) {
            if (sectors[i].special == 9) {  /* Secret sector */
                fixed_t secret_x, secret_y;
                GetSectorCenter(i, &secret_x, &secret_y);
                fixed_t dist = P_AproxDistance(curr_x - secret_x, curr_y - secret_y);
                if (dist < closest_to_secret_dist) {
                    closest_to_secret_dist = dist;
                    closest_to_secret_idx = queue_head;
                }
            }
        }

        /* Explore adjacent sectors */
        for (i = 0; i < s_adjacency_count[current]; i++) {
            adj_idx = s_adjacency_offset[current] + i;
            neighbor = s_adjacency_list[adj_idx];

            if (neighbor >= 0 && neighbor < numsectors && !visited[neighbor]) {
                /* Skip connections blocked by 1-sided walls */
                if (IsConnectionBlocked(current, neighbor)) {
                    continue;
                }
                visited[neighbor] = true;
                queue[queue_tail].sector = neighbor;
                queue[queue_tail].parent = queue_head;
                queue_tail++;
            }
        }

        queue_head++;
    }

    /* Use found secret if available, otherwise use closest reachable to any secret */
    int result_idx = (found_secret >= 0) ? found_secret : closest_to_secret_idx;

    /* Reconstruct path */
    if (result_idx >= 0 && queue[result_idx].sector != start_sector) {
        int path_idx = 0;
        int trace = result_idx;
        int temp_path[SECRET_MAX_PATH_LENGTH];

        /* Trace back to start */
        while (trace >= 0 && path_idx < SECRET_MAX_PATH_LENGTH) {
            temp_path[path_idx++] = queue[trace].sector;
            trace = queue[trace].parent;
        }

        /* Reverse path (start to target) */
        out_path->path_length = path_idx;
        out_path->target_sector = queue[result_idx].sector;
        for (i = 0; i < path_idx; i++) {
            out_path->path[i] = temp_path[path_idx - 1 - i];
        }
        out_path->valid = true;

        free(queue);
        free(visited);
        return true;
    }

    free(queue);
    free(visited);
    return false;
}

/**
 * Check if linedef special is a door/switch/lift that can open.
 * Includes doors, lifts, and platforms that might lead to secrets.
 */
static boolean IsDoorSpecial(int special) {
    switch (special) {
    /* Push/Use doors (DR type) - common for hidden doors */
    case 1:    /* DR Door Open Wait Close */
    case 31:   /* D1 Door Open Stay */
    case 117:  /* DR Door Blazing Open Wait Close */
    case 118:  /* D1 Door Blazing Open Stay */
    /* Keyed doors */
    case 26:   /* DR Blue Door */
    case 27:   /* DR Yellow Door */
    case 28:   /* DR Red Door */
    case 32:   /* D1 Blue Door Open Stay */
    case 33:   /* D1 Red Door Open Stay */
    case 34:   /* D1 Yellow Door Open Stay */
    /* Switch doors */
    case 103:  /* S1 Door Open Stay */
    case 29:   /* S1 Door Raise */
    case 50:   /* S1 Door Close */
    /* Walk-trigger doors */
    case 2:    /* W1 Door Open Stay */
    case 3:    /* W1 Door Close */
    case 4:    /* W1 Door Raise */
    case 16:   /* W1 Door Close Wait Open */
    case 90:   /* WR Door Raise */
    case 46:   /* GR Door Open Stay (gunfire) */
    /* Lifts/Platforms - also common for secret access */
    case 10:   /* W1 Lift */
    case 21:   /* S1 Lift Lower Wait Raise */
    case 62:   /* SR Lift Lower Wait Raise */
    case 88:   /* WR Lift */
    case 120:  /* WR Lift Fast */
    case 121:  /* W1 Lift Fast */
    case 122:  /* S1 Lift Fast */
    case 123:  /* SR Lift Fast */
    /* Floor lower/raise - might reveal secrets */
    case 19:   /* W1 Floor Lower to Highest */
    case 36:   /* W1 Floor Lower to 8 above Highest */
    case 37:   /* W1 Floor Lower to Lowest Change */
    case 38:   /* W1 Floor Lower to Lowest */
        return true;
    default:
        return false;
    }
}

/**
 * Check if this is a lift special (subset of door specials that move floors).
 */
static boolean IsLiftSpecial(int special) {
    switch (special) {
    case 10:   /* W1 Lift */
    case 21:   /* S1 Lift */
    case 62:   /* SR Lift */
    case 88:   /* WR Lift */
    case 120:  /* WR Lift Fast */
    case 121:  /* W1 Lift Fast */
    case 122:  /* S1 Lift Fast */
    case 123:  /* SR Lift Fast */
        return true;
    default:
        return false;
    }
}

/**
 * Check if a wall is absolutely impassable (cannot ever be opened).
 * Returns true for solid walls, false for doors/lifts that can be opened.
 */
static boolean IsImpassableWall(line_t *line) {
    if (!line) {
        return false;
    }

    /* 1-sided wall = absolutely impassable (solid geometry) */
    if (!line->backsector) {
        return true;
    }

    /* 2-sided line with door/lift special = can be opened later */
    if (IsDoorSpecial(line->special) || IsLiftSpecial(line->special)) {
        return false;
    }

    /* 2-sided line with ML_BLOCKING but no door/lift special = impassable */
    if (line->flags & ML_BLOCKING) {
        return true;
    }

    /* Otherwise it's passable (or will become passable) */
    return false;
}

/**
 * Find lift that provides access to an elevated secret sector.
 * Searches for ANY lift linedef with tag matching sectors connected to the secret.
 * Returns the linedef index of the lift, and outputs the sector to reach.
 */
static int FindLiftToSecret(int target_secret, int *out_access_sector) {
    int i, j;
    fixed_t secret_floor = sectors[target_secret].floorheight;

    /* Search ALL lift linedefs */
    for (i = 0; i < numlines; i++) {
        line_t *line = &lines[i];

        if (!IsLiftSpecial(line->special)) continue;

        /* Check if this lift's tag matches any sector at secret's height */
        if (line->tag > 0) {
            for (j = 0; j < numsectors; j++) {
                if (sectors[j].tag == line->tag) {
                    fixed_t height_diff = sectors[j].floorheight - secret_floor;
                    if (height_diff < 0) height_diff = -height_diff;

                    if (height_diff <= 24 * FRACUNIT) {
                        /* This lift could provide access! Find the lower side */
                        if (line->frontsector && line->backsector) {
                            if (line->frontsector->floorheight < line->backsector->floorheight) {
                                *out_access_sector = line->frontsector - sectors;
                            } else {
                                *out_access_sector = line->backsector - sectors;
                            }
                        } else if (line->frontsector) {
                            *out_access_sector = line->frontsector - sectors;
                        }
                        return i;
                    }
                }
            }
        }

        /* Also check if lift directly connects to a sector at secret's level */
        if (line->frontsector && line->backsector) {
            fixed_t front_diff = line->frontsector->floorheight - secret_floor;
            fixed_t back_diff = line->backsector->floorheight - secret_floor;
            if (front_diff < 0) front_diff = -front_diff;
            if (back_diff < 0) back_diff = -back_diff;

            if (front_diff <= 24 * FRACUNIT || back_diff <= 24 * FRACUNIT) {
                if (line->frontsector->floorheight < line->backsector->floorheight) {
                    *out_access_sector = line->frontsector - sectors;
                } else {
                    *out_access_sector = line->backsector - sectors;
                }
                return i;
            }
        }
    }

    return -1;
}

/**
 * Check if sector is adjacent to an undiscovered secret via a hidden door.
 * Returns the linedef index of the hidden door, or -1 if none found.
 */
static int FindHiddenDoorToSecret(int sector_idx) {
    int i;

    if (sector_idx < 0 || sector_idx >= numsectors) {
        return -1;
    }

    /* Check all linedefs to find doors leading FROM this sector TO a secret */
    for (i = 0; i < numlines; i++) {
        line_t *line = &lines[i];
        int other_sector = -1;

        /* Check if this line connects our sector to another */
        if (line->frontsector && (line->frontsector - sectors) == sector_idx) {
            if (line->backsector) {
                other_sector = line->backsector - sectors;
            }
        } else if (line->backsector && (line->backsector - sectors) == sector_idx) {
            if (line->frontsector) {
                other_sector = line->frontsector - sectors;
            }
        }

        /* If the other side is a secret and line has door special */
        if (other_sector >= 0 && IsUndiscoveredSecret(other_sector)) {
            if (IsDoorSpecial(line->special)) {
                return i;  /* Found hidden door to secret */
            }
        }
    }

    return -1;
}

/**
 * Debug: Find all linedefs adjacent to any secret sector.
 * Helps understand why no path is found.
 */
static void DebugPrintSecretLinedefs(void) {
    int i, j;
    printf("[SECRET DEBUG] Scanning for secrets and their adjacent linedefs...\n");

    for (i = 0; i < numsectors; i++) {
        if (sectors[i].special == 9) {
            printf("[SECRET DEBUG] Secret sector %d found (floor=%d, ceil=%d)\n",
                   i, sectors[i].floorheight >> 16, sectors[i].ceilingheight >> 16);

            /* Find all linedefs touching this sector */
            for (j = 0; j < numlines; j++) {
                line_t *line = &lines[j];
                int is_front = (line->frontsector && (line->frontsector - sectors) == i);
                int is_back = (line->backsector && (line->backsector - sectors) == i);

                if (is_front || is_back) {
                    int other = -1;
                    if (is_front && line->backsector) {
                        other = line->backsector - sectors;
                    } else if (is_back && line->frontsector) {
                        other = line->frontsector - sectors;
                    }

                    /* Show why CanPassLine might fail */
                    if (line->frontsector && line->backsector) {
                        fixed_t f_floor = line->frontsector->floorheight;
                        fixed_t f_ceil = line->frontsector->ceilingheight;
                        fixed_t b_floor = line->backsector->floorheight;
                        fixed_t b_ceil = line->backsector->ceilingheight;
                        fixed_t high_floor = (f_floor > b_floor) ? f_floor : b_floor;
                        fixed_t low_ceil = (f_ceil < b_ceil) ? f_ceil : b_ceil;
                        fixed_t opening = (low_ceil - high_floor) >> 16;

                        printf("  Line %d: special=%d, flags=%d, other=%d, opening=%d units, passable=%s\n",
                               j, line->special, line->flags, other, (int)opening,
                               CanPassLine(line) ? "YES" : "NO");
                    } else {
                        printf("  Line %d: special=%d, flags=%d, other=%d, one-sided\n",
                               j, line->special, line->flags, other);
                    }
                }
            }
        }
    }

    /* Also show reachable sector count */
    printf("[SECRET DEBUG] Adjacency graph has %d total connections\n", s_adjacency_total);
}

/**
 * BFS to find path to sector adjacent to a hidden door leading to secret.
 * Used when direct path to secret is not available.
 */
static boolean BFSFindHiddenDoor(int start_sector, secret_path_t *out_path, int *out_door_line) {
    bfs_node_t *queue;
    boolean *visited;
    int queue_head = 0;
    int queue_tail = 0;
    int i, current, adj_idx, neighbor;
    int found_door_sector = -1;
    int found_door_line = -1;

    if (start_sector < 0 || !s_adjacency_list) {
        return false;
    }

    /* Allocate BFS structures */
    queue = (bfs_node_t*)malloc(numsectors * sizeof(bfs_node_t));
    visited = (boolean*)calloc(numsectors, sizeof(boolean));

    if (!queue || !visited) {
        if (queue) free(queue);
        if (visited) free(visited);
        return false;
    }

    /* Initialize BFS from start sector */
    queue[queue_tail].sector = start_sector;
    queue[queue_tail].parent = -1;
    queue_tail++;
    visited[start_sector] = true;

    /* BFS loop - look for sectors with hidden doors to secrets */
    while (queue_head < queue_tail) {
        current = queue[queue_head].sector;

        /* Check if this sector has a hidden door to a secret */
        int door_line = FindHiddenDoorToSecret(current);
        if (door_line >= 0) {
            found_door_sector = queue_head;
            found_door_line = door_line;
            break;
        }

        /* Explore adjacent sectors */
        for (i = 0; i < s_adjacency_count[current]; i++) {
            adj_idx = s_adjacency_offset[current] + i;
            neighbor = s_adjacency_list[adj_idx];

            if (neighbor >= 0 && neighbor < numsectors && !visited[neighbor]) {
                /* Skip connections blocked by 1-sided walls */
                if (IsConnectionBlocked(current, neighbor)) {
                    continue;
                }
                visited[neighbor] = true;
                queue[queue_tail].sector = neighbor;
                queue[queue_tail].parent = queue_head;
                queue_tail++;
            }
        }

        queue_head++;
    }

    /* Reconstruct path if hidden door found */
    if (found_door_sector >= 0) {
        int path_idx = 0;
        int trace = found_door_sector;
        int temp_path[SECRET_MAX_PATH_LENGTH];

        /* Trace back from door sector to start */
        while (trace >= 0 && path_idx < SECRET_MAX_PATH_LENGTH) {
            temp_path[path_idx++] = queue[trace].sector;
            trace = queue[trace].parent;
        }

        /* Reverse path (start to door) */
        out_path->path_length = path_idx;
        out_path->target_sector = queue[found_door_sector].sector;
        for (i = 0; i < path_idx; i++) {
            out_path->path[i] = temp_path[path_idx - 1 - i];
        }
        out_path->valid = true;
        *out_door_line = found_door_line;

        free(queue);
        free(visited);
        return true;
    }

    free(queue);
    free(visited);
    return false;
}

/* Track if current path leads to hidden door (not direct to secret) */
static boolean s_path_to_hidden_door = false;
static int s_hidden_door_line = -1;

/* Track if current path is blocked by an impassable wall */
static boolean s_path_blocked = false;
static int s_blocking_wall_line = -1;  /* Line index of blocking wall */
static int s_blocking_check_counter = 0;  /* Counter for periodic rechecks */
#define BLOCKING_RECHECK_INTERVAL 35  /* Check every ~1 second (35 tics) */

/* Note: 1-sided wall blocking no longer triggers recalculation.
 * Instead, we keep partial path to guide player toward secret. */

/**
 * Get the center point of a linedef (for hidden door marker).
 */
static void GetLinedefCenter(int line_idx, fixed_t *out_x, fixed_t *out_y, fixed_t *out_z) {
    line_t *line;

    if (line_idx < 0 || line_idx >= numlines) {
        *out_x = 0;
        *out_y = 0;
        *out_z = 0;
        return;
    }

    line = &lines[line_idx];
    *out_x = (line->v1->x + line->v2->x) / 2;
    *out_y = (line->v1->y + line->v2->y) / 2;

    /* Use floor height of front sector */
    if (line->frontsector) {
        *out_z = line->frontsector->floorheight;
    } else {
        *out_z = 0;
    }
}

/* Spacing between arrow sprites in map units */
#define ARROW_SPACING (96 * FRACUNIT)

/* Minimum distance from walls (32 units = player radius) */
#define WALL_MARGIN (32 * FRACUNIT)

/**
 * Calculate squared distance from point to line segment.
 * Returns the closest point on the segment in out_closest_x/y.
 */
static fixed_t PointToLineDistSq(fixed_t px, fixed_t py,
                                  fixed_t x1, fixed_t y1,
                                  fixed_t x2, fixed_t y2,
                                  fixed_t *out_closest_x, fixed_t *out_closest_y) {
    fixed_t dx = x2 - x1;
    fixed_t dy = y2 - y1;
    fixed_t len_sq = FixedMul(dx, dx) + FixedMul(dy, dy);
    fixed_t t;
    fixed_t closest_x, closest_y;
    fixed_t dist_x, dist_y;

    if (len_sq == 0) {
        /* Degenerate line (point) */
        closest_x = x1;
        closest_y = y1;
    } else {
        /* Project point onto line, clamped to segment */
        t = FixedDiv(FixedMul(px - x1, dx) + FixedMul(py - y1, dy), len_sq);
        if (t < 0) t = 0;
        if (t > FRACUNIT) t = FRACUNIT;

        closest_x = x1 + FixedMul(t, dx);
        closest_y = y1 + FixedMul(t, dy);
    }

    if (out_closest_x) *out_closest_x = closest_x;
    if (out_closest_y) *out_closest_y = closest_y;

    dist_x = px - closest_x;
    dist_y = py - closest_y;

    return FixedMul(dist_x, dist_x) + FixedMul(dist_y, dist_y);
}

/**
 * Check if a line is a wall (blocking) for path avoidance purposes.
 */
static boolean IsWallLine(line_t *line) {
    /* One-sided line = solid wall */
    if (!line->backsector) {
        return true;
    }

    /* Two-sided but impassable (blocking flag without door/lift special) */
    if (line->flags & ML_BLOCKING) {
        if (!IsDoorSpecial(line->special) && !IsLiftSpecial(line->special)) {
            return true;
        }
    }

    /* Check for physical blocking (floor/ceiling gap) */
    if (line->frontsector && line->backsector) {
        fixed_t front_floor = line->frontsector->floorheight;
        fixed_t back_floor = line->backsector->floorheight;
        fixed_t front_ceil = line->frontsector->ceilingheight;
        fixed_t back_ceil = line->backsector->ceilingheight;
        fixed_t high_floor = (front_floor > back_floor) ? front_floor : back_floor;
        fixed_t low_ceil = (front_ceil < back_ceil) ? front_ceil : back_ceil;
        fixed_t opening = low_ceil - high_floor;

        /* If opening is too small to pass, treat as wall */
        if (opening < 56 * FRACUNIT) {
            /* But doors/lifts can open, so skip them */
            if (!IsDoorSpecial(line->special) && !IsLiftSpecial(line->special)) {
                return true;
            }
        }
    }

    return false;
}

/**
 * Push a single arrow point away from nearby walls.
 * Modifies arrow->x and arrow->y in place.
 */
static void PushArrowFromWalls(secret_arrow_t *arrow) {
    int i;
    fixed_t px = arrow->x;
    fixed_t py = arrow->y;
    fixed_t push_x = 0, push_y = 0;
    int push_count = 0;
    fixed_t margin_sq = FixedMul(WALL_MARGIN, WALL_MARGIN);

    /* Scan all lines for nearby walls */
    for (i = 0; i < numlines; i++) {
        line_t *line = &lines[i];
        fixed_t closest_x, closest_y;
        fixed_t dist_sq;
        fixed_t dist;
        fixed_t push_dir_x, push_dir_y;
        fixed_t push_len;
        fixed_t push_strength;

        if (!IsWallLine(line)) {
            continue;
        }

        /* Calculate distance to this wall */
        dist_sq = PointToLineDistSq(px, py,
                                     line->v1->x, line->v1->y,
                                     line->v2->x, line->v2->y,
                                     &closest_x, &closest_y);

        /* Skip if too far */
        if (dist_sq > margin_sq) {
            continue;
        }

        /* Calculate push direction (away from wall) */
        push_dir_x = px - closest_x;
        push_dir_y = py - closest_y;
        dist = P_AproxDistance(push_dir_x, push_dir_y);

        if (dist < FRACUNIT) {
            /* Point is on the wall - use line normal */
            fixed_t ldx = line->v2->x - line->v1->x;
            fixed_t ldy = line->v2->y - line->v1->y;
            /* Normal is perpendicular to line: (-dy, dx) or (dy, -dx) */
            push_dir_x = -ldy;
            push_dir_y = ldx;
            dist = P_AproxDistance(push_dir_x, push_dir_y);
            if (dist < FRACUNIT) dist = FRACUNIT;
        }

        /* Normalize and scale push by how close we are */
        /* Closer = stronger push */
        push_strength = WALL_MARGIN - P_AproxDistance(px - closest_x, py - closest_y);
        if (push_strength < 0) push_strength = 0;
        if (push_strength > WALL_MARGIN) push_strength = WALL_MARGIN;

        /* Accumulate push */
        push_len = FixedDiv(push_strength, dist);
        push_x += FixedMul(push_dir_x, push_len);
        push_y += FixedMul(push_dir_y, push_len);
        push_count++;
    }

    /* Apply accumulated push */
    if (push_count > 0) {
        arrow->x = px + push_x;
        arrow->y = py + push_y;
    }
}

/**
 * Push all arrows in a path away from nearby walls.
 * Call this after generating all arrows.
 */
static void PushPathFromWalls(secret_path_t *path) {
    int i;

    for (i = 0; i < path->num_arrows; i++) {
        PushArrowFromWalls(&path->arrows[i]);
    }
}

/**
 * Add intermediate arrows between two points at regular intervals.
 * Internal function - does not check for wall collisions.
 * Returns number of arrows added.
 */
static int AddIntermediateArrowsDirect(secret_path_t *path, int start_idx,
                                        fixed_t x1, fixed_t y1, fixed_t z1,
                                        fixed_t x2, fixed_t y2, fixed_t z2) {
    fixed_t dx = x2 - x1;
    fixed_t dy = y2 - y1;
    fixed_t dist = P_AproxDistance(dx, dy);
    fixed_t dz = (z2 > z1) ? (z2 - z1) : (z1 - z2);
    int num_segments;
    int added = 0;
    int i;
    fixed_t prev_z = z1;
    angle_t arrow_angle = CalcAngle(x1, y1, x2, y2);

    /* Height change threshold for inserting extra sprites (16 units) */
    const fixed_t HEIGHT_THRESHOLD = 16 * FRACUNIT;

    /* Check if we need sprites based on horizontal distance OR height change */
    if (dist < ARROW_SPACING && dz < HEIGHT_THRESHOLD) {
        return 0;  /* Points are close enough AND no significant height change */
    }

    /* Calculate segments based on both horizontal distance and height change */
    int horiz_segments = (dist > 0) ? ((dist + ARROW_SPACING - 1) / ARROW_SPACING) : 0;
    int vert_segments = (dz > 0) ? ((dz + HEIGHT_THRESHOLD - 1) / HEIGHT_THRESHOLD) : 0;
    
    /* Use the larger of the two to ensure coverage */
    num_segments = (horiz_segments > vert_segments) ? horiz_segments : vert_segments;
    if (num_segments < 2) num_segments = 2;

    /* Add intermediate points (skip first point, include last) */
    for (i = 1; i <= num_segments && (start_idx + added) < SECRET_MAX_ARROWS; i++) {
        fixed_t t = (i * FRACUNIT) / num_segments;
        fixed_t point_x = x1 + FixedMul(dx, t);
        fixed_t point_y = y1 + FixedMul(dy, t);
        fixed_t point_z;

        /* Get actual floor height at this position instead of linear interpolation.
         * This handles stairs and height changes correctly. */
        int sector_idx = GetSectorAt(point_x, point_y);
        if (sector_idx >= 0) {
            point_z = sectors[sector_idx].floorheight;
        } else {
            /* Fallback to linear interpolation if sector lookup fails */
            point_z = z1 + FixedMul(z2 - z1, t);
        }

        /* Check for significant height change and insert extra sprites */
        fixed_t height_diff = (point_z > prev_z) ? (point_z - prev_z) : (prev_z - point_z);
        if (height_diff > HEIGHT_THRESHOLD && i > 1 && (start_idx + added + 1) < SECRET_MAX_ARROWS) {
            /* Insert intermediate sprite at height transition point */
            /* Sample midpoint between previous and current position */
            fixed_t mid_t = ((i - 1) * FRACUNIT + t) / 2;
            fixed_t mid_x = x1 + FixedMul(dx, mid_t);
            fixed_t mid_y = y1 + FixedMul(dy, mid_t);
            fixed_t mid_z;

            int mid_sector = GetSectorAt(mid_x, mid_y);
            if (mid_sector >= 0) {
                mid_z = sectors[mid_sector].floorheight;
            } else {
                mid_z = (prev_z + point_z) / 2;
            }

            /* Add extra sprite at transition */
            path->arrows[start_idx + added].x = mid_x;
            path->arrows[start_idx + added].y = mid_y;
            path->arrows[start_idx + added].z = mid_z;
            path->arrows[start_idx + added].angle = arrow_angle;
            added++;

            /* If height change is very large (stairs), add more intermediate points */
            if (height_diff > HEIGHT_THRESHOLD * 2 && (start_idx + added + 1) < SECRET_MAX_ARROWS) {
                /* Add another sprite at 3/4 position */
                fixed_t quarter_t = ((i - 1) * FRACUNIT * 3 + t) / 4;
                fixed_t quarter_x = x1 + FixedMul(dx, quarter_t);
                fixed_t quarter_y = y1 + FixedMul(dy, quarter_t);
                fixed_t quarter_z;

                int quarter_sector = GetSectorAt(quarter_x, quarter_y);
                if (quarter_sector >= 0) {
                    quarter_z = sectors[quarter_sector].floorheight;
                } else {
                    quarter_z = (mid_z + point_z) / 2;
                }

                path->arrows[start_idx + added].x = quarter_x;
                path->arrows[start_idx + added].y = quarter_y;
                path->arrows[start_idx + added].z = quarter_z;
                path->arrows[start_idx + added].angle = arrow_angle;
                added++;
            }
        }

        /* Add the regular sprite */
        if ((start_idx + added) < SECRET_MAX_ARROWS) {
            path->arrows[start_idx + added].x = point_x;
            path->arrows[start_idx + added].y = point_y;
            path->arrows[start_idx + added].z = point_z;
            path->arrows[start_idx + added].angle = arrow_angle;
            added++;
        }

        prev_z = point_z;
    }

    return added;
}

/**
 * Add intermediate arrows between two points, avoiding walls.
 * If the direct path crosses a wall, finds waypoints to go around it.
 * Uses recursion with depth limit to handle multiple walls.
 * Returns number of arrows added.
 */
static int AddIntermediateArrows(secret_path_t *path, int start_idx,
                                  fixed_t x1, fixed_t y1, fixed_t z1,
                                  fixed_t x2, fixed_t y2, fixed_t z2) {
    int added = 0;
    int depth = 0;
    const int MAX_DEPTH = 8;  /* Prevent infinite recursion */

    /* Waypoint stack for iterative processing */
    struct {
        fixed_t x, y, z;
    } waypoints[MAX_DEPTH + 2];
    int num_waypoints = 0;

    /* Start with destination as first target */
    waypoints[num_waypoints].x = x2;
    waypoints[num_waypoints].y = y2;
    waypoints[num_waypoints].z = z2;
    num_waypoints++;

    fixed_t curr_x = x1;
    fixed_t curr_y = y1;
    fixed_t curr_z = z1;

    /* Process waypoints, finding intermediate points around walls */
    while (num_waypoints > 0 && depth < MAX_DEPTH && (start_idx + added) < SECRET_MAX_ARROWS) {
        fixed_t target_x = waypoints[num_waypoints - 1].x;
        fixed_t target_y = waypoints[num_waypoints - 1].y;
        fixed_t target_z = waypoints[num_waypoints - 1].z;

        /* Check if direct path is clear */
        if (CheckLineOfSight(curr_x, curr_y, target_x, target_y)) {
            /* Path is clear - add arrows directly */
            SECRET_DEBUG("[SECRET] LOS clear: (%d,%d) -> (%d,%d)\n",
                   curr_x >> FRACBITS, curr_y >> FRACBITS,
                   target_x >> FRACBITS, target_y >> FRACBITS);
            added += AddIntermediateArrowsDirect(path, start_idx + added,
                                                  curr_x, curr_y, curr_z,
                                                  target_x, target_y, target_z);
            curr_x = target_x;
            curr_y = target_y;
            curr_z = target_z;
            num_waypoints--;  /* Remove processed waypoint */
        } else {
            /* Path blocked - find waypoint around wall */
            line_t *wall = GetBlockingLine();
            SECRET_DEBUG("[SECRET] LOS BLOCKED: (%d,%d) -> (%d,%d), wall=%p\n",
                   curr_x >> FRACBITS, curr_y >> FRACBITS,
                   target_x >> FRACBITS, target_y >> FRACBITS, (void*)wall);
            if (wall) {
                SECRET_DEBUG("[SECRET]   Wall vertices: (%d,%d) -> (%d,%d)\n",
                       wall->v1->x >> FRACBITS, wall->v1->y >> FRACBITS,
                       wall->v2->x >> FRACBITS, wall->v2->y >> FRACBITS);
            }
            if (wall && num_waypoints < MAX_DEPTH + 1) {
                fixed_t wp_x, wp_y;
                FindWaypointAroundWall(wall, curr_x, curr_y, target_x, target_y, &wp_x, &wp_y);

                /* Check if waypoint is same as current position (stuck in loop) */
                fixed_t diff_x = (wp_x > curr_x) ? (wp_x - curr_x) : (curr_x - wp_x);
                fixed_t diff_y = (wp_y > curr_y) ? (wp_y - curr_y) : (curr_y - wp_y);
                boolean is_stuck = (diff_x < (8 * FRACUNIT) && diff_y < (8 * FRACUNIT));

                /* Also check if waypoint doesn't help (same side of wall as current) */
                if (!is_stuck && depth > 0) {
                    /* Check if new waypoint would make progress toward target */
                    fixed_t old_dist = P_AproxDistance(curr_x - target_x, curr_y - target_y);
                    fixed_t new_dist = P_AproxDistance(wp_x - target_x, wp_y - target_y);
                    if (new_dist >= old_dist) {
                        is_stuck = true;  /* Not making progress */
                    }
                }

                if (is_stuck) {
                    /* Waypoint doesn't help - check if wall is impassable or openable */
                    if (IsImpassableWall(wall)) {
                        /* 1-sided wall blocking our direct path to target.
                         * Instead of stopping, skip to the next waypoint and continue.
                         * The BFS path is valid, we just can't draw a direct line here. */
                        SECRET_DEBUG("[SECRET]   1-sided wall at (%d,%d), skipping direct path to this waypoint\n",
                               wp_x >> FRACBITS, wp_y >> FRACBITS);

                        /* Skip this waypoint - move to current position closer to waypoint */
                        /* Don't try to draw through the wall, just accept we can't reach this intermediate point directly */
                        num_waypoints--;

                        /* If this was the last waypoint (final target), mark as blocked for polling */
                        if (num_waypoints == 0) {
                            SECRET_DEBUG("[SECRET]   Final target blocked by 1-sided wall, storing for recheck\n");
                            s_path_blocked = true;
                            s_blocking_wall_line = wall - lines;
                            SECRET_DEBUG("[SECRET]   Stored blocking wall line=%d for recheck\n",
                                   s_blocking_wall_line);
                        }
                    } else {
                        /* Door/lift that can be opened - draw direct line through it */
                        SECRET_DEBUG("[SECRET]   Openable door/lift at (%d,%d), drawing through\n",
                               wp_x >> FRACBITS, wp_y >> FRACBITS);
                        added += AddIntermediateArrowsDirect(path, start_idx + added,
                                                              curr_x, curr_y, curr_z,
                                                              target_x, target_y, target_z);
                        curr_x = target_x;
                        curr_y = target_y;
                        curr_z = target_z;
                        num_waypoints--;
                    }
                } else {
                    SECRET_DEBUG("[SECRET]   Adding waypoint around wall: (%d,%d), depth=%d\n",
                           wp_x >> FRACBITS, wp_y >> FRACBITS, depth);

                    /* Insert new waypoint before current target */
                    waypoints[num_waypoints].x = wp_x;
                    waypoints[num_waypoints].y = wp_y;
                    waypoints[num_waypoints].z = curr_z;  /* Use current Z */
                    num_waypoints++;
                    depth++;
                }
            } else {
                /* No wall info or max depth - force direct path */
                SECRET_DEBUG("[SECRET]   Forcing direct path (no wall or max depth)\n");
                added += AddIntermediateArrowsDirect(path, start_idx + added,
                                                      curr_x, curr_y, curr_z,
                                                      target_x, target_y, target_z);
                curr_x = target_x;
                curr_y = target_y;
                curr_z = target_z;
                num_waypoints--;
            }
        }
    }

    return added;
}

/**
 * Generate arrow waypoints from sector path.
 * Uses portal waypoints (linedef center pushed into destination sector) to ensure
 * the path goes through actual walkable doorways/openings, not through walls.
 */
static void GenerateArrows(secret_path_t *path) {
    int i;
    fixed_t prev_x, prev_y, prev_z;
    fixed_t curr_x, curr_y, curr_z;
    fixed_t portal_x, portal_y;
    int arrow_count = 0;
    player_t *player = &players[consoleplayer];
    int prev_sector;

    /* Save previous blocking wall to preserve counter if same wall blocks again */
    int prev_blocking_wall = s_blocking_wall_line;
    int prev_counter = s_blocking_check_counter;

    /* Reset blocking state before generating new arrows.
     * If we hit a blocking wall during generation, it will be set again. */
    s_path_blocked = false;
    s_blocking_wall_line = -1;
    /* Don't reset counter here - will restore if same wall blocks */

    if (path->path_length < 1) {
        path->num_arrows = 0;
        return;
    }

    /* Start from player position */
    if (player->mo) {
        prev_x = player->mo->x;
        prev_y = player->mo->y;
        prev_z = player->mo->z;
        SECRET_DEBUG("[SECRET] GenerateArrows: Starting from player pos (%d, %d)\n",
               prev_x >> FRACBITS, prev_y >> FRACBITS);
    } else {
        GetSectorCenter(path->path[0], &prev_x, &prev_y);
        prev_z = sectors[path->path[0]].floorheight;
        SECRET_DEBUG("[SECRET] GenerateArrows: No player->mo, using sector center (%d, %d)\n",
               prev_x >> FRACBITS, prev_y >> FRACBITS);
    }
    prev_sector = path->path[0];

    /* Add first arrow at player/start position */
    if (arrow_count < SECRET_MAX_ARROWS) {
        path->arrows[arrow_count].x = prev_x;
        path->arrows[arrow_count].y = prev_y;
        path->arrows[arrow_count].z = prev_z;
        path->arrows[arrow_count].angle = 0;  /* Will be updated below */
        arrow_count++;
    }

    /* Generate arrows along path using portal waypoints */
    for (i = 1; i < path->path_length && arrow_count < SECRET_MAX_ARROWS; i++) {
        int sector_idx = path->path[i];
        boolean is_last = (i == path->path_length - 1);
        int connecting_line;

        /* For last segment, go to the actual target position */
        if (is_last) {
            /* Use target coordinates if available, otherwise use linedef center for triggers */
            if (path->target_x != 0 || path->target_y != 0) {
                curr_x = path->target_x;
                curr_y = path->target_y;
                curr_z = sectors[sector_idx].floorheight;
            } else if (s_path_to_hidden_door && s_hidden_door_line >= 0) {
                GetLinedefCenter(s_hidden_door_line, &curr_x, &curr_y, &curr_z);
            } else {
                GetSectorCenter(sector_idx, &curr_x, &curr_y);
                curr_z = sectors[sector_idx].floorheight;
            }

            /* Check if direct path is clear, if not go through portal first */
            if (!CheckLineOfSight(prev_x, prev_y, curr_x, curr_y)) {
                connecting_line = FindConnectingLine(prev_sector, sector_idx);
                if (connecting_line >= 0) {
                    /* Add portal waypoint first */
                    GetPortalWaypoint(connecting_line, sector_idx, &portal_x, &portal_y);
                    arrow_count += AddIntermediateArrows(path, arrow_count,
                                                          prev_x, prev_y, prev_z,
                                                          portal_x, portal_y, sectors[sector_idx].floorheight);
                    prev_x = portal_x;
                    prev_y = portal_y;
                    prev_z = sectors[sector_idx].floorheight;
                }
            }
        } else {
            /* Find the linedef connecting previous sector to current sector */
            connecting_line = FindConnectingLine(prev_sector, sector_idx);
            if (connecting_line >= 0) {
                /* Use portal waypoint (pushed into destination sector) */
                GetPortalWaypoint(connecting_line, sector_idx, &curr_x, &curr_y);
                curr_z = sectors[sector_idx].floorheight;

                /* Validate: check if line of sight is clear */
                if (!CheckLineOfSight(prev_x, prev_y, curr_x, curr_y)) {
                    /* Line blocked - try linedef center as intermediate point */
                    GetLineMidpoint(connecting_line, &portal_x, &portal_y);

                    /* Go to linedef center first */
                    arrow_count += AddIntermediateArrows(path, arrow_count,
                                                          prev_x, prev_y, prev_z,
                                                          portal_x, portal_y, curr_z);
                    prev_x = portal_x;
                    prev_y = portal_y;
                }
            } else {
                /* Fallback to sector center if no connecting line found */
                GetSectorCenter(sector_idx, &curr_x, &curr_y);
                curr_z = sectors[sector_idx].floorheight;
            }
        }

        /* Add arrows to current waypoint */
        arrow_count += AddIntermediateArrows(path, arrow_count,
                                              prev_x, prev_y, prev_z,
                                              curr_x, curr_y, curr_z);

        prev_x = curr_x;
        prev_y = curr_y;
        prev_z = curr_z;
        prev_sector = sector_idx;
    }

    /* Handle path_length == 1 case: player is in target sector but need arrows to target point */
    if (path->path_length == 1 && (path->target_x != 0 || path->target_y != 0)) {
        fixed_t target_z = sectors[path->path[0]].floorheight;
        SECRET_DEBUG("[SECRET] GenerateArrows: path_length=1, adding direct arrows to target (%d, %d)\n",
               path->target_x >> FRACBITS, path->target_y >> FRACBITS);
        arrow_count += AddIntermediateArrowsDirect(path, arrow_count,
                                                    prev_x, prev_y, prev_z,
                                                    path->target_x, path->target_y, target_z);
    }

    path->num_arrows = arrow_count;

    /* Update first arrow's angle to point to second arrow (if exists) */
    if (arrow_count >= 2) {
        path->arrows[0].angle = CalcAngle(path->arrows[0].x, path->arrows[0].y,
                                           path->arrows[1].x, path->arrows[1].y);
    }

    /* Push arrows away from walls to avoid clipping */
    PushPathFromWalls(path);

    /* If blocked by same wall as before, preserve the counter to allow polling */
    if (s_path_blocked && s_blocking_wall_line == prev_blocking_wall && prev_blocking_wall >= 0) {
        s_blocking_check_counter = prev_counter;
        SECRET_DEBUG("[SECRET] Same blocking wall line=%d, preserved counter=%d\n",
               s_blocking_wall_line, s_blocking_check_counter);
    } else if (s_path_blocked) {
        /* New blocking wall - reset counter */
        s_blocking_check_counter = 0;
        SECRET_DEBUG("[SECRET] New blocking wall line=%d, reset counter\n", s_blocking_wall_line);
    }

    /* For 1-sided wall blocks, just mark as blocked and keep partial path.
     * Don't trigger BFS recalculation - the partial path guides toward the secret.
     * The arrows generated up to the blocking point show the best route we found. */
    if (s_path_blocked && s_blocking_wall_line >= 0) {
        line_t *blocking_line = &lines[s_blocking_wall_line];

        /* 1-sided wall means it permanently blocks - record for adjacency graph but don't recalc */
        if (!blocking_line->backsector) {
            int line_front_sector = blocking_line->frontsector - sectors;

            /* Find this sector in the path and block connection for future BFS attempts */
            for (i = 0; i < path->path_length - 1; i++) {
                int curr_sector = path->path[i];
                int next_sector = path->path[i + 1];

                /* Check if blocking line's frontsector is involved in this path segment */
                if (curr_sector == line_front_sector || next_sector == line_front_sector) {
                    AddBlockedConnection(curr_sector, next_sector, s_blocking_wall_line);
                    SECRET_DEBUG("[SECRET] 1-sided wall blocks path segment %d -> %d, keeping %d arrows as partial path\n",
                           curr_sector, next_sector, path->num_arrows);
                    /* Don't recalc - keep partial path that guides toward secret */
                    break;
                }
            }

            /* If line_front_sector not in path, block based on proximity to target */
            if (path->path_length >= 2) {
                boolean already_blocked = false;
                for (i = 0; i < path->path_length - 1; i++) {
                    if (path->path[i] == line_front_sector || path->path[i + 1] == line_front_sector) {
                        already_blocked = true;
                        break;
                    }
                }
                if (!already_blocked) {
                    /* Block the last segment as it's likely where we're trying to reach */
                    int last_sector = path->path[path->path_length - 1];
                    int prev_sect = path->path[path->path_length - 2];
                    AddBlockedConnection(prev_sect, last_sector, s_blocking_wall_line);
                    SECRET_DEBUG("[SECRET] 1-sided wall blocks final segment %d -> %d (proximity), keeping %d arrows\n",
                           prev_sect, last_sector, path->num_arrows);
                    /* Don't recalc - keep partial path */
                }
            }
        }
    }
}

/**
 * Remove all spawned hint sprites.
 */
static void RemoveHintSprites(void) {
    int i;

    for (i = 0; i < s_num_sprites; i++) {
        if (s_hint_sprites[i]) {
            P_RemoveMobj(s_hint_sprites[i]);
            s_hint_sprites[i] = NULL;
        }
    }
    s_num_sprites = 0;
}

/**
 * Spawn plasma ball sprites at all arrow waypoints.
 */
static void SpawnHintSprites(secret_path_t *path) {
    int i;
    mobj_t *mobj;
    fixed_t height_offset = 16 * FRACUNIT;  /* Float 16 units above floor */

    /* Remove any existing sprites first */
    RemoveHintSprites();

    if (!path || path->num_arrows == 0) {
        return;
    }

    /* Log first sprite position */
    SECRET_DEBUG("[SECRET] SpawnHintSprites: First arrow at (%d, %d), total %d arrows\n",
           path->arrows[0].x >> FRACBITS, path->arrows[0].y >> FRACBITS, path->num_arrows);

    for (i = 0; i < path->num_arrows && i < SECRET_MAX_ARROWS; i++) {
        /* Spawn an imp fireball at this waypoint (SPR_BAL1 - always available) */
        mobj = P_SpawnMobj(
            path->arrows[i].x,
            path->arrows[i].y,
            path->arrows[i].z + height_offset,
            MT_TROOPSHOT
        );

        if (mobj) {
            /* Make it a static decoration (not a dangerous missile) */
            /* Clear missile flag so it doesn't explode on contact */
            mobj->flags &= ~MF_MISSILE;
            /* Keep NOBLOCKMAP and NOGRAVITY so it floats and doesn't block */
            mobj->flags |= MF_NOBLOCKMAP | MF_NOGRAVITY;
            /* Zero out velocity so it stays in place */
            mobj->momx = 0;
            mobj->momy = 0;
            mobj->momz = 0;

            s_hint_sprites[i] = mobj;
            s_num_sprites++;
        }
    }

}

/* Public API Implementation */

void Secret_Init(void) {
    memset(&s_current_path, 0, sizeof(s_current_path));
    memset(s_hint_sprites, 0, sizeof(s_hint_sprites));
    s_num_sprites = 0;
    s_path_active = false;
    /* Note: BuildSectorAdjacency() is called from Secret_OnLevelLoad()
       because sectors aren't loaded during initial SCE_Init() */
}

void Secret_OnLevelLoad(void) {
    int i;

    /* Clean up any existing sprites (old level's objects are invalid now) */
    memset(s_hint_sprites, 0, sizeof(s_hint_sprites));
    s_num_sprites = 0;

    /* Rebuild sector adjacency when a new level is loaded */
    s_adjacency_dirty = true;
    EnsureAdjacencyValid();
    s_path_active = false;
    s_last_player_sector = -1;  /* Reset sector tracking */
    s_last_player_x = 0;        /* Reset position tracking */
    s_last_player_y = 0;
    memset(&s_current_path, 0, sizeof(s_current_path));

    /* Clear blocked connections from previous level */
    ClearBlockedConnections();
    s_path_blocked = false;
    s_blocking_wall_line = -1;
    s_blocking_check_counter = 0;

    /* Save original secret sectors (before any are discovered) */
    s_original_secret_count = 0;
    for (i = 0; i < numsectors && s_original_secret_count < SECRET_MAX_TARGETS; i++) {
        if (sectors[i].special == 9) {
            s_original_secret_sectors[s_original_secret_count] = i;
            s_original_secret_count++;
        }
    }
    SECRET_DEBUG("[SECRET] Found %d original secret sectors\n", s_original_secret_count);

    /* Scan for all targets (secrets + triggers) */
    Secret_ScanTargets();
}

void Secret_Cleanup(void) {
    RemoveHintSprites();

    if (s_adjacency_list) {
        free(s_adjacency_list);
        s_adjacency_list = NULL;
    }
    if (s_adjacency_offset) {
        free(s_adjacency_offset);
        s_adjacency_offset = NULL;
    }
    if (s_adjacency_count) {
        free(s_adjacency_count);
        s_adjacency_count = NULL;
    }
    s_adjacency_total = 0;
    s_path_active = false;
}

boolean Secret_FindPath(secret_path_t *out_path) {
    player_t *player = &players[consoleplayer];
    int player_sector;

    /* Reset hidden door tracking */
    s_path_to_hidden_door = false;
    s_hidden_door_line = -1;

    if (!player->mo) {
        out_path->valid = false;
        return false;
    }

    /* Ensure adjacency graph reflects current sector heights */
    /* Force rebuild on path request to capture door/lift changes */
    s_adjacency_dirty = true;
    EnsureAdjacencyValid();

    if (!s_adjacency_list) {
        out_path->valid = false;
        return false;
    }

    /* Get player's current sector */
    player_sector = GetSectorAt(player->mo->x, player->mo->y);

    if (player_sector < 0) {
        out_path->valid = false;
        return false;
    }

    /* Find path to nearest secret (or closest reachable point if blocked) */
    if (BFSFindSecret(player_sector, out_path)) {
        /* Check if we reached an actual secret or just got as close as possible */
        boolean is_secret = (sectors[out_path->target_sector].special == 9);

        if (is_secret) {
            SECRET_DEBUG("[SECRET] Path to secret %d found (%d steps)\n",
                   out_path->target_sector, out_path->path_length);
        } else {
            SECRET_DEBUG("[SECRET] Partial path (%d steps) - closest reachable to secrets\n",
                   out_path->path_length);
        }

        GenerateArrows(out_path);
        memcpy(&s_current_path, out_path, sizeof(secret_path_t));
        s_path_active = true;
        s_path_to_hidden_door = false;

        /* Spawn plasma ball sprites along the path */
        SpawnHintSprites(&s_current_path);

        return true;
    }

    SECRET_DEBUG("[SECRET] No path available from sector %d\n", player_sector);

    DebugPrintSecretLinedefs();

    /* Debug: Show how to reach unreachable secret sectors */
    printf("[SECRET DEBUG] === HOW TO REACH SECRETS ===\n");
    for (int s = 0; s < numsectors; s++) {
        if (sectors[s].special == 9) {
            printf("[SECRET DEBUG] Secret sector %d neighbors:\n", s);
            for (int j = 0; j < numlines; j++) {
                line_t *line = &lines[j];
                int is_front = (line->frontsector && (line->frontsector - sectors) == s);
                int is_back = (line->backsector && (line->backsector - sectors) == s);
                if (is_front || is_back) {
                    int other = -1;
                    if (is_front && line->backsector) other = line->backsector - sectors;
                    else if (is_back && line->frontsector) other = line->frontsector - sectors;
                    if (other >= 0) {
                        printf("  -> Sector %d (floor=%d) via line %d (special=%d)\n",
                               other, sectors[other].floorheight >> 16, j, line->special);
                    }
                }
            }
        }
    }

    out_path->valid = false;
    return false;
}

const secret_path_t* Secret_GetCurrentPath(void) {
    if (s_path_active && s_current_path.valid) {
        return &s_current_path;
    }
    return NULL;
}

void Secret_ClearPath(void) {
    RemoveHintSprites();
    s_path_active = false;
    s_current_path.valid = false;
    /* Reset blocking wall state */
    s_path_blocked = false;
    s_blocking_wall_line = -1;
    s_blocking_check_counter = 0;
}

boolean Secret_CheckReached(void) {
    player_t *player = &players[consoleplayer];
    int player_sector;
    static int last_secret_count = -1;
    int current_secret_count;

    if (!player->mo) {
        return false;
    }

    /* Track secret count changes (detects ANY secret discovery, not just target) */
    current_secret_count = Secret_GetRemainingCount();
    if (last_secret_count < 0) {
        last_secret_count = current_secret_count;
    }

    /* Secret was discovered (count decreased) */
    if (current_secret_count < last_secret_count) {
        last_secret_count = current_secret_count;

        /* Keep path visible even after secret discovery */
        /* Path will only be cleared when user selects a different target */

        return true;  /* Signal that a secret was reached */
    }

    return false;
}

/**
 * Reset secret tracking state (call on level load).
 */
void Secret_ResetTracking(void) {
    /* Force re-initialization of secret count tracking on next check */
}

int Secret_GetRemainingCount(void) {
    int count = 0;
    int i;

    for (i = 0; i < numsectors; i++) {
        if (sectors[i].special == 9) {
            count++;
        }
    }

    return count;
}

/**
 * Recalculate BFS path from current sector to selected target.
 * Common function used by both sector change and position update paths.
 *
 * @param current_sector Player's current sector index
 * @param force_adjacency_rebuild If true, rebuild adjacency graph
 * @return true if path calculation succeeded (or kept existing path)
 */
static boolean RecalculatePathToTarget(int current_sector, boolean force_adjacency_rebuild) {
    target_info_t info;
    secret_path_t new_path;
    int target_sector;

    if (!Secret_GetCurrentTarget(&info)) {
        SECRET_DEBUG("[SECRET] RecalculatePath: GetCurrentTarget failed\n");
        return false;
    }

    /* Rebuild adjacency if requested */
    if (force_adjacency_rebuild) {
        s_adjacency_dirty = true;
    }
    EnsureAdjacencyValid();

    /* Determine target sector */
    if (info.type == TARGET_SECRET) {
        target_sector = info.index;
    } else {
        line_t *line = &lines[info.index];
        if (line->frontsector) {
            target_sector = line->frontsector - sectors;
        } else if (line->backsector) {
            target_sector = line->backsector - sectors;
        } else {
            target_sector = -1;
        }
    }

    SECRET_DEBUG("[SECRET] RecalculatePath: current=%d, target=%d, discovered=%d\n",
                 current_sector, target_sector, info.discovered);

    /* Special case: already at target sector */
    if (current_sector == target_sector) {
        SECRET_DEBUG("[SECRET] RecalculatePath: Already at target sector\n");
        if (s_current_path.valid && s_current_path.path_length > 0) {
            s_current_path.target_x = info.x;
            s_current_path.target_y = info.y;
            GenerateArrows(&s_current_path);
            SpawnHintSprites(&s_current_path);
        }
        return true;
    }

    /* Run BFS to find path */
    if (target_sector >= 0 && BFSFindPath(current_sector, target_sector, &new_path)) {
        SECRET_DEBUG("[SECRET] RecalculatePath: path found, length=%d\n", new_path.path_length);

        /* Set target coordinates */
        new_path.target_x = info.x;
        new_path.target_y = info.y;

        /* Check if path leads to hidden door (for triggers) */
        if (info.type != TARGET_SECRET && new_path.target_sector == target_sector) {
            s_path_to_hidden_door = true;
            s_hidden_door_line = info.index;
        } else {
            s_path_to_hidden_door = false;
            s_hidden_door_line = -1;
        }

        GenerateArrows(&new_path);
        memcpy(&s_current_path, &new_path, sizeof(secret_path_t));
        SpawnHintSprites(&s_current_path);
        return true;
    }

    /* BFS failed - keep existing path if we have one */
    SECRET_DEBUG("[SECRET] RecalculatePath: BFS failed from %d to %d\n",
                 current_sector, target_sector);

    if (s_current_path.valid && s_current_path.num_arrows > 0) {
        SECRET_DEBUG("[SECRET] RecalculatePath: keeping existing path with %d arrows\n",
                     s_current_path.num_arrows);
        GenerateArrows(&s_current_path);
        SpawnHintSprites(&s_current_path);
        return true;  /* Treat as success to prevent retry spam */
    }

    SECRET_DEBUG("[SECRET] RecalculatePath: no existing path to keep\n");
    return true;  /* Still return true to prevent retry spam */
}

void Secret_UpdateArrows(void) {
    player_t *player = &players[consoleplayer];
    int current_sector;
    fixed_t dx, dy, dist_moved;
    const char *state;

    if (!player->mo) {
        return;
    }

    /* Check current SCXML state - only update in showing or no_path states */
    state = sce_secret_get_state();
    if (!state) {
        return;
    }

    /* Process updates in showing, found, or no_path states */
    boolean is_showing = (strcmp(state, "showing") == 0);
    boolean is_found = (strcmp(state, "found") == 0);
    boolean is_no_path = (strcmp(state, "no_path") == 0);

    if (!is_showing && !is_found && !is_no_path) {
        return;
    }

    /* Get player's current sector */
    current_sector = GetSectorAt(player->mo->x, player->mo->y);

    /* In no_path state, periodically retry path calculation.
     * Throttle to avoid BFS every frame (performance). */
    if (is_no_path) {
        static int no_path_throttle = 0;
        #define NO_PATH_RECALC_INTERVAL 5  /* ~140ms at 35 tics/sec */

        if (++no_path_throttle < NO_PATH_RECALC_INTERVAL) {
            return;
        }
        no_path_throttle = 0;

        /* Update position tracking even with invalid sector */
        s_last_player_x = player->mo->x;
        s_last_player_y = player->mo->y;
        if (current_sector >= 0) {
            s_last_player_sector = current_sector;
        }

        /* Trigger recalculation */
        ClearBlockedConnections();
        s_path_blocked = false;
        s_blocking_wall_line = -1;
        s_blocking_check_counter = 0;
        sce_secret_recalculate();
        return;
    }

    /* For showing/found states, require valid sector */
    if (current_sector < 0) {
        return;
    }

    /* Check if player moved to a different sector - need recalculation */
    boolean need_recalc = (current_sector != s_last_player_sector);

    /* Check if player moved enough to warrant update */
    dx = player->mo->x - s_last_player_x;
    dy = player->mo->y - s_last_player_y;
    dist_moved = P_AproxDistance(dx, dy);
    boolean need_arrow_update = (dist_moved >= PLAYER_MOVE_THRESHOLD);

    /* Polling: Check if a blocking wall has become passable (door/lift opened).
     * Skip polling for 1-sided walls since they can never open. */
    if (s_path_blocked && s_blocking_wall_line >= 0) {
        line_t *blocking_line = &lines[s_blocking_wall_line];

        /* 1-sided walls are permanently impassable - no need to poll */
        if (!blocking_line->backsector) {
            /* Keep showing partial path, no polling needed */
        } else {
            /* 2-sided wall (door/lift) - poll to check if it opened */
            s_blocking_check_counter++;
            if (s_blocking_check_counter >= BLOCKING_RECHECK_INTERVAL) {
                SECRET_DEBUG("[SECRET] Polling check for blocking wall line=%d (counter reached %d)\n",
                       s_blocking_wall_line, BLOCKING_RECHECK_INTERVAL);
                s_blocking_check_counter = 0;

                /* Check if blocking wall is still impassable using both methods:
                 * 1. IsImpassableWall - checks wall type (ML_BLOCKING, special)
                 * 2. P_LineOpening - checks actual vertical opening (detects open doors) */
                boolean is_still_blocked = true;
                boolean is_impassable = IsImpassableWall(blocking_line);

                if (!is_impassable) {
                    is_still_blocked = false;
                    SECRET_DEBUG("[SECRET]   IsImpassableWall returned false - wall is now passable\n");
                } else {
                    /* 2-sided line: check actual vertical opening */
                    P_LineOpening(blocking_line);
                    SECRET_DEBUG("[SECRET]   2-sided line: openrange=%d (need >%d)\n",
                           openrange >> FRACBITS, 40);
                    if (openrange > 40 * FRACUNIT) {  /* Enough space to walk through */
                        is_still_blocked = false;
                        SECRET_DEBUG("[SECRET]   Vertical opening sufficient - wall is now passable\n");
                    }
                }

                if (!is_still_blocked) {
                    /* Wall became passable (door/lift opened) - force recalculation */
                    SECRET_DEBUG("[SECRET] Blocking wall line=%d is now passable! Forcing recalculation\n",
                           s_blocking_wall_line);
                    s_path_blocked = false;
                    s_blocking_wall_line = -1;
                    need_recalc = true;
                    s_last_player_sector = -1;  /* Force recalc by invalidating sector cache */
                    s_adjacency_dirty = true;   /* Rebuild adjacency to include new connection */
                } else {
                    SECRET_DEBUG("[SECRET]   Wall still blocked, will check again in %d tics\n",
                           BLOCKING_RECHECK_INTERVAL);
                }
            }
        }
    }

    /* Debug: log sector changes */
    if (need_recalc) {
        SECRET_DEBUG("[SECRET] UpdateArrows: sector changed %d -> %d, triggering recalculation\n",
               s_last_player_sector, current_sector);
    }

    if (need_recalc || need_arrow_update) {
        /* Clear blocked connections when player moves.
         * This allows BFS to find new paths from the new position. */
        ClearBlockedConnections();
        s_path_blocked = false;
        s_blocking_wall_line = -1;
        s_blocking_check_counter = 0;

        /* Update position tracking */
        s_last_player_sector = current_sector;
        s_last_player_x = player->mo->x;
        s_last_player_y = player->mo->y;

        /* Trigger SCXML recalculate event - this will go to calculating state
         * and onCalculating callback will run BFS and update path */
        sce_secret_recalculate();
    }
}

void Secret_RenderArrows(void) {
    /* Placeholder for sprite-based arrow rendering */
    /* Will be implemented when we add arrow sprites */
}

boolean Secret_IsEnabled(void) {
    return s_enabled;
}

void Secret_SetEnabled(boolean enabled) {
    s_enabled = enabled;
    if (!enabled) {
        Secret_ClearPath();
    }
}

void Secret_PrepareForSave(void) {
    /* Remove hint sprites before save - they're mobj and would be serialized */
    RemoveHintSprites();
}

void Secret_RestoreAfterSave(void) {
    /* Respawn hint sprites after save if path is still active */
    if (s_path_active && s_current_path.valid && s_current_path.num_arrows > 0) {
        SpawnHintSprites(&s_current_path);
    }
}

boolean Secret_IsPathToHiddenDoor(void) {
    return s_path_to_hidden_door;
}

/* ============================================
 * Trigger Type Detection
 * ============================================ */

static boolean IsSwitchSpecial(int special) {
    /* General switches (S1 = once, SR = repeatable) */
    switch (special) {
    case 7:    /* S1 Build Stairs */
    case 9:    /* S1 Donut */
    case 14:   /* S1 Floor Raise */
    case 15:   /* S1 Floor Lower to Highest */
    case 18:   /* S1 Floor Raise to Next Higher */
    case 20:   /* S1 Floor Raise to Next Higher (texture) */
    case 21:   /* S1 Lift */
    case 22:   /* S1 Floor Raise to Next */
    case 23:   /* S1 Floor Lower to Lowest */
    case 29:   /* S1 Door Raise */
    case 41:   /* S1 Ceiling Lower to Floor */
    case 42:   /* SR Door Close */
    case 43:   /* SR Ceiling Lower to Floor */
    case 45:   /* SR Floor Lower to Highest */
    case 49:   /* S1 Ceiling Crush and Raise */
    case 50:   /* S1 Door Close */
    case 51:   /* S1 Secret Exit */
    case 55:   /* S1 Floor Raise Crush */
    case 60:   /* SR Floor Lower to Lowest */
    case 61:   /* SR Door Open */
    case 62:   /* SR Lift */
    case 63:   /* SR Door Raise */
    case 64:   /* SR Floor Raise */
    case 65:   /* SR Floor Raise Crush */
    case 66:   /* SR Floor Raise x24 */
    case 67:   /* SR Floor Raise x32 */
    case 68:   /* SR Floor Raise to Next (texture) */
    case 69:   /* SR Floor Raise to Next */
    case 70:   /* SR Floor Lower to Next */
    case 71:   /* S1 Floor Lower to Next Fast */
    case 102:  /* S1 Floor Lower to Highest */
    case 103:  /* S1 Door Open Stay */
    case 111:  /* S1 Lift Fast */
    case 112:  /* S1 Lift Fast */
    case 113:  /* S1 Lift Fast */
    case 114:  /* SR Lift Fast */
    case 115:  /* SR Lift Fast */
    case 116:  /* SR Lift Fast */
    case 122:  /* S1 Lift Fast */
    case 123:  /* SR Lift Fast */
    case 127:  /* S1 Stairs Fast */
        return true;
    default:
        return false;
    }
}

static boolean IsTeleporterSpecial(int special) {
    switch (special) {
    case 39:   /* W1 Teleport */
    case 97:   /* WR Teleport */
    case 125:  /* W1 Teleport Monsters Only */
    case 126:  /* WR Teleport Monsters Only */
    case 174:  /* Teleport (ACS) */
    case 195:  /* SR Teleport */
    case 207:  /* W1 Teleport (silent) */
    case 208:  /* WR Teleport (silent) */
    case 209:  /* S1 Teleport (silent) */
    case 210:  /* SR Teleport (silent) */
    case 243:  /* W1 Teleport (silent, line-to-line) */
    case 244:  /* WR Teleport (silent, line-to-line) */
    case 262:  /* W1 Teleport (silent, line-to-line, reversed) */
    case 263:  /* WR Teleport (silent, line-to-line, reversed) */
    case 264:  /* W1 Teleport (monsters, silent, line-to-line, reversed) */
    case 265:  /* WR Teleport (monsters, silent, line-to-line, reversed) */
    case 266:  /* W1 Teleport (silent, line-to-line) */
    case 267:  /* WR Teleport (silent, line-to-line) */
    case 268:  /* W1 Teleport (monsters, silent, line-to-line) */
    case 269:  /* WR Teleport (monsters, silent, line-to-line) */
        return true;
    default:
        return false;
    }
}

static boolean IsExitSpecial(int special) {
    switch (special) {
    case 11:   /* S1 Exit Normal */
    case 51:   /* S1 Exit Secret */
    case 52:   /* W1 Exit Normal */
    case 124:  /* W1 Exit Secret */
    case 197:  /* G1 Exit Normal */
    case 198:  /* G1 Exit Secret */
        return true;
    default:
        return false;
    }
}

static boolean IsKeyDoorSpecial(int special) {
    switch (special) {
    case 26:   /* DR Blue Door */
    case 27:   /* DR Yellow Door */
    case 28:   /* DR Red Door */
    case 32:   /* D1 Blue Door Open Stay */
    case 33:   /* D1 Red Door Open Stay */
    case 34:   /* D1 Yellow Door Open Stay */
    case 99:   /* SR Blue Door */
    case 100:  /* WR Blue Door */
    case 133:  /* S1 Blue Door */
    case 134:  /* SR Red Door */
    case 135:  /* S1 Red Door */
    case 136:  /* SR Yellow Door */
    case 137:  /* S1 Yellow Door */
        return true;
    default:
        return false;
    }
}

static const char* GetKeyDoorColor(int special) {
    switch (special) {
    case 26: case 32: case 99: case 100: case 133:
        return "Blue";
    case 28: case 33: case 134: case 135:
        return "Red";
    case 27: case 34: case 136: case 137:
        return "Yellow";
    default:
        return "";
    }
}

/* ============================================
 * Target Scanning
 * ============================================ */

static void GetLineCenter(int line_idx, fixed_t *out_x, fixed_t *out_y) {
    line_t *line = &lines[line_idx];
    *out_x = (line->v1->x + line->v2->x) / 2;
    *out_y = (line->v1->y + line->v2->y) / 2;
}

void Secret_ScanTargets(void) {
    int i, j;
    int secret_num = 0;
    int door_num = 0;
    int lift_num = 0;
    int switch_num = 0;
    int teleporter_num = 0;
    int exit_num = 0;
    int keydoor_num = 0;

    /* Distance threshold for considering two targets as duplicates (64 units) */
    const fixed_t DUPLICATE_THRESHOLD = 64 * FRACUNIT;

    /* Clear all counts */
    for (i = 0; i < TARGET_TYPE_COUNT; i++) {
        s_target_counts[i] = 0;
    }

    SECRET_DEBUG("[SECRET] Scanning level for targets...\n");

    /* Use saved original secret sectors (includes discovered ones) */
    for (i = 0; i < s_original_secret_count && secret_num < SECRET_MAX_TARGETS; i++) {
        int sector_idx = s_original_secret_sectors[i];
        target_info_t *t = &s_targets[TARGET_SECRET][secret_num];
        t->type = TARGET_SECRET;
        t->index = sector_idx;
        GetSectorCenter(sector_idx, &t->x, &t->y);
        snprintf(s_target_names[secret_num], 32, "Secret %d", secret_num + 1);
        t->name = s_target_names[secret_num];
        /* Check if secret has been discovered (special changes from 9 to 0) */
        t->discovered = (sectors[sector_idx].special != 9);
        t->reachable = true;
        secret_num++;
    }
    s_target_counts[TARGET_SECRET] = secret_num;
    SECRET_DEBUG("[SECRET]   Secrets: %d\n", secret_num);

    /* Scan linedefs for triggers */
    for (i = 0; i < numlines; i++) {
        line_t *line = &lines[i];
        int special = line->special;
        fixed_t line_x, line_y;
        boolean is_duplicate;

        if (special == 0) continue;

        /* Get line center for duplicate check */
        GetLineCenter(i, &line_x, &line_y);

        /* Key doors (highest priority doors) */
        if (IsKeyDoorSpecial(special) && keydoor_num < SECRET_MAX_TARGETS) {
            /* Check for duplicate (same position) */
            is_duplicate = false;
            for (j = 0; j < keydoor_num; j++) {
                fixed_t dx = line_x - s_targets[TARGET_KEY_DOOR][j].x;
                fixed_t dy = line_y - s_targets[TARGET_KEY_DOOR][j].y;
                if (P_AproxDistance(dx, dy) < DUPLICATE_THRESHOLD) {
                    is_duplicate = true;
                    break;
                }
            }
            if (!is_duplicate) {
                target_info_t *t = &s_targets[TARGET_KEY_DOOR][keydoor_num];
                t->type = TARGET_KEY_DOOR;
                t->index = i;
                t->x = line_x;
                t->y = line_y;
                snprintf(s_target_names[keydoor_num], 32, "%s Key Door", GetKeyDoorColor(special));
                t->name = s_target_names[keydoor_num];
                t->discovered = false;
                t->reachable = true;
                keydoor_num++;
            }
        }
        /* Exit triggers */
        else if (IsExitSpecial(special) && exit_num < SECRET_MAX_TARGETS) {
            is_duplicate = false;
            for (j = 0; j < exit_num; j++) {
                fixed_t dx = line_x - s_targets[TARGET_EXIT][j].x;
                fixed_t dy = line_y - s_targets[TARGET_EXIT][j].y;
                if (P_AproxDistance(dx, dy) < DUPLICATE_THRESHOLD) {
                    is_duplicate = true;
                    break;
                }
            }
            if (!is_duplicate) {
                target_info_t *t = &s_targets[TARGET_EXIT][exit_num];
                t->type = TARGET_EXIT;
                t->index = i;
                t->x = line_x;
                t->y = line_y;
                t->name = (special == 51 || special == 124 || special == 198) ? "Secret Exit" : "Exit";
                t->discovered = false;
                t->reachable = true;
                exit_num++;
            }
        }
        /* Teleporters */
        else if (IsTeleporterSpecial(special) && teleporter_num < SECRET_MAX_TARGETS) {
            is_duplicate = false;
            for (j = 0; j < teleporter_num; j++) {
                fixed_t dx = line_x - s_targets[TARGET_TELEPORTER][j].x;
                fixed_t dy = line_y - s_targets[TARGET_TELEPORTER][j].y;
                if (P_AproxDistance(dx, dy) < DUPLICATE_THRESHOLD) {
                    is_duplicate = true;
                    break;
                }
            }
            if (!is_duplicate) {
                target_info_t *t = &s_targets[TARGET_TELEPORTER][teleporter_num];
                t->type = TARGET_TELEPORTER;
                t->index = i;
                t->x = line_x;
                t->y = line_y;
                t->name = "Teleporter";
                t->discovered = false;
                t->reachable = true;
                teleporter_num++;
            }
        }
        /* Lifts (before general doors since some overlap) */
        else if (IsLiftSpecial(special) && lift_num < SECRET_MAX_TARGETS) {
            is_duplicate = false;
            for (j = 0; j < lift_num; j++) {
                fixed_t dx = line_x - s_targets[TARGET_LIFT][j].x;
                fixed_t dy = line_y - s_targets[TARGET_LIFT][j].y;
                if (P_AproxDistance(dx, dy) < DUPLICATE_THRESHOLD) {
                    is_duplicate = true;
                    break;
                }
            }
            if (!is_duplicate) {
                target_info_t *t = &s_targets[TARGET_LIFT][lift_num];
                t->type = TARGET_LIFT;
                t->index = i;
                t->x = line_x;
                t->y = line_y;
                t->name = "Lift";
                t->discovered = false;
                t->reachable = true;
                lift_num++;
            }
        }
        /* Switches (excluding lifts and doors that are handled elsewhere) */
        else if (IsSwitchSpecial(special) && !IsLiftSpecial(special) && switch_num < SECRET_MAX_TARGETS) {
            is_duplicate = false;
            for (j = 0; j < switch_num; j++) {
                fixed_t dx = line_x - s_targets[TARGET_SWITCH][j].x;
                fixed_t dy = line_y - s_targets[TARGET_SWITCH][j].y;
                if (P_AproxDistance(dx, dy) < DUPLICATE_THRESHOLD) {
                    is_duplicate = true;
                    break;
                }
            }
            if (!is_duplicate) {
                target_info_t *t = &s_targets[TARGET_SWITCH][switch_num];
                t->type = TARGET_SWITCH;
                t->index = i;
                t->x = line_x;
                t->y = line_y;
                t->name = "Switch";
                t->discovered = false;
                t->reachable = true;
                switch_num++;
            }
        }
        /* Regular doors (non-key) */
        else if (IsDoorSpecial(special) && !IsKeyDoorSpecial(special) && !IsLiftSpecial(special) && door_num < SECRET_MAX_TARGETS) {
            is_duplicate = false;
            for (j = 0; j < door_num; j++) {
                fixed_t dx = line_x - s_targets[TARGET_DOOR][j].x;
                fixed_t dy = line_y - s_targets[TARGET_DOOR][j].y;
                if (P_AproxDistance(dx, dy) < DUPLICATE_THRESHOLD) {
                    is_duplicate = true;
                    break;
                }
            }
            if (!is_duplicate) {
                target_info_t *t = &s_targets[TARGET_DOOR][door_num];
                t->type = TARGET_DOOR;
                t->index = i;
                t->x = line_x;
                t->y = line_y;
                t->name = "Door";
                t->discovered = false;
                t->reachable = true;
                door_num++;
            }
        }
    }

    s_target_counts[TARGET_DOOR] = door_num;
    s_target_counts[TARGET_LIFT] = lift_num;
    s_target_counts[TARGET_SWITCH] = switch_num;
    s_target_counts[TARGET_TELEPORTER] = teleporter_num;
    s_target_counts[TARGET_EXIT] = exit_num;
    s_target_counts[TARGET_KEY_DOOR] = keydoor_num;

    SECRET_DEBUG("[SECRET]   Doors: %d, Lifts: %d, Switches: %d\n", door_num, lift_num, switch_num);
    SECRET_DEBUG("[SECRET]   Teleporters: %d, Exits: %d, Key Doors: %d\n", teleporter_num, exit_num, keydoor_num);

    /* Reset selection to first secret (or first available type) */
    s_current_type = TARGET_SECRET;
    s_current_index = 0;

    /* If no secrets, find first available type */
    if (s_target_counts[TARGET_SECRET] == 0) {
        for (i = 0; i < TARGET_TYPE_COUNT; i++) {
            if (s_target_counts[i] > 0) {
                s_current_type = (target_type_t)i;
                break;
            }
        }
    }
}

/* ============================================
 * Target Selection
 * ============================================ */

const char* Secret_GetTargetTypeName(target_type_t type) {
    switch (type) {
    case TARGET_SECRET:     return "Secret";
    case TARGET_DOOR:       return "Door";
    case TARGET_LIFT:       return "Lift";
    case TARGET_SWITCH:     return "Switch";
    case TARGET_TELEPORTER: return "Teleporter";
    case TARGET_EXIT:       return "Exit";
    case TARGET_KEY_DOOR:   return "Key Door";
    default:                return "Unknown";
    }
}

int Secret_GetTargetCount(target_type_t type) {
    if (type == TARGET_TYPE_COUNT) {
        int total = 0;
        for (int i = 0; i < TARGET_TYPE_COUNT; i++) {
            total += s_target_counts[i];
        }
        return total;
    }
    if (type < 0 || type >= TARGET_TYPE_COUNT) return 0;
    return s_target_counts[type];
}

boolean Secret_IsDiscovered(int index) {
    if (index < 0 || index >= s_target_counts[TARGET_SECRET]) {
        return false;
    }
    /* Check if the secret sector's special type has been cleared (0 = discovered) */
    int sector_idx = s_targets[TARGET_SECRET][index].index;
    return (sectors[sector_idx].special != 9);
}

boolean Secret_SelectNextTarget(void) {
    int original_type = s_current_type;
    int original_index = s_current_index;

    /* Try to advance to next target within current type */
    s_current_index++;

    /* If past end of current type, move to next type */
    if (s_current_index >= s_target_counts[s_current_type]) {
        s_current_index = 0;
        /* Find next type with targets */
        for (int i = 1; i <= TARGET_TYPE_COUNT; i++) {
            int next_type = (s_current_type + i) % TARGET_TYPE_COUNT;
            if (s_target_counts[next_type] > 0) {
                s_current_type = (target_type_t)next_type;
                break;
            }
        }
    }

    boolean changed = (s_current_type != original_type || s_current_index != original_index);

    if (changed) {
        target_info_t info;
        if (Secret_GetCurrentTarget(&info)) {
            SECRET_DEBUG("[SECRET] Selected: %s (%s %d/%d)\n",
                   info.name,
                   Secret_GetTargetTypeName(s_current_type),
                   s_current_index + 1,
                   s_target_counts[s_current_type]);
        }
    }

    return changed;
}

boolean Secret_SelectPrevTarget(void) {
    int original_type = s_current_type;
    int original_index = s_current_index;

    /* Try to go to previous target within current type */
    s_current_index--;

    /* If before start of current type, move to previous type */
    if (s_current_index < 0) {
        /* Find previous type with targets */
        for (int i = 1; i <= TARGET_TYPE_COUNT; i++) {
            int prev_type = (s_current_type - i + TARGET_TYPE_COUNT) % TARGET_TYPE_COUNT;
            if (s_target_counts[prev_type] > 0) {
                s_current_type = (target_type_t)prev_type;
                s_current_index = s_target_counts[prev_type] - 1;
                break;
            }
        }
    }

    boolean changed = (s_current_type != original_type || s_current_index != original_index);

    if (changed) {
        target_info_t info;
        if (Secret_GetCurrentTarget(&info)) {
            SECRET_DEBUG("[SECRET] Selected: %s (%s %d/%d)\n",
                   info.name,
                   Secret_GetTargetTypeName(s_current_type),
                   s_current_index + 1,
                   s_target_counts[s_current_type]);
        }
    }

    return changed;
}

boolean Secret_SelectTarget(target_type_t type, int index) {
    /* Validate type */
    if (type < 0 || type >= TARGET_TYPE_COUNT) {
        return false;
    }

    /* Validate index */
    if (index < 0 || index >= s_target_counts[type]) {
        return false;
    }

    /* Check if selecting the same target */
    if (type == s_current_type && index == s_current_index) {
        /* Same target - return true but don't change anything */
        return true;
    }

    /* Clear blocked connections when selecting a new target */
    ClearBlockedConnections();
    s_path_blocked = false;
    s_blocking_wall_line = -1;
    s_blocking_check_counter = 0;

    /* Set selection */
    s_current_type = type;
    s_current_index = index;

    /* Log selection */
    target_info_t info;
    if (Secret_GetCurrentTarget(&info)) {
        SECRET_DEBUG("[SECRET] Selected %s %d/%d: %s\n",
               Secret_GetTargetTypeName(s_current_type),
               s_current_index + 1,
               s_target_counts[s_current_type],
               info.name);
    }

    return true;
}

boolean Secret_GetCurrentTarget(target_info_t *out_info) {
    if (s_target_counts[s_current_type] == 0) {
        return false;
    }
    if (s_current_index < 0 || s_current_index >= s_target_counts[s_current_type]) {
        return false;
    }

    *out_info = s_targets[s_current_type][s_current_index];
    return true;
}

void Secret_GetSelectionInfo(target_type_t *out_type, int *out_index, int *out_total) {
    if (out_type) *out_type = s_current_type;
    if (out_index) *out_index = s_current_index;
    if (out_total) *out_total = s_target_counts[s_current_type];
}

boolean Secret_FindPathToCurrentTarget(secret_path_t *out_path) {
    target_info_t info;
    player_t *player = &players[consoleplayer];
    int player_sector;

    /* Initialize output to safe defaults for all early returns */
    out_path->valid = false;
    out_path->num_arrows = 0;
    out_path->path_length = 0;

    if (!Secret_GetCurrentTarget(&info)) {
        SECRET_DEBUG("[SECRET] FindPath: No current target\n");
        return false;
    }

    if (!player->mo) {
        SECRET_DEBUG("[SECRET] FindPath: No player mobj\n");
        return false;
    }

    /* Ensure adjacency graph reflects current sector heights */
    /* Force rebuild on path request to capture door/lift changes */
    s_adjacency_dirty = true;
    EnsureAdjacencyValid();

    if (!s_adjacency_list) {
        SECRET_DEBUG("[SECRET] FindPath: No adjacency list\n");
        return false;
    }

    player_sector = GetSectorAt(player->mo->x, player->mo->y);
    if (player_sector < 0) {
        SECRET_DEBUG("[SECRET] FindPath: Invalid player sector (x=%d y=%d)\n",
                     player->mo->x >> FRACBITS, player->mo->y >> FRACBITS);
        return false;
    }

    int target_sector;

    if (info.type == TARGET_SECRET) {
        /* For secrets, target is the sector itself */
        target_sector = info.index;
    } else {
        /* For triggers, find the sector containing the linedef */
        line_t *line = &lines[info.index];
        /* Use frontsector as target (player activates from front) */
        if (line->frontsector) {
            target_sector = line->frontsector - sectors;
        } else if (line->backsector) {
            target_sector = line->backsector - sectors;
        } else {
            out_path->valid = false;
            return false;
        }
    }

    /* Log player position and sector info */
    SECRET_DEBUG("[SECRET] === FindPathToCurrentTarget ===\n");
    SECRET_DEBUG("[SECRET] Player position: (%d, %d)\n", player->mo->x >> FRACBITS, player->mo->y >> FRACBITS);
    SECRET_DEBUG("[SECRET] Player sector: %d\n", player_sector);
    {
        fixed_t sector_x, sector_y;
        GetSectorCenter(player_sector, &sector_x, &sector_y);
        SECRET_DEBUG("[SECRET] Player sector center: (%d, %d)\n", sector_x >> FRACBITS, sector_y >> FRACBITS);
    }
    SECRET_DEBUG("[SECRET] Target sector: %d\n", target_sector);

    /* Handle case where player is already in target sector */
    if (player_sector == target_sector) {
        SECRET_DEBUG("[SECRET] Player already in target sector - creating direct path\n");
        out_path->valid = true;
        out_path->path_length = 1;
        out_path->path[0] = player_sector;
        out_path->target_sector = target_sector;
        out_path->target_x = info.x;
        out_path->target_y = info.y;

        /* For triggers, set the exact linedef position */
        if (info.type != TARGET_SECRET) {
            s_path_to_hidden_door = true;
            s_hidden_door_line = info.index;
        } else {
            s_path_to_hidden_door = false;
            s_hidden_door_line = -1;
        }

        GenerateArrows(out_path);
        memcpy(&s_current_path, out_path, sizeof(secret_path_t));
        s_path_active = true;
        s_last_player_sector = player_sector;
        s_last_player_x = player->mo->x;
        s_last_player_y = player->mo->y;

        SpawnHintSprites(&s_current_path);
        return true;
    }

    /* Use BFS to find path (returns partial path if target unreachable) */
    if (BFSFindPath(player_sector, target_sector, out_path)) {
        boolean is_partial = (out_path->target_sector != target_sector);

        /* Set target coordinates for final arrow destination */
        out_path->target_x = info.x;
        out_path->target_y = info.y;

        if (is_partial) {
            SECRET_DEBUG("[SECRET] Partial path to %s (%d steps, closest reachable)\n",
                   info.name, out_path->path_length);
            /* For partial paths, don't set hidden door - just go as far as possible */
            s_path_to_hidden_door = false;
            s_hidden_door_line = -1;
        } else {
            SECRET_DEBUG("[SECRET] Path to %s found (%d steps)\n", info.name, out_path->path_length);

            /* For triggers (lift, door, etc.), set the exact linedef position */
            if (info.type != TARGET_SECRET) {
                s_path_to_hidden_door = true;
                s_hidden_door_line = info.index;  /* info.index is linedef index for triggers */
            } else {
                s_path_to_hidden_door = false;
                s_hidden_door_line = -1;
            }
        }

        /* Log path sectors */
#ifdef DEBUG_SECRET
        SECRET_DEBUG("[SECRET] Path sectors: ");
        for (int i = 0; i < out_path->path_length && i < 10; i++) {
            SECRET_DEBUG("%d ", out_path->path[i]);
        }
        if (out_path->path_length > 10) SECRET_DEBUG("...");
        SECRET_DEBUG("\n");
#endif

        GenerateArrows(out_path);
        memcpy(&s_current_path, out_path, sizeof(secret_path_t));
        s_path_active = true;
        s_last_player_sector = player_sector;  /* Set initial sector */
        s_last_player_x = player->mo->x;       /* Set initial position */
        s_last_player_y = player->mo->y;

        SpawnHintSprites(&s_current_path);
        return true;
    }

    /* This should rarely happen now - only if completely surrounded */
    SECRET_DEBUG("[SECRET] Cannot find any path from sector %d to target sector %d\n",
                 player_sector, target_sector);
    /* out_path already initialized at function start */
    return false;
}
