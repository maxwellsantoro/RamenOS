# Agentic Workflow

**Last Updated:** 2026-10-04
**Status:** Contributor coordination contract; tooling is not an OS security boundary

Use one coordinator and simultaneous workers to deliver small, independently
reviewable packets. Optimize the time to an integrated, tested consumer, not the
number of agents or patches. [AGENTS.md](../AGENTS.md) owns the stable contract;
[Current Status](../CURRENT_STATUS.md) owns landed behavior and
[Next Tasks](../NEXT_TASKS.md) the ready work and dependencies. This workflow does
not copy the queue. Development with agents does not itself demonstrate the
[Agent Task Proof](plans/2026-09-16-agent-task-proof.md) or a speed advantage.

## Select and prepare a batch

The coordinator reads the status/task pair, selects the smallest ready packets
that shorten a product milestone's critical path, and checks the affected source
and maintained contracts. Distinguish code dependencies from evidence or authority
needed for graduation. Host/QEMU preparation can proceed wherever the queue
permits it; missing physical evidence, study controls, or funding still blocks
the corresponding run or claim. Keep unrelated ready work moving.

Before dispatching dependent implementation, settle the shared contract and write
its behavior, denial, and failure assertions. Record the expected initial result:
an unimplemented behavior must fail, not pass an empty scaffold. Allocate protocol
IDs, namespaces, generated outputs, and shared-file writers before fan-out. A
short prerequisite packet can establish that foundation; consumers then implement
against the same contract. Do not parallelize two competing versions of one API.

Bound active work by the available workers, disjoint writable scopes, and review
capacity. For a four-agent team, a useful starting layout is coordinator, two
implementers, and one independent reviewer or short-prerequisite worker. Rotate
roles as work finishes. Expand implementation concurrency only while the coordinator
can keep reviews and integration current. Finish or unblock work before opening
another batch; avoid leaving a queue of completed patches waiting for integration.

## Dispatch packet

Give each worker the following information in its assignment. Ordinary task
assignments can carry it directly; RamenOrg execution must use the existing
[WorkOrderV0](org/WORK_ORDER_V0.md) and [HandoffPacketV0](org/HANDOFF_PACKET_V0.md)
artifacts. This table is coordination guidance, not another packet schema or an
authority grant.

| Field | Required content |
|-------|------------------|
| Outcome | Queue/slice reference, one consumer-visible behavior, explicit exclusions |
| Starting state | Base commit, checkout/worktree path, existing local changes relevant to the task |
| Dependencies | Ready prerequisites, frozen contract revision, assumptions that would require replanning |
| Write scope | Exact files/directories and authorized new paths; name the owner of shared files |
| Context | Status/task references plus only the relevant contract, source, and evidence paths |
| Completion | Prewritten assertions, focused gate commands, affected consumers, allowed evidence level |
| Resources | Gate-output paths, sockets/ports, build directories, lab access and any exclusive reservations |
| Handoff | Expected patch/evidence format, reviewer, integration owner, and stop/replan triggers |

Keep the packet small enough to act on without rereading all documentation. Read
additional references only when they affect the task. An explicit RamenOrg
[ContextGrantV0](org/CONTEXT_GRANT_V0.md) remains binding: request expansion when
needed rather than treating the coordinator's summary as permission to read more.
Direct user authorization and the applicable work order define scope; delegation
cannot expand either.

## Ownership and workspace rules

- **Coordinator:** selects work, freezes dependencies, resolves ownership, assigns
  review, integrates results, and updates the authoritative queue/status/history.
  It can implement a bounded prerequisite while workers run, but does not erase
  independent review of its own changes.
- **Implementer:** changes only its assigned paths, runs focused checks, and
  returns evidence plus remaining limits. Request a scope adjustment from the
  coordinator before editing another worker's files.
- **Reviewer:** reads the contract, patch, and evidence independently. Check real
  behavior, consumer effects, denied operations, failures/recovery, and claim
  scope. Return actionable findings or an explicit no-findings result. This is
  technical review, not A3 approval or merge authority.
- **Integration owner:** normally the coordinator; it alone edits shared
  registries, `justfile`, codegen registration, `Cargo.lock`, root planning files,
  and shared generated outputs during the batch. It can explicitly delegate one
  of these resources to a named worker, with dependent writers paused.

