# Contributing

Thank you for your interest in contributing to Tau and the Tau compiler!  
This guide provides all the information you need to get started.

## Open an Issue

You can open an issue on GitHub [here](https://github.com/tau-lang/tau/issues/new/choose).

Please follow these guidelines:

- **Bug reports:** Use the _bug report_ template for bugs, unexpected behaviour, or build failures.
- **Documentation requests:** Use the _documentation request_ template if you find false, outdated, or missing documentation.
- **Feature requests:** Use the _feature request_ template for proposing new features or enhancements.

Providing clear and detailed information helps us reproduce issues quickly and understand your requests more effectively.

## Open a Pull Request

> [!TIP]
> If you use Nix, you can enter a development shell by running
>
> ```sh
> nix develop
> ```
>
> from the repository root.

To contribute code, please follow these steps:

1. Fork the repository on GitHub.
2. Clone your fork locally.
3. Make your changes.
4. Run the test suite with
   ```sh
   cargo test
   ```
   to ensure all tests pass after your changes.
5. Format your code by running
   ```sh
   cargo fmt
   ```
6. Commit your changes and open a pull request.
   In the pull request:
   - Use the pull request template.
   - Describe the changes you made.
   - Reference any related issues.
   - Complete the checklist.

## Commit Conventions

We use the [conventional commits](https://www.conventionalcommits.org/) conventions. Please write your commit messages **in English only**! Your commit messages should follow this structure:

- feat(scope): add your cool new feature
- fix: fix a bug
- docs: add or improve documentation
- style: reformat code (no logic changes)

Write commit messages in the imperative mood (e.g., “add feature”, not “added feature”).

## Code Conventions

We follow the [rust style guide](https://doc.rust-lang.org/beta/style-guide/).

Please ensure your code adheres to these conventions before submitting a pull request.

---

Thank you for helping improve Tau!
