# WeWeb workspace-key endpoints

The canonical workspace is `3f66defb-254f-4495-99dd-d1ecb6960259`. Run `weweb-api capabilities` for the machine-readable catalogue.

## Project operations

All project operations require the project to belong to the canonical workspace and the workspace/project plan to enable the relevant public API feature.

- `deploy PROJECT_ID`: `POST .../deploy`; accepts optional JSON fields such as `commit`, `env`/`environment`, `builtZip`, `rawZip`, and `githubEnabled`.
- `deploy-last PROJECT_ID`: `GET .../deploy/last`.
- `version config PROJECT_ID VERSION`: retrieve a deployed version configuration.
- `version download PROJECT_ID VERSION --kind generated|raw --output-file PATH`: download a version ZIP atomically.
- `sitemap get PROJECT_ID`; `sitemap set PROJECT_ID --value|--file|--stdin`.
- `robots get PROJECT_ID`; `robots set PROJECT_ID --value|--file|--stdin`.
- `metadata list PROJECT_ID` and `metadata create|update|upsert|delete PROJECT_ID --params.*|--file|--stdin`.

Page metadata endpoints have a server-side daily quota and currently contain a known counter-initialization defect. Surface the server response; do not bypass it.

## Figma operations

These routes infer the workspace directly from the API key:

- `figma workspace-info`
- `figma coded-components`
- `figma library-components`
- `figma mappings list`
- `figma mappings upsert --params.*|--file|--stdin`
- `figma mappings delete --ww-object-base-id UUID` or `--library-component-id UUID`

The mappings upsert body must contain `mapping` and exactly one of `wwObjectBaseId` or `libraryComponentId`.

## Raw requests

`request` is restricted to relative paths beginning with `/public/v1/`. Build an object with `--params` or dotted `--params.*` flags. Values that parse as JSON keep their JSON type; other values become strings. Dotted paths create nested objects. Use `--file` or `--stdin` for arbitrary JSON, including top-level arrays.

```bash
weweb-api --env staging request GET /public/v1/figma/workspace-info
weweb-api --env staging request POST /public/v1/... --params.key value --apply
weweb-api --env staging request POST /public/v1/... --params '{"key":"value"}' --apply
weweb-api --env staging request PATCH /public/v1/... --file payload.json --apply
printf '%s' '{"key":"value"}' | weweb-api --env staging request PUT /public/v1/... --stdin --apply
```

For example, `--params.env staging --params.githubEnabled true --params.mapping.tag button` builds:

```json
{
  "env": "staging",
  "githubEnabled": true,
  "mapping": { "tag": "button" }
}
```

Quote a JSON string explicitly when a value such as `true`, `42`, or `null` must remain a string.

The deprecated `/publish` and `/publish/status` routes remain reachable only through `request`.
