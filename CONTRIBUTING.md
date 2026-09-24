# Contributing to DiceRPC

Thank you for helping improve DiceRPC. Documentation fixes, tests, bug reports, and small focused changes are welcome alongside larger features.

## Before you begin

- Search the existing [issues](https://github.com/dicethedev/DiceRPC/issues) and pull requests to avoid duplicate work.
- Open an issue before starting a breaking change, security-sensitive change, or large feature.
- Report vulnerabilities privately by following [SECURITY.md](SECURITY.md). Do not open a public issue for an undisclosed vulnerability.
- Keep each pull request focused on one problem.

Good first contributions include documentation corrections, regression tests, clearer error messages, and small protocol-compliance improvements.

## Development setup

DiceRPC uses Rust 2024 edition and requires Rust 1.85 or newer.

```bash
git clone https://github.com/dicethedev/DiceRPC.git
cd DiceRPC
cargo build --all-features
cargo test --all-features
```

Create a branch with a short, descriptive name:

```bash
git switch -c fix/tcp-authentication
```

Common prefixes include `fix/`, `feature/`, `docs/`, `test/`, and `refactor/`.

## Making a change

- Follow standard Rust naming and formatting conventions.
- Prefer small, reviewable commits over unrelated refactors.
- Preserve backward compatibility unless the change has been discussed first.
- Validate all untrusted input and consider resource limits for network-facing code.
- Add a regression test for a bug fix.
- Add unit and integration tests for new behavior.
- Update public documentation and examples when an API or command changes.
- Never commit API keys, credentials, private transactions, or sensitive logs.

## Quality checks

Run these commands before opening a pull request:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features
cargo test --all-features
cargo build --examples --all-features
```

If a check fails for a reason unrelated to your change, mention the exact command and failure in the pull request. Do not hide, disable, or weaken a test merely to make the suite pass.

## Commit messages

Write short, imperative commit messages. Conventional Commit-style prefixes are encouraged:

```text
feat: add request timeout configuration
fix: enforce authentication for framed TCP requests
docs: clarify HTTP quick start
test: cover empty batch requests
```

Use the commit body to explain why a change was needed when that is not obvious from the diff.

## Pull requests

A useful pull request includes:

- a clear description of the problem and solution;
- links to related issues;
- tests for changed behavior;
- documentation for user-visible changes;
- compatibility, performance, and security considerations; and
- the commands used to verify the change.

Before requesting review, confirm that:

- [ ] the change is focused and contains no accidental files;
- [ ] formatting and lint checks pass;
- [ ] relevant tests pass;
- [ ] new behavior is tested;
- [ ] documentation and examples are current; and
- [ ] no secrets or sensitive data are present.

Reviewers may ask for changes. Treat review as collaboration: explain tradeoffs, ask questions when feedback is unclear, and keep follow-up commits scoped to the pull request.

## Bug reports

Please include:

- the DiceRPC version or commit;
- your Rust version and operating system;
- the transport and feature flags involved;
- minimal reproduction steps;
- expected and actual behavior; and
- sanitized logs or error output.

Security vulnerabilities belong in a private report, not a bug issue. See [SECURITY.md](SECURITY.md).

## Feature proposals

Describe the problem before prescribing a solution. Include the intended users, example usage, alternatives considered, compatibility impact, and security implications. A small API sketch is helpful for public-facing changes.

Current areas of interest include:

- JSON-RPC 2.0 compliance and interoperability
- consistent authentication and authorization
- request, frame, batch, connection, and timeout limits
- persistent state adapters
- WebSocket transport
- fuzzing, benchmarks, and load tests
- deployment and API documentation

## Licensing contributions

DiceRPC is licensed under `MIT OR Apache-2.0`.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in DiceRPC is licensed under the same terms, with no additional terms or conditions. By submitting a contribution, you confirm that you have the right to do so.

See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE) for the complete terms.
