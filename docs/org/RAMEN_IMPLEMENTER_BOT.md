# ramen-implementer (A2 implementer bot)

**Last Updated:** 2026-06-24
**Status:** Active operational identity

`ramen-implementer` is a GitHub App that acts as RamenOrg's **A2 implementer
identity**. It opens pull requests attributed to `ramen-implementer[bot]`
(GitHub reports the author as `app/ramen-implementer`), which is a **distinct
identity from the human maintainer** (`maxwellsantoro`). That separation is what
makes the A3 "require one approving review" branch rule enforceable: the bot
opens, the human approves and merges — never the same identity for both.

This is how PRs get created *as the org, not as a person*.

## Identity

| Field | Value |
|-------|-------|
| App slug | `ramen-implementer` |
| App ID | `4129163` |
| Client ID | `Iv23liPH0guXIg4sMIsp` (OAuth device flows only — not used by the token flow below) |
| Installation ID | `142233521` (on `maxwellsantoro/RamenOS`) |
| PR author shown as | `ramen-implementer[bot]` (API: `app/ramen-implementer`) |
| Authority level | **A2** — branch, code, run gates, open PRs. No self-approve, merge, release, hardware, or public-support authority. |

### Installation permissions (least-privilege, repo-scoped)

- `contents: write` — push branches, commits
- `pull_requests: write` — open / review / merge PRs
- `workflows: write` — modify `.github/workflows/`
- `metadata: read` — baseline (required)

The App is installed on `maxwellsantoro/RamenOS` only. GitHub does not dispatch
to self-hosted runners for fork PRs, so external contributors' PRs never run on
your infrastructure.

## The private key (crown jewel)

- Stored at: `~/.config/ramenos/ramen-implementer.private-key.pem` (outside the
  repo; perms `600`; directory `700`).
- Never committed (`.gitignore` covers `*.pem`/`*.key`).
- It can mint installation tokens indefinitely until **rotated**. Treat it as the
  A2 credential it is. Rotate via GitHub → Settings → Developer settings →
  GitHub Apps → ramen-implementer → Generate a new private key.

## Minting an installation token

`tools/org/mint_app_token.py` is dependency-free (stdlib + openssl). It signs a
JWT with the PEM and exchanges it for a ~1-hour installation access token, which
it prints to stdout.

```sh
export GH_TOKEN=$(python3 tools/org/mint_app_token.py \
  --app-id 4129163 \
  --key ~/.config/ramenos/ramen-implementer.private-key.pem)
```

`gh` prefers `GH_TOKEN` over stored auth, so the session acts as the bot until
you `unset GH_TOKEN`. The token never touches shell history when captured this
way. To just resolve the installation id: add `--print-installation-id`.

### Verifying the bot

GitHub App installation tokens **cannot call `GET /user`** — it returns
`403 "Resource not accessible by integration"`. That is expected, not a failure.
Verify with repo-scoped calls instead:

```sh
gh api installation/repositories --jq '.repositories[].full_name'   # → maxwellsantoro/RamenOS
gh api repos/maxwellsantoro/RamenOS --jq .full_name
```

## Opening a PR as the bot

```sh
# 1. push the branch (pusher identity is irrelevant; only the PR author matters)
git push -u origin <branch>

# 2. open the PR AS THE BOT so its author != the human reviewer
export GH_TOKEN=$(python3 tools/org/mint_app_token.py \
  --app-id 4129163 \
  --key ~/.config/ramenos/ramen-implementer.private-key.pem)
gh pr create --base main --head <branch> \
  --title "..." --body "..."
```

The PR is authored by `ramen-implementer[bot]`. Confirm:

```sh
gh pr view <N> --json author --jq .author.login   # → app/ramen-implementer
```

## Approving + merging (the human, A3)

GitHub forbids a PR author from approving their own PR, so the bot-opened PR
**requires** a different identity to approve — i.e. the human:

```sh
unset GH_TOKEN                       # back to maxwellsantoro (reviewer/A3)
gh pr review <N> --approve --body "..."
gh pr merge  <N> --squash --delete-branch
```

This is the A2→A3 split in action: implementer opens, reviewer approves+merges.

## Separation of duties (non-negotiable)

- The **bot** (A2) writes code, runs gates, and opens PRs.
- The **human** (A3) approves and merges. The branch rule requires ≥1 approval
  from a non-author.
- Never let one identity write, approve, **and** merge the same change.
- Do not grant the bot merge/release/hardware authority. The App's permissions
  stay at contents/pull_requests/workflows write only.

## Stage 2 (future): a reviewer bot

Today the human is the A3 reviewer/merger. When the loop is trusted, add a second
App (e.g. `ramen-reviewer`) as A3 so merges become fully agent-driven while
preserving the author≠approver separation. Do not promote `ramen-implementer`
itself to A3 — keep the two roles on distinct identities.

## See also

- `AUTHORITY_LEVELS.md` — the A0–A6 ladder this bot occupies at A2.
- `MERGE_GATE_V0.md` — the A3 preconditions (separation, evidence, votes).
- `tools/org/mint_app_token.py` — the token minter.
