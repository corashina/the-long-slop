# Survival and weather merge plan

Goal: Merge codex/survival-weather into current master. Preserve both feature sets.
Authority: The user requested "Merge to master". Earlier "Ignore tests" remains active.
Master base: 95973d4e38ce7a8bf99fb1edf18d078485c63117.
Feature head: 5c7c1fbe2edbae0a381d49eb4dc946d46056e709.
Workspace: Existing managed survival-weather worktree, integration branch codex/survival-weather-merge.

## Global Constraints

- Preserve master audio, radio, fleet, mirrors, saves, rendering, and lifecycle behavior.
- Preserve source-supported survival, sleep, weather, consumption, feedback, and lifecycle behavior.
- Keep one current clock, sky/light, and shared audio architecture. Reuse master owners.
- Keep physics at 1/60 second. Route sleep factors to every affected vehicle and existing radio clock.
- Add no compatibility layer or unrelated feature. Expanded survival/weather save persistence remains outside scope.
- Preserve existing supported save fields. Report unsupported new state persistence and load behavior explicitly.
- Leave behavioral test code unchanged where resolving conflicts allows. Retain master test conflicts without new assertions.
- Do not run tests, browser acceptance, runtime shader checks, or audible checks.
- Format, native/WASM library builds, syntax, source/schema validation, and relevant static shader compilation remain required.
- Source build 11140890 and accepted reference library are authoritative and read-only.
- Preserve raw evidence and ignored artifacts before worktree cleanup. Do not push.
- Use AGENTS.md and stop-slop for documentation. Avoid per-frame allocation and repeated setup.

### Task 1: Resolve and verify the integration

1. Merge the feature into this master-based integration branch with no automatic commit.
2. Resolve each conflict by preserving the master architecture and adding source feature behavior.
3. Unify audio schemas, events, playback, source exports, and required gameplay gates. Do not retain two audio owners.
4. Integrate sleep clocks with existing radio, fleet, save lifecycle, renderer histories, and camera changes.
5. Preserve wind/shelter bindings for all current bodies and vehicles. Update source records only where integration requires them.
6. Resolve the schema fixture from current data. Preserve master behavioral test conflict content under the waiver.
7. Derive the current combined selected runtime export. Keep primary source data read-only.
8. Run permitted source/build/static checks. Preserve exact commands, exits, current hashes, and limits.
9. Update the completion report for current combined capabilities and unsupported dependencies. Commit the merge.

### Task 2: Review and promote

1. Review the complete merged delta against pinned master, including conflict resolutions and source behavior.
2. Fix confirmed integration defects, then obtain scoped review.
3. Verify primary master is clean and still the pinned ancestor. Fast-forward master to the reviewed merge.
4. Verify feature ancestry and build provenance. Preserve evidence before managed worktree cleanup.
5. Report the local merge and skipped tests. Do not push.