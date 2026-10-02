#!/usr/bin/env python3
from __future__ import annotations

import argparse
import re
from pathlib import Path

INVOKE_RE = re.compile(r"\binvoke(?:<[^>]+>)?\(\s*['\"]([a-zA-Z0-9_]+)['\"]")
COMMAND_RE = re.compile(r"#\[tauri::command\][\s\S]{0,300}?\b(?:async\s+)?fn\s+([a-zA-Z0-9_]+)\s*\(")
HANDLER_RE = re.compile(r"generate_handler!\s*\[([^\]]*)\]", re.MULTILINE)
IDENT_RE = re.compile(r"(?:[a-zA-Z0-9_]+::)*([a-zA-Z_][a-zA-Z0-9_]*)")


def collect_frontend(root: Path) -> set[str]:
    names: set[str] = set()
    for path in root.joinpath('src').rglob('*'):
        if path.suffix in {'.ts', '.tsx'}:
            names.update(INVOKE_RE.findall(path.read_text(encoding='utf-8')))
    return names


def collect_backend(root: Path) -> tuple[set[str], set[str]]:
    defined: set[str] = set()
    registered: set[str] = set()
    rust_root = root / 'src-tauri' / 'src'
    for path in rust_root.rglob('*.rs'):
        text = path.read_text(encoding='utf-8')
        defined.update(COMMAND_RE.findall(text))
        for block in HANDLER_RE.findall(text):
            registered.update(IDENT_RE.findall(block))
    return defined, registered


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument('--root', default='.')
    args = parser.parse_args()
    root = Path(args.root).resolve()

    frontend = collect_frontend(root)
    defined, registered = collect_backend(root)
    missing_definition = sorted(frontend - defined)
    missing_registration = sorted(frontend - registered)

    if missing_definition or missing_registration:
        if missing_definition:
            print('Frontend invokes without #[tauri::command] definition:', ', '.join(missing_definition))
        if missing_registration:
            print('Frontend invokes not registered in generate_handler!:', ', '.join(missing_registration))
        return 1

    print(f'invoke contract ok: {len(frontend)} frontend command(s) checked')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
