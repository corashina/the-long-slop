# Local GLES sampler patch

Source: crates.io `wgpu-hal 29.0.4`.
Upstream revision: `e99f5305ded96ff7006f0714d043a7f735bd45c2`.
Published crate SHA-256: `97ace1c17727311c22a46e4e3faf56ea6de81af99dcc839bdfb54857b94d448d`.

The GLES encoder records the last emitted sampler for each texture unit.
It skips a repeated binding after resolving the existing pipeline sampler map.
An outer `None` means unknown. `Some(None)` means an explicit unbind.
Each encoding and each render or compute pass starts with unknown entries.
The fixed array adds no heap allocation to the render path.

The patch changes `src/gles/command.rs` and adds `src/gles/sampler_tests.rs`.
It does not cache textures, attributes, or buffer bindings.
Uploads do not bind samplers. Presentation can unbind unit zero between submissions.
The encoder never reuses a cached binding across passes, encoder restarts, or context replacement.
Existing wgpu resource validation owns resource lifetime.
Sampler handles and parameters remain immutable while recorded commands use them.
Resource deletion cannot reuse a cached handle in another encoding or pass.
WebGL sampler bindings belong to texture units. They do not belong to texture targets or vertex arrays.
Texture targets, texture parameters, uploads, vertex arrays, attributes, and buffer ranges keep their existing behavior.

Keep the upstream licenses and package sources.
The upstream README's final blank line is removed to pass the repository whitespace check.
On dependency updates, review sampler mutation sites and encoder boundaries again.
Remove this patch when upstream provides equivalent behavior.
Run the four `sampler_cache_` backend tests and repeat matched browser tests after updates.
Use `scripts/check-gles-samplers.ps1` from the project root.
The native checks use this crate's published lockfile, including its upstream test dependencies.
The game uses the project's lockfile.
