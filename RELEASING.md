# Release process

## Development and CI

Rust is pinned in `rust-toolchain.toml`. Commit Cargo.lock. Main pushes, PRs and
manual dispatch build all four platform archives. Native jobs run formatting,
Clippy, unit tests and `--dump-demo`; Linux builds in AlmaLinux 8 (glibc 2.28)
and runs the JSONL GUI smoke test under Xvfb on the Ubuntu runner. This is not
the same as testing Debian 10, real IME composition or physical Hyper keys.

No signing certificates or personal access tokens are required for draft
releases. The tag-only release job receives the repository GITHUB_TOKEN with
contents-write; build jobs have read-only permissions. Third-party Actions are
SHA-pinned. The AlmaLinux 8 image/toolchain downloads still need network access.

## Cut a version

1. Select project/dependency licensing before any public binary distribution
   (see THIRD_PARTY.md). Decide whether unsigned packages are acceptable.
2. Change Cargo.toml's version, update Cargo.lock with `cargo check`, and edit
   RELEASE_NOTES.md. Run tests, review the diff and commit with the intended
   Git identity. Ship through the repository's saved code-ship policy.
3. Wait for the main build to pass. Tag that exact commit, for example
   `git tag -a v0.1.0 -m 'Hyper Palette 0.1.0'` and `git push origin v0.1.0`.
   Never move or force-push a published tag.
4. All four builds must pass. Packaging checks that the tag exactly matches the
   Cargo version. The final job verifies the complete asset set, writes
   SHA256SUMS, and creates a **draft** GitHub Release. No automatic public publish.
5. Review assets, hashes, notes, license notices and native desktop acceptance.
   Publish the draft manually only after that review; mark prototypes prerelease.

## Failure and rollback

Before the draft exists, rerun failed jobs on the same immutable tag. An existing
release is never silently replaced: inspect it and decide how to handle the
conflict. After publication, fix forward with a new patch/prerelease version.
Consumers should pin a version and checksum, not download `latest` at startup.
Roll back consumers to the previous verified archive; no configuration migration
or startup service is performed by these packages.

## Build/package locally

```sh
cargo test --locked
cargo build --release --locked --target aarch64-apple-darwin
python3 scripts/package.py --target aarch64-apple-darwin \
  --binary target/aarch64-apple-darwin/release/hyper-palette
```

Use a fresh output directory (`--output /path/to/new-directory`) for repeated
packaging; asset files are never overwritten. Linux compatibility builds use
`docker run --rm -v "$PWD:/work" -w /work almalinux:8 sh scripts/build-linux.sh`.
The Linux build script installs packages **inside that disposable container**;
do not run it directly on your host.
