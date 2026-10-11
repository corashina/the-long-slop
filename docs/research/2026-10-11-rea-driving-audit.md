# REA driving audit

Date: 2026-10-11, Europe/Warsaw.
Reference: installed Steam build `11140890`, fixed Lada 2101.
Product commit: `7e333eb`.

The audit confirms gaps in keyboard input, bare-rim grip, and combined braking.
Source input smoothing is the best first change. It does not require a new physics model.
Keep ILSpy, dnfile/dncil, and UnityPy as the main research tools.
REA 6.4.0 identifies this assembly, but its public member reader cannot complete the inspection.

## Tool results

REA ran from `.cache/rea-audit/runtime/`. Node was `24.18.0`; npm was `11.16.0`.
The cache lockfile retains the downloaded package versions and integrity values.
Installation used `--ignore-scripts`. No agent registration or product dependency changed.

| Check | Result |
| --- | --- |
| Public `inspect-managed-artifact` | Complete metadata; Unity Mono; 637 types; 3,124 methods |
| Public `inspect-managed-members` | Unavailable; fixed 67,108,864-byte metadata representation budget |
| Targeted internal reader probe | 136 method bodies present; no reported decode issues |
| Independent dnfile/dncil comparison | All 136 selected methods passed |
| Fresh ILSpy reconstruction | Eight selected types recovered |
| Live Unity parameter reads | Selected car, global grip values, and steering sliders recovered |

The public member command returned exit code zero and `ok: true`, despite unavailable coverage.
Check coverage and issues before using an REA result.

The targeted probe calls unchanged internal readers from the installed REA package.
It keeps each method within the existing decode budget. It does not change REA files or limits.
This throwaway probe is not a public CLI capability or canonical REA Evidence.
Do not add it as a product tool or claim complete assembly coverage.

The sandbox blocked REA's `realpath` checks, including checks on the cached copy.
The public static commands passed those checks outside the sandbox.
They inspected assembly bytes without executing the target.

## Source identity and verification

Assembly: `TheLongDrive_Data/Managed/Assembly-CSharp.dll`.
Size: `964096` bytes.
SHA-256: `9919a9eeddcce23bb4ff86e890437f32b4a3ef3c10af2efe3b8d889d894dd62e`.
MVID: `564f67d4-f740-4580-9ac6-28c175752469`.

The cached assembly hash matches the installed assembly.
dnfile/dncil independently checks method identity, signatures, RVAs, raw IL hashes, lengths, instruction counts, and opcode counts.
It also checks decoded anchor offsets, opcodes, and metadata tokens.
The oracle checks the module identity and metadata counts against REA's artifact result.

Selected types: `carscript`, `inputscript`, `wheelgraphicsscript`, `enginescript`, `gumiscript`, `wheelscript`, `slidersettingscript`, and `settingsscript`.
ILSpy reads the installed assembly with cached `ICSharpCode.Decompiler` version `11.1.0.9782`.
Its C# reconstruction remains an interpretation, separate from the bytecode checks.

## Confirmed differences

### Keyboard input

`inputscript.DriveStuff`, token `0x060002f2`, moves keyboard steering toward its target over time.
`settingsscript.RefreshSettings` obtains the rate through controls slider 7.
The selected menu and gameplay sliders both have default value `1.25`.
`slidersettingscript.FStart` changes their zero multiplier to one before `GetValue` returns the value.
This establishes the source default. A saved player setting can override it.

The Lada's `speedBasedKeyboardSens` and `keySteerSensMultiplier` are both zero.
Use the ordinary setting rate. Do not add speed scaling for this selected car.

Our `starting_area.rs` constructs steering from held key bits. It supplies full lock in the first step.
It also passes a binary throttle value to the engine.
The source moves keyboard throttle and brake values toward their targets at five units per second.

The formula probe uses a 60 Hz step and the recovered defaults:

| Time after press | Source steering magnitude | Current steering magnitude | Source throttle |
| --- | --- | --- | --- |
| 1/60 second | 0.020833 | 1 | 0.083333 |
| 0.1 second | 0.125 | 1 | 0.5 |
| 0.2 second | 0.25 | 1 | 1 |
| 0.4 second | 0.5 | 1 | 1 |
| 0.8 second | 1 | 1 | 1 |

These are computed input values. They are not Unity or browser driving measurements.

### Tires and bare rims

