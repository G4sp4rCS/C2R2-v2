# Native Golsta collector inside C2R2

## Objective

Run the Golsta collector and result API from the `c2r2-server` process without
changing or recompiling the existing `golsta.exe` client.

The agent-side flow remains unchanged:

```text
/harvest
  -> C2R2 uploads golsta.exe to the agent
  -> the agent executes it once and removes it
  -> golsta.exe connects to its collector
  -> C2R2 receives, stores, indexes, and exposes the result
```

## Implemented

- `c2r2-server/src/golsta_collector.rs` implements the GLST receiver in-process.
- C2R2 starts a TLS listener on `--golsta-bind`/`--golsta-port` (defaults to
  `0.0.0.0:9090`) and uses the existing C2R2 certificate pair.
- The receiver verifies the GLST magic/version, bounded ciphertext length,
  HMAC-SHA256, and AES-256-GCM before accepting an archive.
- ZIP entry paths are validated, the archive is written through a temporary
  file and atomic rename, and its metadata is published to the existing
  authenticated `/api/golsta/*` routes.
- `/harvest` now snapshots the local store and watches it directly; no
  `GOLSTA_INTEGRATION_TOKEN`, `--golsta-url`, or external HTTP facade is used.
- The Golsta client artifact and source project remain untouched.

## Protocol source

The receiver follows Golsta's private `server/transport/protocol.go` and
`go/pkg/net/protocol.go` contract:

```text
[4] GLST [1] version [8] timestamp LE [32] key XOR transport key
[12] nonce [4] ciphertext length LE [N] AES-256-GCM ciphertext [32] HMAC
transport key = HMAC-SHA256(shared secret, timestamp LE bytes)
```

The optional ECDSA trailer is not needed for decryption, matching the Golsta
server implementation. The receiver does not alter or rebuild the client.

## Reused API and CLI

The upstream HTTP client was replaced by an in-process `GolstaStore` handle.
The existing authenticated operator routes remain:

- `GET /api/golsta/status`
- `GET /api/golsta/harvests`
- `GET /api/golsta/harvests/:id/archive`

The `/harvest` CLI snapshots the current result IDs, dispatches the existing
agent commands, and watches the local store instead of polling an external HTTP
service.

## Validation and rollback

Use synthetic, operator-authored archives and a loopback collector fixture:

- valid packet decryption;
- invalid secret/HMAC and truncated header/body;
- oversized payload;
- malformed archive identifier and traversal attempts;
- atomic write failure cleanup;
- `/harvest` success and timeout;
- clean shutdown with no partial files left behind.

Rollback is to restore the current upstream HTTP adapter and remove the native
collector listener; the agent-side `golsta.exe` remains untouched throughout.

The native collector intentionally stores the validated raw Golsta ZIP rather
than porting Golsta's optional post-processing/panel. The existing API exposes
that archive and redacted metadata; richer normalization can be added later
only if it is required by the operator workflow.
