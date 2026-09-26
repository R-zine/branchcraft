# Authentication design (future Milestone 5)

Authentication is deliberately absent from the current local-only build.
Milestone 5 will implement GitHub OAuth Authorization Code flow with random
state, a loopback-only ephemeral callback, and PKCE wherever GitHub supports it
for OAuth Apps at implementation time.

Tokens will be represented by a secret wrapper in Rust and persisted through a
`SecretStore` trait backed by the Linux Secret Service/GNOME Keyring. They will
not be serialized, returned to React, written to ordinary settings, placed in a
remote URL, or included in logs. Current research selects the maintained
`keyring` 4.2 line as the candidate; the exact backend and GitHub OAuth behavior
must be revalidated when the milestone begins.

