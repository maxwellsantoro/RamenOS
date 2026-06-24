# RamenOrg Authority Levels

**Last Updated:** 2026-06-23
**Status:** G0 scaffold

Authority levels stage autonomy. Advancement is earned by evidence, gates, and
separated review, not by confidence.

| Level | Name | Allowed by default | Explicitly not allowed |
|-------|------|--------------------|------------------------|
| A0 | Read-only board | Read repo state, produce board packets, identify drift, propose work | Write files, open PRs, actuate hardware |
| A1 | Issue/doc proposal | Draft issues, docs, research questions, and plans | Merge, release, hardware control |
| A2 | Implementation PR | Branch, code, run gates, open PRs | Self-approve, bypass reviews, public release claims |
| A3 | Conditional merge | Merge only after branch protection, checks, reviews, evidence validation, and board vote | Merge when any condition is missing |
| A4 | Release | Tag and publish when release and evidence officers approve matching claims | Claim beyond evidence level |
| A5 | Hardware | Actuate HIL appliance through bounded commands for approved work orders | Direct unbounded shell/hardware access |
| A6 | Community/customer | Triage and draft responses from docs/evidence | Unsupported promises or technical truth claims |

G0.8.1 permits **A2-local** implementation trials: code and gate work inside a
single active work order, without merge, release, self-approval, HIL actuation,
public support, credential, or identity-level role authority. A3+ still requires
separate tools, branch protection, credentials, and explicit decisions.
