---
name: shipping-an-issue
description: 'Use when asked to pick up, work on, or ship a GitHub issue end to end without supervision ("ship #123", "/shipping-an-issue 123", "take this issue to a reviewed PR"). Implements in a worktree, commits as it goes, loops independent review and fixes until a fresh reviewer finds nothing blocking, opens the PR, gets CI green, and stops at the merge question.'
---

# Shipping an Issue

Autonomous end to end: implement in a worktree, review, fix, review again,
until clean. Runs unattended until the merge question. Token cost is not a
constraint; review quality is.

This skill is self-contained. Do not load other skills while running it
(worktree, commit, and PR steps are inlined below, and other skills ask for
consent that breaks autonomy).

**Input:** an issue number or URL (`$ARGUMENTS`). None given → ask once, then
go.

**Autonomy contract:** invoking this skill is standing permission to create a
worktree and branch, install dependencies, commit, push the branch, and open
the PR, all without asking. The only allowed stops before the end are listed
in each section (blocked issue, undecidable product decision, review-loop
exhaustion, CI failing the same way three times). **Never merge without an
explicit yes** (section 6).

## 1. Read the issue

- `gh issue view <n> --comments`, plus every issue it links or is blocked by.
  Blocked by an open issue → stop and report.
- Read `AGENTS.md` (commit rules), `README.md`, and `.cursor/rules/`. Read the
  code the issue touches, and trace its callers, before planning.
- The issue leaves a product decision open and the code gives no default →
  stop and ask. Otherwise pick the sensible default and note it for the PR.

## 2. Worktree (no consent ask)

Work in a git worktree so the main checkout, and any other agent using it,
stays untouched.

```bash
git fetch origin
git worktree add .worktrees/<branch> -b <branch> origin/main
```

- `<branch>` is `<type>/<n>-<short-slug>`, e.g. `fix/42-iban-lowercase`.
  `.worktrees/` is already gitignored.
- Already in a linked worktree (`git rev-parse --git-dir` differs from
  `git rev-parse --git-common-dir`) → stay there and create the branch in it.
- Run every later command from the worktree directory. If the shell does not
  keep `cd` between calls, prefix each command with `cd <worktree> &&`.
- Install in the worktree:
  - `pnpm install --frozen-lockfile` (always: the `.githooks/commit-msg` hook
    runs commitlint from `node_modules` in the worktree root and falls back to
    a weaker check without it).
  - Python: `uv venv && uv pip install -e ".[dev]"` (or `python -m venv .venv`
    and `pip install -e ".[dev]"`), then use that venv.
  - TypeScript, only if `typescript/` is touched: `(cd typescript && npm ci)`.
- Smoke the baseline for the areas you will touch (section 3 lists the
  commands). A failure on untouched `origin/main` → record it, work around it,
  and mention it in the PR. Do not fix unrelated breakage in this PR.
- Detector work: before changing anything, run
  `python eval/score.py --save-baseline` (the perf gate is silently skipped
  without it), then `python eval/score.py` and record the loss to beat.

## 3. Implement and commit (no commit ask)

- Plan briefly, then build it. Commit as you go; do not wait to be told.
- Commits follow `AGENTS.md`: atomic Conventional Commits,
  `<type>(scope): description`, lowercase imperative description, no trailing
  period, about 72 chars max, body wrapped at 72. One coherent change per
  commit that passes its checks on its own. Never `--no-verify`. Never commit
  to or force-push `main`. End each message with the `Co-Authored-By` trailer
  your harness specifies, if any.
- The commit type drives the release (release-please): `feat` → minor,
  `fix`/`perf` → patch, `docs`/`chore`/`test`/`refactor`/`ci` → none. A
  breaking change bumps the major. Pick the type
  that reflects real user impact. Never edit the version or `CHANGELOG.md`.
- Don't write comments unless they are really necessary. Code should explain
  itself; add a comment only when the code would not make sense without it.
  API contracts go in docstrings, not `#` comments (`.cursor/rules/`).
- In all writing (commit messages, PR body, review triage, the final report):
  no metaphors. Use clear, direct language. Be concise and objective. No
  fluff.
- Tests must assert real behavior or catch a real failure mode. No
  tautologies, no coverage-only fillers, never weaken or skip an existing
  test to get green.
- The detectors exist in three backends: Python (`src/pii_mcp/`), Rust
  (`crates/`), TypeScript (`typescript/`). A behavior change in one must land
  in all of them, unless the issue says otherwise. Detector order and
  language packs (`_detectors_for` in `scrub.py`) must match too.
- New or changed detector or pack → update the README coverage table and
  examples.
