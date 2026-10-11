# DCO remediations

Commits on `main` that landed without a `Signed-off-by:` line, and the
later commit in which the maintainer signed off on them. History on `main`
is never rewritten, so a missing sign-off is fixed by a follow-up commit
listed here.

A remediation takes effect when the commit that adds its entry is merged
with the maintainer's `Signed-off-by:` line. That sign-off certifies the
Developer Certificate of Origin 1.1 (see [CONTRIBUTING.md](../CONTRIBUTING.md))
for each commit listed in the entry, as if it had been signed off at merge.

## 2026-10-10

Squash merges whose merge messages omitted the sign-off (flagged by the
`dco-audit` job), plus the initial seed commit, pushed directly to `main`
before the pull-request workflow existed.

| Commit | Subject |
| --- | --- |
| `2b916e7539dfd7dea51b5ea0b84c5716809fb0a0` | Seed repository: license, policies, design doc, Claude Code guide |
| `261c253caba09bbc6c681a56fe717492bee97ec4` | ci: add CodeQL code scanning (#3) |
| `923ed51d36cd222ed5245a2715b50347b934c7da` | ci: add cargo-deny and cargo-vet supply-chain checks (#6) |
| `06b4e05dcea3578d2b7cf716426022ad12e27284` | ci: add Rust CI for the core workspace (#5) |
| `36a40413e1eb4cd3896a6adba5da3b6affeb481f` | ci: add OpenSSF Scorecard workflow (#4) |

## 2026-10-10 (second entry)

The squash merge of the first remediation also omitted the sign-off. The
repository's "require sign-off on web-based commits" setting does not add a
sign-off to squash merges, so the line must still be added by hand.

| Commit | Subject |
| --- | --- |
| `dd590235e170fd456f95ff298aff3c3969e61fd2` | docs: record retroactive DCO sign-off for five commits on main (#7) |
