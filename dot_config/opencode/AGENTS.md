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

- NEVER EVER USE THIS MESSAGE IN COMMIT / PULL REQUEST : "🤖 Generated with [opencode](https://opencode.ai)"
- NEVER EVER ADD this Co-Authored-By : "Co-Authored-By: opencode <noreply@opencode.ai>"
- When including a jira ticket reference, the commit message should be formatted as follows:
  - "fix(JIRA-1234): Fix this issue"
  - "feat(JIRA-1234): Add this feature"
  - "chore(JIRA-1234): Cleanup this code"
- And the pull request title should be formatted as follows:
  - "[JIRA-1234] Add this feature"
- In pull request, never write a body message only a title
- In JS/TS, prefer for-of and for-in over forEach loops
