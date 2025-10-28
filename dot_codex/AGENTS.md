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

### Javascript / Typescript

- In JS/TS, prefer for-of and for-in over forEach loops
- Prefer Object.hasOwn() over Object.prototype.hasOwnProperty.call()

## Note on ripgrep (rg) and fd

`rg` and `fd` both respect `.gitignore` files, if you need to search things such as node_modules or env config, don't forget to add the option in the CLI.

## Commit / PR

- **When adding new lines to commit message, use multiple `-m`s**. Example: `git commit -m "feat: new feature" -m "- description of the feature 1" -m "- description of the feature 2"`.

## Refactoring / Clean code

- At the end of each code change, if you think the code is currently very complex and could be simplified by refactoring, inform the user at the end of your message with some quick propositions
