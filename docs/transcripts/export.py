#!/usr/bin/env python3
"""Export a Claude Code session of THIS project into docs/transcripts/, scrubbed.

A thin wrapper around `export_transcript.py` from github.com/c4ffein/presentations
(not vendored: pass its path). It only ADDS this project's redaction patterns to
the exporter's own (emails, home paths, API keys, scratch paths):

  - the private deploy host and LAN addresses the sessions mention,
  - ISO timestamps — the `?perf` / `?gpuprobe` dumps pasted into a chat carry
    `"timestamp": "...Z"` in their meta, and the exporter deliberately reveals
    no work-time pattern.

    python3 docs/transcripts/export.py PATH/TO/export_transcript.py SESSION.jsonl \\
        -o docs/transcripts/NAME.json --title "..."

Everything after the exporter's path is passed to it unchanged. Sessions live in
~/.claude/projects/-home-dev-workspace-open-miami/<session-id>.jsonl — the RAW
file is never for the repo (tool outputs, unscrubbed). REVIEW the output.
"""
import importlib.util
import re
import sys

if len(sys.argv) < 3:
    sys.exit(__doc__)
spec = importlib.util.spec_from_file_location("export_transcript", sys.argv[1])
exporter = importlib.util.module_from_spec(spec)
spec.loader.exec_module(exporter)
exporter.REDACTIONS[:0] = [  # first: before the generic patterns can split them
    (re.compile(r"\b(?:[a-z0-9-]+\.)+c4ffein\.io\b"), "[deploy-host]"),
    (re.compile(r"\b(?:192\.168|10\.\d{1,3}|172\.(?:1[6-9]|2\d|3[01]))\.\d{1,3}\.\d{1,3}\b"), "[lan-ip]"),
    (re.compile(r"\b\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z?"), "[timestamp]"),
]
sys.argv = [sys.argv[1]] + sys.argv[2:]
raise SystemExit(exporter.main())
