# Security Policy

Thank you for helping keep DiceRPC and its users safe.

DiceRPC is under active development and has not received an independent security audit. Version `0.1.x` should be treated as experimental software rather than a hardened public RPC gateway.

## Supported versions

Security fixes are currently made on a best-effort basis against the `main` branch and the latest `0.1.x` release, when applicable.

| Version | Security support |
| --- | --- |
| `main` | Best effort |
| Latest `0.1.x` | Best effort |
| Older commits or releases | Not supported |

There is currently no guaranteed response or maintenance service-level agreement. Pin a reviewed commit in sensitive environments and evaluate changes before upgrading.

## Reporting a vulnerability

Do not open a public issue, discussion, or pull request for an undisclosed vulnerability.

Submit a private report through [GitHub Security Advisories](https://github.com/dicethedev/DiceRPC/security/advisories/new). Include as much of the following as possible:

- the affected version, commit, transport, and feature flags;
- a description of the vulnerability and its impact;
- reproducible steps or a minimal proof of concept;
- required configuration or preconditions;
- whether exploitation is remote or requires authentication;
- suggested mitigations or fixes, if known; and
- sanitized logs, traces, or packet examples.

Remove real API keys, private transactions, personal data, and third-party secrets from the report.

The maintainers will review the report, ask for more information when needed, and coordinate remediation and disclosure with the reporter. Please allow a reasonable investigation period before publishing details. If the report is not security-sensitive, it may be redirected to the public issue tracker with your agreement.

## Research guidelines

When investigating a potential vulnerability:

- test only systems and data you own or have explicit permission to test;
- avoid privacy violations, data destruction, service disruption, and resource exhaustion;
- use the smallest proof of concept needed to demonstrate impact;
- stop testing and report immediately if you encounter sensitive data; and
- do not demand payment or threaten disclosure.

Good-faith research that follows these guidelines is appreciated. This policy does not authorize testing of third-party deployments and is not a promise of compensation.

## Known security limitations

The following limitations are already known. Reports are still useful when they demonstrate a new bypass, a greater impact, or a practical fix.

- The built-in transports do not provide TLS; example HTTP and TCP traffic is plaintext.
- `ApiKeyInParams` places credentials in the JSON body, where logs and tracing systems may capture them.
- `ApiKeyInHeader` is currently a placeholder and does not validate an HTTP header.
- The framed TCP authentication path currently does not enforce its configured `AuthMiddleware`. Do not rely on the `--auth` option or TCP authentication examples for access control.
- Development keys printed by the CLI and examples are public and are not secrets.
- Production-grade limits for request bodies, frames, batches, connections, concurrency, and execution time are not yet enforced by the framework.
- The included state store is demonstration code: it is in memory and does not provide durable or distributed consistency.
- Metrics endpoints are unauthenticated unless access is restricted outside DiceRPC.

## Deployment guidance

Until the built-in security controls are hardened:

- bind to `127.0.0.1` unless remote access is required;
- place DiceRPC behind a trusted TLS reverse proxy or API gateway;
- enforce authentication and per-method authorization at that gateway;
- generate strong credentials, store them outside source control, rotate them, and redact them from logs;
- validate and bound every method parameter;
- configure request-body, frame, batch, connection, concurrency, and timeout limits at the edge;
- restrict `/metrics` and other operational endpoints;
- run the process with minimal operating-system and network privileges;
- pin and audit dependencies; and
- perform abuse, fuzz, and load testing appropriate to the deployment.

Authentication proves a caller's identity; authorization determines which methods and resources that caller may access. Production systems need both.

## Security-related contributions

After a vulnerability is disclosed or fixed, security hardening pull requests are welcome. Follow [CONTRIBUTING.md](CONTRIBUTING.md), include regression tests, and explain compatibility or performance tradeoffs.
