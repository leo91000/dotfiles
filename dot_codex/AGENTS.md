# Environment Information

## CLI Tools Available

- `fd` (preferred over `find`)
- `ripgrep` (`rg`) (preferred over `grep`)
- `jq`
- gh (github cli)

## Programming Languages & Runtimes

- Node.js
- Python

## Rules

- NEVER EVER USE THIS MESSAGE IN COMMIT / PULL REQUEST : "🤖 Generated with [Claude Code](https://claude.ai/code)" or any other publicity for the tool used
- NEVER EVER ADD this Co-Authored-By : "Co-Authored-By: Claude <noreply@anthropic.com>"
- When including a jira ticket reference, the commit message should be formatted as follows:
  - "fix(JIRA-1234): Fix this issue"
  - "feat(JIRA-1234): Add this feature"
  - "chore(JIRA-1234): Cleanup this code"
- And the pull request title should be formatted as follows:
  - "[JIRA-1234] Add this feature"
- In pull request, never write a body message, only a title. (with gh cli, use --body "")
- In JS/TS, prefer for-of and for-in over forEach loops

## Commit / PR

- **When adding new lines to commit message, use multiple `-m`s**. Example: `git commit -m "feat: new feature" -m "- description of the feature 1" -m "- description of the feature 2"`.
