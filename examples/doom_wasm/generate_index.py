#!/usr/bin/env python3
"""Generate index.html from template with embedded SCXML Base64 data."""

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
        'GAME': 'game_state.scxml',
        'PLAYER': 'player_state.scxml',
        'WEAPON': 'weapon_state.scxml',
        'ENEMY': 'enemy_state.scxml',
        'SECRET': 'secret_hint_state.scxml',
        'AIM': 'aim_assist_state.scxml',
        'COMBO': 'combo_state.scxml'
    }

    scxml_base64 = {}
    for key, filename in scxml_files.items():
        filepath = os.path.join(scxml_dir, filename)
        if not os.path.exists(filepath):
            print(f"Error: SCXML file not found: {filepath}")
            sys.exit(1)
        with open(filepath, 'rb') as f:
            scxml_base64[key] = base64.b64encode(f.read()).decode('utf-8')

    # Read template file
    script_dir = os.path.dirname(os.path.abspath(__file__))
    template_path = os.path.join(script_dir, 'index.html.template')
    
    if not os.path.exists(template_path):
        print(f"Error: Template file not found: {template_path}")
        sys.exit(1)
    
    with open(template_path, 'r', encoding='utf-8') as f:
        html = f.read()

    # Replace placeholders with Base64-encoded SCXML data
    for key, base64_data in scxml_base64.items():
        placeholder = '{{SCXML_' + key + '}}'
        html = html.replace(placeholder, base64_data)

    # Write output file
    output_file = os.path.join(output_dir, 'index.html')
    with open(output_file, 'w', encoding='utf-8') as f:
        f.write(html)

    print(f"Generated: {output_file}")

if __name__ == "__main__":
    main()
