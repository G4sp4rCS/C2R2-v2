#pragma once
#include <windows.h>

/*
 * pe_loader.h — In-memory PE loader (Reflective DLL Loading)
 *
 * Loads a raw DLL from a byte buffer into executable memory
 * and calls DllMain via CreateThread.
 */

/*
 * reflective_load
 *
 * buf      - Raw DLL bytes (PE format, NOT shellcode)
 * buf_size - Size of the buffer
 *
 * Returns TRUE on success (DllMain thread launched).
 */
BOOL reflective_load(BYTE* buf, DWORD buf_size);
