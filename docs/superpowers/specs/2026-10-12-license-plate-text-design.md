# License plate text

Date: 2026-10-12, Europe/Warsaw.
Status: The user approved this spec on 2026-10-12. Product implementation has not started.
Integration base: local `master`, mirror commit `62393dca1aab66406edb6e446d63708483dc08fa`.
Reference: installed Steam build `11140890`, Unity `2021.3.18f1`.

## Outcome and scope

The selected Lada must show generated text on its two source plate objects.
Text must remain correct when a plate moves, detaches, drops, or mounts again.
Use the existing carry, interaction, physics, and lighting systems.
Complete plate verification before starting portable flashlights.

The current recreation has empty plate slots.
Recovered activation and attachment records support two startup plates in this selected vehicle.
Restore only these source instances. Do not add spare plates, loot, purchase paths, or other vehicles.

This feature includes the plate attachment targets needed for removal and mounting.
It excludes free-surface gluing, trailers, multiplayer, save-format changes, and full deferred rendering completion.
These exclusions do not change the recovered source rules.

## Source authority

Use the accepted source library and checked managed code.
Keep extracted font data, textures, shader blobs, and generated vehicle assets outside Git.
Leave installed Steam files and the primary source library unchanged.

| Receipt | SHA-256 |
| --- | --- |
| Accepted source library manifest | `92a93abde82f7933090293e8c95880f98dcee1d50a7463e74b108e29e7b2db99` |
| Managed `Assembly-CSharp.cs` | `22ddc62b91954205d90ff513232f39becebbc58a71b718685075f91f64b1f3ee` |

Selected records were read through the library reader.
A full library audit and matched Unity runtime capture were not performed for this recovery.

| Object | Front | Rear |
| --- | --- | --- |
| Plate root transform | `level1:22542` | `level1:30779` |
| Game object | `level1:1283` | `level1:10596` |
| Current runtime item | `104` | `111` |
| `rendszamscript` | `level1:78750` | `level1:73955` |
| `TextMeshPro` | `level1:64254` | `level1:64270` |
| Text transform | `level1:61809` | `level1:62878` |
| `attachablescript` | `level1:66420` | `level1:71250` |
| Startup attachment target | `level1:66346` | `level1:66601` |
| Target transform | `level1:28032` | `level1:38387` |
| Save component | `level1:65530` | `level1:72350` |
| Pickup component | `level1:65528` | `level1:72351` |

Runtime item indices identify the current export. Bind persistent references through source IDs.
There are 64 plate script records in the accepted catalog.
Their presence does not authorize spawning all 64 objects.

## Startup evidence

`TerrainGenerationSettings`, `level1:64898`, links `StartCar02` to `Car02Start`, `level1:206`.
Its `EnableAfterStart` list includes `StartSpawnItems`, `level1:117`.
The selected `Car02Full` hierarchy contains both active plate descendants.
Its parent containers start inactive.

`TerrainGenerationSettings.FStart` enables startup objects for a new single-player game.
Its `car02` selection activates `StartCar02` and removes the other car containers.
This exposes the two existing plate instances.

Each plate has an `attachAtStartTarget` link.
`attachablescript.Start` calls the save component's `FStart`, then attaches to that target.
Both save records have `manualStart=false` and no startup removal or random scale rule.

This code and record chain supports restoring the two startup plates.
It does not prove the exact appearance or callback order of an assembled Unity run.

## Text generation and lifetime

Recover `rendszamscript` from managed code at lines 51227 through 51281.
Preserve its `Set`, `Same`, `Updated`, and conditional `Start` behavior.

`Start` generates three uppercase letters, a hyphen, and three digits when `same` is false.
Letters use integer `Random.Range(65, 91)`. Digits use integer `Random.Range(48, 58)`.
Both serialized strings are empty. Both serialized `same` values are false.
The rear script's `sameAsThis` link points to the front script.
The front script has no such link.

A rear update copies its value to the front and marks the front as shared.
Both visible labels must agree after startup completes.
Preserve this link and initialization rule. Do not replace them with a fixed example string.

Use the source Unity integer random calls as the generation contract.
Keep startup draws separate from the lamp generator, which implements a different source random API.
Inject integer draws in critical generation tests.
Record any difference in random state or callback order in the acceptance report.
Matching the generated format does not prove an identical Unity random sequence.

Store each plate's string with its item identity.
Removal, carrying, dropping, mounting, pause, resize, and origin changes must not regenerate it.
A new game initializes new plate instances through the startup rules.
Restart must release the old instances and their text meshes.

## Font and layout

Both labels use `sharedassets0.assets:1366`, `LiberationSans SDF`, version `1.1.0`.
The source font contains 250 glyphs and 250 character entries.
All generated letters, digits, and the hyphen exist in this font.

Export the source glyph metrics, character mapping, pair adjustments, and atlas coordinates.
Use atlas `sharedassets0.assets:366`, with a 1024 by 1024 extent and padding 9.
Use the source font scale, baseline, bearings, and advance values.
Do not use a browser font, system font, or replacement atlas.

| Text setting | Recovered value |
| --- | --- |
| Font size | `0.8` |
| Font style | `1`, bold |
| Font weight | `400` |
| TMP text color | White; the shared material supplies the dark face color |
| Horizontal alignment | `2` |
| Vertical alignment | `512` |
| Kerning | Enabled |
| Word wrapping | Disabled |
| Margins and spacing | Zero |
| Local rotation and scale | Identity rotation; scale one |
| Local text offset | Source Z `+0.003125`; runtime Z `-0.003125` |

