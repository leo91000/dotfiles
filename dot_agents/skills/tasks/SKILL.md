---
name: tasks
description: Summarize task status from a project's tasks.yaml file. Use when the user asks to list tasks, show progress, filter pending/completed tasks, inspect a specific group, or identify the next task.
---

# Tasks

1. Read `tasks.yaml` from the current working directory.
2. Compute overall progress as completed/total and percentage.
3. Group tasks by `parallel_group`.
4. Display tasks with clear status markers:
- `✅` for completed
- `⏳` for pending
5. For each group, include done/total counts.
6. Support user filters:
- `pending`: show only incomplete tasks.
- `completed`: show only completed tasks.
- `group N`: show only tasks in group N.
- `next`: show only the next task to execute.
7. Keep output concise and scannable.
