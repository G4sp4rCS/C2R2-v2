# Private Golsta Backend Integration

This integration keeps Golsta's source code and storage private while exposing
the minimum operator workflow through C2R2. C2R2 depends on a compiled PE and a
small authenticated HTTP contract; it does not vendor, link, or publish the
Golsta source tree.

## Data Flow

1. The operator runs `/harvest` for a selected Windows agent.
2. C2R2 reads `c2r2-server/modules/golsta.exe` and uploads it over the existing
   TLS agent session.
3. The agent runs the PE once, waits for completion, and removes the temporary
   executable.
4. Golsta sends its result to the private collector.
5. C2R2 polls Golsta's loopback-only integration API and prints the new result
   metadata in the interactive CLI.
6. C2R2 also exposes status, metadata, and archive streaming through the
   existing authenticated C2 API.

The collector listener used by the lab and the private integration listener are
separate services. In the conference setup, the collector uses
`192.168.2.7:9090` and the integration API uses `127.0.0.1:18080`.

## Private Boundary

- Golsta remains in its private repository (`J:\chino\golsta`).
- Only the compiled `c2r2-server/modules/golsta.exe` is consumed by C2R2.
- `modules/`, conference artifacts, local control files, and harvested archives
  must remain ignored by Git.
- C2R2 receives result metadata as JSON and streams ZIP archives without
  extracting or indexing their contents.

## Configuration

Both backends must receive the same non-empty integration token. Keep it out of
source control and shell history where practical.

```powershell
$env:GOLSTA_INTEGRATION_TOKEN = '<random-lab-token>'

# Private Golsta backend
.\golsta-server.exe -integration-token $env:GOLSTA_INTEGRATION_TOKEN

# C2R2 facade
.\c2r2-server.exe --golsta-url http://127.0.0.1:18080
```

Golsta accepts the token from its environment or its private
`-integration-token` flag. C2R2 reads it from `GOLSTA_INTEGRATION_TOKEN` only.
If the variable is absent, the Golsta facade is disabled and returns HTTP 503.

C2R2 deliberately accepts only plain HTTP URLs on loopback for this adapter.
This keeps the bearer token off the LAN while avoiding unnecessary TLS between
two processes on the same host.

## C2R2 API Contract

All routes require the normal C2R2 session bearer token:

| Method | Route | Result |
|--------|-------|--------|
| `POST` | `/api/agents/:id/harvest` | Upload and execute the compiled Golsta PE |
| `GET` | `/api/golsta/status` | Private backend health and harvest count |
| `GET` | `/api/golsta/harvests` | Result metadata without archive contents |
| `GET` | `/api/golsta/harvests/:id/archive` | Stream one ZIP archive |

Archive identifiers reject empty values, path separators, traversal sequences,
and characters outside ASCII letters, digits, `.`, `_`, and `-`. Archive
responses use `Cache-Control: private, no-store` and
`X-Content-Type-Options: nosniff`. The archive request timeout is ten minutes;
health and metadata requests use a fifteen-second timeout.

Expected error behavior:

| Status | Meaning |
|--------|---------|
| `401 Unauthorized` | Missing or invalid C2R2 session token |
| `503 Service Unavailable` | Golsta integration token is not configured |
| `502 Bad Gateway` | Golsta is unavailable or rejected the request |

## Validation

The adapter has a focused test covering bearer authentication, health, result
listing, archive streaming, traversal rejection, and non-loopback URL
rejection. The server test also verifies that the live watcher filters archives
that existed before `/harvest`. The complete release check is:

```bash
cargo test --release -p c2r2-server
cargo build --release -p c2r2-server --target x86_64-pc-windows-gnu
```

The private Golsta repository validates its side with `go test ./...`. The lab
end-to-end check must verify that a new result appears after `/harvest`, that an
archive downloaded through C2R2 matches the private backend's SHA-256, and that
the temporary agent executable is removed.

## Rollback

1. Stop C2R2 and remove `GOLSTA_INTEGRATION_TOKEN` from its environment.
2. Restart C2R2; `/api/golsta/*` will return HTTP 503.
3. Remove `c2r2-server/modules/golsta.exe` to disable `/harvest` dispatch.
4. Stop the private Golsta integration listener if it is no longer needed.

This integration does not add persistence or change the agent's persistence
configuration.
