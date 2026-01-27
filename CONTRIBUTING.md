# Contributing to EdgeFirst IMU

Thank you for your interest in contributing to EdgeFirst IMU! This document provides guidelines for contributing to this project.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [How to Contribute](#how-to-contribute)
- [Code Style Guidelines](#code-style-guidelines)
- [Testing Requirements](#testing-requirements)
- [CI/CD Workflows](#cicd-workflows)
- [Pull Request Process](#pull-request-process)
- [Developer Certificate of Origin (DCO)](#developer-certificate-of-origin-dco)
- [License](#license)

## Code of Conduct

This project adheres to the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code. Please report unacceptable behavior to support@au-zone.com.

## Getting Started

EdgeFirst IMU is an IMU sensor service for the EdgeFirst Maivin platform. Before contributing:

1. Read the [README.md](README.md) to understand the project
2. Review the [TESTING.md](TESTING.md) for testing procedures
3. Browse the [EdgeFirst Documentation](https://doc.edgefirst.ai/latest/maivin/) for context
4. Check existing [issues](https://github.com/EdgeFirstAI/imu/issues) and [discussions](https://github.com/EdgeFirstAI/imu/discussions)
5. Review the [EdgeFirst Samples](https://github.com/EdgeFirstAI/samples) to see usage examples

## Development Setup

### Prerequisites

- **Rust**: 1.90 or later ([install instructions](https://rustup.rs/))
- **Git**: For version control
- **Hardware** (optional): BNO08x IMU sensor for integration testing

### Clone and Build

```bash
# Clone the repository
git clone https://github.com/EdgeFirstAI/imu.git
cd imu

# Build
cargo build
```

### Rust Development

```bash
cargo fmt         # Format code
cargo clippy      # Run linter
cargo test        # Run tests
cargo doc         # Generate documentation
```

## How to Contribute

### Reporting Bugs

Before creating bug reports, please check existing issues to avoid duplicates.

**Good Bug Reports** include:

- Clear, descriptive title
- Steps to reproduce the behavior
- Expected vs. actual behavior
- Environment details (OS, Rust version)
- Minimal code example demonstrating the issue

### Contributing Code

1. **Fork the repository** and create your branch from `main`
2. **Make your changes** following our code style guidelines
3. **Add tests** for new functionality (minimum 70% coverage)
4. **Ensure all tests pass** (`cargo test`)
5. **Update documentation** for API changes
6. **Run formatters and linters** (`cargo fmt`, `cargo clippy`)
7. **Submit a pull request** with a clear description

## Code Style Guidelines

### Rust Guidelines

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` (enforced in CI)
- Address all `cargo clippy` warnings
- Write doc comments for public APIs
- Maximum line length: 100 characters

## Testing Requirements

All contributions with new functionality must include tests:

- **Unit Tests**: Minimum 70% code coverage
- Critical paths require 100% coverage
- **Integration Tests**: For hardware-dependent functionality (see [TESTING.md](TESTING.md))

### Running Tests

```bash
# Run unit tests
cargo test

# Run tests with coverage (requires cargo-llvm-cov)
cargo llvm-cov nextest --workspace --lcov --output-path coverage.lcov

# Run integration tests (requires hardware)
cargo test --test integration_test -- --include-ignored
```

See [TESTING.md](TESTING.md) for detailed testing procedures including on-target testing.

## CI/CD Workflows

This project uses GitHub Actions for continuous integration and deployment. All workflows
are located in `.github/workflows/`.

### Workflow Overview

| Workflow | Trigger | Purpose |
|----------|---------|---------|
| **test.yml** | Push/PR to `main`, `develop` | Run tests, linting, and coverage |
| **build.yml** | Push/PR to `main`, `develop` | Build release binaries for x86_64 and aarch64 |
| **sbom.yml** | Push/PR to `main` | Generate SBOM and check license compliance |
| **release.yml** | Push `vX.Y.Z` tag | Create GitHub release, publish to crates.io |

### Test Workflow

The test workflow implements a three-phase architecture for on-target testing:

1. **Phase 1**: Build and run unit tests on GitHub-hosted runners (x86_64, aarch64)
2. **Phase 2**: Run hardware integration tests on self-hosted `raivin` runner with BNO08x IMU
3. **Phase 3**: Process coverage data and report to SonarCloud

Hardware tests run automatically on `main` branch or when the `test-hardware` label is applied
to a pull request.

### Build Workflow

Builds release binaries for both architectures:

- **x86_64**: Built on `ubuntu-22.04` runner
- **aarch64**: Built on `ubuntu-22.04-arm` runner

Artifacts are retained for 90 days and used by the release workflow.

### Release Workflow

Triggered by pushing a version tag (e.g., `v3.0.1`):

1. Waits for build and SBOM workflows to complete
2. Downloads artifacts from those workflow runs
3. Creates GitHub Release with binaries, SBOM, and changelog
4. Publishes to crates.io

**Release Process:**

```bash
# 1. Update version in Cargo.toml
# 2. Update CHANGELOG.md with version entry
# 3. Commit and push changes
git add -A && git commit -m "Release v3.0.1"
git push origin main

# 4. Wait for build.yml and sbom.yml to complete
# 5. Create and push tag
git tag v3.0.1
git push origin v3.0.1
```

### SBOM Workflow

Generates a Software Bill of Materials (SBOM) in CycloneDX format and validates:

- License compliance against Au-Zone policy
- NOTICE file accuracy
- SBOM format validity

## Pull Request Process

### Branch Naming

```text
feature/<description>       # New features
bugfix/<description>        # Bug fixes
docs/<description>          # Documentation updates
```

### Commit Messages

Write clear, concise commit messages:

```text
Add [feature] for [purpose]

- Implementation detail 1
- Implementation detail 2
```

**Guidelines:**

- Use imperative mood ("Add feature" not "Added feature")
- First line: 50 characters or less
- Body: Wrap at 72 characters

### Pull Request Checklist

Before submitting, ensure:

- [ ] Code follows style guidelines (`cargo fmt`, `cargo clippy`)
- [ ] All tests pass (`cargo test`)
- [ ] New tests added for new functionality
- [ ] Documentation updated for API changes
- [ ] SPDX headers present in new files
- [ ] CHANGELOG.md updated (for user-facing changes)

## Developer Certificate of Origin (DCO)

All contributors must sign off their commits:

```bash
git commit -s -m "Add new feature"
```

**Configure git:**

```bash
git config user.name "Your Name"
git config user.email "your.email@example.com"
```

## License

By contributing to EdgeFirst IMU, you agree that your contributions will be licensed under the [Apache License 2.0](LICENSE.txt).

All source files must include the SPDX license header:

```rust
// Copyright 2025 Au-Zone Technologies Inc.
// SPDX-License-Identifier: Apache-2.0
```

## Questions?

- **Documentation**: https://doc.edgefirst.ai/latest/maivin/
- **Discussions**: https://github.com/EdgeFirstAI/imu/discussions
- **Issues**: https://github.com/EdgeFirstAI/imu/issues
- **Email**: support@au-zone.com

Thank you for helping make EdgeFirst IMU better!
