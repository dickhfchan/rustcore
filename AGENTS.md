# Repository Guidelines

## Project Structure & Module Organization
- `kernel/` hosts the `#![no_std]` core (see `memory.rs`, `ipc_bridge.rs`, `scheduler.rs`, `sync.rs`).
- `arch/<target>/` implements HALs; `arch/x86_64/` wires up GDT/IDT, paging, and timer/IPC trap registration for the custom target.
- `ipc/` provides the bounded channel shared between kernel and user space.
- `services/init/` carries the bootstrap service that consumes the kernel handshake.
- `kernel/tests/` stores QEMU binaries; `scripts/` keeps emulator helpers.

## Build, Test, and Development Commands
- `cargo +nightly build` — compile the workspace for the custom target with `build-std`.
- `cargo +nightly build -p init` — confirm the init service builds for the same target.
- `cargo +nightly build --tests -p kernel` — emit QEMU smoke binaries (runner skips when QEMU is absent).
- `cargo +nightly fmt` / `cargo +nightly clippy --workspace` — format and lint under nightly.
- `./scripts/run-qemu.sh [--release]` — launch `qemu-system-x86_64`; pass a test ELF path to exercise harness builds.

## Coding Style & Naming Conventions
- Use `rustfmt` defaults (4-space indents, grouped imports); keep files ASCII unless hardware demands otherwise.
- Preface modules with doc comments summarizing their role.
- Every `unsafe` block needs a `// SAFETY:` note tied to the enforced invariant.
- Naming: `snake_case` for functions/modules, `CamelCase` for types/traits, `SCREAMING_SNAKE_CASE` for constants and syscall IDs.

## Testing Guidelines
- Host tests stay behind `#[cfg(test)]`; use table-driven coverage and limit `unsafe`.
- Emulator binaries live in `kernel/tests/`; run them via `./scripts/run-qemu.sh <path>` once QEMU is available.
- Target ≥85% coverage for scheduling, IPC, and memory when `cargo +nightly xtask coverage` lands; note gaps in PRs.
- Attach failing serial logs to issues to preserve regression details.

## Commit & Pull Request Guidelines
- Follow Conventional Commits with scopes (`feat(kernel)`, `fix(ipc)`, `chore(services)`).
- Each commit must pass `cargo +nightly build`; squash fixups before review.
- PRs need a summary, `Fixes #ID`, a "Safety" section for new `unsafe`, and QEMU logs when obtainable.
- Flag MSRV or IPC contract changes and update `CHANGELOG.md` accordingly.

## Security & Configuration Tips
- Keep secrets and signing keys out of the repo; base overrides on `config/kernel.env.example` once introduced.
- Run `cargo +nightly audit` and `cargo +nightly deny check advisories` before releases; document accepted risks.
- Revalidate linker or memory-layout edits with the helpers in `scripts/` and attach diffs in review.
