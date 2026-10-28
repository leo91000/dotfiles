---
name: browser-session-capsule
description: Restore a cookie-backed session from local Google Chrome into browser_isolated when authenticated browser access is required and usable test credentials are unavailable or expired. Export only the requested site and related origins into an encrypted 1Password-gated capsule; use the Chrome connector instead when cookies cannot carry the required state.
---

# Browser Session Capsule

Keep browser automation in `browser_isolated` while reusing an existing Chrome login. The helper exports target-scoped cookies from Chrome, encrypts them at rest, and exposes decrypted cookies once over a random loopback URL for direct Playwright-context import.

## Route authentication

1. Open the target in `browser_isolated` and determine whether it is already authenticated.
2. Prefer a dedicated test login when one is available and suitable.
3. Use this capsule workflow when the required session already exists in local Chrome and credentials are unavailable, inappropriate, or expired.
4. Use the Chrome connector when the workflow depends on non-cookie browser state, an extension, CAPTCHA, 2FA, or a manual interaction that the capsule cannot reproduce.

## Export a fresh capsule

Resolve `scripts/session-capsule.rs` relative to this skill directory. Run:

```bash
<skill-dir>/scripts/session-capsule.rs export \
  --target https://app.example.com \
  --related-origin https://api.example.com
```

Include each application or API origin whose cookies may be needed. Omit `--related-origin` when the target alone is sufficient. The command prints only metadata and the encrypted output path. It reads Google Chrome's `Default` profile unless `--profile` is supplied.

The helper resolves the encryption key through 1Password in this order:

1. The shared `${XDG_STATE_HOME:-$HOME/.local/state}/1password/service-account.token`, when readable and authorized for the item.
2. Traditional `op` desktop approval if the shared token is unavailable, expired, or insufficiently scoped.

Treat an export as complete only when `roundTripValidated` is `true` and `cookieCount` is nonzero. A zero count usually means the wrong origin, Chrome profile, or login state.

## Import into browser_isolated

Start a one-shot bridge and keep its process/session alive:

```bash
<skill-dir>/scripts/session-capsule.rs serve \
  --capsule /absolute/path/to/capsule.cookies.enc.json \
  --target https://app.example.com \
  --related-origin https://api.example.com
```

The `ready` JSON line contains a random loopback `url` that expires after 45 seconds. Pass that URL to a single `browser_run_code_unsafe` call:

```javascript
async page => {
    const response = await page.context().request.get('ONE_SHOT_LOOPBACK_URL');
    if (!response.ok()) throw new Error(`Capsule bridge returned ${response.status()}`);
    const { cookies } = await response.json();
    await page.context().addCookies(cookies);
    return { importedCookieCount: cookies.length };
}
```

Wait for the bridge to report `served`, then navigate or reload the target. Verify authentication from observable UI or a harmless authenticated request. A successful import count alone is not authentication proof.

Use `inspect --capsule PATH --target URL [--related-origin URL]` to check only counts, origins, and expiry metadata without starting the bridge.

## Secret boundary

- Keep cookie values, decrypted storage state, Chrome keyring secrets, 1Password tokens, and the encryption key out of messages, tool arguments, logs, screenshots, shell history, environment variables, and plaintext files.
- Pass the one-shot URL only to the isolated browser. It is loopback-only, random, single-use, and short-lived.
- Keep capsules under `~/.local/share/codex/browser-session-capsules/` unless a different encrypted-file location is required. The helper creates new files with mode `0600` and never overwrites an existing capsule.
- Re-import after the isolated browser closes. Export again when cookies expire or Chrome receives a replacement session.

For envelope and cookie-field compatibility work, read [references/capsule-format.md](references/capsule-format.md).