- Before leaving this step, every check for every touched area passes:

  | Touched | Run |
  |---|---|
  | anything | `pytest -q` |
  | `src/pii_mcp/detectors.py` or `scrub.py` | `python eval/score.py`: all gates pass and loss is not above the section 2 baseline |
  | `crates/` | `cargo test -p pii-core`; `uv pip install -e ".[dev,native]" && maturin develop --release`; `PII_MCP_BACKEND=native pytest -q`; `python eval/parity.py` |
  | `typescript/` | in `typescript/`: `npm run typecheck && npm run build && npm run build:native && PII_MCP_BACKEND=js npm test && PII_MCP_BACKEND=native npm test` |

  Runtime-visible change (MCP tool output, scrub result) → exercise it once
  for real, e.g. a short `python -c` against the public API, and keep the
  command for the PR's manual verification steps.

## 4. Review loop

Repeat, at most **5 rounds**:

1. **Spawn a fresh reviewer** with the Agent tool (`general-purpose`, new
   context every round, never a fork: an agent that wrote the code can't
   review it cold). Prompt it with:
   - the issue number and text,
   - the worktree path and the command to see the change:
     `git diff origin/main...HEAD`,
   - the **rejected findings ledger** (below), so it doesn't re-raise them
     without a new argument,
   - this brief:
     > Review this branch against the issue and `AGENTS.md`. Read changed
     > files in full, not just hunks, and trace callers. Hunt for: bugs, PII
     > that leaks through unscrubbed (missed formats, boundary cases,
     > separators), false positives on clean text, catastrophic or quadratic
     > regexes, behavior that differs between the Python, Rust, and
     > TypeScript backends, fail-open paths, security holes, comments the
     > code does not need (code should explain itself; a comment is only
     > justified when the code would not make sense without it), metaphors
     > or fluff in docs and messages, tautological or weakened tests,
     > missing tests, commit messages that break the Conventional Commits
     > rules or carry the wrong release type, and parts of the issue left
     > undone, including README coverage table updates. Run the tests
     > yourself if it helps. Do not edit files. Return findings as
     > `severity | file:line | problem | concrete failure scenario`,
     > severity one of `blocker`, `should-fix`, `nit`. No praise.
     > Nothing found → say `NO FINDINGS`.
2. **Triage every finding.** Verify it against the code yourself.
   - Real → fix it, add or adjust a test, commit (`fix(scope): ...`).
   - Wrong → add it to the ledger with a one-line reason. Don't fix to
     appease.
   - `nit` → fix if cheap, else ledger it.
3. Re-run the section 3 checks for touched areas.
4. **Exit** when a round returns no `blocker` or `should-fix`.

**Stop and ask the user instead of looping** when:
- round 5 still has blockers,
- a finding reverses a fix from an earlier round (oscillation),
- a fix needs a product or scope decision the issue doesn't settle.

## 5. Open the PR and get CI green

- `git status` is clean (commit or drop leftovers), then
  `git push -u origin <branch>`.
- If a PR for the branch exists, `gh pr edit` it. Otherwise `gh pr create
  --base main` (not a draft: CI skips drafts). Title is a Conventional Commit
  subject with the real release type. Body:
  - **Summary**: what changed and why. `Closes #<n>`.
  - **Test plan**: *Automated*: the commands from section 3 with pass counts
    and eval loss before/after if run. *Manual verification*: numbered steps a
    reviewer can run to see the change work (skip only for docs or
    refactors with no runtime behavior).
  - **Review rounds**: rounds run, findings fixed, findings rejected and why.
  - Any defaults chosen for questions the issue left open, and any
    pre-existing failures found in section 2.
  - The generated-with footer your harness requires, if any.
- `gh pr checks <pr> --watch`. Red → read the failing log
  (`gh run view <id> --log-failed`), fix, commit, push, watch again. Same
  failure three times → stop and report. A PR touching only `**.md` runs no
  CI; say so instead of waiting.

## 6. Stop at the merge question

Report: PR link, rounds, what changed after review, anything ledgered, and
manual verification still worth doing. Then ask whether to merge.

**Never merge without an explicit yes.** After yes:

```bash
gh pr merge <pr> --merge --delete-branch \
  --subject "<type>(scope): <description> (#<pr>)" \
  --body "<summary wrapped at 72 chars>"
```

- `--merge` only: squash is disabled by branch protection, rebase is not
  wanted.
- The merge commit subject must be conventional and every body line wrapped
  at 72. GitHub's merge skips the local hook, and a bad message makes the
  release workflow skip this release (see `AGENTS.md`).
- Then `git worktree remove .worktrees/<branch>` and fast-forward local
  `main` from the main checkout.
