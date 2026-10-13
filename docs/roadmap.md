# Rendering roadmap

Updated: 2026-10-13.

The user approved source rendering completion in an isolated worktree based on `dccd516`.
On 2026-10-12, the user authorized merging rendering commit `67798ec` into local master `a5b9778`.
Keep the accepted gameplay, road, library, menus, instruments, and day cycle.
The merged package passed verification on 2026-10-13. Retain the rendering worktree and its local evidence.
Do not push without a separate request.

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
The rendering branch passed 240 native tests before this merge.
The merged checks passed 318 native tests, 58 installed-source tests, 87 web tests, and 182 exporter tests.
Seven final export checks passed. The final default release package built and passed browser startup and Load.
Controlled lifecycle checks passed six stages. Both vehicle records survived a manual save and full page reload.
Both probes published. Three main views completed, and three mirror submissions passed GPU validation.
The merged motion pool retained 953 slots through the checked scene replacements.
See the [implementation plan](superpowers/plans/2026-10-11-rendering-completion.md) and [verification record](verification.md).

Local evidence remains in the rendering worktree under `.superpowers/sdd/2026-10-11-rendering-completion/` and `.cache/layer*-browser/`.
Installed source reports remain under `.superpowers/evidence/material-parity/reports/`.
The old `codex/material-parity` branch is not required for this implementation.

## Limits

- No matched native frame establishes identical pixels.
- Exact native blend rounding remains unverified.
- Equal-distance transparent ties use target identities. Native runtime identities remain unknown.
- Nine exported primitives lack authored tangents.
- Logical GPU byte counts exclude driver memory and overhead.
- Native operating-system minimize cannot be automated with the available browser tools.
- The in-app browser refused pointer capture. Continuous mouse control remains unverified for the final package.

The installed restart caller reloads the active scene with `LoadSceneMode.Single`.
That source proof resolves the former reload-versus-reset uncertainty.

Source vehicle, interaction, and Lada radio audio are implemented. Assembled audible acceptance remains incomplete.
The user stopped further checks. See the [audio verification report](research/2026-10-12-audio-verification.md).
Master now includes save/load, the source fleet, and live mirrors.
Survival, multiplayer, and deployment remain outside scope.
