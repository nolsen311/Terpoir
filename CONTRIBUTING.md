# Contributing to Terpoir

Thanks for your interest. Terpoir is privacy-first by design, and
contributions are held to that bar.

## Before you start

Read [docs/design-decisions.md](docs/design-decisions.md). It records the
architecture, the threat model, and the decisions already made. Proposals that
change a recorded decision should open an issue first.

## Ground rules

- **No third-party SDKs** for analytics, advertising, crash reporting, or
  attribution. New dependencies need a stated reason and are reviewed for
  network access, telemetry, and license compatibility.
- **No journal content in logs**, ever, in any build type.
- **Every privacy toggle ships as a pair:** an on-action and an off-action
  that cleans up whatever the on-action created.
- **Defaults are the most private option.** Anything that leaks information
  must be opt-in and disclosed in plain language.
- **Crypto, data model, sync, and merge logic live in the Rust core**, not in
  platform code.

## Developer Certificate of Origin

All commits must be signed off, certifying the
[Developer Certificate of Origin 1.1](https://developercertificate.org/):
that you wrote the change or otherwise have the right to submit it under the
project's license, including the additional permission in
[LICENSING.md](LICENSING.md).

Sign off by committing with `-s`:

```
git commit -s -m "Describe the change"
```

This adds a line such as:

```
Signed-off-by: Your Name <you@example.com>
```

You may use a pseudonym and a no-reply address. A GitHub
`users.noreply.github.com` address works. The email in your sign-off must
match the commit's author email.

The `DCO` check enforces this on every pull request. GitHub bots such as
Dependabot are exempt. If the check fails, add sign-offs with
`git rebase --signoff <base>` and force-push your branch.

## Security issues

Do not open public issues for vulnerabilities. See [SECURITY.md](SECURITY.md).
