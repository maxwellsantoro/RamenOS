---
name: Hardware evidence report
about: Report HIL appliance behavior, a golden-machine run, or a provenance question
title: "[hil] "
---

<!--
HIL/hardware claims are provenance-sensitive. Read EVIDENCE_LEVELS.md and
docs/HIL_APPLIANCE_EVIDENCE_V0.md before asserting PASS/METAL.
-->

**Claim level observed**
<!-- e.g. PASS/HIL-LOG (development replay), PASS/HIL-APPLIANCE (live appliance), PASS/METAL. -->

**claim_path**
<!-- One of: operator-golden-machine | appliance-mediated | appliance-live | operator-live | development-log-replay -->

**Appliance / target**
- Appliance id (if `RAMEN_HIL_APPLIANCE=1`):
- Target board / device:
- Controller evidence ref / log (with SHA256 if available):

**Environment flags used**
- `RAMEN_HIL_APPLIANCE`=
- `RAMEN_HIL_GRADUATION`=
- `RAMEN_HIL_GOLDEN_MACHINE`=
- `RAMEN_HIL_SERIAL_DEV` / `RAMEN_HIL_SERIAL_LOG`=

**Serial / power transcript**
<!-- Paste the observed output and any power/reset actuation events. -->

**Provenance**
<!-- How was this run produced? Reproducible from the repo, or a one-off hardware run? -->
