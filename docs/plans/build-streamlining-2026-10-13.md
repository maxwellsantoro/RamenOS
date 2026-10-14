# Build/test streamlining review resolution — 2026-10-13

This completes the build/test review alongside the [R1–R4 host reliability
repairs](editor-reliability-review-2026-10-13.md). Changes are implemented and
independently reviewed in the local candidate. Hosted five-lane qualification
and complete Linux56 preflight remain separate prerequisites.

## Changes and scope decisions

| Review issue | Implemented behavior and controls |
| --- | --- |
| Full developer work on every iteration | `dev-test`, `dev-compile` and `dev-lint` run one selected Cargo command; `dev-check` retains build/lint/test. Target/mode changes share package/feature compiler state with fresh command records. |
| Stripped caller job limits | Admit canonical 1..32 jobs, explicit recipe/CLI first, environment second, default two. Acceptance restores only the admitted limit after environment sanitation. Invalid settings deny before compiler children. |
| Unrelated Store WAT dependency | WAT is optional under `agent_task_v1_dev`; actual locked default/editor graphs exclude it. Agent fixtures and exporter retain the explicit feature and actual valid Wasm bytes. No lock/dependency version change. |
| Fresh Task/reliability compiler targets | Exact source/lock/configuration/toolchain/platform/environment keys and owned phase locks; default-off, enabled and std verifier targets remain distinct. Fresh assertions/Clippy/readers/nonces and retained binaries on every run. Publication failure quarantines rather than clearing ownership. |
| Repeated filtered builds and tests | Cargo-selected executable metadata, real discovery and exact positive selector unions. Semantic subsets are verified in the complete suite; Projection's nine Store selector families run together. Mixed library kinds, explicit `test=false` integration selection and genuine example harnesses are preserved. Eligible library doctests execute once; binary-only packages record the actual no-doctest decision. Missing Wasm/empty/ignored/extra/missing/failed results deny. |
| Cross-gate generic Native Runner duplication | Investigated with actual source/binary/fixture/environment identities. Warm scope 1.065s, actual 3-test runtime 0.141s; retain both fresh assertions rather than add result-sharing authority for that small saving. |
| Poor runtime/boot diagnostics | Original18 and Reader93 unchanged. Bounded observed case intervals, command phases and reader timing are diagnostic only. Fresh per-probe firmware variables, staged inputs and actual marker/stop/reap observations preserve 30-second QEMU budgets with no retries. Selected commands have 600-second work budgets and late zero exits deny. |
| Host CI critical path and stale cache accumulation | Move eleven existing editor stages to an independent fifth mandatory lane. Preserve 56 full/40 extended names, commands, overrides and relative order. Outer fallback restores compiler candidates only; exact inner keys still gate reuse. Read-only inspection and exact completed-entry retirement reject active, abandoned, changed or unsafe entries. |

Private Cargo-home validation caught an existing producer-order mismatch between
Path-component sorting and the key's full-string canonical order. The one-line
`key=str` correction preserves every candidate, absence row and ownership/mode
check. Actual gate validation also caught mixed library kinds and an unconditional
`--doc` invocation on binary-only Domain Manager. Each correction has gate-first
RED controls, an independently reviewed patch and fresh successful actual gates.

## Actual local measurements

The paired macOS runs used identical admitted source, compiler phases, feature
sets, environment keys and two-job limits. Every warm phase hit its exact cold
key, but actual commands and complete assertions ran again with new nonces.
Cargo sums include build and strict Clippy commands; they exclude runtime tests,
reader work and source/output checks. Host load was not controlled.

| Gate | Cold Cargo seconds | Warm Cargo seconds | Fresh assertions |
| --- | ---: | ---: | --- |
| Reliability | 100.105 | 1.217 | Desktop7, Store5, app-private1 |
| Original editor task | 88.605 | 1.451 | Original18 and complete Reader93 |

Task runtime remained 83.895/88.444 seconds; reader work 6.710/9.281 seconds.
Both diagnostic streams observed all eighteen cases with no missing/unknown case
or parsing error. In the cold run, capacity/counter-limit coverage was the largest
case (~24s), followed by recovery/original-outcome scenarios. The intervals measure
observed output prefix to completion, not CPU usage. The earlier ~325s Task run
occurred under different host load/source and is not a controlled speedup baseline.
No hosted or end-to-end speedup is claimed.

[Structured paired measurements](../../out/review-streamlining-cache-measurements.json)
and [Native Runner decision](../../out/review-runner-sharing/decision.json)
retain actual phase durations, admitted identities and exits/reaps.

## Fresh validation and retained artifacts

- macOS/Linux reliability13 passes on the same 227-path source snapshot, with strict
  Clippy for all three targets and actual compiler cache admission.
- Mac cold/warm original18 and Reader93 pass on the same 223-path source snapshot.
- All affected volatile13, Store7, Preview codec7, Save codec8, Read9, Preview5
  and adapter4 gates pass freshly with their feature/strict checks. Launcher17 and
  Save schema9 also pass freshly with their feature/default-off/strict controls.
- Actual Native Runner3, Semantic25 selected cases and Projection24 selected
  cases pass with their existing extra schema/export/doctest assertions. Standalone
  scopes remain complete.
- CI tooling 130 Python/seven Rust controls pass. Final source admission, Python
  compilation, Rust formatting, YAML syntax and diff whitespace checks pass.
