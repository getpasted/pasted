# Analysis performance baselines

Analysis performance is measured separately from ordinary correctness CI. Wall-clock thresholds are intentionally not enforced on shared runners because host load and architecture variance would make them noisy; deterministic safety and work bounds remain part of normal tests.

Run the portable release-mode harness from the repository root:

```sh
npm run bench:analysis -- --iterations 100
```

The harness warms each workload, records nine samples, and prints versioned JSON containing median and worst-sample (p95 for this nine-sample set) nanoseconds per iteration. It covers:

- structural inspection of approximately 64 KiB of text;
- a worst-case no-match scan across 256 Detectors;
- Smart Action enrichment across 256 candidate Transforms with the match last;
- the interactive whole-Analyzer text path, including database-backed participant loading.

Capture a baseline with a clean release build, record the commit, operating system, architecture, CPU, and power mode beside the JSON, and compare on the same machine. Investigate a repeatable median regression of 20% or more before changing the stored reference. Use at least 1,000 iterations for release decisions; the default 100 is intended for development checks.

The checked-in `docs/baselines/analysis-macos-arm64.json` is the initial development reference. It is descriptive rather than a cross-machine pass/fail threshold; replace it only after a repeatable same-machine run and review of the change.

Deterministic bounds remain the primary regression guard: four ordered passes, bounded clipboard inputs and source metadata, at most 16 patterns per Detector definition, 256 Transform candidates per Enricher run, and 12 returned recommendations. The benchmark stress count is explicit in the harness so future changes to those budgets are deliberate and reviewable.
