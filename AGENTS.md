# Agent guidelines

## Formatting

This repository enforces formatting in CI via `cargo fmt --all -- --check`
(the `fmt` job in `.github/workflows/release.yml`). A pull request will fail
if any code is not formatted.

After making any Rust code changes, always run:

```sh
cargo fmt --all
```

before committing, so that the formatting check passes.