Resolve alignment codes through the source TMP definitions.
Preserve the source text bounds and sizing flags in the typed contract.
Apply the existing coordinate conversion once.
Preserve parent transforms, signed scale, glyph winding, and the outward reading direction on each plate.

Serialized editor text contains `-23` followed by a degree symbol and `C`.
Its baked glyph geometry is obsolete.
Generate geometry from the runtime plate string and source glyph data.
Keep unrelated TMP text outside this feature.

## Material and rendering

Both labels share material `sharedassets0.assets:18`, named `rendszam`.
Its shader is `sharedassets0.assets:730`, `TextMeshPro/Distance Field (Surface)`.
Source passes include `FORWARDBASE`, `FORWARDADD`, and `SHADOWCASTER`.
Use its lit distance-field surface behavior with the current lighting system.

| Material setting | Source value |
| --- | --- |
| Face RGB and alpha | `16/255` per channel; alpha `1` |
| Outline color | White |
| Face dilation | `0.1` |
| Outline width and softness | `0.1`; `0.2` |
| Gradient scale | `10` |
| Perspective filter | `0.875` |
| Scale ratio A | `0.9` |
| Bold weight | `0.75` |
| Main texture | Atlas `366` |
| Face and outline textures | `252` |
| Bump texture | `271` |
| Valid keywords | Empty |

Recover active shader semantics from the source shader and material.
Do not enable optional effects merely because their serialized fields exist.
Use source sampling, alpha, lighting, and depth rules for the active shader.
Record any unsupported effect as a verification limit.

Share the atlas and material across both labels.
Build glyph geometry during startup or a text change.
Do not allocate font assets, materials, or geometry during frame rendering.
Release replaced geometry and game-owned instances through the existing lifecycle.

Register text layouts and material variants during renderer loading.
Warm main-view and mirror-capture variants before readiness.
Mounting, removal, resize, and restart must not create unknown pipelines after readiness.
Preserve the mirror capture feedback guard and bounded render targets.
Coordinate these additions with the rendering completion plan.
This feature must not expand into deferred camera or lighting replacement.

## Attachment and interaction

The current part exporter handles `partscript` and `partslotscript`.
Plates instead use `attachablescript` and `attachTargetscript`.
Add a separate typed attachment contract for the selected plate records.
Do not invent part slots or duplicate plate items.

Both plate and target types include `licenseplate`.
Both plates use `parentAtAttach=true`, `lockRotation=true`, and `targetPosRot=true`.
The front target belongs to the front bumper. The rear target belongs to the car.

Use the existing mount and removal actions.
Source removal uses `mountDown`; target mounting uses `mountUp`.
A mounted plate follows its target position and rotation.
Removal applies source `removeTargetPos=(0, 0, -0.1)` in plate coordinates before unparenting.
Recover source mass and collider rules without assigning guessed values.

A detached plate uses the existing carry and rigid-body systems.
Carry acquisition and mounting must keep item, collider, body, and target ownership consistent.
A rejected transaction must retain the previous state and string.
Mounting must release the carry constraint only when the attachment can complete.

Removing the front bumper must retain its mounted plate and text.
Removing the plate alone must detach its own hierarchy.
An origin change must preserve world pose, target alignment, and text orientation.

## Failures and data flow

Source records feed a typed plate contract, font assets, and selected attachment targets.
Startup creates item state and strings. Text changes create glyph meshes.
Interaction changes item ownership. Rendering reads retained meshes and shared materials.

Validate required source links, generated glyph coverage, atlas bounds, and material dependencies during export.
Reject incomplete required data with a source-ID diagnostic.
Do not silently substitute a font or create an unlinked plate.
Keep export staging and promotion transactional.

At runtime, reject invalid mount or carry transitions without partial ownership changes.
Use existing loading diagnostics for missing text assets or pipeline failures.
Keep source data recovery separate from interaction and presentation code.

## Acceptance

Implement only critical tests and visible browser checks.

| Check | Required evidence |
| --- | --- |
| Source binding and startup | Exactly two selected plate instances; correct pickup, script, and target links |
| Generation | Correct character ranges and format; source sharing rule; stable string through item transitions |
| Glyph layout | Source metrics, kerning, bold style, alignment, atlas bounds, and coordinate conversion |
| Mount and removal | Same item identity; correct target pose; source removal offset; no orphan body or collider |
| Rejected interaction | Previous ownership and carry constraint retained |
| Bumper removal | Front label follows the removed bumper while its plate remains mounted |
| Resource reuse | Shared atlas and material; no growth across repeated detach, mount, resize, and restart cycles |
| Origin and pause | Correct pose after rebase; no text mutation while paused |
| Mirror integration | Text renders when eligible; no sampler conflict, feedback, or late pipeline creation |
| Visible browser checks | Readable front and rear text from outside; visible carry, drop, remount, resize, and restart states |

Run the repository checks and the selected installed-source binding tests.
Use the actual imported Lada for visible checks.
Record screenshots, console errors, pipeline counts, and resource counts.
Label scripted interaction checks and any trusted-input limits.

Completion requires passing critical interactions and visible browser checks on the same implementation revision.
Do not claim matched Unity appearance or random-sequence parity without corresponding source runtime evidence.
