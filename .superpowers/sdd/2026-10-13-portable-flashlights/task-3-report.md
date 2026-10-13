# Task 3 report: portable flashlight beams

Status: Task 3 complete. Source export, release build, paired route, and parent browser review passed. No automated tests or assertions were run.

## Changes

- Imported flashlight roots now bind as ItemVisual roots. Their authored light children follow live item poses after carry, storage, and origin updates.
- Both source controllers use the existing SourceLighting path, source spot values, shadows, render masks, and on/off material choices. The newer body remains source-inactive in gameplay.
- The custom source cookie and both 256-pixel flashlight cookies share one 512-pixel three-layer texture array. Layer 0 keeps the existing 512-pixel cookie. Layers 1 and 2 center the flashlight images with edge padding. Each source mip is copied; array mip 9 repeats each flashlight's 1-pixel mip. The existing sampler and single custom-cookie binding stay in use.
- Binding 103 now declares a D2Array view in the source material layout and both source shaders. The same source layout serves normal, mirror, and plate materials.
- Storage initialization runs after physical binding, across two yielded loading frames and three source QuickFreeze passes. Only the first pass prescans. initialize_frame(..., dt: f32) returns ownership change; initialization_ready() gates readiness after frame 5. GarageRuntime syncs car mass and item poses on ownership change. Paused frames add zero storage time.
- Existing renderer, plate, mirror, and restart gates remain in use. The retained array handle avoids a new cookie asset on each restart.
- The full selected export now retains the inactive newer source Body and Light meshes. The node loop refreshes each source identity before it applies the mesh gate. A source-to-glTF validation checks every enabled renderer mesh in both flashlight hierarchies.

## Source and control flow

The imported item root owns the visual transform. Its light child supplies the source local pose; the shared lighting binder reads the updated global pose. World E and held LMB use the existing LightControl action. A state change selects retained material choices and the same source spot and shadow path. Restart rebuilds runtime ownership and OFF state through GarageRuntime::new. No camera light, second beam renderer, or newer-model acquisition path was added.

The source mountStuff.StartFreeze sequence yields twice and then calls QuickFreeze three times. QuickFreeze prescans only on pass one; FreezeUpd compares prior and current overlap sets and still checks the dropped-time delay. Loading advances storage time once per real unpaused frame. Normal storage advance starts after readiness.

## Build and paired receipts

The final scoped ./scripts/build.ps1 release build passed in 1m 39s against the corrected full source export. Parent product check at 1280x720 reached garage ready, enabled Start and Controls, with ready=true, errors=0, glErrors=0, captureDraws=383, and three mirrors.

The ignored diagnostic route is http://127.0.0.1:4187/assets/reference/flashlight-beam-stage/index.html. The final builder compiled the route in 1m 28s. It saved production src/app.rs and all four web/pkg files before the probe, restored them in finally, and compared exact bytes. Receipt: .cache/equipment-checks/flashlight-beam-stage-pairing.json.

| File | Production SHA-256 before and after |
| --- | --- |
| src/app.rs | 5567e49f6bfd6307bebd436a1b17f1452b82d66c9400baa6fd276274e8a37c0a |
| my_summer_slop.d.ts | 59d9168bacda322f41c9f7473dc6893c8229cc46227e96766623dd37e54b5949 |
| my_summer_slop.js | a6a58cad9b9e2df83c335255e6a6238f60da9a702c640bd075d04bb9e57be40e |
| my_summer_slop_bg.wasm | c3c736cb7a64a6263073b22a153ab3adc627030e286b1e75ee58df008b374611 |
| my_summer_slop_bg.wasm.d.ts | 5b3a208bb989d28af8df99437fca52f6209e9dc5b953c03e83fc64cc49c862ab |

Probe WASM SHA-256: 083aacc30115fa58d05c7afa32435180d597d2ee63629e021286eb2ee61737bb.

The accepted source manifest stayed at 254830925305facc97113479ecdea6f154389ed354afa42aa996ead492d5c04d. The accepted library manifest stayed at 92a93abde82f7933090293e8c95880f98dcee1d50a7463e74b108e29e7b2db99. Managed Assembly-CSharp.cs stayed at 22ddc62b91954205d90ff513232f39becebbc58a71b718685075f91f64b1f3ee. The atomic full export produced items.gltf SHA-256 015a25dfb58694d7ca8700b40886b6aa43165a186bbbf9251f88593432158191 and manifest SHA-256 3e1914fe710147da8b7d39cc17518a78c8a5502ca9b94adbd96a49bfe33db7f2. Its inactive newer Body node has mesh 17 from sharedassets1.assets:3286. Light has mesh 18 from sharedassets1.assets:3858. Generated reference assets are gitignored. The committed exporter regenerates this pair.

## Diagnostic route

The route labels scripted stages, source models, and the current physics state. Its header marks stage 11 ACTIVE PHYSICS and stages 0 through 10 PAUSED PHYSICS. Start the garage, choose Model, Beam, or Wide beam view, then select stages in order: 0 world ON; 1 pick up; 2 held OFF; 3 held ON; 4 drop; 5 trunk lock; 6 rejected pickup; 7 origin shift; 8 unlock and pick up; 9 held plus world hint; 10 scripted inactive newer prototype ON; 11 active mirror view with scripted prototype ON and physics running. Stage 10 places the disabled prototype 0.9 m above the player-front point and unmasks it for inspection. Stage 11 aims the camera at the first source mirror. It logs the active capture count after four frames. Neither stage adds gameplay acquisition.

## Visible review

Parent saw the actual older green model and lens. Wide beam view showed warm ceiling illumination with world ON. Held OFF visibly darkened the same area while the item stayed held. Source stage screenshots were saved by the parent. The stage log shows carry, trunk lock, rejected pickup, rebase, and unlock with the light ON. Trunk and post-rebase camera views clip car geometry, so those views do not establish beam shape. The corrected prototype route reached ready=true, errors=0, glErrors=0, and captureDraws=385. Its settled root local/global poses equal item/body pose [37.781002, 3.4149985, -69.666]. The imported newer hierarchy has eight mapped nodes and two visible ECS mesh children. Parent saw the actual yellow/black newer model and white wall beam. The wide view showed the beam under occlusion. A stage 11 active mirror check reported started=true, paused=false, mirror GPU ready=true, one active capture camera, and the newer prototype body still disabled. Target 1 pixels were nonzero and changed hashes across active frames while captureDraws rose; the route stayed ready with zero browser and GL errors. Parent saw reflected car and interior geometry in the real mirror. Capture draws rose from 385 to 806435. The white beam detail was not separately readable in the mirror, so the mirror check establishes shared-path eligibility and an active reflected scene, not beam-shape parity.

## Self-review and limits

The custom cookie array is created once after all three source images load. No image packing runs in a frame update after binding. The original sampler is retained. The array uses 10 mip levels indexed 0 through 9; the two 256-pixel images have 9 source levels indexed 0 through 8. The padded edge and final repeated mip keep each layer isolated.

The first product browser check exposed a binding 103 D2-versus-D2Array validation error. The shared bind-group layout and plate shader were corrected; the later product check reached ready with zero browser and GL errors. The first prototype check exposed two missing scene meshes. A full-export trace found a stale per-node identity in the exporter mesh gate. The corrected full export and provenance validation now include both source meshes. The diagnostic uses scripted placement, paused physics in stages 0 through 10, and active physics in stage 11. It shows source models and state changes but does not establish matched Unity appearance or continuous driving. No automated tests or assertions were authored or run.
