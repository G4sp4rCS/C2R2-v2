use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct BuildRequest {
    pub(crate) server: String,
    pub(crate) name: String,
    pub(crate) production: bool,
}

#[derive(Debug)]
pub(crate) struct BuildOutput {
    pub(crate) artifact: PathBuf,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

pub(crate) fn parse_request(args: &[String]) -> Result<BuildRequest, String> {
    let mut server = None;
    let mut ip = None;
    let mut port = None;
    let mut name = String::from("agent");
    let mut production = false;
    let mut positional = Vec::new();
    let mut index = 0;

    while index < args.len() {
        let arg = &args[index];
        match arg.as_str() {
            "--production" | "-p" => production = true,
            "--dev" => production = false,
            "--server" | "--ip" | "--port" | "--name" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| format!("Falta valor para {}", arg))?;
                assign_option(arg, value, &mut server, &mut ip, &mut port, &mut name)?;
            }
            _ if arg.starts_with("--server=")
                || arg.starts_with("--ip=")
                || arg.starts_with("--port=")
                || arg.starts_with("--name=") =>
            {
                let (option, value) = arg.split_once('=').expect("checked option contains '='");
                assign_option(option, value, &mut server, &mut ip, &mut port, &mut name)?;
            }
            _ if arg.starts_with('-') => {
                return Err(format!("Opción desconocida: {}", arg));
            }
            _ => positional.push(arg.clone()),
        }
        index += 1;
    }

    if server.is_none() && ip.is_none() {
        match positional.as_slice() {
            [host_port, ..] if host_port.contains(':') => server = Some(host_port.clone()),
            [host] => ip = Some(host.clone()),
            [host, positional_port, ..] => {
                ip = Some(host.clone());
                if port.is_none() {
                    port = Some(positional_port.clone());
                }
            }
            [] => {}
        }
    } else if server.is_none() && port.is_none() {
        if let Some(positional_port) = positional.first() {
            port = Some(positional_port.clone());
        }
    }
    if name == "agent" {
        let name_index = if positional.first().is_some_and(|value| value.contains(':')) {
            1
        } else if ip.is_some() {
            2
        } else {
            usize::MAX
        };
        if let Some(value) = positional.get(name_index) {
            name = value.clone();
        }
    }

    if server.is_some() {
        if let (Some(ip), Some(port)) = (ip.as_ref(), port.as_ref()) {
            let expected = format!("{}:{}", ip, port);
            if server.as_deref() != Some(expected.as_str()) {
                return Err("No combines --server con --ip/--port".to_string());
            }
        }
    }

    let server = match server {
        Some(server) => validate_endpoint(&server)?,
        None => {
            let ip = ip
                .as_deref()
                .ok_or_else(|| "Falta --ip/--server".to_string())?;
            let port = port.as_deref().ok_or_else(|| "Falta --port".to_string())?;
            let server = format!("{}:{}", ip, port);
            validate_endpoint(&server)?
        }
    };

    validate_name(&name)?;
    Ok(BuildRequest {
        server,
        name,
        production,
    })
}

fn assign_option(
    option: &str,
    value: &str,
    server: &mut Option<String>,
    ip: &mut Option<String>,
    port: &mut Option<String>,
    name: &mut String,
) -> Result<(), String> {
    if value.is_empty() || value.contains('\0') {
        return Err(format!("Valor vacío o inválido para {}", option));
    }
    match option {
        "--server" => *server = Some(value.to_string()),
        "--ip" => *ip = Some(value.to_string()),
        "--port" => *port = Some(value.to_string()),
        "--name" => *name = value.to_string(),
        _ => return Err(format!("Opción desconocida: {}", option)),
    }
    Ok(())
}

fn validate_endpoint(endpoint: &str) -> Result<String, String> {
    if endpoint.chars().any(char::is_whitespace) || endpoint.contains('\0') {
        return Err("El servidor no puede contener espacios ni NUL".to_string());
    }

    let (host, port) = endpoint
        .rsplit_once(':')
        .ok_or_else(|| "El servidor debe tener formato host:puerto".to_string())?;
    if host.is_empty() {
        return Err("El servidor debe incluir un host".to_string());
    }
    let port = port
        .parse::<u16>()
        .map_err(|_| "El puerto debe ser un número entre 1 y 65535".to_string())?;
    if port == 0 {
        return Err("El puerto debe ser un número entre 1 y 65535".to_string());
    }
    Ok(endpoint.to_string())
}

fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name == "." || name == ".." || name.len() > 64 {
        return Err("El nombre debe tener entre 1 y 64 caracteres".to_string());
    }
    if !name
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        return Err("El nombre solo puede usar letras, números, '-', '_' y '.'".to_string());
    }
    Ok(())
}

