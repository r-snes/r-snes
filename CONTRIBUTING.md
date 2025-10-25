# Contributing to the R-SNES main repo

This document describes our contribution policy, guidelines to follow when making any change or addition to this repository.

## Current Contribution Status

At this stage of the project, **external contributions are temporarily restricted**.

The R-SNES emulator is currently being developed as part of a **university project**, and the core systems (CPU, APU, PPU, etc.) are undergoing active, fundamental development.  
To maintain internal consistency and meet academic requirements, **we are not yet able to accept external pull requests or direct code contributions** to the main repository.

Once the project reaches a stable and useable state:
- A full **list of accepted contribution types** (code, documentation, plug-ins, tools, etc.) will be published.

Until then, we welcome:
- **Feedback, suggestions, and discussions** through GitHub Issues and Discussions.
- **External experiments** or **plug-ins** built on top of released code, as long as they are developed independently.

> When contributions open officially, this document will be updated, and an announcement will be made in the repository.

---


## Regarding pull requests

Pull requests targetting the main branch will systematically be squashed and merged when validated. This will hopefully keep the commit history of the main branch simple and clear, while leaving the possibility to check a more detailed commit history by going back to the commit history of the original branch from the PR.

## Component isolation

The code of each emulated component (CPU, audio processor, special chips) will be kept in (at least) one separate crate. This makes it so that each component can be developed individually, without potentially breaking others.  
It also makes it easier to write tests for each component, which is a must before any change or addition to a component.

## Testing policy

When adding any feature (including starting a new component from scratch), you always need to provide unit tests for it.  
Since features should at first be developed in isolation, these tests will be the primary way for the PR reviewers to assert that the added feature indeed works as intended.

When providing fixes for bugs, please add tests which demonstrate what was going wrong and which should now pass thanks to your changes.

When changing the behaviour of some feature, you must also update the tests so that they are still passing, and potentially add tests to make sure you did things right.

When simply refactoring existing code, just make sure existing tests are still passing.

The overall code coverage has to be at minimum 80% per crate for a PR to be allowed to go to the main branch. Any failing test will block merging.

## Code quality

All code must be formatted by using `cargo fmt`, which automatically applies the formatting options configured for the project.

Make sure your code has 0 warnings. If you think some warn is expected or normal in certain situation, make use of `#[expect(...)]` or `#[allow(...)]`. Such uses should stay very limited, as warnings are almost always worth actually fixing, so please also add a reason indicating why this particular warning can be ignored in this case.  
For example, the struct for CPU registers has all of its members named in uppercase to fit the naming scheme of documentation of the CPU, so we allow (just for the struct) non-snake_case names:
```rs
#[allow(non_snake_case, reason = "We are naming registers in all caps")]
```

Also, document all your code using Rust doc comments (`///`). This will be enforced by the `missing_docs` lint rule which checks that all public members are documented.
