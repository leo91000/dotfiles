---
description: List tasks from tasks.yaml with status
---

Show a summary of tasks from the project's `tasks.yaml` file.

## Current tasks.yaml content:

!`cat tasks.yaml 2>/dev/null || echo "No tasks.yaml found in current directory"`

## Instructions:

$ARGUMENTS

## Output Format:

1. Show task progress: X/Y completed (Z%)
2. Group tasks by `parallel_group`
3. For each group, show:
   - Group number and description (infer from task titles)
   - List of tasks with status: ✅ completed, ⏳ pending
   - Count: X/Y done in this group

If an argument is provided:
- `pending` - Only show incomplete tasks
- `completed` - Only show completed tasks
- `group N` - Only show tasks in group N
- `next` - Show just the next task to work on

Keep the output concise and scannable.