`gumiscript.Refresh`, token `0x060002ca`, maps condition states 0 through 4 to stiffness values 1.25 through 0.75.
Our quality value is `1 - state / 4`. Our formula `0.75 + 0.5 * quality` matches those numbers.
The earlier concern about unsupported tire-condition constants does not establish a numeric defect for this build.
Unity stiffness and Rapier `friction_slip` still describe different physics models.

`wheelscript.Refresh` uses global stiffness values when a rim has no tire.
The selected source values are about `0.4` longitudinally and `0.3` laterally.
`wheelgraphicsscript.WheelSize`, token `0x06000985`, transfers these values into separate friction curves.
Our wheel module assigns a bare rim quality of one. The vehicle code then assigns `friction_slip = 1.25`.
The adapter loses the distinction between a bare rim and a good tire.
A direct numeric substitution needs physics measurements; the two engines use different quantities.

### Combined braking

`carscript.SetWheel`, token `0x06000156`, assigns engine braking, then adds service and handbrake torque on eligible wheels.
Our `vehicle.rs` chooses handbrake, service brake, or engine brake through an `if/else` chain.
It discards other contributions when more than one applies.
The exported Lada has four service-brake wheels, two handbrake wheels, and two driven wheels.
Preserve source wheel eligibility when correcting this combination.

## First change to design

Add source keyboard input smoothing before drivetrain and steering updates.
Recover the default steering rate through the exporter. Carry it in the typed manifest.
Keep input state in a small module. Update it in fixed steps without allocation.
Keep walking input and push-grip steering on their existing paths.
Reset the smoothing state on seat changes, pause, capture loss, recovery, and restart.
Do not add settings UI or speed scaling in this change.

Use a critical test for steering rise, return, direction reversal, and lifecycle reset.
Verify throttle and brake ramps when extending the same module to those controls.
Then measure a normal-input browser route at different render rates.
Compare handling with the installed reference before claiming matching vehicle physics.

## Evidence and reproduction

Raw evidence, source reconstruction, and throwaway probes remain in ignored `.cache/rea-audit/`:

- `artifact.json` and `members.json`: public REA command results.
- `targeted.mjs` and `targeted.json`: selected internal reader probe.
- `decompile.ps1` and `ilspy/`: fresh ILSpy reconstruction.
- `verify.py` and `oracle-verification.json`: independent bytecode checks.
- `oracle-il/`: independently decoded instruction traces.
- `parameters.py` and `parameters.json`: live Unity fields and byte-consumption evidence.
- `behavior-probe.py` and `behavior-probe.json`: formula comparison.
- `runtime/package-lock.json`: installed package versions and integrity values.

Run the targeted checks from the repository root:

```powershell
node .cache/rea-audit/targeted.mjs
& .cache/rea-audit/decompile.ps1
tools/reference/.venv/Scripts/python.exe .cache/rea-audit/verify.py
tools/reference/.venv/Scripts/python.exe .cache/rea-audit/parameters.py
tools/reference/.venv/Scripts/python.exe .cache/rea-audit/behavior-probe.py
```

The research phase changed no product code. No live driving route or original runtime measurement formed part of that phase.
Steam files remain unchanged. The assembly copy, source reconstruction, and downloaded tools remain outside version control.

References: [REA managed analysis](https://github.com/morluto/rea/blob/main/docs/managed-code-analysis.md),
[REA installation](https://github.com/morluto/rea/blob/main/docs/installation.md), and the existing [verification record](../verification.md).

## Steering implementation

The first implementation adds keyboard steering smoothing. It uses the recovered default rate of 1.25 units per second.
The exporter reads controls slider 7 and writes the required `vehicle.keyboard_steering_rate` field.
The runtime rejects missing, nonpositive, or nonfinite rates. Regenerate older exports; there is no old-manifest reader.

`simulation/input.rs` keeps steering state and advances it at 60 Hz without allocation.
The runtime resets that state on seat changes, pause, capture loss, recovery, and restart.
Walking and push-grip steering retain their existing input paths.
Throttle ramps, bare-rim grip, and combined braking remain separate changes.

Verification passed: 162 native tests, 32 installed-source checks, 20 browser unit tests, and 19 exporter tests.
The release WASM package and isolated Linux browser startup check passed.
The [verification record](../verification.md#keyboard-steering-verification) states the browser coverage and physics limits.
