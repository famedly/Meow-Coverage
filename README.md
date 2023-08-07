# Meow! Coverage

A GitHub-integrated code coverage visualiser, written in Rust.

## Getting Started

Here are two sample GitHub action workflows, one for handling PRs, and one for handling new commits in main.

### Pull Request Sample

```yaml
on: [pull_request]

jobs:
  pull_coverage:
    name: Generate Coverage
    permissions:
      contents: read
      issues: write
      pull-requests: write
    runs-on: ubuntu-latest
    steps:
      - name: Checkout base repository
        uses: actions/checkout@v3
        with:
          ref: ${{ github.event.pull_request.base.ref }}
          repository: ${{ github.event.pull_request.base.full_name }}
          path: base

      - name: Checkout current repository
        uses: actions/checkout@v3
        with:
          path: head

      - name: Install toolchain
        run: curl https://sh.rustup.rs -sSf | sh -s -- --profile minimal --default-toolchain nightly --component llvm-tools-preview -y

      - name: Install cargo-llvm-cov
        run: cargo install cargo-llvm-cov

      - name: Run cargo-llvm-cov on base
        run: cargo llvm-cov --lcov --output-path $GITHUB_WORKSPACE/old-lcov.info
        working-directory: base

      - name: Run cargo-llvm-cov on current
        run: cargo llvm-cov --lcov --output-path $GITHUB_WORKSPACE/new-lcov.info
        working-directory: head

      - name: Meow Coverage
        id: coverage-report
        uses: famedly/meow-coverage@main
        with:
          new-lcov-file: 'new-lcov.info'
          old-lcov-file: 'old-lcov.info'
          source-prefix: 'src/'
          pr-number: ${{ github.event.pull_request.number }}
          repo-name: ${{ github.repository }}
          commit-id: ${{ github.event.pull_request.head.sha }}
          github-token: ${{ secrets.GITHUB_TOKEN }}
```

### Change to `main` Branch Sample

```yaml
on:
  push:
    branches:
      - main

jobs:
  main_coverage:
    name: Generate Main Coverage
    permissions:
      contents: write
    runs-on: ubuntu-latest
    steps:
      - name: Checkout repository
        uses: actions/checkout@v3

      - name: Install toolchain
        run: curl https://sh.rustup.rs -sSf | sh -s -- --profile minimal --default-toolchain nightly --component llvm-tools-preview -y

      - name: Install cargo-llvm-cov
        run: cargo install cargo-llvm-cov

      - name: Run cargo-llvm-cov on main branch
        run: cargo llvm-cov --lcov --output-path $GITHUB_WORKSPACE/lcov.info

      - name: Meow Coverage
        id: coverage-report
        uses: famedly/meow-coverage@main
        with:
          new-lcov-file: 'lcov.info'
          source-prefix: 'src/'
          repo-name: ${{ github.repository }}
          commit-id: ${{ github.event.after }}
          github-token: ${{ secrets.GITHUB_TOKEN }}
```

## Lints

We have plenty of lints in `lints.toml` that we use. Cargo currently does not natively support an extra file for lints, so we use `cargo-lints`. To check everything with our lints, run this locally:

```sh
cargo lints clippy --workspace --all-targets
```

and this in your IDE:

```sh
cargo lints clippy --workspace --all-targets --message-format=json
```

A few lints are commented out in `lints.toml`. This is because they should not be enabled by default, because e.g. they have false positives. However, they can be very useful sometimes.

## Pre-commit usage

1. If not installed, install with your package manager, or `pip install --user pre-commit`
2. Run `pre-commit autoupdate` to update the pre-commit config to use the newest template
3. Run `pre-commit install` to install the pre-commit hooks to your local environment

---

# Famedly

**This project is part of the source code of Famedly.**

We think that software for healthcare should be open source, so we publish most
parts of our source code at [gitlab.com/famedly](https://gitlab.com/famedly/company).

Please read [CONTRIBUTING.md](CONTRIBUTING.md) for details on our code of
conduct, and the process for submitting pull requests to us.

For licensing information of this project, have a look at the [LICENSE](LICENSE.md)
file within the repository.

If you compile the open source software that we make available to develop your
own mobile, desktop or embeddable application, and cause that application to
connect to our servers for any purposes, you have to aggree to our Terms of
Service. In short, if you choose to connect to our servers, certain restrictions
apply as follows:

- You agree not to change the way the open source software connects and
  interacts with our servers
- You agree not to weaken any of the security features of the open source software
- You agree not to use the open source software to gather data
- You agree not to use our servers to store data for purposes other than
  the intended and original functionality of the Software
- You acknowledge that you are solely responsible for any and all updates to
  your software

No license is granted to the Famedly trademark and its associated logos, all of
which will continue to be owned exclusively by Famedly GmbH. Any use of the
Famedly trademark and/or its associated logos is expressly prohibited without
the express prior written consent of Famedly GmbH.

For more
information take a look at [Famedly.com](https://famedly.com) or contact
us by [info@famedly.com](mailto:info@famedly.com?subject=[GitLab]%20More%20Information%20)
