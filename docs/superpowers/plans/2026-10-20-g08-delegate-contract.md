# G08 delegate contract

Read the [approved specification](../specs/2026-10-20-g08-inventory-design.md) and [implementation plan](2026-10-20-g08-inventory.md) first.
This document is the shared recipe for G08 implementers and reviewers.
Only the coordinating agent changes this contract.

## Scope and ownership

Use `C:\Users\Corashina\.codex\worktrees\g08-inventory\the-long-drive-wasm` on `codex/g08-inventory`.
The main checkout contains unrelated edits and read-only reference assets.
Read the worktree's `AGENTS.md` before edits.
Use GPT-6.1 Sol with high reasoning for implementation and review.
Change only the files assigned by the current plan task.
Do not implement later tasks, merge branches, or change reference exports.
Report interface or source conflicts to the coordinator before changing the approved contract.

## Verification recipe

Follow the plan's test-first steps and exact interfaces.
Run the new critical native test before implementing its missing behavior.
Record its failing assertion, then run it again after the change.
Use the plan's Docker setup, read-only asset mount, and worktree-specific target directory.
Run ignored export tests explicitly. Report the executed count.
Run both WASM compile checks for Rust changes.
Never overlap build or test commands with another worker using the same target namespace.
Do not run full checks, release builds, or browser acceptance without separate authorization.

## Handoff evidence

Report changed files, public interface changes, source evidence, test failures, and final command results.
List deferred checks and unverified runtime behavior.
Do not equate compilation, prepared state, or native tests with browser acceptance.
Reviewers inspect the actual diff and verification output.
The coordinator owns integration and the final roadmap claim.
