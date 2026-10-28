---
name: weweb-api
description: Call WeWeb public workspace-key APIs on local, staging, preprod, or production. Use when an agent needs to deploy a project, inspect deploy state or version artifacts, manage sitemap, robots or page metadata, work with Figma component mappings, or send a raw request to a `/public/v1/` WeWeb endpoint.
---

# WeWeb API

Use the Rust CLI at `/home/leoc/.agents/skills/weweb-api/scripts/weweb-api` for every request. It retrieves the canonical WeWeb workspace key through `weweb-psql-env-executor`, unless `WEWEB_API_KEY` is already set.

## Workflow

1. Select the environment explicitly with `--env local|staging|preprod|prod`.
2. Run `capabilities` when the needed command or payload is unclear. Read [references/endpoints.md](references/endpoints.md) for endpoint-specific details.
3. Prefer a typed command. Use `request METHOD /public/v1/...` for a route without a typed wrapper, including deprecated routes.
4. Reads run directly. Every non-GET/HEAD/OPTIONS request requires `--apply`; production mutations also require `--allow-prod`.
5. Keep JSON on stdout for composition. Use `--output table` only for human inspection.
6. Report the HTTP result separately from any downstream effect. A successful deploy request is not proof that deployment completed.

## Safety

- The workspace key authenticates only public workspace-key routes. It does not grant editor, admin, user-JWT, OAuth, or microservice access.
- Never pass the key as an argument or print it. The CLI accepts only `WEWEB_API_KEY` or automatic database lookup.
- Use `--reveal-secrets` only when the user explicitly requests unredacted output and understands that the result will be visible.
- Do not retry mutations manually without first determining whether the server accepted the original request.
- `request` accepts relative `/public/v1/` paths only and reserves authentication and host headers.

## Examples

```bash
/home/leoc/.agents/skills/weweb-api/scripts/weweb-api --env staging capabilities
/home/leoc/.agents/skills/weweb-api/scripts/weweb-api --env staging figma workspace-info
/home/leoc/.agents/skills/weweb-api/scripts/weweb-api --env staging deploy <project-id> --params.env staging --apply
/home/leoc/.agents/skills/weweb-api/scripts/weweb-api --env prod deploy-last <project-id>
```
