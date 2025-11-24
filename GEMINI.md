# Repository Guidelines                                                                                                                                
                                                                                                                                                       
## Workflow Overview                                                                                                                                   
Start each session by opening `status.md`. That log captures the latest accomplishments, open questions, and next actions. Update it before you begin c
oding (intent) and before you stop (results and follow-ups). Commit `status.md` changes with the work they describe so the Git history mirrors project 
progress.                                                                                                                                              
                                                                                                                                                       
## Project Structure & Module Organization                                                                                                             
The fresh Rust crate keeps the binary entry point in `src/main.rs` and shared logic in `src/lib.rs`. Core scaffolding modules now live at `src/cellular
.rs`, `src/signaling.rs`, `src/telemetry.rs`, and `src/orchestration.rs`; expand them as Phase 1 features mature. Add new files in `src/` and expose th
em through `lib.rs`. Use `tests/` for integration scenarios once they are created, and tuck datasets or helper scripts back under `data/` and `scripts/
` when you revive them. Preserve the legacy prototype under `legacy_project_backup/` for reference only—do not modify it unless migrating specific idea
s.                                                                                                                                                     
                                                                                                                                                       
## Build, Test, and Development Commands                                                                                                               
- `cargo build` — compile the core binary; add `--release` when benchmarking morphogenetic rules.                                                      
- `cargo run -- <args>` — exercise the evolving defense kernel locally with scenario-specific parameters.                                              
- `cargo test` — execute unit and integration suites; use `-- --nocapture` to stream debugging output.                                                 
- `cargo fmt` and `cargo clippy --all-targets --all-features` — enforce style and lint gates before commits.                                           
- `scripts/run_demo.sh` / `scripts/performance_test.py` — resurrect or rewrite these utilities as the fresh stack takes shape.                         
                                                                                                                                                       
## Coding Style & Naming Conventions                                                                                                                   
Stick to Rust defaults (`cargo fmt` with four-space indentation). Use `snake_case` for functions and variables, `PascalCase` for types, and `UPPER_SNAK
E_CASE` for constants. Document complex morphogenetic behaviors with `///` doc comments so they surface in generated docs. Keep modules focused; prefer
 composable functions for cellular rules and swarm protocols.                                                                                          
                                                                                                                                                       
## Status Handoff Expectations                                                                                                                         
Never end a session without logging what changed, why it matters, unresolved blockers, and the precise next task in `status.md`. If you branch or proto
type locally, record the branch name and purpose. New collaborators must read the most recent log entry plus this guide before contributing. When in do
ubt about priorities, follow the phase roadmap in `status.md` starting with Phase 1 cellular automata foundations.