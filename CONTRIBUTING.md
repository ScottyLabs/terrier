# Contributing

Thank you for your interest in contributing to Terrier!

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone ssh://git@codeberg.org/ScottyLabs/terrier.git`
3. Create a branch: `git checkout -b feature/your-feature-name`
4. Make your changes
5. Write a commit using the [conventional commit format](https://www.conventionalcommits.org/)
6. Push and open a pull request

## Types of Contributions

### Bug Fixes & Small Features

For minor changes:

1. Open an issue describing the bug or feature
2. Submit a PR referencing the issue

### Major Features

For significant changes that affect multiple parts of the application:

1. Read the [RFC process](./rfcs/README.md)
2. Draft an RFC document
3. Open a PR with your RFC
4. Participate in discussion
5. Once accepted, implement the feature

### For Maintainers

When reviewing PRs:

- Be wary of AI-assisted contributions and place emphasis on code quality
- If code seems incorrect, provide constructive feedback, even if it was obviously AI-generated
- Encourage contributors to explain their approach

## Pull Request Process

1. Update documentation if behavior changes
2. Ensure CI passes (tests, lints, formatting)
3. Request review from at least one maintainer
4. Address feedback and iterate
5. Once approved, a maintainer will merge

## Merge Policy

This project maintains a linear history without merge commits. We use the rebase and squash strategies exclusively.

### Submitting Pull Requests

When your PR is ready to merge, maintainers will choose between:

- **Rebase and merge.** Use when your commits are clean and meaningful. Each commit will appear in the main branch history as-is.
- **Squash and merge.** Use when commits should be condensed (e.g., multiple "WIP" or "fix typo" commits). All PR commits combine into a single commit.

### Handling Merge Conflicts

If your PR falls behind the main branch, never merge upstream changes into your branch. Always rebase instead:

```bash
# Fetch the latest changes
git fetch origin main

# Rebase your branch onto main
git rebase origin/main

# Resolve any conflicts, then continue
git rebase --continue

# Force-push to update your PR
git push --force
```

Do not use `git merge` to update your branch. This ensures a clean, linear history when your PR is merged.

## Questions?

- Open an issue for bugs or feature requests
- Join [our Discord](https://go.scottylabs.org/discord) for real-time discussion
- Review [RFCs](./rfcs) for architectural context

## License

By contributing, you agree that your contributions will be licensed under the [AGPL-3.0 License](./LICENSE).