In a shared checkout, use disjoint file scopes and preserve peers' uncommitted
changes. Only the integration owner switches branches, creates commits, or runs
repository-wide mutating tools such as formatting and codegen. Workers may format
their own files; they must inspect tool side effects. Do not use broad reset,
restore, cleanup, or stash operations to discard someone else's changes.

Use separate worktrees when source overlap, incompatible baselines, or test side
effects make a shared checkout unsafe. Worktrees isolate files, not protocol ID
allocation, external services, hardware, or authority. Record each worktree's
base, preserve needed work before cleanup, and integrate through one owner.
Do not create worktrees for read-only review or disjoint edits merely to fill a
process step.

Foundry gates often share fixed `out/` paths, images, sockets, and build artifacts.
Reserve these resources with the coordinator. Use separate output locations only
when the gate supports them; otherwise serialize conflicting gates, including
their evidence capture. Avoid concurrent full preflights and duplicate expensive
builds. Record who ran each gate and the code and inputs it tested; reuse a
worker's result only for that unchanged scope, not as proof of later integration.

## Handoff, review, and integration

Each worker returns a compact handoff with:

1. The behavior delivered, changed paths, and outstanding acceptance items.
2. Base/head commits, or a retained diff and its digest for uncommitted work;
   identify dependent peer changes included in the tested state.
3. Exact commands, material environment/feature settings, exit results, and
   evidence paths. Preserve evidence before fixed output paths are reused.
4. The evidence level, skipped or incomplete checks, remaining risks, and any
   contract or scope change requested. Never include credentials in evidence.

The reviewer evaluates that patch before integration. In a shared checkout, this
means before accepting it into the milestone or creating the integration commit,
even though its files are already visible. Have a worker who did not author the
change review it; rotate roles when the pool is small. Resolve material findings
and rerun checks affected by the fixes.

The integration owner reconciles the contracts and patches, generates outputs
once shared IDLs are settled, and tests affected consumers plus required combined
gates on the integrated state. Reuse a focused result only if its relevant code,
inputs, and environment are unchanged. Record the integrated revision/diff and
results, then update `CURRENT_STATUS.md`, `NEXT_TASKS.md`, `CHANGELOG.md`, and
`DECISIONS.md` as appropriate. Workers propose wording in their handoffs rather
than racing to edit those shared files.

Completion means accepted behavior and a reviewed, integrated result with matching
evidence. A scaffold, worker report, or green local hook is not completion of a
slice. `just preflight` needs Linux and its pinned dependencies; missing
prerequisites remain `INCOMPLETE`. See [Getting Started](GETTING_STARTED.md) and
[Evidence Levels](../EVIDENCE_LEVELS.md) for environment and claim boundaries.

## Stop and replan conditions

Pause the affected packet and notify the coordinator when a frozen contract must
change, scopes overlap, a prerequisite is false, a gate fails outside the packet's
boundary, or a required environment/evidence source is unavailable. Keep
independent work moving. The coordinator narrows or resequences the packet,
coordinates consumers, and assigns the missing evidence; it does not silently
broaden the worker's task or treat a skipped assertion as success. Do not spin on
the same external blocker or launch replacement workers with the same missing
inputs.

Choose routine design defaults within the Constitution and record consequential
choices. User input is needed for missing authority or a product decision that
cannot be inferred from the task. No coordinator or worker can grant itself
spending, HIL actuation, release, public-support, or merge authority. Follow the
separate author/reviewer identities in [the PR workflow](org/RAMEN_IMPLEMENTER_BOT.md).

## Client hooks and skills

[`.claude/settings.json`](../.claude/settings.json) attempts Rust formatting and
package Clippy after Claude `Edit`/`Write` events, and rejects direct edits to
lockfiles, generated Rust, and the Constitution through those events. These
client-specific hooks do not cover arbitrary shell writes or other clients;
their output handling does not guarantee lint failure blocks work. They are not
a sandbox. Use Cargo for lockfiles, `just codegen` for generated content, and the
governed Constitution change process regardless of hook availability.

Repository skills in [`.agents/skills/`](../.agents/skills/) keep task-specific
guidance discoverable: `coordinate-work` routes this workflow, `new-slice`
defines a new bounded slice, `new-idl` handles native contracts, `foundry-gate`
runs scoped evidence checks, and `ramen-conventions` focuses architecture review.
Keep client skill entrypoints linked to these sources so fixes do not drift.
