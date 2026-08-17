#!/usr/bin/env python3
"""Example Pasted Extractor backed by Poppler's pdftotext command."""

import json
import subprocess
import sys
from pathlib import Path


def main() -> int:
    if sys.argv[1:] == ["--version"]:
        print("Pasted pdftotext example 1.0.0")
        return 0
    if len(sys.argv) != 3 or sys.argv[1] != "--pasted-extract-v1":
        return 2

    request = json.loads(Path(sys.argv[2]).read_text(encoding="utf-8"))
    if request.get("protocolVersion") != 1:
        return 2
    input_value = request.get("input", {})
    if input_value.get("kind") != "file_references":
        print(json.dumps({"text": None}))
        return 0

    extracted = []
    for value in input_value.get("paths", [])[:8]:
        path = Path(value)
        if path.suffix.lower() != ".pdf" or not path.is_file():
            continue
        result = subprocess.run(
            ["pdftotext", str(path), "-"],
            stdin=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            check=False,
            timeout=45,
        )
        if result.returncode == 0:
            extracted.append(result.stdout.decode("utf-8", errors="replace"))

    print(json.dumps({"text": "\n\n".join(extracted) or None}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
