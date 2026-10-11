# Interaction decisions

These rulings preserve the approved implementation choices and their rework costs.

Ruling: Task 3 includes recovered glass and handle animation records because selected window slide endpoints are null. Cost if wrong: window data and motion need rework.

Ruling: Namespace Cargo target output by absolute checkout path inside the existing target volume. Shared /workspace paths reused stale application artifacts. Cost if wrong: extra cache disk use and rebuild time.

Ruling: Use source toggle/cycle commands for car switches and the three-state interior lamp mode. Serialized usable integers alone cannot represent the callbacks. Cost if wrong: control API and lamp state need rework.

Ruling: Keep the dedicated presentation/lighting.rs module instead of adding lamp code to presentation/mechanisms.rs — this follows the specification's modular boundary and isolates cached lamp bindings — cost if wrong: move the same bindings between presentation modules.

Ruling: Preserve selected-source seat and plate limits — Task5 recovered no pickup/part links for selected Lada seats, and plate slots start empty; export eligible empty slots but do not invent removable parts. Test occupied-seat rejection using compatible fixtures and record source IDs/fields — cost if wrong: recover and add omitted seat records and bindings.

Ruling: Ordinary point push stops on F release; source steering grip persists until next F press, excess distance, invalid target, or lifecycle cancellation — fpscontroller.cs3356-3388 explicitly latches the selected required grip, so the brief's broad release cancellation applies to ordinary push — cost if wrong: revise grip latch behavior and control wording.

Grip invalidation means loss of the physical owner or collider. Changing the aim ray does not release the grip.

Ruling: Replace general-part attachment-ray selection with the recovered held-part proximity scan — partscript.Scan uses scanLayer/radius and CanCraft, while attachTargetscript ray explicitly skips partslotscript triggers; this restores authorized source parity. Keep wheel targeting separate, shared HUD/F selection, atomic rejection, and cached query storage — cost if wrong: revise general-part scan selection, prompt wiring, and source acceptance tests.

Ruling: General-part overlap installation has no added player-aim or line-of-sight gate — source Scan selects nearby CanCraft slots; preserve occlusion on existing action rays and collision-safe held-part motion instead of inventing a scan restriction — cost if wrong: add an explicit scan obstruction rule and revise its source-parity acceptance.

Ruling: Keep pygltflib validation and its unconditional provisional-API warning — final reviewer confirmed the warning is dependency API status, not invalid export — cost if wrong: inspect future warning changes and add targeted handling if they indicate real failures.
Ruling: Preserve the continuous physics route on53a9a4e while applying the authorized mirror label-only patch; verify names on the rebuilt package separately. Source behavior and physics stay unchanged. Cost if wrong: repeat affected browser actions on the final package if the patch changes more than labels.

Ruling: Preserve the interrupted continuous preparation/cabin route and use a normal-spawn session for remaining feasible gates, as root directed. Do not repeat the bench/fill prefix solely for browser closure or claim full continuous acceptance. Cost if wrong: rerun the full route after resolving the browser closure.

Ruling: Run remaining genuine-input browser checks in a Linux Docker browser with no host display mount, as root authorized. Match official Playwright image1.62.1 to the installed library; mount checkout read-only and evidence output separately. This avoids Windows desktop cursor confinement. Cost if wrong: repeat affected checks on a proven isolated surface; Linux results do not establish Windows rendering or performance.
