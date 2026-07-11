---
description: Execute task(s) from tasks.yaml and mark completed
---

You are tasked with executing items from the project's `tasks.yaml` file.

## Current tasks.yaml content:

!`cat tasks.yaml 2>/dev/null || echo "No tasks.yaml found in current directory"`

## Instructions:

$ARGUMENTS

## Rules:

1. **Task Selection**:
   - If no argument provided, pick the first incomplete task (where `completed: false`)
   - If a number is provided, execute that specific task (1-indexed)
   - If "group N" is provided, execute all incomplete tasks in parallel_group N
   - If "next" is provided, pick the next incomplete task

2. **Task Execution**:
   - Read the task title carefully and execute what it asks
   - For REVIEW tasks, run all the checks mentioned (leptosfmt, cargo check, cargo clippy, etc.)
   - Create files, write code, run commands as needed
   - Follow the project's coding conventions from CLAUDE.md

3. **Marking Complete**:
   - After successfully completing a task, update `tasks.yaml` to set `completed: true` for that task
   - Use exact YAML formatting when editing the file
   - Only mark a task complete if it was fully successful

4. **Output**:
   - Report which task(s) you executed
   - Summarize what was done
   - Note any issues encountered

## Examples:

- `/task` - Execute the next incomplete task
- `/task 5` - Execute task #5
- `/task group 2` - Execute all incomplete tasks in group 2
- `/task next` - Same as no argument, execute next incomplete task
