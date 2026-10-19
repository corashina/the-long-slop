# Startup Brotli delivery

Date: 2026-10-19. Branch: `codex/startup-brotli`. Base: `1103368e8f99ae0c1d20c7514820876b7166a7c7`.
The branch remains unmerged.

## Changes

The build prepares Brotli for WASM, the root manifest, and JSON files in the current runtime package.
It uses Node's Brotli stream at quality 9. It excludes images and historical runtime packages.
It writes compressed files and an index to `.cache/http-compression`, outside receipt-checked directories.
It publishes the index after the compressed files. Repeat runs preserve source bytes and receipts.

The server negotiates identity or Brotli. It checks source and compressed hashes before selecting Brotli.
It caches those checks until file metadata changes. Stale variants use identity when the request permits it.
It sets `Vary: Accept-Encoding`, distinct representation ETags, and encoded lengths.
It keeps GET and HEAD headers consistent and supports conditional requests.

The browser checks decoded asset lengths and SHA-256 against receipts.
It checks encoded header length syntax separately. Identity header lengths still must match receipts.

## Frozen inputs

We reused the installed WASM and export without changing their bytes. We did not rebuild Rust.
The installed WASM does not accept this export. Its release profile is unverified for this comparison.
The native package receipt check passed for the current export after compression preparation.

| Input | Decoded bytes | Brotli bytes | SHA-256 of decoded bytes |
| --- | ---: | ---: | --- |
| Manifest | 138,974,951 | 9,845,921 | `a041ac29e18acf889c3c5aa50a909ad103aa5ceb4b98e50402f4e7a714cef288` |
| WASM | 40,402,392 | 7,788,703 | `79f22d41b7a1f03ec1d3e3a1c9cceee580baea00debd99016642722f25fea4cc` |

The generator prepared 393 files. Their total size fell from 188,883,961 to 18,606,844 bytes.
These totals include current package JSON beyond the root manifest.

## HTTP checks

Run `node scripts/measure-startup-downloads.mjs` after compression preparation.
The script requests WASM, the manifest, and one current prototype plan through the real server.
It verifies decoded hashes and lengths, then repeats requests with their ETags.
It restarts the server for each identity/Brotli/identity run.
Its output is `.cache/startup-download-measurements.json`.

| Run | Transferred body bytes | Local elapsed time | Conditional responses |
| --- | ---: | ---: | --- |
| A, identity | 179,398,546 | 1,108.9 ms | 304, 304, 304 |
| B, Brotli | 17,636,925 | 1,475.3 ms | 304, 304, 304 |
| A2, identity | 179,398,546 | 785.2 ms | 304, 304, 304 |

Brotli reduced these body bytes by 90.17%. Conditional responses had no body.
These are localhost HTTP checks, not cold or cached playable startup measurements.
Other Docker builds were active. The window was not exclusive. These timings establish no startup speed gain.
The Brotli run was slower on this local connection, including decompression and integrity checks.

## Verification and limits

The web suite passed 46 checks, including negotiation, ETags, HEAD, stale files, decoded receipts, corruption, and cancellation.
The preparation check confirmed repeat runs, unchanged package files, and rejection of unsafe receipt paths.
PowerShell parsing passed for the build and check scripts.

The browser stopped before garage readiness with this installed-package error under identity and Brotli delivery:

```text
unknown field `exclusions`, expected one of `floats`, `colors`, `textures`, `defaults` at line 1 column 7001259
```

New game remained disabled. Playable startup, real input, physics, frame submission, driving, streaming, and Save/Load remain unverified.
Garage and driving average, p95, and p99 frame times remain unmeasured.
Compression changes transport only. It does not resolve the installed WASM/export mismatch.
The identity baseline also failed after a cached reload.

## Run

From this worktree, with a matching export and WASM:

```powershell
./scripts/build.ps1
node scripts/serve.mjs
```

Open `http://127.0.0.1:4173`. Restart the server after preparing new compressed files.
