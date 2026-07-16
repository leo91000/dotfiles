---
name: weweb-back-migrations
description: Create production-safe Sequelize migrations in WeWeb's `weweb-back` repository. Use when adding a new backend migration file, changing database schema or data through `migrations/`, naming or timestamping a migration, implementing `up`/`down`, or designing migration rollout safety for prod/staging, especially around rolling ECS deploys, old-app writes, `NOT NULL` backfills, unique indexes, constraints, locks, transactions, idempotency, and `SequelizeMeta` behavior.
---

# WeWeb Back Migrations

## Goal

Create `weweb-back` migrations that can survive rolling production deploys.

The migration task can run before all old ECS service tasks are replaced. Old app code may keep writing the old schema while a migration is running or immediately after it commits. Design migrations so old and new app versions can coexist, or explicitly call out when a coordinated write pause is required.

Default repo path: `/home/leoc/projects/weweb/weweb-docker/weweb-back`.

## Creation Workflow

1. Inspect existing patterns:
   - current branch/base and dirty state,
   - adjacent files in `migrations/`,
   - affected Sequelize models,
   - current code paths that create/update affected rows,
   - previous migrations touching the same tables/columns.
2. Pick a timestamped file name:
   - use `YYYYMMDDHHMMSS-clearPurpose.js`,
   - keep names descriptive of the schema change, not the incident,
   - do not rename an already-run migration unless `SequelizeMeta` impact is understood.
3. Split the rollout mentally into phases:
   - expand schema in a backward-compatible way,
   - backfill existing data,
   - make new app code write/read the new shape,
   - enforce stricter constraints only after old writers are gone or DB-side compatibility exists.
4. Implement `up` and `down`.
5. Validate syntax, diff hygiene, conflict markers, and migration safety notes.

## Required Design Questions

Before writing the migration, answer these in your own reasoning:

- Can currently deployed code insert or update this table while the migration runs?
- If yes, what value will old code write for new columns?
- Does any new `NOT NULL`, foreign key, check, or unique constraint reject old-code writes?
- Is the migration safe if it fails halfway and is rerun?
- Is the operation likely to lock a hot table long enough to matter?
- Does this migration need an app deploy before enforcement?

If the answer is unclear, inspect code or query live/staging state before guessing.

## Safe Patterns

### Add a required column

Do not add or enforce a required column in one step unless old writers cannot touch the table.

Preferred bridge:

```sql
ALTER TABLE "Table" ADD COLUMN IF NOT EXISTS "newColumn" text;
UPDATE "Table" SET "newColumn" = <safe_value> WHERE "newColumn" IS NULL;
```

Then choose one:

- keep it nullable for one deploy and enforce in a later migration,
- add a DB default that old app writes can tolerate,
- add a temporary trigger that derives the value from existing row data,
- use a coordinated write pause when product/ops explicitly accepts it.

Only add `SET NOT NULL` after old writers are handled.

### Backfill then constrain

Backfill immediately before adding a constraint, but remember this is not race-proof. A concurrent insert can happen after the backfill and before constraint enforcement unless a lock, default, trigger, nullable bridge, or write pause protects the window.

For `Designs.slug`, a safe bridge is either:

- leave `slug` nullable while new code starts writing it, or
- add a `BEFORE INSERT` trigger that fills missing `slug` from `id`.

### Unique indexes

Before adding a unique index, check for duplicates:

```sql
SELECT "column", count(*)
FROM "Table"
WHERE "column" IS NOT NULL
GROUP BY "column"
HAVING count(*) > 1;
```

Postgres unique indexes allow multiple `NULL` values. If relying on that as a bridge, say so explicitly.

### Foreign keys and constraints

Before adding a foreign key:

- check orphaned rows,
- choose `ON DELETE` / `ON UPDATE` deliberately,
- prefer `SET NULL` only when the column is nullable and the app tolerates it,
- use idempotent constraint creation in recovery-sensitive migrations.

Idempotent constraint pattern:

```sql
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'constraint_name'
    ) THEN
        ALTER TABLE "Child"
            ADD CONSTRAINT "constraint_name"
            FOREIGN KEY ("parentId")
            REFERENCES "Parent" ("id")
            ON UPDATE CASCADE
            ON DELETE SET NULL;
    END IF;
END $$;
```

## Transaction Guidance

Use transactions for groups of DDL/data changes that should commit together.

Do not assume a transaction solves rolling deploy compatibility. It may keep locks until commit, but old tasks can still write incompatible rows after commit unless the schema remains compatible.

Use explicit locks only after considering hot-table blocking:

```sql
LOCK TABLE "Designs" IN ACCESS EXCLUSIVE MODE;
```

If using a lock, still provide old-writer protection after commit.

## Idempotency

Prefer idempotent SQL when a migration may be rerun or when prod has partial state:

- `ADD COLUMN IF NOT EXISTS`
- `DROP ... IF EXISTS`
- `CREATE UNIQUE INDEX IF NOT EXISTS`
- `ALTER COLUMN ... DROP NOT NULL`
- repeatable `UPDATE ... WHERE column IS NULL OR column = ''`
- `DO $$ ... IF NOT EXISTS ... ADD CONSTRAINT ... END $$;`

Never manually insert into `SequelizeMeta` before all intended schema and data changes are complete.

## Down Migrations

Implement `down` with the same care:

- drop constraints before columns they reference,
- restore nullability only when existing data satisfies it,
- avoid destructive drops when rollback would destroy user data unless that is already accepted in local migration patterns,
- use `IF EXISTS` for recovery-friendly reversals when practical.

## Naming and SequelizeMeta

Sequelize migration file names are migration identities. If a file is renamed after it has run in an environment, Sequelize treats the new name as a different migration.

Before renaming a migration, state whether target environments have the old name in `SequelizeMeta`. If unknown, check or warn.

## Validation

After creating or editing a migration:

- Run `node --check <migration-file>`.
- Run `git diff --check`.
- Search for conflict markers: `rg -n '<<<|====' <migration-file>`.
- Inspect `git diff` to ensure the migration name, timestamp, `up`, and `down` are intentional.
- For risky changes, include the SQL query that would verify preconditions in prod/staging.

## Red Flags

Stop and explain risk before proceeding when the migration:

- adds `NOT NULL` to a column old app code does not write,
- backfills once then immediately constrains without protecting concurrent inserts,
- creates a unique index without duplicate checks,
- renames a possibly-run migration,
- is not safe to rerun after partial failure,
- requires `SequelizeMeta` edits as part of the normal path,
- assumes new ECS service tasks are live before the migration completes,
- drops a column/table to retry when in-place recovery would be safer.
