# Environment Information

## CLI Tools Available

- `fd` (preferred over `find`)
- `ripgrep` (`rg`) (preferred over `grep`)
- `jq`
- gh (github cli)

## Programming Languages & Runtimes

- Node.js
- Python

## Scripting

For new standalone scripts, use the first suitable option in this order while preserving existing project conventions and tooling:

1. Cargo nightly scripts (`cargo +nightly -Zscript`)
2. JavaScript
3. Python
4. Bash

## Rules

- NEVER EVER USE THIS MESSAGE IN COMMIT / PULL REQUEST : "🤖 Generated with [Claude Code](https://claude.ai/code)" or any other publicity for the tool used
- NEVER EVER ADD this Co-Authored-By : "Co-Authored-By: Claude <noreply@anthropic.com>"
