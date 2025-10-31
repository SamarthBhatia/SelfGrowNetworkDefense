# Morphogenetic Security Prototype

This repository rebuilds the morphogenetic cybersecurity stack from the ground up: a cellular automata defence kernel, telemetry analytics, and an adversarial evolution harness.

## Quick Pitch Demo

To generate a pitch-ready walkthrough, run:

```bash
scripts/pitch_demo.sh
```

The script produces telemetry summaries, dashboard-ready CSVs, and adversarial harness insights for both a calm baseline and an intense attack scenario. See `docs/pitch_prototype.md` for the storytelling flow and talking points.

## Development Basics

- `cargo build` — compile the runtime.
- `cargo run -- --config <path>` — execute a scenario (add `--telemetry` and `--stimulus` as needed).
- `cargo test` — run unit tests.
- `cargo fmt` / `cargo clippy --all-targets --all-features` — formatting and linting.

Additional analytics helpers live under `scripts/`. Check `status.md` for detailed project history and current focus.
