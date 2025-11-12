## Pronunciation Logging Implementation

### Agreements Made
- (2025-11-12) User: "Plan out logging. Be thorough. Focus first on startup, then reference file loading. Afterward, on recording."
- (2025-11-12) User: "Implement the plan as specified, it is attached for your reference. Do NOT edit the plan file itself."
- (2025-11-12) User: "To-do's from the plan have already been created. Do not create them again. Mark them as in_progress as you work, starting with the first one. Don't stop until you have completed all the to-dos."

### Explicitly Rejected
- (2025-11-12) No additional logging plans beyond the approved attachment; follow the attached phase breakdown exactly.

### Implementation Details
- Logging stack will use `tracing` with `tracing-subscriber::FmtSubscriber` configured via `EnvFilter`.
- Phase 1: initialize logging early in the CLI binary and add startup diagnostics within `SessionRuntime::new`.
- Phase 2: extend instrumentation around configuration resolution and reference asset loading inside `EngineRunner::build`.
- Phase 3: capture lifecycle logging within `SessionEngine` and propagate capture errors to UI snapshots.

### Phase Progress
- (2025-11-12) Phase 1 – Logging Infrastructure & Startup Diagnostics: completed. Added `tracing` dependencies, CLI subscriber initialization, and startup diagnostics in `SessionRuntime::new` / `EngineRunner::run`. Commands executed: `cargo fmt`, `cargo clippy --all-targets --all-features`, `cargo test`.
- Phase 2 – Reference Asset Loading Visibility: pending
- Phase 3 – Live Recording & UI Feedback Logging: pending

### Issues Encountered
- None recorded yet for this effort.

