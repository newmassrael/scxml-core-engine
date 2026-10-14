"""Targets for a document and its binding: one logical interface, a profile per platform.

`split` cuts a binding into what every target shares (the logical interface) and what only the
platform behind it needs (the profile). `standalone` builds a library from the logical interface
alone, and `replay` turns a pack's examples into a script that library is judged by. A platform's
own target is a program that reads the same two files and lives with that platform.
"""
