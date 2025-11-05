"""
RSM License Configuration - Single Source of Truth

This file contains all licensing information used throughout the project.
Modify pricing, contact info, or license text here, and regenerate all files.

Usage:
    from license_config import LICENSE_CONFIG
    print(LICENSE_CONFIG['pricing']['individual'])
"""

LICENSE_CONFIG = {
    # Pricing Information
    'pricing': {
        'individual': {
            'model': 'GitHub Sponsors',
            'amount': '$100',
            'period': 'cumulative',
            'description': 'For individual developers and freelancers'
        },
        'enterprise': {
            'model': 'GitHub Sponsors',
            'amount': '$500',
            'period': 'cumulative',
            'description': 'For companies and organizations (5+ developers)'
        }
    },

    # Contact Information
    'contact': {
        'licensing': 'newmassrael@gmail.com',
        'support': 'newmassrael@gmail.com',
        'sales': 'newmassrael@gmail.com',
        'github_sponsors': 'https://github.com/newmassrael'
    },

    # Project Information
    'project': {
        'name': 'RSM (Reactive State Machine)',
        'repository': 'https://github.com/newmassrael/reactive-state-machine',
        'website': 'https://github.com/newmassrael/reactive-state-machine',
        'copyright_holder': 'newmassrael',
        'copyright_year': '2025'
    },

    # License URLs
    'urls': {
        'license_main': 'https://github.com/newmassrael/reactive-state-machine/blob/main/LICENSE',
        'license_generated': 'LICENSE-GENERATED.md',
        'license_commercial': 'https://github.com/newmassrael/reactive-state-machine/blob/main/LICENSE-COMMERCIAL.md',
        'license_third_party': 'LICENSE-THIRD-PARTY.md',
        'license_lgpl': 'https://www.gnu.org/licenses/old-licenses/lgpl-2.1.html',
        'pricing_page': 'https://github.com/newmassrael/reactive-state-machine#pricing',
        'docs_licensing': 'https://github.com/newmassrael/reactive-state-machine#license',
        'faq': 'https://github.com/newmassrael/reactive-state-machine#faq'
    },

    # Generated Code License Text (MIT, Unrestricted)
    'generated_code_header': {
        'title': 'GENERATED CODE - MIT License (Unrestricted Use)',
        'description': 'Generated code is MIT licensed and may be used in both open source and commercial projects without restriction.',
        'copyright_holder': '[Author of input SCXML file]'
    },

    # Engine License Text (Dual License)
    'engine_license': {
        'title': 'RSM Execution Engine - DUAL LICENSED',
        'lgpl_requirement': 'LGPL-2.1 allows unmodified engine use in any project (open source or proprietary). If modified, follow LGPL-2.1 terms (share modifications) or purchase Commercial License.',
        'commercial_requirement': 'For proprietary modifications without LGPL compliance, obtain a commercial license via GitHub Sponsors ($100 Individual / $500 Enterprise).'
    },

    # Commercial License Benefits
    'commercial_benefits': [
        'Use in proprietary products without source code disclosure',
        'Priority email support',
        'License compliance assistance',
        'Custom licensing terms available for enterprises'
    ],

    # Support Response Times (for future use)
    'support_tiers': {
        'community': {
            'name': 'Community (LGPL-2.1)',
            'price': 'Free',
            'response_time': 'Best effort (GitHub Issues)',
            'channels': ['GitHub Issues', 'Discussions']
        },
        'individual': {
            'name': 'Individual (GitHub Sponsors)',
            'price': '$100 cumulative',
            'response_time': '48 business hours',
            'channels': ['Email', 'GitHub Issues Priority']
        },
        'enterprise': {
            'name': 'Enterprise (GitHub Sponsors)',
            'price': '$500 cumulative',
            'response_time': '24 business hours',
            'channels': ['Email Priority', 'Slack/Discord', 'Video Calls']
        }
    }
}


def get_pricing_text(tier='individual'):
    """Get formatted pricing text for a specific tier"""
    if tier not in LICENSE_CONFIG['pricing']:
        raise ValueError(f"Unknown pricing tier: {tier}")

    info = LICENSE_CONFIG['pricing'][tier]
    return f"{info['amount']} {info['period']} via {info['model']}"


def get_contact_text():
    """Get formatted contact information"""
    contact = LICENSE_CONFIG['contact']
    return f"""
**Licensing Inquiries:** {contact['licensing']}
**GitHub Sponsors:** {contact['github_sponsors']}
**Technical Support:** {contact['support']}
**Enterprise Sales:** {contact['sales']}
""".strip()


def get_copyright_line():
    """Get formatted copyright line"""
    project = LICENSE_CONFIG['project']
    return f"Copyright (c) {project['copyright_year']} {project['copyright_holder']}"


def get_generated_code_mit_header():
    """
    Get SPDX-compliant MIT license header for generated code.
    
    This header is included in all SCXML → C++ generated files.
    Uses SPDX-License-Identifier for modern standard compliance.
    """
    config = LICENSE_CONFIG
    project = config['project']
    
    return f"""// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 {{{{ model.scxml_author | default('[Author of input SCXML file]') }}}}
//
// Generated by RSM Code Generator
// From: {{{{ model.scxml_source_path | default('unknown.scxml') }}}}
//
// This generated code is MIT licensed and may be freely used in any project.
// Runtime engine dependency: LGPL-2.1-or-later OR LicenseRef-RSM-Commercial
// Full license: {config['urls']['license_main']}
"""



def get_engine_dual_license_header():
    """Get SPDX-compliant Dual License header for engine code"""
    config = LICENSE_CONFIG
    pricing = config['pricing']
    contact = config['contact']
    
    return f"""// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-RSM-Commercial
// SPDX-FileCopyrightText: {get_copyright_line()}
//
// This file is part of RSM (Reactive State Machine).
//
// Dual Licensed:
// 1. LGPL-2.1: Free for unmodified use (see LICENSE-LGPL-2.1.md)
// 2. Commercial: For modifications (contact {contact['licensing']})
//
// Commercial License:
//   Individual: {pricing['individual']['amount']} {pricing['individual']['period']}
//   Enterprise: {pricing['enterprise']['amount']} {pricing['enterprise']['period']}
//   Contact: {contact['github_sponsors']}
//
// Full terms: {config['urls']['license_main']}
"""


if __name__ == '__main__':
    # Test the configuration
    print("=== RSM License Configuration ===\n")
    print(f"Project: {LICENSE_CONFIG['project']['name']}")
    print(f"Copyright: {get_copyright_line()}")
    print(f"\nPricing:")
    print(f"  Individual: {get_pricing_text('individual')}")
    print(f"  Enterprise: {get_pricing_text('enterprise')}")
    print(f"\nContact:\n{get_contact_text()}")
    print("\n=== Generated Code Header ===")
    print(get_generated_code_mit_header()[:500] + "...")
    print("\n=== Engine Dual License Header ===")
    print(get_engine_dual_license_header()[:500] + "...")
