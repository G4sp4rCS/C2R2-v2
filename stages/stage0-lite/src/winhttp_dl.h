#pragma once
#include <windows.h>

/*
 * winhttp_dl.h — HTTPS download helper
 */

/*
 * winhttp_download - Download bytes from https://<host>:<port><path>
 *
 * Returns TRUE on success.
 * *out_buf is LocalAlloc'd + XOR-decrypted — caller must LocalFree.
 */
BOOL winhttp_download(
    const char*  host_a,
    WORD         port,
    const char*  path_a,
    BYTE**       out_buf,
    DWORD*       out_len
);

/* XOR decrypt in-place (also usable from other modules) */
void xor_decrypt(BYTE* buf, DWORD len, const BYTE* key, DWORD key_len);
