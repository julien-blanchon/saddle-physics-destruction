# `destruction_lab`

Crate-local showcase and verification app for `destruction`.

## Purpose

- demonstrate the shared crate with richer visuals than the minimal examples
- provide a stable BRP target for live inspection
- host crate-local E2E scenarios for behavior and screenshot verification

## How To Run

```bash
cargo run -p destruction_lab
```

## E2E Scenarios

```bash
cargo run -p destruction_lab --features e2e -- destruction_smoke
cargo run -p destruction_lab --features e2e -- destruction_supports
cargo run -p destruction_lab --features e2e -- destruction_hierarchy
cargo run -p destruction_lab --features e2e -- destruction_lod
cargo run -p destruction_lab --features e2e -- destruction_budget
```

## BRP

Launch the app, then inspect it with the BRP CLI:

```bash
uv run --project .codex/skills/bevy-brp/script brp ping
uv run --project .codex/skills/bevy-brp/script brp resource get destruction::config::DestructionDiagnostics
uv run --project .codex/skills/bevy-brp/script brp extras screenshot /tmp/destruction_lab.png
uv run --project .codex/skills/bevy-brp/script brp extras shutdown
```
