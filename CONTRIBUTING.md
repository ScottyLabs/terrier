# Contributing

Thank you for your interest in contributing to Terrier!

## Getting Started

1. Fork the repository
1. Clone your fork: `git clone ssh://git@codeberg.org/ScottyLabs/terrier.git`
1. Create a branch: `git checkout -b feature/your-feature-name`
1. Make your changes
1. Write a commit using the [conventional commit format](https://www.conventionalcommits.org/)
1. Push and open a pull request

## Standards

### Artificial Intelligence

We acknowledge that AI can be a useful tool when used responsibly. However, to ensure code quality, we enforce:

- **Strict Quality Control**: You are responsible for every line of code you submit. AI-generated code must be thoroughly reviewed, understood, and tested.
- **Maintainability**: Any maintainability issues (e.g. bad code style, poor documentation) must be fixed immediately.

### Quality Standards

- **Atomic Commits**: A PR should do one thing only.
- **Testing**: Unit and integration tests should be included with every change.
- **Human-Centered**: Ensure changes are user-friendly. Do not expect anything from your users. Seek opinions from others, especially non-technical people.
- **Documentation**: Use RFCs and issues as a way to keep a searchable history of decisions and bugs. For user-facing changes, update the documentation website.

## Types of Contributions

### Bug Fixes & Small Features

For minor changes:

1. Open an issue describing the bug or feature
1. Submit a PR referencing the issue

### Major Features

For significant changes that affect multiple parts of the application:

1. Read the [RFC process](./rfcs/README.md)
1. Draft an RFC document
1. Open a PR with your RFC
1. Participate in discussion
1. Once accepted, implement the feature

### Claiming Features

Before starting work on a feature/bug:

1. Check existing RFCs and issues to see if the topic is already being tracked.
1. If it does not exist, create an RFC or issue (see above).
1. If the item is unclaimed, assign yourself or comment to indicate you are working on it, then follow the PR process.
1. If it is already claimed, coordinate with the current assignee before starting work.
1. If you can no longer work on a feature, please unassign yourself and comment so that others can claim it.

### For Maintainers

When reviewing PRs:

- Be wary of AI-assisted contributions and place emphasis on code quality
- If code seems incorrect, provide constructive feedback, even if it was obviously AI-generated
- Encourage contributors to explain their approach

## Pull Request Process

1. Update documentation if behavior changes
1. Ensure CI passes (tests, lints, formatting)
1. Request review from at least one maintainer
1. Address feedback and iterate
1. Once approved, a maintainer will merge

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
