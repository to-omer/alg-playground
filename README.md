# Algorithm Playground (Rust)

This repository implements and benchmarks algorithms in Rust and publishes
Criterion reports to GitHub Pages.

## Structure

- `crates/<algo>`: one crate per algorithm
- `crates/bench`: Criterion benchmarks that compare implementations

## Quick start

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo bench -p bench
```

## Add a new algorithm

1. Create a crate under `crates/<algo>`.
2. Implement the algorithm and tests in the same file.
3. Document the algorithm and references in `crates/<algo>/README.md`.
4. Add benchmarks under `crates/bench/benches` if needed.

## Benchmark reports

The benchmark workflow publishes `target/criterion/report` to GitHub Pages.
After enabling Pages (Source: GitHub Actions), the report should be available at:

```
https://to-omer.github.io/alg-playground/
```
