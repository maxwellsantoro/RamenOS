# Queue Item Artifact V0 Schema

**Last Updated:** 2026-02-18
**Status:** Active

## 0) Purpose
Queue items are content-addressed JSON documents that represent entries in the porting queue.
Each item includes:
- Program identification
- Target graduation level
- Evidence references (traces, observed caps)
- Prerequisites (blockers)
- Scoring inputs and computed priority

---

## 1) Artifact schema (v0)
```json
{
  "schema_version": 1,
  "program_id": "org.example.app",
  "target_level": "posix",
  "evidence": {
    "scenario_traces": ["sha256:..."],
    "observed_caps": "sha256:...",
    "protocol_traces": ["sha256:..."]
  },
  "prereqs": [
    { "kind": "portal", "name": "clipboard", "notes": "optional" }
  ],
  "scoring": {
    "vote_weight": 3,
    "leverage": 4,
    "reuse": 2,
    "effort": 3,
    "risk": 2
  },
  "priority": 4.0,
  "explanation": ["Effort is moderate (3/5)", "Risk is low (2/5)"]
}
```

---

## 2) Fields

### program_id (required)
Unique identifier for the program being queued for porting.

### target_level (required)
Target graduation level. One of:
- `compat`: Linux Domain / Flatpak compatibility
- `posix`: POSIX Personality / rebuild
- `wasi`: WASI sandbox
- `native`: Native portal/harness-first

### evidence (required)
References to evidence artifacts. At least one must be present.
- `scenario_traces`: Content IDs of scenario trace artifacts
- `observed_caps`: Content ID of observed capabilities artifact
- `protocol_traces`: Content IDs of protocol trace artifacts

### prereqs (optional)
Prerequisites blocking this item. Each prereq has:
- `kind`: Type of prereq ("portal", "harness", "service", "driver")
- `name`: Name of the missing component
- `notes`: Optional notes

### scoring (required)
Scoring inputs for priority calculation. All values are 1-5.
- `vote_weight`: How many votes / how important
- `leverage`: How much does porting this enable?
- `reuse`: How reusable is the work?
- `effort`: How hard is it to port? (higher = harder)
- `risk`: How risky is the port? (higher = riskier)

### priority (required)
Computed priority value. Must match the formula:
```
priority = (vote_weight × leverage × reuse) / (effort × risk)
```

### explanation (optional)
Human-readable explanation strings for the priority score.

---

## 3) Validation rules
- `schema_version` must be 1
- `program_id` must be non-empty
- At least one evidence artifact must be present
- All content IDs must be `sha256:` prefixed
- Scoring values must be 1-5
- `priority` must match computed value (tolerance: 0.001)
- Prereq `kind` and `name` must be non-empty

---

## 4) CLI commands
```bash
# Validate queue item
store_cli validate-queue-item --src queue_item.json

# Explain priority score
store_cli explain-priority --src queue_item.json

# Generate prerequisites graph
store_cli prereq-graph --src item1.json --src item2.json --out graph.json --dot graph.dot
```
