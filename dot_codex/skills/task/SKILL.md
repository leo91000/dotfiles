---
name: task
description: Execute one or more items from a project's tasks.yaml and mark completed tasks when work is fully done. Use when the user asks to run a specific task number, run the next task, run a task group, or continue task execution from tasks.yaml.
---

# Task

1. Read `tasks.yaml` from the current working directory.
2. Select tasks using user intent:
- If no selection is provided, pick the first incomplete task.
- If a number is provided, run that specific task (1-indexed).
- If `group N` is provided, run all incomplete tasks in `parallel_group` N.
- If `next` is provided, run the next incomplete task.
3. Execute each selected task exactly as specified by its title and details.
4. For review-oriented tasks, run all required checks mentioned in task text.
5. After successful completion, update `tasks.yaml` and set `completed: true` only for tasks that are fully done.
6. Preserve valid YAML formatting when editing `tasks.yaml`.
7. Report executed tasks, summarize results, and call out blockers or partial completions.
8. Do not mark tasks completed when execution failed or is incomplete.
