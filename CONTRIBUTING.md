# Contributing to R-SNES

Thanks for your interest in R-SNES! This document explains what kind of contributions we accept, how changes reach the `main` branch, and the quality bar every change must meet.

## Table of contents

- [Core team](#core-team)
- [What you can contribute](#what-you-can-contribute)
- [Development setup](#development-setup)
- [Project structure](#project-structure)
- [Branches and commits](#branches-and-commits)
- [Workflow and pull requests](#workflow-and-pull-requests)
- [Testing policy](#testing-policy)
- [Code quality](#code-quality)
- [Checklist before opening a PR](#checklist-before-opening-a-pr)

---

## Core team

R-SNES is maintained by its core team:

- [@ClementBaziret](https://github.com/ClementBaziret)
- [@C0Florent](https://github.com/C0Florent)
- [@Tholarr](https://github.com/Tholarr)
- [@tibaudlemaout](https://github.com/tibaudlemaout)

The core team develops the core emulation, and only their approvals count towards the "at least 2 approvals" requirement for merging pull requests (see [Workflow and pull requests](#workflow-and-pull-requests)).

---

## What you can contribute

**R-SNES is open to contributions!** If you're looking for a place to start, check the issues labelled [`good first issue`](https://github.com/r-snes/r-snes/issues?q=is%3Aissue+is%3Aopen+label%3A%22good+first+issue%22): they are scoped to be approachable for newcomers.

We welcome:

- **Features and fixes around the emulator**: user interface, controls, tooling, quality-of-life features, etc.
- **Bug reports**, with as much detail as possible (ROM, steps to reproduce, expected vs. actual behaviour).
- **Documentation** improvements.
- **Tests** that increase coverage or reproduce known bugs.
- **Plugins**:
  - examples showcasing or teaching parts of the plugin API.
  - complex plugins that bring impressive functionality to a game and that you'd like to share.
- **Questions and ideas** in [GitHub Discussions](https://github.com/r-snes/r-snes/discussions).

> [!IMPORTANT]
> **The core emulation** (the internal behaviour of the CPU, APU, PPU and other emulated chips) **is developed by the core team.** We do not accept pull requests implementing major features or reworking large parts of it, but **minor contributions are welcome**, as long as the core team remains the author of the work as a whole.

If you're unsure whether your idea falls within scope, **open an issue before starting to work on it** so we can discuss it first.

---

## Development setup

You will need:

- **Rust 1.95.0** (the version used by CI), with the `rustfmt` and `clippy` components:

  ```sh
  rustup toolchain install 1.95.0 --component rustfmt clippy
  ```

- **SDL2** and **SDL2_image** development libraries (e.g. `libsdl2-dev` and `libsdl2-image-dev` on Debian/Ubuntu).
- **cargo-tarpaulin**, used to run the tests and measure code coverage:

  ```sh
  cargo install cargo-tarpaulin
  ```

Then check that everything builds:

```sh
cargo check --workspace
```

---

## Project structure

Each emulated component (CPU, audio processor, special chips, etc.) lives in **at least one dedicated crate** of the Cargo workspace. This lets each component:

- be developed on its own, without risking breaking the others;
- be tested in isolation (see [Testing policy](#testing-policy)).

---

## Branches and commits

### Branch names

Even when working on your fork, give your branch a meaningful name: it appears on your pull request.

- Start with a **verb** describing what the branch does. For new features, use the `implement` verb written as `impl`
- Separate words with **hyphens** (`-`), in lowercase.
- Keep it **short**: a few words are enough.

```text
impl-ppu-mode5
add-reset-button
fix-controller-input-lag
update-contributing-guide
```

> [!NOTE]
> Branches whose name starts with `ga-ignore-` are skipped by CI. Don't use this prefix for a branch you intend to open a PR from.

### Commit messages

We use [gitmoji](https://gitmoji.dev/) for commit messages. Each gitmoji has a specific meaning, so pick the one that best matches your change.

A commit message is made of:

1. the **gitmoji** matching the change;
2. a **verb** in the imperative form (`Add`, `Fix`, `Use`, `Replace`, `Switch`, ...);
3. a **short summary** of the change.

```text
✨ Add reset button to the emulator window
🐛 Fix crash when loading a ROM without header
♻️ Replace manual parsing with a match statement
✅ Add tests for controller input mapping
📝 Update build instructions in README
```

Keep commit messages **short** (aim for about 50 characters, and no more than 72). If a change needs more explanation, put it in the commit body or the PR description rather than in the title.

Since pull requests are squash-merged, **your PR title is used as the default commit message on `main`**. Maintainers can still edit it before merging, but please write it following the same convention.

---

## Workflow and pull requests

1. Fork the repository and create a branch from `main` for your change, named as described in [Branch names](#branch-names).
2. Push your work and open a pull request targeting `main`. If it addresses an issue, reference it in the description (e.g. `Closes #42`).
3. CI runs automatically on every push and pull request. **All checks must pass** before a PR can be merged.
4. The PR is reviewed. Anyone is welcome to review pull requests and leave comments, but only approvals from [core team](#core-team) members count: a PR needs **2 core team approvals** before it can be merged.
5. Once approved, the PR is **squashed and merged** by a core team member.

Squash-merging keeps the history of `main` short and readable: one commit per PR. The detailed commit history remains available on the PR itself if you need to dig into how a change was built.

---

## Testing policy

Tests are how reviewers verify that a change does what it claims.

| Type of change | What is expected |
| --- | --- |
| **New feature** | Unit tests covering the new behaviour. |
| **Bug fix** | A test that reproduces the bug and now passes thanks to the fix. |
| **Behaviour change** | Update the affected tests, and add new ones where needed to prove the new behaviour is correct. |
| **Refactor** | No new tests required, but all existing tests must still pass. |

Tests are run and coverage is measured with `cargo tarpaulin`, configured by the `tarpaulin.toml` file at the root of the repository:

```sh
cargo tarpaulin
```

When running the tests several times in a row, add `--skip-clean` to reuse existing build artifacts (tarpaulin cleans them by default) and get faster rebuilds:

```sh
cargo tarpaulin --skip-clean
```

- **Any failing test blocks the merge.**
- The workspace must keep **at least 80% code coverage**. Tarpaulin fails below this threshold, which fails CI.

Add `--out html` to generate a `tarpaulin-report.html` file showing which lines are covered. The report for `main` is also published on the project's GitHub Pages.

---

## Code quality

### Formatting

All code must be formatted with `cargo fmt`, which applies the project's formatting configuration:

```sh
cargo fmt --all
```

CI rejects any unformatted code.

### Zero warnings

Code must compile with **no warnings**, including Clippy lints. CI runs Clippy with warnings treated as errors:

```sh
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Warnings are almost always worth fixing. If a warning is genuinely expected in a specific case, silence it with `#[expect(...)]`, as locally as possible, and **always give a reason**.

Do not use `#[allow(...)]`: an `allow` stays silently in place even once the lint no longer triggers, leaving useless noise behind. An `expect` emits a warning as soon as the expected lint stops triggering, so unnecessary exceptions get caught and removed.

For example, CPU register fields are named in uppercase to match the CPU documentation, so the lint is silenced for that struct only:

```rust
#[expect(non_snake_case, reason = "We are naming register in all caps")]
```

Keep these exceptions rare.

### Documentation

Document your code with Rust doc comments (`///`). The `missing_docs` lint is enabled, so every public item must be documented, or CI will fail.

---

## Checklist before opening a PR

Run these commands locally; they mirror what CI checks:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo tarpaulin
```

- [ ] Commit messages and the PR title follow the [gitmoji convention](#commit-messages).
- [ ] Code is formatted and produces no warnings.
- [ ] Public items are documented.
- [ ] Tests are added or updated according to the [testing policy](#testing-policy).
- [ ] All tests pass and workspace coverage is at least 80%.
