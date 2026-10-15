# Obsolete file audit

Date: 2026-10-15, Europe/Warsaw.
Base: `fa2265c0b67baa3f8e4eabcd68833ed5305a4d75`, local `master`, including existing local changes.

Mark four completed plans for removal. Keep their approved specifications and source evidence.
The four files contain 1,018 lines and 75,012 bytes. Removing them would remove no dependencies.
This audit adds removal marks here. It deletes no files.

## Removal marks

| Mark | File | Evidence |
| --- | --- | --- |
| REMOVE | [License plate plan](../superpowers/plans/2026-10-12-license-plate-text.md) | Explicit Complete status; closure commit `542be78` belongs to master. All 23 required checklist steps are checked. Two further steps record a user test waiver. 461 lines. |
| REMOVE | [Live mirror plan](../superpowers/plans/2026-10-12-live-mirrors.md) | All 27 steps are checked. Closure commit `62393dc` and equipment merge `2ba0c51` belong to master. 262 lines. |
| REMOVE | [Portable flashlight plan](../superpowers/plans/2026-10-13-portable-flashlights.md) | Explicit Complete status; all 18 steps are checked. Closure commit `b9749b7` and equipment merge `2ba0c51` belong to master. 253 lines. |
| REMOVE | [Survival merge plan](../superpowers/plans/2026-10-13-survival-weather-merge.md) | The task covered integration into master. Merge `ce15f82` belongs to master. The [completion report](2026-10-12-survival-sleep-weather-completion.md#master-integration-2026-10-13) records that integration. 42 lines. |

The four plans have no incoming filename references in the other checked tracked text files.
Git retains their instructions, user overrides, and historical results.
The completion marks do not establish current gameplay or source parity.

Keep the [plate specification](../superpowers/specs/2026-10-12-license-plate-text-design.md),
[mirror specification](../superpowers/specs/2026-10-12-live-mirrors-design.md),
[flashlight specification](../superpowers/specs/2026-10-13-portable-flashlights-design.md), and
[survival specification](../superpowers/specs/2026-10-12-survival-sleep-weather-design.md).

## Plans to retain

Do not infer completion from feature names or file dates.
The following plans need their current evidence or remaining work.

| Plan under `docs/superpowers/plans/` | Mark | Reason |
| --- | --- | --- |
| `2026-10-09-garage-start.md` | HOLD | 116 unchecked steps; approved garage decisions remain relevant. |
| `2026-10-10-complete-asset-library.md` | HOLD | 50 unchecked steps; the library remains a required development input. |
| `2026-10-10-day-night-lighting.md` | HOLD | Two visual checks remain unchecked. |
| `2026-10-10-interaction-parity.md` | HOLD | 69 unchecked steps; reconcile completion before marking removal. |
| `2026-10-10-material-parity.md` | HOLD | 62 unchecked steps; retain source and rendering decisions. |
| `2026-10-11-rendering-completion.md` | HOLD | The roadmap links this plan as acceptance evidence. |
| `2026-10-12-save-load.md` | HOLD | 73 unchecked steps; retained browser fixtures still support save checks. |
| `2026-10-12-source-audio.md` | HOLD | 34 unchecked steps; audible acceptance remains incomplete. |
| `2026-10-12-source-world.md` | HOLD | 56 unchecked steps; current world export and runtime acceptance remain incomplete. |
| `2026-10-12-survival-sleep-weather.md` | HOLD | 12 unchecked steps; retain the feature acceptance limits. |
| `2026-10-14-cleaning-painting.md` | HOLD | 20 unchecked steps; the roadmap lists this feature as Partial. |
| `2026-10-14-correct-driving-physics.md` | HOLD | 30 unchecked steps; the roadmap retains driving acceptance work. |
| `2026-10-14-fast-startup.md` | HOLD | 119 unchecked steps; an active worktree owns this feature. |

## Files to keep

| Files | Mark | Evidence |
| --- | --- | --- |
| `src/**/*.rs` | KEEP | Module tracing reaches all 215 Rust source files, including conditional modules and explicit paths. No orphan file found. |
| Exporter Python modules | KEEP | Relative import tracing from `export.py` and `world_probe.py` reaches all 47 tracked modules outside tests. |
| `web` HTML, CSS, and JavaScript | KEEP | Link and import tracing from `web/index.html` reaches all 16 tracked shell files. |
| `web/assets/shaders/*.wgsl` | KEEP | All 36 shaders have code load or shader import references. |
| `web/assets/shaders/source_lighting.wgsl` | KEEP | Grass, terrain, and impostor shaders still import this file, despite an older removal claim in verification notes. |
| `scripts/*` | KEEP | All five scripts have callers or documented commands. The world sampling script supports missing source probes. |
| `tests/browser/save-menu.html` and `save-menu.js` | HOLD | The page has no incoming filename reference. It remains a manual menu fault fixture; that alone does not prove obsolescence. |
| Other retained tests and fixtures | KEEP | The existing test reduction retains critical checks. Missing world fixture data requires repair, not a file removal mark. |
| `vendor/bevy_render/` | KEEP | Cargo patches this crate into the build. Its ownership patch remains required. Keep upstream provenance and licenses together. |
| Approved specs, research, and `.superpowers` reports | KEEP | These files retain design authority, source findings, and acceptance limits. |
| `.cache`, ignored exports, `web/pkg`, and recovery files | HOLD | Local evidence and current runtime inputs share these locations. No blanket removal mark applies. |

## Audit limits

The Git index listed 586 paths. Of these, 21 test files already had local deletion marks before this audit.
Those deletions belong to the existing test reduction. This audit does not count them as new findings.
The checkout also contained existing code, test, script, and verification edits.
Two registered feature worktrees remain active: `codex/fast-startup` and `codex/inventory-clothing`.

Reference tracing establishes file connections. It does not prove that each function or shader operation runs.
No build or gameplay test ran for this document change.
Recheck status and references before executing removal, since other project work can add new dependencies.

net: -1018 lines, -0 deps possible.
