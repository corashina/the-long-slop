# Interaction decisions

These rulings preserve the approved implementation choices and their rework costs.

Ruling: Task 3 includes recovered glass and handle animation records because selected window slide endpoints are null. Cost if wrong: window data and motion need rework.

Ruling: Namespace Cargo target output by absolute checkout path inside the existing target volume. Shared /workspace paths reused stale application artifacts. Cost if wrong: extra cache disk use and rebuild time.

Ruling: Use source toggle/cycle commands for car switches and the three-state interior lamp mode. Serialized usable integers alone cannot represent the callbacks. Cost if wrong: control API and lamp state need rework.

Ruling: Keep the dedicated presentation/lighting.rs module instead of adding lamp code to presentation/mechanisms.rs — this follows the specification's modular boundary and isolates cached lamp bindings — cost if wrong: move the same bindings between presentation modules.

Ruling: Preserve selected-source seat and plate limits — Task5 recovered no pickup/part links for selected Lada seats, and plate slots start empty; export eligible empty slots but do not invent removable parts. Test occupied-seat rejection using compatible fixtures and record source IDs/fields — cost if wrong: recover and add omitted seat records and bindings.

Ruling: Ordinary point push stops on F release; source steering grip persists until next F press, excess distance, invalid target, or lifecycle cancellation — fpscontroller.cs3356-3388 explicitly latches the selected required grip, so the brief's broad release cancellation applies to ordinary push — cost if wrong: revise grip latch behavior and control wording.

Grip invalidation means loss of the physical owner or collider. Changing the aim ray does not release the grip.
