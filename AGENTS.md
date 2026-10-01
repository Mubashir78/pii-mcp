# AGENTS.md

Guidance for AI coding agents working in this repository. Prefer this file over
tool-specific instruction files; keep commit and integration rules here so every
agent follows the same contract.

## Commit discipline

Make atomic commits: each commit is one coherent, self-contained change that
builds and passes its checks on its own. A single issue normally produces several
commits — one per logical step — not one squashed commit. Conventional Commits
are enforced by `commitlint` via a Git hook (`<type>(scope): description`).

**Commitlint rules (config-conventional):**
- **Type** (required): lowercase, one of: `feat`, `fix`, `chore`, `ci`, `docs`, `style`, `refactor`, `perf`, `test`
- **Scope** (optional): wrap in parentheses, e.g. `feat(python): add IBAN pack`. Can be any word describing the subsystem.
- **Description** (required): start with lowercase, use imperative mood ("add" not "adds" or "added"), no trailing period, max ~72 chars
- **Body** (optional): separated from description by a blank line, wrapped at 72 chars
- **Breaking changes:** mark with `BREAKING CHANGE: ` in footer if needed

**Examples:**
```
feat(python): add optional Rust scrub backend
fix(typescript): reject empty language pack lists
docs: document native build for Python
chore(ci): run commitlint on PR range
```

Commitlint runs on every commit. If it fails, fix the message and try again (do not use `--no-verify`).

**Merge commits must be conventional too.** commitlint ignores them by default,
so Git's auto-generated `Merge branch ...` subject is *not* rejected — give it a
conventional message anyway (e.g. `chore: merge origin/main into <branch>`)
so history stays uniform.

**Wrap every line of a merge commit body**, same as any other commit. This is
the rule that actually bites, because a PR-merge body is usually pasted prose
rather than a hand-wrapped paragraph. `body-max-line-length` and
`footer-max-line-length` are both **100** under config-conventional, and
commitlint treats the last paragraph of a multi-paragraph body as the footer —
so a two-paragraph summary with long lines fails on `footer-max-line-length`,
not on the body rule you'd expect. Wrap at 72 like everywhere else and neither
can fire.

The Git hook does not run on a merge performed by GitHub, so a bad merge
message lands on `main` unchecked. release-please skips commits it cannot
parse, so it costs a changelog entry rather than a release — still worth
avoiding.

**Versioning is automated.** release-please derives the version from commit
messages on `main` and accumulates them on one Release PR
(`chore(main): release x.y.z`); merging that PR tags and publishes. Never
hand-edit a version or `CHANGELOG.md`. On the 1.x line a breaking change
bumps the major.

For how changes get integrated — commit everything, group into reviewed PRs by
concern, self-review and commit fixes, and **ask before merging** — follow the
`committing-and-pr-workflow` skill under `.agents/skills/`. Never merge a PR
without explicit approval.

## Comments

Don't write comments unless they are really necessary. Code should explain
itself; add a comment only when the code would not make sense without it
(it looks wrong but isn't). API contracts, invariants and the "why" behind a
detector go in docstrings, not `#` / `//` comments above a branch or regex.
Never narrate what the next lines do. Reviewers flag unneeded comments as a
finding. Details and examples: `.cursor/rules/docstrings-over-comments.mdc`.
