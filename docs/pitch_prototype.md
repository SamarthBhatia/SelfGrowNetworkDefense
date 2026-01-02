# Pitch-Ready Prototype Walkthrough

This guide packages a demo flow you can use when pitching the project. The goal is to showcase an end-to-end morphogenetic defence cycle: simulate a calm baseline, escalate to an intense adversarial run, and surface the analytics + harness guidance that emerge.

## 1. Run the scripted demo

```bash
scripts/pitch_demo.sh
```

The script:
- Seeds a stimulus schedule for the intense scenario.
- Executes both `baseline-growth` and `intense-defense` configurations with telemetry capture.
- Generates per-step metrics, lineage CSVs, and ready-to-plot Vega-Lite specs.
- Scores each run with the adversarial harness, persisting backlog state for follow-up mutations.
- Drops a quick-reference cheatsheet under `target/pitch_demo/`.
- Makes the terminal dashboard (`cargo run --bin pitch_tui -- target/pitch_demo`) useful immediately.

All artefacts land in `target/pitch_demo/`, keeping the repository clean.

## 2. Tell the story

Use the generated assets to narrate the system’s behaviour:
1. **Baseline stability** — open `baseline_analysis.txt` to highlight low threat + steady replication.
2. **Adversarial escalation** — contrast with `intense_analysis.txt`, pointing out spikes in threat, stimuli totals, and harness fitness.
3. **Adaptive guidance** — inspect `intense_outcome.json` to show the harness recommending the next mutation and expanding the backlog.
4. **Visual pulse** — load `intense_vega.json` in a Vega viewer (Observable or VS Code extension) to plot threat vs. replication trajectories.
5. **Lineage dynamics** — layer `intense_lineage.csv` into your favourite charting tool to emphasise morphogenetic differentiation under pressure.
6. **Live dashboard** — open the TUI (`cargo run --bin pitch_tui -- target/pitch_demo`) and hit `r` to refresh while discussing follow-up mutations.

## 3. Portable talking points

- *Morphogenetic kernel*: Reaction–diffusion-inspired cells replicate, differentiate, and emit signals in response to changing threats.
- *Telemetry analytics*: JSONL streams are transformed into dashboard-ready datasets and lineage trajectories with a single script call.
- *Adversarial harness*: Each run is scored, producing follow-up mutations and a persisted backlog that fuels evolutionary attack loops.
- *Ready for iteration*: The prototype is self-contained—no services to provision—making it easy to hand over during an internship pitch.

## 4. Optional polish

If time permits before presenting:
- Import the Vega specs into Observable notebooks for interactive browsing.
- Capture a short terminal recording (asciinema) while running the script to include in your application materials.
- Prepare a one-slide summary with the fitness score comparison (`baseline_outcome.json` vs. `intense_outcome.json`).

With these steps you can demonstrate initial functionality, articulate the biological inspiration, and show momentum toward a full adversarial evolution platform.
