# Contributing to resync

Thank you for your interest in contributing!

## Commit Messages
This project uses [Conventional Commits](https://www.conventionalcommits.org/) to automate releases and changelogs.
Please format your commits as follows:
- `feat: add ticket lock implementation`
- `fix: prevent deadlock in nested lock release`
- `chore: update dependencies`

## Development
We use `just` for command running. Install it via `cargo install just` or using your package manager.
- `just check` (Format and Clippy)
- `just test` (Run test suite)
