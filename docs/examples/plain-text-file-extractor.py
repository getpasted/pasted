#!/usr/bin/env python3
"""Example Pasted custom Extractor for small UTF-8 text files."""

import json
import pathlib
import sys

MAX_FILE_BYTES = 1_048_576
SUPPORTED_SUFFIXES = {".csv", ".json", ".log", ".md", ".txt"}


def main() -> int:
    if sys.argv[1:] == ["--version"]:
        print("Pasted plain-text example 1.0.0")
        return 0
    if len(sys.argv) != 3 or sys.argv[1] != "--pasted-extract-v1":
        return 2

    request = json.loads(pathlib.Path(sys.argv[2]).read_text(encoding="utf-8"))
    if request.get("protocolVersion") != 1:
        return 2
    input_value = request.get("input", {})
    if input_value.get("kind") != "file_references":
        print(json.dumps({"text": None}))
        return 0

    extracted = []
    for value in input_value.get("paths", [])[:8]:
        path = pathlib.Path(value)
        if (
            path.is_file()
            and path.suffix.lower() in SUPPORTED_SUFFIXES
            and path.stat().st_size <= MAX_FILE_BYTES
        ):
            extracted.append(path.read_text(encoding="utf-8", errors="replace"))
    print(json.dumps({"text": "\n\n".join(extracted) or None}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