pub(crate) async fn build(request: BuildRequest) -> Result<BuildOutput, String> {
    tokio::task::spawn_blocking(move || build_sync(&request))
        .await
        .map_err(|error| format!("El trabajo de build terminó inesperadamente: {}", error))?
}

fn build_sync(request: &BuildRequest) -> Result<BuildOutput, String> {
    let workspace_root = find_workspace_root().ok_or_else(|| {
        "No se encontró la raíz del workspace; ejecutá el server desde el repositorio o junto a su árbol de fuentes".to_string()
    })?;
    let server_dir = std::env::current_exe()
        .ok()
        .and_then(|executable| executable.parent().map(Path::to_path_buf))
        .ok_or_else(|| "No se pudo determinar el directorio del server".to_string())?;

    let output_base = server_dir.join(&request.name);
    let output_path = output_base.with_extension("exe");

    // El builder recibe siempre una ruta relativa al workspace. Si el server
    // está fuera del repositorio, se compila en dist/ y luego se copia junto
    // al ejecutable del server.
    let builder_base = match output_base.strip_prefix(&workspace_root) {
        Ok(relative) if !relative.as_os_str().is_empty() => workspace_root.join(relative),
        _ => workspace_root.join("dist").join(&request.name),
    };
    if let Some(parent) = builder_base.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("No se pudo preparar el directorio de salida: {}", error))?;
    }
    let builder_path = builder_base.with_extension("exe");
    let output_name = builder_base
        .strip_prefix(&workspace_root)
        .map(|relative| relative.to_string_lossy().into_owned())
        .unwrap_or_else(|_| request.name.clone());

    let mut command = Command::new("cargo");
    command
        .current_dir(&workspace_root)
        .env("CARGO_TERM_COLOR", "never")
        .args([
            "run",
            "--release",
            "--package",
            "builder",
            "--",
            "build-agent",
        ])
        .args(["--name", &output_name, "--server", &request.server]);
    if request.production {
        command.arg("--production");
    }

    let output = command
        .output()
        .map_err(|error| format!("No se pudo ejecutar cargo: {}", error))?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    if !output.status.success() {
        return Err(format!(
            "Compilación fallida ({}).\nstdout:\n{}\nstderr:\n{}",
            output.status, stdout, stderr
        ));
    }
    if !builder_path.is_file() {
        return Err("cargo terminó correctamente, pero no apareció el agent generado".to_string());
    }

    if builder_path != output_path {
        std::fs::copy(&builder_path, &output_path)
            .map_err(|error| format!("No se pudo copiar el agent junto al server: {}", error))?;
    }

    Ok(BuildOutput {
        artifact: output_path,
        stdout,
        stderr,
    })
}

fn find_workspace_root() -> Option<PathBuf> {
    let mut starts = Vec::new();
    if let Ok(current_dir) = std::env::current_dir() {
        starts.push(current_dir);
    }
    if let Ok(executable) = std::env::current_exe() {
        if let Some(parent) = executable.parent() {
            starts.push(parent.to_path_buf());
        }
    }

    starts
        .into_iter()
        .filter_map(|start| find_workspace_from(&start))
        .next()
}

fn find_workspace_from(start: &Path) -> Option<PathBuf> {
    for directory in start.ancestors() {
        let manifest = directory.join("Cargo.toml");
        if manifest.is_file()
            && std::fs::read_to_string(manifest)
                .map(|contents| contents.contains("[workspace]"))
                .unwrap_or(false)
        {
            return Some(directory.to_path_buf());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::parse_request;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn parses_ip_port_and_production() {
        let request = parse_request(&args(&[
            "--ip",
            "192.0.2.10",
            "--port",
            "4444",
            "--name",
            "lab-agent",
            "--production",
        ]))
        .unwrap();
        assert_eq!(request.server, "192.0.2.10:4444");
        assert_eq!(request.name, "lab-agent");
        assert!(request.production);
    }

    #[test]
    fn parses_server_form_and_rejects_unsafe_name() {
        let request = parse_request(&args(&["--server=localhost:4444"])).unwrap();
        assert_eq!(request.server, "localhost:4444");
        assert_eq!(request.name, "agent");

        assert!(
            parse_request(&args(&["--server", "localhost:4444", "--name", "../agent"])).is_err()
        );
    }

    #[test]
    fn rejects_invalid_port_and_unknown_options() {
        assert!(parse_request(&args(&["--ip", "127.0.0.1", "--port", "0"])).is_err());
        assert!(parse_request(&args(&["--wat"])).is_err());
    }
}
