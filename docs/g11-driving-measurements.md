# G11 driving measurements

Status: partial evidence. Task 2 remains incomplete. The measured wheel model is not selected.
Tasks 3 through 7 remain blocked. G11 remains Partial in the roadmap.

The source authority is Steam build `11140890`, with Unity `2021.3.18f1`.
This document records one copied-wheel contact case. It does not establish assembled-Lada behavior.

## Contact protocol

The rig copies full-tire wheel `level1:58684` onto measured static support.
The copied body has frozen rotation and zero drag. The original wheels remain disabled.
The initial declared body velocity is world `(0, 0, 5)` m/s.

| Row | Torque scale | Side velocity delta, m/s | Wheel enabled |
| --- | ---: | ---: | --- |
| Baseline | 0 | 0 | Yes |
| Longitudinal | 0.25 | 0 | Yes |
| Lateral | 0 | 0.5 | Yes |
| Combined | 0.25 | 0.5 | Yes |
| Disabled combined | 0.25 | 0.5 | No |

Each row has three fresh native processes and 240 solves. All fifteen processes exited zero.
The capture and explicit production verification qualified all 3,600 solves.
Actual verification output equals the saved receipt byte for byte.

Step 121 applies one additive velocity assignment from saved operands.
The assignment remains present for zero delta and disabled rows.
The motor event covers steps 121 through 144. Step 145 clears that event.

Saved binary32 words qualify the parsed delta, cached sideways vector, pre-assignment velocity, setter argument, and entry readback.
The recorded setter arithmetic uses binary32 multiplication followed by binary32 addition.
These words qualify the direct velocity boundary. Other body observations retain the limits of producer text.

All twelve enabled runs have exact matching physical Initial and step-121 pre-input states.
Matching preserves types, shapes, finite values, and signed zero. The existing instance-ID and source-provenance exclusions remain unchanged.
All three repeats within each row also pass full Initial-state matching.

## Step-121 body response

The private analysis keeps the existing bookkeeping. Each operand conversion below uses the bound `f32` helper.
That helper converts through `struct.pack` and `struct.unpack` with the binary32 format.
Other arithmetic uses Python binary64 operations in the recorded order.
A conversion cannot recover native getter bits lost in producer text.

For each axis, the analysis records a separate control jump:

```python
	m = f32(mass)
	dt = f32(fixed_dt)
	control_jump = m * (f32(entry_velocity) - f32(pre_input_velocity))
	gravity_term = m * f32(gravity) * dt if use_gravity else 0.0
	J = m * (f32(exit_velocity) - f32(entry_velocity)) - gravity_term
```

Gravity is subtracted once. The consuming residual does not subtract the control jump again.
The contrast keeps this order:

```text
	((J_combined - J_longitudinal) - J_lateral) + J_baseline
```

Twelve enabled step-121 extractions and three independent contrasts passed all gates.
The disabled rows retain unavailable responses as `None`. They provide no zero vector or contrast term.

| Repeat | World x, N s | World y, N s | World z, N s |
| --- | ---: | ---: | ---: |
| 1 | 0.0 | 0.0 | -0.05558932296963255 |
| 2 | 0.0 | 0.0 | -0.05558932296963255 |
| 3 | 0.0 | 0.0 | -0.05558932296963255 |

These are stored total body-response contrasts over one solve. No average or response recalculation produced this table.
They do not isolate native tire forces or establish equivalence with an earlier impulse protocol.

## Retained proof

Paths in this table start at `.cache/g11-driving/`.
The records remain local proof artifacts. They are not a complete published fixture set.

| Record | Path | SHA256 |
| --- | --- | --- |
| Qualified native receipt | `task-2-contact-direct-input-qualified-01-storage/native/receipt.json` | `d50757c497e1e0af86ac09dd3a343a7e2be2b6671f399fffee7cb7ea345c1d5d` |
| Native delivery | `task-2-contact-direct-input-qualified-01-execution/delivery.json` | `879648678282c00e2a926f855288a5898194b4e512e139ce1bdf1a3b06a781fc` |
| Comparison result | `task-2-contact-direct-input-first-pulse-execution-01/results/comparison-result.json` | `7d5b23e533d5db50310f66787e8b089c35ba19635573fb73d435ce729feb791b` |
| Analysis delivery | `task-2-contact-direct-input-first-pulse-execution-01/delivery.json` | `6b213244116ffc68831ee6b0e289bef26466024491c4fd486296bb7a7ab10cbf` |

The native compiler retained 3,840 warnings, two hidden diagnostics, and zero errors.
Four runs retain camera teardown concerns after sample flush. Source teardown remains unproved.
Separate packaging and metadata-inspection failures remain disclosed. Neither caused a native or numerical retry.

## Open evidence

The newer angular observer records saved quaternion, tensor, scale, and velocity words with schema 2.
Its four test modules report 39 passes. The retained import checkpoints are empty because the logger compared different path spellings.
The source snapshots and actual C# execution records remain available.
Two focused checks passed with actual module imports recorded before and after each check.
Those records apply only to these two checks. The earlier 39-test import gap remains disclosed.

One private extraction of the existing compiler phase passed against 125 verified original Managed DLLs.
It retained 3,840 warnings, two hidden diagnostics, and zero errors. No game assets were copied.
This proves observer compilation only. Its only Setup failed during asset copying because the disk filled.
That Setup did not reach compilation. It remains a failed record and was not retried.

The compiler delivery is `task-2-angular-shipped-compile-check-01/delivery.json` under the proof root above.
Its SHA256 is `84ad3f2d0c5e380072be8338ee7e12e39b43ec0687e43492c20f99060237239a`.
The separate report addendum preserves and corrects the earlier import claim.

The angular selector remains unresolved. A five-row observation proposal exists. No new query is approved.
Tire, rim, hub, suspension, dynamic-support, assembly, and source save/load measurements remain incomplete.
The full source matrix, independent model confirmation, measured model, fixture publication, and comparison checks remain pending.
Rust solver changes, release builds, full checks, and browser acceptance have not run for this work.