- Native Linux helper controls 66 pass on the final helper bytes. Lifecycle
  disappearance controls use the retained actual child-subreaper supervisor because
  the temporary container's PID1 sleep does not reap orphan zombies. The supervisor
  performs actual consuming waits; production cleanup assertions were not weakened.
  Other Linux dev/dependency/CI/source controls pass separately. Initial bind-mount
  owner instability and insecure image Cargo-home denials are retained as environment
  failures; final acceptance uses a native owned source copy and private Cargo home.
- S11/S12/S13 pass. Net/GOP/block each launched once, completed in 2.430/2.227/2.447s
  and recorded actual group stop and parent reap within the original 30s budgets.
  HIL validation remains docs/QEMU mode; no device/metal actuation claim follows.

Local results and their immutable byte digests:

| Result | Path | SHA256 | Tested source SHA256 |
| --- | --- | --- | --- |
| reliability cold | `out/foundry/editor-reliability/run.tnvtvjbz/result.json` | `e8b27dac73ad7890de39f847bfaf707d8fa11699fcb09d1f4b60bd73ee5c71d0` | `d5fd2ef122a2a4057b1f2cc63148170254771e74385cb46f02bebdd6e550bbad` |
| reliability warm | `out/foundry/editor-reliability/run.db9cp8iz/result.json` | `9c8b6599170db49195c31c218e73d64bedd3dea1f0eff7047614e800e1b8314a` | `d5fd2ef122a2a4057b1f2cc63148170254771e74385cb46f02bebdd6e550bbad` |
| task cold | `out/desktop/native-task-source.Q512tZ7G/run/process/result.json` | `20c33368cadb1193706f807c7e02e025fb1391cba2802f85751af1631d4db2b1` | `e1554a62bf06fdc43ec24804206498ec11aaddc5402cbdce571db76e7f9a4e2f` |
| task warm | `out/desktop/native-task-source.4u1eUfzF/run/process/result.json` | `632d1e7e5aeaf9fceb3dbdfbf32384c2fce2c92b6f9e0c185ed4e9878e4e975c` | `e1554a62bf06fdc43ec24804206498ec11aaddc5402cbdce571db76e7f9a4e2f` |
| Linux reliability | `out/review-linux-optimization/reliability/run.rc10q84o/result.json` | `8eba374bad5aa26f19b03feb81d9c0dc094acaa721e7ad1fa194cdce3418b743` | `d5fd2ef122a2a4057b1f2cc63148170254771e74385cb46f02bebdd6e550bbad` |
| NativeRead | `out/desktop/native-read-c9b82a208cf49300cffd7750cfedae30/evidence/result.json` | `e561c6afe01f8494dfa155ab1fd330b97cd5568efe3bb206a4f79f0946574761` | `773a5f0b8e7a333297efa330ccc464e70f1b32b385c0adc9778f892a7e746fcf` |
| NativePreview | `out/desktop/native-preview-read-fb623593aeeca6b005bcb9a3be6abf3e/evidence/result.json` | `655bb14714b098523ab992fdfeee75ef93640dc6cba8408138f66c5d8b75c13a` | `7332082058521d3f5cd5809b839f7d7b5d4746cf2761706eff8bf00fa6e9dd62` |

The exact registry successors are Task220→223 and reliability224→227 (add
`acceptance_build.py`, `build_cache.py`, `test_timing.py`), Read354→356 and
Preview356→358 (add the adapter/timing helper; build cache already present).
No old path or original assertion pin was removed. Collector enumeration logic
is unchanged apart from these three explicit extra inputs. Membership does not
substitute for actual execution or fix future omission.

Preserved failures and current runs are under `out/review-store-evidence`,
`out/review-dev-optimization`, `out/review-cache-config-order`,
`out/review-linux-optimization`, `out/review-selected-gates*.log`,
`out/review-cache-acceptance*.log`, `out/review-streamlining-*` and the independent
review handoff. Earlier successful source snapshots remain predecessors after
helper/source changes; they are not relabelled current acceptance.

## Controlled retirement and remaining qualification

After the final pair succeeded, the operator explicitly retired exactly five
superseded completed entries created by this task. The entries represented
3,016,239,361 logical file bytes; that count is not a physical-space measurement
because hard links and filesystem sharing may affect allocation. Exact context
checks and locks preceded deletion. Evidence file inventories/sizes and original
result/test/verifier binary hashes remained intact afterwards. Historical and
unrelated outputs were not selected. The temporary Linux container and its
compilation/download preparation were removed after copying complete accepted
regression evidence. [Retirement receipt](../../out/review-streamlining-retirement.json)
retains exact keys, contexts and preservation checks.

Complete Linux56 remains INCOMPLETE in this environment: the local prereq check
reports Linux-required and the already-installed pinned proof image is absent.
The controlled Linux regression container does not provide full proof-suite
Docker/builtin-seccomp qualification. The five-lane hosted workflow also needs
fresh qualification. These checks remain in NEXT_TASKS; no merge/release authority,
actual editor child, target persistence or physical graduation is inferred.

## Qualification successor — 2026-10-13

The missing pinned image is now installed. The complete strict Linux56 preflight
passes on the exact assembled runtime candidate, including original18/Reader93,
all consumers, regression13 and actual Linux evaluator/reconciliation controls.
[The qualification](linux56-qualification-2026-10-13.md) records source and evidence.
Earlier INCOMPLETE reports and failed setup attempts remain historical evidence;
fresh five-lane hosted qualification and process/target/physical work remain
separate. This documentation-only update does not relabel old tested snapshots.
