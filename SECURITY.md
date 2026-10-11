# Security Policy

## Reporting a vulnerability

Please report vulnerabilities privately through GitHub's
**Report a vulnerability** button on this repository's Security tab
(private vulnerability reporting). Do not open a public issue.

Include what you found, how to reproduce it, and the impact you expect. We
will acknowledge reports and keep you informed as we work on a fix.

## Scope

Of particular interest:

- Anything that exposes journal content, photos, locations, or keys outside
  the encrypted store.
- Weaknesses in the key hierarchy, recovery-key handling, or crypto-erasure.
- Metadata leakage to storage providers, catalog providers, or the OS beyond
  what the app discloses.
- Supply-chain or build-reproducibility problems.

## We will never ask for your recovery key

No one from the Terpoir project will ever ask for your recovery key, master
key, or passphrase, by any channel. Anyone who does is attempting to steal
your data.

## Official builds

Only builds signed with the keys published on https://terpoir.app are
official. See [TRADEMARKS.md](TRADEMARKS.md).
