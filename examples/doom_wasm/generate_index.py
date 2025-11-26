#!/usr/bin/env python3
"""Generate index.html with embedded SCXML Base64 data."""

import base64
import os
import sys

def main():
    if len(sys.argv) != 3:
        print("Usage: generate_index.py <scxml_dir> <output_dir>")
        sys.exit(1)

    scxml_dir = sys.argv[1]
    output_dir = sys.argv[2]

    # Read and encode SCXML files
    scxml_files = {
        'game': 'game_state.scxml',
        'player': 'player_state.scxml',
        'weapon': 'weapon_state.scxml',
        'enemy': 'enemy_state.scxml'
    }

    scxml_base64 = {}
    for key, filename in scxml_files.items():
        filepath = os.path.join(scxml_dir, filename)
        if not os.path.exists(filepath):
            print(f"Error: SCXML file not found: {filepath}")
            sys.exit(1)
        with open(filepath, 'rb') as f:
            scxml_base64[key] = base64.b64encode(f.read()).decode('utf-8')

    # Generate index.html
    html = f'''<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>DOOM + SCE State Machines</title>
    <style>
        body {{ margin: 0; padding: 20px; padding-bottom: 50px; background: #1a1a1a; color: #fff; font-family: monospace; }}
        .container {{ width: 100%; margin: 0 auto; }}

        /* Game Section */
        #game-section {{
            display: flex;
            flex-direction: column;
            align-items: center;
            margin-bottom: 20px;
        }}
        #game-container {{
            display: inline-block;
            position: relative;
            overflow: hidden;
            min-width: 320px;
            min-height: 200px;
            border: 2px solid #666;
            background: #000;
            aspect-ratio: 8 / 5;
            resize: horizontal;
            max-width: min(calc(100vw - 60px), calc((100vh - 20px) * 8 / 5));
        }}
        #game-container::after {{
            content: '';
            position: absolute;
            bottom: 0;
            right: 0;
            width: 20px;
            height: 20px;
            cursor: e-resize;
            background: linear-gradient(135deg, transparent 50%, #888 50%);
            pointer-events: none;
        }}
        canvas {{
            display: block;
            width: 100%;
            height: 100%;
            image-rendering: pixelated;
            image-rendering: crisp-edges;
        }}

        /* Tab Section */
        .tab-container {{ margin-top: 20px; }}
        .tab-buttons {{
            display: flex;
            justify-content: center;
            gap: 5px;
            margin-bottom: 0;
        }}
        .tab-btn {{
            padding: 10px 20px;
            background: #333;
            border: 1px solid #555;
            border-bottom: none;
            border-radius: 4px 4px 0 0;
            color: #888;
            cursor: pointer;
            font-family: monospace;
            font-size: 14px;
        }}
        .tab-btn:hover {{ background: #444; }}
        .tab-btn.active {{
            background: #2a2a2a;
            color: #00ff00;
        }}
        .tab-btn .state-indicator {{
            display: inline-block;
            margin-left: 8px;
            padding: 2px 6px;
            background: #444;
            border-radius: 3px;
            font-size: 10px;
            color: #0969da;
        }}

        /* Enemy Section (between tabs and visualizer, only for enemy tab) */
        .enemy-section {{
            display: none;
            padding: 10px;
            background: #252525;
            border: 1px solid #555;
            border-bottom: none;
        }}
        .enemy-section.visible {{
            display: block;
        }}
        .enemy-list {{
            display: flex;
            flex-wrap: wrap;
            gap: 5px;
            max-height: 100px;
            overflow-y: auto;
        }}
        .enemy-btn {{
            padding: 5px 10px;
            background: #333;
            border: 1px solid #555;
            border-radius: 4px;
            font-size: 11px;
            color: #ff6600;
            font-family: monospace;
            cursor: pointer;
        }}
        .enemy-btn:hover {{ background: #444; }}
        .enemy-btn.selected {{
            background: #2a2a2a;
            border-color: #00ff00;
            box-shadow: 0 0 5px rgba(0,255,0,0.3);
        }}
        .enemy-state-text {{
            margin-left: 5px;
            font-size: 10px;
        }}
        .enemy-state-text.DORMANT {{ color: #8888ff; }}
        .enemy-state-text.ALERT {{ color: #ffff00; }}
        .enemy-state-text.CHASING {{ color: #ff9900; }}
        .enemy-state-text.ATTACKING {{ color: #ff0000; }}
        .enemy-state-text.PAIN {{ color: #ff00ff; }}
        .enemy-state-text.DEAD {{ color: #666; text-decoration: line-through; }}

        /* Visualizer Section */
        #viz-section {{
            border: 1px solid #555;
            border-radius: 0 0 8px 8px;
            overflow: hidden;
            background: #2a2a2a;
            height: 350px;
        }}
        #visualizer {{
            width: 100%;
            height: 100%;
            border: none;
        }}

        /* Stats Section */
        #stats {{
            position: fixed;
            bottom: 0;
            left: 0;
            right: 0;
            display: flex;
            justify-content: center;
            gap: 30px;
            padding: 10px 15px;
            background: #222;
            border-top: 1px solid #444;
            z-index: 100;
        }}
        #stats span {{ color: #888; }}
        #stats strong {{ color: #00ff00; }}
    </style>
</head>
<body>
    <div class="container">
        <!-- Game Section -->
        <div id="game-section">
            <div id="game-container">
                <canvas id="canvas" oncontextmenu="event.preventDefault()" tabindex="-1" width="640" height="400"></canvas>
            </div>
        </div>

        <!-- Tab + Visualizer Section -->
        <div class="tab-container">
            <div class="tab-buttons">
                <button class="tab-btn active" data-machine="game" onclick="switchTab('game')">
                    Game <span class="state-indicator" id="ind-game">DEMOSCREEN</span>
                </button>
                <button class="tab-btn" data-machine="player" onclick="switchTab('player')">
                    Player <span class="state-indicator" id="ind-player">ALIVE</span>
                </button>
                <button class="tab-btn" data-machine="weapon" onclick="switchTab('weapon')">
                    Weapon <span class="state-indicator" id="ind-weapon">READY</span>
                </button>
                <button class="tab-btn" data-machine="enemy" onclick="switchTab('enemy')">
                    Enemy <span class="state-indicator" id="ind-enemy">0 active</span>
                </button>
            </div>
            <div id="enemy-section" class="enemy-section">
                <div id="enemy-list" class="enemy-list">
                    <!-- Enemy buttons will be added here -->
                </div>
            </div>
            <div id="viz-section">
                <iframe id="visualizer" src="visualizer/visualizer.html?embed#scxml={scxml_base64['game']}"></iframe>
            </div>
        </div>

        <!-- Stats Section -->
        <div id="stats">
            <span>Game: <strong id="stat-game">DEMOSCREEN</strong></span>
            <span>Player: <strong id="stat-player">ALIVE</strong></span>
            <span>Enemies: <strong id="stat-enemies">0</strong></span>
            <span>Killed: <strong id="stat-killed">0</strong></span>
        </div>

    </div>

    <script>
        // SCXML Base64 data
        const SCXML_DATA = {{
            game: '{scxml_base64['game']}',
            player: '{scxml_base64['player']}',
            weapon: '{scxml_base64['weapon']}',
            enemy: '{scxml_base64['enemy']}'
        }};

        // Valid machine names for validation
        const VALID_MACHINES = ['game', 'player', 'weapon', 'enemy'];

        // State tracking
        const SCE = {{
            currentTab: 'game',
            ready: false,
            lastState: {{ game: null, player: null, weapon: null, enemy: null }},
            enemies: {{}},  // slot -> {{type, state, instanceId, active}}
            selectedEnemy: null
        }};

        // Tab switching
        function switchTab(machine) {{
            SCE.currentTab = machine;

            // Update tab button styles
            document.querySelectorAll('.tab-btn').forEach(btn => {{
                btn.classList.toggle('active', btn.dataset.machine === machine);
            }});

            // Show/hide enemy section
            const enemySection = document.getElementById('enemy-section');
            enemySection.classList.toggle('visible', machine === 'enemy');

            // Load new SCXML in visualizer
            const iframe = document.getElementById('visualizer');
            iframe.src = 'visualizer/visualizer.html?embed&t=' + Date.now() + '#scxml=' + SCXML_DATA[machine];
            SCE.ready = false;
        }}

        // State priority for sorting (lower = first, DEAD and DORMANT at end)
        const STATE_PRIORITY = {{
            'ALERT': 1,
            'CHASING': 2,
            'ATTACKING': 3,
            'PAIN': 4,
            'DORMANT': 98,
            'DEAD': 99
        }};

        // Sort enemy buttons in the list (debounced)
        let sortTimeout = null;
        function sortEnemyButtons() {{
            if (sortTimeout) return;  // Already scheduled
            sortTimeout = setTimeout(() => {{
                sortTimeout = null;
                const container = document.getElementById('enemy-list');
                const buttons = Array.from(container.querySelectorAll('.enemy-btn'));

                buttons.sort((a, b) => {{
                    const stateA = SCE.enemies[a.dataset.slot]?.state || 'UNKNOWN';
                    const stateB = SCE.enemies[b.dataset.slot]?.state || 'UNKNOWN';
                    const priorityA = STATE_PRIORITY[stateA] || 50;
                    const priorityB = STATE_PRIORITY[stateB] || 50;
                    if (priorityA !== priorityB) return priorityA - priorityB;
                    const typeA = SCE.enemies[a.dataset.slot]?.type || '';
                    const typeB = SCE.enemies[b.dataset.slot]?.type || '';
                    return typeA.localeCompare(typeB);
                }});

                buttons.forEach(btn => container.appendChild(btn));
            }}, 500);  // Sort at most every 500ms
        }}

        // Select enemy and show in visualizer
        function selectEnemy(slot) {{
            SCE.selectedEnemy = slot;

            // Update button styles
            document.querySelectorAll('.enemy-btn').forEach(btn => {{
                btn.classList.toggle('selected', parseInt(btn.dataset.slot) === slot);
            }});

            // Switch to enemy tab and highlight state
            SCE.currentTab = 'enemy';
            document.querySelectorAll('.tab-btn').forEach(btn => {{
                btn.classList.toggle('active', btn.dataset.machine === 'enemy');
            }});

            // Load enemy SCXML in visualizer
            const iframe = document.getElementById('visualizer');
            iframe.src = 'visualizer/visualizer.html?embed&t=' + Date.now() + '#scxml=' + SCXML_DATA.enemy;
            SCE.ready = false;
        }}

        // Update enemy button and data
        function updateEnemySubtab(slot, type, state, instanceId, active) {{
            const container = document.getElementById('enemy-list');
            let btn = document.querySelector(`.enemy-btn[data-slot="${{slot}}"]`);

            if (active) {{
                // Create or update button
                if (!btn) {{
                    btn = document.createElement('button');
                    btn.className = 'enemy-btn';
                    btn.dataset.slot = slot;
                    btn.onclick = () => selectEnemy(slot);
                    container.appendChild(btn);
                }}
                btn.innerHTML = `${{type}} <span class="enemy-state-text ${{state}}">${{state}}</span>`;

                // Store enemy data
                SCE.enemies[slot] = {{ type, state, instanceId, active }};

                // If this enemy is selected and visualizer ready, highlight state
                if (SCE.selectedEnemy === slot && SCE.ready) {{
                    const iframe = document.getElementById('visualizer');
                    iframe.contentWindow.postMessage({{
                        type: 'highlight-states',
                        stateIds: [state.toLowerCase()]
                    }}, window.location.origin);
                }}
            }} else {{
                // Remove button
                if (btn) {{
                    btn.remove();
                }}
                delete SCE.enemies[slot];

                // If this enemy was selected, clear selection
                if (SCE.selectedEnemy === slot) {{
                    SCE.selectedEnemy = null;
                }}
            }}

            // Sort buttons after update
            sortEnemyButtons();

            // Update stats
            const activeCount = Object.keys(SCE.enemies).length;
            document.getElementById('stat-enemies').textContent = activeCount;
        }}

        // C++ callback: Called when an enemy is updated
        window.onSceEnemyUpdate = function(slot, type, state, instanceId, active) {{
            // Validate slot is a valid number
            if (typeof slot !== 'number' || slot < 0) {{
                console.warn('[SCE:Enemy] Invalid slot:', slot);
                return;
            }}

            // Validate type and state are strings
            if (typeof type !== 'string' || typeof state !== 'string') {{
                console.warn('[SCE:Enemy] Invalid type or state:', type, state);
                return;
            }}

            console.debug('[SCE:Enemy] Update:', slot, type, state, instanceId, active);
            updateEnemySubtab(slot, type, state, instanceId, !!active);
        }};

        // C++ callback: Called when stats are updated
        window.onSceStatsUpdate = function(enemyCount, enemyKilled) {{
            document.getElementById('stat-enemies').textContent = enemyCount;
            document.getElementById('stat-killed').textContent = enemyKilled;
            document.getElementById('ind-enemy').textContent = enemyCount + ' active';
        }};

        // C++ callback: Called when any state machine state changes
        window.onSceStateChange = function(machine, state) {{
            // Validate machine name
            if (!VALID_MACHINES.includes(machine)) {{
                console.warn('[SCE] Invalid machine name:', machine);
                return;
            }}

            // Validate state is a non-empty string
            if (typeof state !== 'string' || !state) {{
                console.warn('[SCE] Invalid state for', machine, ':', state);
                return;
            }}

            // Save last state
            SCE.lastState[machine] = state;

            // Update indicator
            const indicator = document.getElementById('ind-' + machine);
            if (indicator) indicator.textContent = state;

            // Update stats
            const statEl = document.getElementById('stat-' + machine);
            if (statEl) statEl.textContent = state;

            // Update visualizer if this is the current tab
            if (machine === SCE.currentTab && SCE.ready) {{
                const iframe = document.getElementById('visualizer');
                if (iframe && iframe.contentWindow) {{
                    iframe.contentWindow.postMessage({{
                        type: 'highlight-states',
                        stateIds: [state.toLowerCase()]
                    }}, window.location.origin);
                }}
            }}
        }};

        // Wait for visualizer ready
        window.addEventListener('message', (event) => {{
            if (event.data && event.data.type === 'visualizer-ready') {{
                SCE.ready = true;
                const iframe = document.getElementById('visualizer');

                // Highlight current state based on tab
                if (SCE.currentTab === 'enemy') {{
                    // If enemy is selected, highlight its state
                    if (SCE.selectedEnemy !== null && SCE.enemies[SCE.selectedEnemy]) {{
                        iframe.contentWindow.postMessage({{
                            type: 'highlight-states',
                            stateIds: [SCE.enemies[SCE.selectedEnemy].state.toLowerCase()]
                        }}, window.location.origin);
                    }}
                }} else {{
                    // For game/player/weapon, highlight last known state
                    const lastState = SCE.lastState[SCE.currentTab];
                    if (lastState) {{
                        iframe.contentWindow.postMessage({{
                            type: 'highlight-states',
                            stateIds: [lastState.toLowerCase()]
                        }}, window.location.origin);
                    }}
                }}
            }}
        }});

        // Emscripten Module
        var Module = {{
            canvas: document.getElementById('canvas'),
            onRuntimeInitialized: function() {{
                console.log('[SCE] WASM runtime initialized');
            }},
            print: function(text) {{
                if (text.includes('[SCE:')) {{
                    console.log('%c' + text, 'color: #00ff00; font-weight: bold;');
                }} else {{
                    console.log(text);
                }}
            }},
            printErr: function(text) {{ console.error(text); }}
        }};

    </script>
    <script async src="doom_sce.js"></script>
</body>
</html>
'''

    output_file = os.path.join(output_dir, 'index.html')
    with open(output_file, 'w') as f:
        f.write(html)

    print(f"Generated: {output_file}")

if __name__ == "__main__":
    main()
