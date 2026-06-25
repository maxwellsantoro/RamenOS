---
name: Bug report
about: A gate, build, or runtime path is not behaving as documented
title: "[bug] "
labels: bug
---

Thank you for taking the time to file a reproducible report. RamenOS favors
evidence-bearing reports over summaries — paste the actual output.

**Which gate or command?**
<!-- e.g. `just foundry-s0`, `just preflight`, `cargo run -p store_cli ...` -->

**What did you expect?**

**What happened instead?**
<!-- Paste the relevant transcript, including the boot/gate output, not just the final line. -->

**Environment**
- OS / host:
- Rust toolchain (`rustc --version --verbose`):
- QEMU version (for target gates):
- OVMF / AVMF firmware source (for S0 / S2):

**Evidence level involved** (see [EVIDENCE_LEVELS.md](../../EVIDENCE_LEVELS.md))
<!-- e.g. PASS/QEMU, PASS/HIL-LOG, replay-only. -->

**Reproducer**
<!-- Minimal steps from a clean clone. -->

**Claim boundary**
<!-- Does this make a gate claim something it cannot back (e.g. metal from QEMU)? If so, name it. -->
