"""Pinned B1 Physics Python oracle. Imports source at the exact checkout SHA.

No Physics production code is copied into this repository. This helper is
strictly test-only, and must not be used to serve canonical adapter behavior.
"""
from __future__ import annotations

import importlib
import json
from pathlib import Path
import re
import sys
import types


def error_kind(message: str) -> str:
    if "unmatched closing block" in message:
        return "UnmatchedClose"
    if "unclosed MOOSE block" in message:
        return "UnclosedBlock"
    if "ambiguous parameter" in message or "found 2" in message and "parameter" in message:
        return "AmbiguousParameter"
    if "expected one block" in message or "unique block" in message:
        if re.search(r"found 0\b", message):
            return "BlockNotFound"
        return "AmbiguousBlock"
    return "InvalidMutation"


def main() -> None:
    root = Path(sys.argv[1]).resolve()
    source = root / "physics_harness" / "adapters" / "moose"
    for name in ("input.py", "blocks.py", "parameters.py"):
        if not (source / name).is_file():
            raise SystemExit(f"missing pinned source {source / name}")
    package = types.ModuleType("b1_legacy_moose")
    package.__path__ = [str(source)]
    sys.modules[package.__name__] = package
    inputs = importlib.import_module("b1_legacy_moose.input")
    blocks = importlib.import_module("b1_legacy_moose.blocks")
    params = importlib.import_module("b1_legacy_moose.parameters")

    case = json.load(sys.stdin)
    op = case["op"]
    text = case["text"]
    path = case.get("path", "")
    name = case.get("name", "")
    value = case.get("value", "")
    payload = case.get("payload", "")
    try:
        if op == "paths":
            result = [b.path for b in inputs.MooseInput(text).blocks]
        elif op == "unique":
            result = inputs.MooseInput(text).unique(path).path
        elif op == "span":
            span = inputs.MooseInput(text).unique(path)
            result = [span.start, span.end]
        elif op == "children":
            result = params.direct_children(text, path)
        elif op == "get":
            result = params.get_parameter(text, path, name)
        elif op == "upsert":
            result = params.upsert_parameter(text, path, name, value)
        elif op == "remove_param":
            result = params.remove_parameter(text, path, name)
        elif op == "remove_block":
            result = blocks.remove_block(text, path)
        elif op == "replace_block":
            result = blocks.replace_block(text, path, payload)
        elif op == "append":
            result = blocks.append_top_level_block(text, payload)
        elif op == "insert_child":
            result = blocks.insert_child_block(text, path, payload)
        else:
            raise SystemExit(f"unexpected operation: {op}")
        response = {"status": "ok", "value": result}
    except (inputs.MooseInputError, blocks.MooseBlockError, params.MooseParameterError) as exc:
        response = {"status": "error", "kind": error_kind(str(exc))}
    json.dump(response, sys.stdout, ensure_ascii=False)


if __name__ == "__main__":
    main()
