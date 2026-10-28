# Dotfiles

This is my dotfiles, managed by [chezmoi](https://github.com/twpayne/chezmoi).

To get the files you can run the following command :

```
sh -c "$(curl -fsLS get.chezmoi.io)" -- init --apply leo91000
```

## Secrets

Sensitive configuration is restored through chezmoi’s native 1Password `onepasswordRead` templates. Sign in with the 1Password CLI before applying the dotfiles.

The `Dev` vault item `chezmoi-local-configs` stores npm, AWS, ngrok, rclone, Graphite, and shell-profile configuration. Context7 uses the existing `context7-codex-api-key` item. Templates render the sensitive files with private permissions. When updating these files locally, update the corresponding 1Password field and keep the template in the repository.

`chezmoi add` rejects detected plaintext secrets. Private SSH keys, session stores, cookies, histories, and the local 1Password service-account token are not managed by this repository.
