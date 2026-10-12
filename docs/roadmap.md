# Rendering roadmap

Updated: 2026-10-12.

The user approved source rendering completion in an isolated worktree.
The active branch is `codex/rendering-completion`, based on `dccd516`.
Keep the accepted gameplay, road, library, menus, instruments, and day cycle.
Do not merge or push this branch without a separate request.

## Accepted layers

| Layer | Evidence | Checkpoint |
| --- | --- | --- |
| Texture storage | Source storage audit, corrected exports, startup and pause checks | Included in shadow checkpoint |
| Shadows and sky | Three camera views, four main cascades, issued draws, GPU validation, upright image | `8182153` |
| Reflection probe | Six 64x64 faces, two mip jobs, validated publication, resize | `1b77a1a` |
| Motion producer | Camera and rigid motion fixture, matching submission tokens, resize | `538810e` |
| Camera effects | AO, TAA, blur, exposure, bloom, grading, dither, callbacks, resize and steady storage | `6148e66` |
| Stained glass | Both real door materials, two ordered overlap captures, probe views, resize and steady storage | `6f84766` |
| Lifecycle and streaming | Cold restart, readiness, terminal faults, probe distance, two actual rebases, fixed motion slots | Final branch commit |

The implementation uses exact 8-sample HDR rendering on the accepted hardware.
The WebGL backend uses attachment-only multisample renderbuffers and explicit resolves.
Shader consumers read the original D24 depth storage through a supported unfiltered float view.

## Final acceptance

Task 7 connects scene restart, view suspension, current-scene readiness, probe distance activation, and failure handling.
Healthy startup, pause, resize, restart, missing assets, invalid pipelines, and context loss passed native checks.
Restart resets scene histories and retains compatible storage. Terminal failure stops extraction, rendering, and history updates.
Whole-branch review found one motion streaming fault. The fixed-slot correction passed three critical tests.
Its scoped review and all five real-body native stages passed. Motion retained 780 GPU slots through streaming and both rebases.
The final native suite passed 240 tests. The branch is ready for review and remains isolated.
See the [implementation plan](superpowers/plans/2026-10-11-rendering-completion.md) and [verification record](verification.md).

Local evidence remains in the rendering worktree under `.superpowers/sdd/2026-10-11-rendering-completion/` and `.cache/layer*-browser/`.
Installed source reports remain under `.superpowers/evidence/material-parity/reports/`.
The old `codex/material-parity` branch is not required for this implementation.

## Limits

- No matched native frame establishes identical pixels.
- Equal-distance transparent ties use target identities. Native runtime identities remain unknown.
- Nine exported primitives lack authored tangents.
- Logical GPU byte counts exclude driver memory and overhead.
- Native operating-system minimize cannot be automated with the available browser tools.
- The in-app browser refused pointer capture. Continuous mouse control remains unverified for the final package.

The installed restart caller reloads the active scene with `LoadSceneMode.Single`.
That source proof resolves the former reload-versus-reset uncertainty.

Dynamic vehicle mirrors, plate text, audio, survival, other vehicles, saves, multiplayer, and deployment remain outside scope.
