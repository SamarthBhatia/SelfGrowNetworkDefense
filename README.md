# Morphogenetic Security Prototype

This repository rebuilds the morphogenetic cybersecurity stack from the ground up: a cellular automata defence kernel, telemetry analytics, and an adversarial evolution harness.

## Quick Pitch Demo

To generate a pitch-ready walkthrough, run:

```bash
scripts/pitch_demo.sh
```

The script produces telemetry summaries, dashboard-ready CSVs, and adversarial harness insights for both a calm baseline and an intense attack scenario. See `docs/pitch_prototype.md` for the storytelling flow and talking points.

Want a quick visual? Launch the terminal dashboard after the demo artifacts exist:

```bash
cargo run --bin pitch_tui -- target/pitch_demo
```

The TUI compares each scenario’s fitness, threat levels, and recommended mutations, reloading live with `r`.

## Swarm Immune Response (Phase 3)

The system now supports distributed coordination and hardware-backed trust:
- **Consensus Quarantine:** Cells coordinate isolation of infected nodes using attested signals.
- **TPM Attestation:** Every cell uses a simulated TPM to sign anomaly reports, preventing poisoning attacks.
- **Acquired Immunity:** Lineages adapt their genomes in real-time when surviving threats and pass this memory to offspring.
- **Trust-Based Topology:** Cells actively manage links based on neighbor reputation scores.

See `docs/swarm-immune-response.md` for details.

## Development Basics

- `cargo build` — compile the runtime.
- `cargo run -- --config <path>` — execute a scenario (add `--telemetry` and `--stimulus` as needed).
- `cargo test` — run unit tests.
- `cargo fmt` / `cargo clippy --all-targets --all-features` — formatting and linting.

Additional analytics helpers live under `scripts/`. Check `status.md` for detailed project history and current focus.
