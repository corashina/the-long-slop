# Source performance design

Implement the five changes approved in this chat on 2026-10-18.
Base: 7867d0e065cefabb87957cdb9addf1815fc94086.
Branch: codex/source-performance. Leave it unmerged and do not push.

1. Share identical immutable generated primary and shadow materials. Preserve independent mutable surface and LOD state.
2. Batch eligible motion-vector data and draws. Preserve history, forced-zero motion, camera isolation, and draw coverage.
3. Retain unchanged caster bounds. Avoid redundant caster validation and preparation. Preserve view and cascade membership.
4. Update physics query bounds only for changed colliders. Preserve query timing and collider lifecycle correctness.
5. Use the source physics interval, 0.02 seconds, through one authoritative constant.

Keep source camera settings, shadows, mirrors, probes, postprocessing, physics outcomes, and save behavior intact.
Do not add packages or change world generation, CCD policy, unrelated terrain changes, or source export tools.

Verify focused correctness checks, the native suite, and a locked release WASM build.
Use frozen assets and matched release packages for garage and driving comparisons.
Keep graphics counters separate from frame timings. Report runtime blockers and unmeasured gains.
