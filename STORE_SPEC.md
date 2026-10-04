# Store Specification

**Last Updated:** 2026-10-03
**Status:** Product specification; implemented host contracts and target flows distinguished
**Version:** Package Intelligence Store v1

The Store serves the [Vision](VISION.md) of an everyday OS for humans and AI
agents: approachable software discovery and launch, understandable permission
previews, structured evidence for agent decisions, and a gated path from useful
compatibility to native software. These are product responsibilities; the complete
user flow remains work as recorded in [Current Status](CURRENT_STATUS.md).

## 0) Goals
The Store must:
1) Run software now using the best available runner.
2) Generate a dossier that is rich, structured context for humans and agents.
3) Provide a vote/claim queue that prioritizes leverage, not just popularity.
4) Provide a guided “Port It Now” wizard that produces gated, publishable artifacts.

The product goal is offline execution of installed software through verified
launch plans. Current host runners still contact the local Store service for
artifact verification. `store_cli emit-plan` emits canonical
`ExecutionLaunchPlanV0` JSON from a catalog entry; it does not sign the plan or
implement automatic confidence-based selection. Artifact signatures and launch-
plan publication are distinct contracts.

The dossier, selection policy, and wizard below describe target product behavior.
Landed schemas and host tooling are linked from [Current Status](CURRENT_STATUS.md)
and [Development Reference](docs/DEVELOPMENT_REFERENCE.md).

## 1) Native Levels (User-facing truth)
- Compat: runs in Linux domain / container / microVM
- Hybrid: host compositor + portals; runtime is compat
- POSIX Personality: rebuilt to POSIX compat layer
- Native: portal/harness-first with least privilege

## 2) Dossier Schema (v1)
A dossier is a single JSON document stored in Artifact Store, versioned.

### 2.1 Identity
- program_id (stable)
- upstream:
  - name
  - homepage_url
  - repo_url
  - license_ids[]
- aliases[]: { ecosystem, name, id }

### 2.2 Evidence (per ecosystem)
evidence[]:
- ecosystem: "flatpak" | "arch" | "nix" | "other"
- version
- source_urls[]
- build_recipe_ref (hash/ref to stored recipe text)
- patches_ref (optional)
- dependencies[] (best-effort)
- metadata_ref (raw page/manifest snapshot; optional)

### 2.3 Capability Profile
capabilities:
- declared[]  (from manifests like Flatpak)
- observed[]  (from run instrumentation + portal calls)
- minimal_recommended[] (computed + validated)

The [observed-capabilities contract](docs/OBSERVED_CAPS_V0.md) records measured
scenario evidence. General compat-domain instrumentation is a design choice
that still needs its own capture, completeness, and privacy evidence.

Each capability entry:
- name: string (e.g., "portal.filepicker", "net.tcp", "device.audio.capture")
- scope: "none" | "prompt" | "always"
- notes: string
- evidence: "declared"|"observed"|"computed"

### 2.4 Execution Options
execution:
- best_runner: "native"|"posix"|"linux_domain"|"flatpak_capsule"|"wasi"
- confidence: 0..100
- fallback_runners[] ordered
- known_issues[] (short)

### 2.5 Porting State Machine
porting:
- level: "compat"|"hybrid"|"posix"|"native"
- status: "not_started"|"in_progress"|"blocked"|"published"
- blockers[]: { kind: "missing_portal"|"missing_harness"|"license"|"test_gap", detail }
- prerequisites[]: pointers to portal/harness tasks

### 2.6 Scenario Artifacts
scenarios[]:
- scenario_id
- description
- trace_ref (Artifact Store reference)
- expected_capabilities[] (for validation)
- last_passed: timestamp + channel

## 3) Runner Selection Algorithm (v1)
Proposed policy: prefer an available validated native build, then an eligible
POSIX personality, packaged compatibility capsule, or generic Linux domain.
WASI is eligible only when a compatible WASM artifact and runner exist. If no
candidate is available, return an explicit unsupported result. Confidence
thresholds and fallback behavior need a recorded policy decision and tests;
current emission uses the catalog entry's selected runner.

Selection must output a Launch Plan artifact:
launch_plan:
- runner
- artifact refs
- capability policy (initial + escalation strategy)
- scenario to execute (optional)
- UX labels (native level, confidence, warnings)

## 4) Voting + Queue
### 4.1 Vote model
votes:
- count
- tags: "work"|"hobby"|"dependency"|"accessibility" etc.
- optional: pledged_testing (bool), pledged_porting (bool)

### 4.2 Priority score (v1)
The implemented [Queue Item V0](docs/QUEUE_ITEM_V0.md) computes:

`priority = (vote_weight * leverage * reuse) / (effort * risk)`

`vote_weight` is a bounded scoring input, not an unbounded raw vote count.

Where:
- leverage: 1..5 (toolkits/libs high)
- reuse: 1..5 (common dependencies high)
- effort: 1..5 (based on capability gap + build complexity)
- risk: 1..5 (permissions/special devices/licensing)

The Store must show users:
- why score is high/low
- what would reduce effort (e.g., “screen capture portal missing”)

## 5) Port It Now Wizard (v1)
Target wizard states (complete UI/orchestration remains work):
1) Choose target outcome: wrapper | posix | native
2) Run in best runner and capture:
   - observed capabilities
   - portal call log
   - scenario trace
3) Propose minimal policy:
   - start minimal, escalate on failure
   - record which features require which capability
4) Attempt rebuild path:
   - if posix chosen, attempt rebuild via hermetic builder
   - if wrapper chosen, produce launcher + policy bundle
   - if native chosen, create blockers/prereqs if missing portals/harnesses
5) Gate + publish:
   - smoke scenario replay must pass
   - publish to Experimental channel with rollback metadata

## 6) Foundry Gates (Store-side)
Target publication requirements, beyond current local plan emission:
- reproducible build inputs (where applicable)
- scenario trace(s)
- replay results
- minimal policy bundle (if using portals)
- signed Launch Plan

## 7) What the Store must NOT do
- Require always-on network access to run installed software
- Bypass portals with raw host mounts for convenience
- Create native APIs that mirror Linux/Posix baggage
