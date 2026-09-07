# Contributing

## Scope

This project accepts contributions for bootloader, firmware samples, tooling, tests, and documentation.

## Development workflow

1. Build and test locally.
2. Validate on real RP2040 hardware for behavior changes.
3. Open PR with a clear description and test evidence.

## Required checks

Run before opening a PR:

```bash
make clippy
make lint-md
make test
```

For hardware-impacting changes, also run:

```bash
make test-integration
```

(or explain why it could not be run).

## Release Process

- Create and push a tag in `vX.Y.Z` format to trigger the release workflow.
- Version conventions and CI effect per tag: [`docs/reference/versioning.md`](docs/reference/versioning.md).

## Documentation conventions

- Keep `README.md` short and entry-point oriented.
- Put task procedures in `docs/how-to/`.
- Put stable facts in `docs/reference/`.
- Put design rationale in `docs/explanation/`.
- Record architecture-impacting choices in `docs/explanation/adr/`.
- Update docs in the same PR as behavior changes.

## Commit hygiene

- Use focused commits.
- Keep messages explicit about behavior changes.
- Avoid mixing refactors and functional changes in one commit when possible.
