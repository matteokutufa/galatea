//! Funzioni di utilità generale per Galatea
//!
//! Questo modulo fornisce funzioni di supporto generali utilizzate in diverse parti dell'applicazione.

use std::path::{Path, PathBuf};
use std::fs;
use std::process::Command;
use std::env;
use anyhow::{Context, Result, anyhow};
use log::error;

/// Verifica se l'applicazione è in esecuzione con privilegi di root
///
/// # Returns
///
/// `true` se l'applicazione è in esecuzione come root, altrimenti `false`
pub fn is_running_as_root() -> bool {
    #[cfg(unix)]
    {
        return unsafe { libc::geteuid() == 0 };
    }

    #[cfg(windows)]
    {
        use std::process::Command;
        // Su Windows, verifica con il comando 'net session' che fallisce se non amministratore
        match Command::new("net")
            .args(&["session"])
            .output() {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }

    #[cfg(not(any(unix, windows)))]
    {
        false
    }
}

/// Verifica se è la prima esecuzione come root
///
/// # Returns
///
/// `true` se è la prima esecuzione come root, altrimenti `false`
pub fn is_first_root_execution() -> bool {
    if !is_running_as_root() {
        return false;
    }

    // Controlla se esiste un file di stato che indica che l'applicazione è già stata eseguita come root
    let state_file = PathBuf::from("/opt/galatea/state/root_execution");

    if state_file.exists() {
        return false;
    }

    // Crea il file di stato
    if let Some(parent) = state_file.parent() {
        if !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                error!("Failed to create state directory: {}", e);
                return false;
            }
        }
    }

    // Crea il file di stato
    if let Err(e) = fs::write(&state_file, "executed") {
        error!("Failed to write state file: {}", e);
        return false;
    }

    true
}

/// Restituisce il nome dell'utente corrente
///
/// # Returns
///
/// Il nome dell'utente, o "unknown" se non determinabile
pub fn get_current_username() -> String {
    if let Ok(username) = env::var("USER") {
        return username;
    }

    if let Ok(username) = env::var("USERNAME") {
        return username;
    }

    #[cfg(unix)]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("whoami").output() {
            if output.status.success() {
                if let Ok(username) = String::from_utf8(output.stdout) {
                    return username.trim().to_string();
                }
            }
        }
    }

    "unknown".to_string()
}

/// Ottiene la home directory dell'utente corrente
///
/// # Returns
///
/// Il percorso della home directory, o None se non determinabile
pub fn get_home_dir() -> Option<PathBuf> {
    dirs::home_dir()
}

/// Ottiene la directory temporanea
///
/// # Returns
///
/// Il percorso della directory temporanea
pub fn get_temp_dir() -> PathBuf {
    env::temp_dir()
}

/// Verifica se un percorso è accessibile in scrittura
///
/// # Arguments
///
/// * `path` - Il percorso da verificare
///
/// # Returns
///
/// `true` se il percorso è accessibile in scrittura, altrimenti `false`
pub fn is_path_writable(path: &Path) -> bool {
    // Se il percorso non esiste, verifica la sua directory padre
    if !path.exists() {
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                return false;
            }

            return is_path_writable(parent);
        }

        return false;
    }

    // Verifica con un file temporaneo
    let temp_file = path.join(".galatea_write_test");
    let result = fs::write(&temp_file, "test");

    // Pulisci
    if temp_file.exists() {
        let _ = fs::remove_file(&temp_file);
    }

    result.is_ok()
}

/// Verifica se un programma è installato
///
/// # Arguments
///
/// * `program` - Il nome del programma da verificare
///
/// # Returns
///
/// `true` se il programma è installato, altrimenti `false`
pub fn is_program_installed(program: &str) -> bool {
    let result = if cfg!(target_os = "windows") {
        Command::new("where")
            .arg(program)
            .output()
    } else {
        Command::new("which")
            .arg(program)
            .output()
    };

    match result {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

/// Ottiene la lista dei file in una directory che corrispondono a un pattern
///
/// # Arguments
///
/// * `dir` - La directory da esaminare
/// * `extension` - L'estensione da cercare (senza il punto iniziale)
///
/// # Returns
///
/// Lista di percorsi dei file trovati
pub fn get_files_with_extension(dir: &Path, extension: &str) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    if !dir.exists() || !dir.is_dir() {
        return Err(anyhow!("Directory not found: {:?}", dir));
    }

    for entry in fs::read_dir(dir)
        .context(format!("Failed to read directory: {:?}", dir))? {

        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();

        if path.is_file() && path.extension().map_or(false, |ext| ext == extension) {
            files.push(path);
        }
    }

    Ok(files)
}

/// Formatta una dimensione in byte in una stringa leggibile
///
/// # Arguments
///
/// * `size` - La dimensione in byte
///
/// # Returns
///
/// La dimensione formattata come stringa (es. "4.2 MB")
pub fn format_file_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.2} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.2} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.2} KB", size as f64 / KB as f64)
    } else {
        format!("{} B", size)
    }
}

/// Restituisce il nome del sistema operativo
///
/// # Returns
///
/// Il nome del sistema operativo
pub fn get_os_name() -> String {
    #[cfg(target_os = "linux")]
    {
        if let Ok(output) = Command::new("lsb_release")
            .args(&["-ds"])
            .output() {

            if output.status.success() {
                if let Ok(os_name) = String::from_utf8(output.stdout) {
                    return os_name.trim().to_string();
                }
            }
        }

        if Path::new("/etc/os-release").exists() {
            if let Ok(content) = fs::read_to_string("/etc/os-release") {
                for line in content.lines() {
                    if line.starts_with("PRETTY_NAME=") {
                        let name = line.trim_start_matches("PRETTY_NAME=")
                            .trim_matches('"');
                        return name.to_string();
                    }
                }
            }
        }

        "Linux".to_string()
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(output) = Command::new("cmd")
            .args(&["/C", "ver"])
            .output() {

            if output.status.success() {
                if let Ok(os_name) = String::from_utf8(output.stdout) {
                    return os_name.trim().to_string();
                }
            }
        }

        "Windows".to_string()
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = Command::new("sw_vers")
            .args(&["-productName", "-productVersion"])
            .output() {

            if output.status.success() {
                if let Ok(os_name) = String::from_utf8(output.stdout) {
                    return format!("macOS {}", os_name.trim());
                }
            }
        }

        "macOS".to_string()
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        env::consts::OS.to_string()
    }
}