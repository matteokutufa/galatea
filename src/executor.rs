//! Modulo per l'esecuzione di script e comandi
//!
//! Questo modulo fornisce funzionalità per eseguire script bash,
//! playbook ansible e comandi generici.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::fs;
use anyhow::{Context, Result, anyhow};
use log::info;

/// Esegue un comando generico
///
/// # Arguments
///
/// * `command` - Il comando da eseguire
///
/// # Returns
///
/// `Ok(())` in caso di successo, altrimenti un errore
pub fn run_command(command: &str) -> Result<()> {
    info!("Running command: {}", command);

    let output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(&["/C", command])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
    } else {
        Command::new("sh")
            .args(&["-c", command])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
    }.context(format!("Failed to execute command: {}", command))?;

    if !output.status.success() {
        return Err(anyhow!(
            "Command failed with exit code: {}",
            output.status.code().unwrap_or(-1)
        ));
    }

    Ok(())
}

/// Esegue uno script bash
///
/// # Arguments
///
/// * `script_path` - Il percorso dello script o della directory contenente lo script
/// * `args` - Gli argomenti da passare allo script
///
/// # Returns
///
/// `Ok(())` in caso di successo, altrimenti un errore
pub fn run_bash_script(script_path: &Path, args: &[&str]) -> Result<()> {
    // Determina il percorso dello script
    let script = if script_path.is_dir() {
        find_script_in_dir(script_path, "install.sh")?
    } else {
        script_path.to_path_buf()
    };

    info!("Running bash script: {:?} with args: {:?}", script, args);

    // Verifica che lo script esista
    if !script.exists() {
        return Err(anyhow!("Script not found: {:?}", script));
    }

    // Imposta i permessi di esecuzione per lo script
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut perms = fs::metadata(&script)
            .context(format!("Failed to get file permissions: {:?}", script))?
            .permissions();

        perms.set_mode(0o755); // rwx r-x r-x

        fs::set_permissions(&script, perms)
            .context(format!("Failed to set file permissions: {:?}", script))?;
    }

    // Esegui lo script
    let output = Command::new(&script)
        .args(args)
        .current_dir(script.parent().unwrap_or(Path::new(".")))
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .context(format!("Failed to execute script: {:?}", script))?;

    if !output.status.success() {
        return Err(anyhow!(
            "Script failed with exit code: {}",
            output.status.code().unwrap_or(-1)
        ));
    }

    Ok(())
}

/// Esegue un playbook ansible
///
/// # Arguments
///
/// * `playbook_path` - Il percorso del playbook o della directory contenente il playbook
/// * `tag` - Il tag ansible da usare (install, uninstall, reset, remediate)
///
/// # Returns
///
/// `Ok(())` in caso di successo, altrimenti un errore
pub fn run_ansible_playbook(playbook_path: &Path, tag: &str) -> Result<()> {
    // Determina il percorso del playbook
    let playbook = if playbook_path.is_dir() {
        find_script_in_dir(playbook_path, "playbook.yml")?
    } else {
        playbook_path.to_path_buf()
    };

    info!("Running ansible playbook: {:?} with tag: {}", playbook, tag);

    // Verifica che il playbook esista
    if !playbook.exists() {
        return Err(anyhow!("Playbook not found: {:?}", playbook));
    }

    // Esegui il playbook
    let output = Command::new("ansible-playbook")
        .arg("-i")
        .arg("localhost,")
        .arg("--connection=local")
        .arg(format!("--tags={}", tag))
        .arg(&playbook)
        .current_dir(playbook.parent().unwrap_or(Path::new(".")))
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .context(format!("Failed to execute ansible playbook: {:?}", playbook))?;

    if !output.status.success() {
        return Err(anyhow!(
            "Ansible playbook failed with exit code: {}",
            output.status.code().unwrap_or(-1)
        ));
    }

    Ok(())
}

/// Cerca uno script all'interno di una directory
///
/// # Arguments
///
/// * `dir` - La directory in cui cercare
/// * `script_name` - Il nome dello script da cercare
///
/// # Returns
///
/// Il percorso dello script, se trovato
fn find_script_in_dir(dir: &Path, script_name: &str) -> Result<PathBuf> {
    // Verifica che la directory esista
    if !dir.exists() || !dir.is_dir() {
        return Err(anyhow!("Directory not found: {:?}", dir));
    }

    // Cerca lo script direttamente nella directory
    let direct_path = dir.join(script_name);
    if direct_path.exists() {
        return Ok(direct_path);
    }

    // Altrimenti, cerca in tutte le sottodirectory
    for entry in fs::read_dir(dir)
        .context(format!("Failed to read directory: {:?}", dir))? {

        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();

        if path.is_dir() {
            let script_path = find_script_in_dir(&path, script_name);
            if script_path.is_ok() {
                return script_path;
            }
        }
    }

    Err(anyhow!("Script {} not found in directory {:?}", script_name, dir))
}

/// Verifica se un comando è disponibile nel sistema
///
/// # Arguments
///
/// * `command` - Il comando da verificare
///
/// # Returns
///
/// `true` se il comando è disponibile, altrimenti `false`
pub fn is_command_available(command: &str) -> bool {
    let result = if cfg!(target_os = "windows") {
        Command::new("where")
            .arg(command)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
    } else {
        Command::new("which")
            .arg(command)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
    };

    match result {
        Ok(status) => status.success(),
        Err(_) => false,
    }
}

/// Verifica se ansible è installato
///
/// # Returns
///
/// `true` se ansible è installato, altrimenti `false`
pub fn is_ansible_available() -> bool {
    is_command_available("ansible-playbook")
}

/// Esegue un comando con privilegi elevati
///
/// # Arguments
///
/// * `command` - Il comando da eseguire
///
/// # Returns
///
/// `Ok(())` in caso di successo, altrimenti un errore
pub fn run_with_sudo(command: &str) -> Result<()> {
    info!("Running command with sudo: {}", command);

    let output = Command::new("sudo")
        .args(&["-S", "sh", "-c", command])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .output()
        .context(format!("Failed to execute command with sudo: {}", command))?;

    if !output.status.success() {
        return Err(anyhow!(
            "Command with sudo failed with exit code: {}",
            output.status.code().unwrap_or(-1)
        ));
    }

    Ok(())
}