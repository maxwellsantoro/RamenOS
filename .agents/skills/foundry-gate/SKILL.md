---
name: foundry-gate
description: Run Foundry gates for a specific slice and report results
disable-model-invocation: true
allowed-tools: Read, Bash, Grep, Glob
---

Run the Foundry gate for slice: $ARGUMENTS

## Available gates

| Argument | Command | What it tests |
|----------|---------|---------------|
| s0 | `just foundry-s0` | QEMU boot x86_64 + aarch64 |
| store-s0 | `just foundry-store-s0` | Store catalog + launch plan |
| s1 | `just foundry-artifact-s1` | Artifact store lifecycle |
| s2 | `just foundry-compat-s2` | Compat domain boot |
| s3-trace | `just foundry-trace-s3` | Trace artifact schema + replay |
| s3-portal | `just foundry-portal-file-ro-s3` | Portal file picker RO + observed caps |
| all | `just foundry-all-s0-s1-s2-s3` | Full umbrella gate (S0–S3) |

## Steps

1. Run the requested gate command
2. Read any log files produced in `out/logs/`
3. Report pass/fail status for each assertion
4. If failed, identify the failing assertion and the relevant source file to investigate
5. If the gate script itself errors, report the exit code and stderr output
