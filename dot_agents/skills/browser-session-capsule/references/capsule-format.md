# Capsule format

Read this reference only when changing the helper or diagnosing format compatibility.

## Encrypted envelope

The persistent JSON file contains no plaintext cookie data:

```json
{
  "format": "browser-cookie-capsule",
  "version": 1,
  "cipher": "AES-256-GCM",
  "kdf": {
    "name": "HKDF-SHA256",
    "salt": "base64",
    "info": "browser-cookie-capsule/v1"
  },
  "nonce": "base64",
  "tag": "base64",
  "ciphertext": "base64"
}
```

Associated data is `browser-cookie-capsule:1:AES-256-GCM`. Salt and nonce are newly generated for every export. The 1Password item supplies the HKDF input key material.

## Decrypted payload

The authenticated plaintext is versioned separately as `browser-session-capsule` version 1. It records creation time, requested origins, source Chrome profile and database schema, and a Playwright-compatible `storageState` object.

Only cookies matching the requested target or related-origin hostnames are exported. `serve` additionally excludes expired cookies and secure cookies for HTTP origins. It preserves `domain`, `path`, expiry, `httpOnly`, `secure`, `sameSite`, and Chrome partition keys.

The current helper exports cookies only. Local storage, IndexedDB, service-worker state, extensions, client certificates, and browser-profile identity are outside this format; use an appropriate authenticated-browser route when a site requires them.
