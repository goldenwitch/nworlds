# Security Policy

nworlds is an early research and development repository. The supported
security target is the current `main` branch.

## Reporting a Vulnerability

Please do not open a public issue for a suspected vulnerability. Use GitHub's
private vulnerability reporting or a private contact to `@goldenwitch` so the
report can be triaged without exposing exploit details.

Include:

- the affected commit, crate, or workflow;
- a concise description of the impact;
- reproduction steps or a minimal proof; and
- any suggested mitigation.

Do not include credentials, tokens, or other secrets in an issue or pull
request. If a secret is exposed, revoke or rotate it immediately and report
the exposure privately.

## Scope

Reports involving dependency vulnerabilities, unsafe data handling, workflow
permissions, secret exposure, or unintended publication are in scope. Design
limitations that are documented as intentional deferred work should be raised
as normal issues unless they create a security vulnerability.
