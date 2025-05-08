//! Modulo per l'esecuzione di script e comandi
//!
//! Questo modulo fornisce funzionalità per eseguire script bash,
//! playbook ansible e comandi generici.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::fs;
use std::time::Duration;
use anyhow::{Context, Result, anyhow};
use log::{info, warn};

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

    let mut child = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(&["/C", command])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
    } else {
        Command::new("sh")
            .args(&["-c", command])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
    }.context(format!("Failed to execute command: {}", command))?;

    // Attendi la terminazione del processo e verifica il codice di uscita
    let status = child.wait()
        .context(format!("Failed to wait for command: {}", command))?;

    if !status.success() {
        return Err(anyhow!(
            "Command failed with exit code: {}",
            status.code().unwrap_or(-1)
        ));
    }

    Ok(())
}

/// Esegue un comando con timeout
///
/// # Arguments
///
/// * `command` - Il comando da eseguire
/// * `timeout_secs` - Timeout in secondi
///
/// # Returns
///
/// `Ok(())` in caso di successo, altrimenti un errore
pub fn run_command_with_timeout(command: &str, timeout_secs: u64) -> Result<()> {
    info!("Running command with timeout {}: {}", timeout_secs, command);

    let mut child = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(&["/C", command])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
    } else {
        Command::new("sh")
            .args(&["-c", command])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
    }.context(format!("Failed to execute command: {}", command))?;

    // Implementa un timeout manuale
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return Err(anyhow!(
                        "Command failed with exit code: {}",
                        status.code().unwrap_or(-1)
                    ));
                }
                return Ok(());
            }
            Ok(None) => {
                // Processo ancora in esecuzione
                if start.elapsed() > Duration::from_secs(timeout_secs) {
                    // Timeout raggiunto, termina il processo
                    info!("Timeout reached for command: {}", command);
                    #[cfg(unix)]
                    {
                        // Su Unix, invia un SIGTERM
                        unsafe {
                            libc::kill(child.id() as i32, libc::SIGTERM);
                        }
                    }
                    #[cfg(windows)]
                    {
                        child.kill().ok();
                    }
                    return Err(anyhow!("Command timed out after {} seconds", timeout_secs));
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => return Err(anyhow!("Error waiting for command: {}", e)),
        }
    }
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
        find_script_in_dir(script_path, &["install.sh"])?
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
    let mut child = Command::new(&script)
        .args(args)
        .current_dir(script.parent().unwrap_or(Path::new(".")))
        //.stdout(Stdio::inherit())
        //.stderr(Stdio::inherit())
        .spawn()
        .context(format!("Failed to execute script: {:?}", script))?;

    // Attendi la terminazione del processo e verifica il codice di uscita
    let status = child.wait()
        .context(format!("Failed to wait for script: {:?}", script))?;

    if !status.success() {

        return Err(anyhow!(
            "Script failed with exit code: {}",
            status.code().unwrap_or(-1)
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
    info!("Attempting to run ansible playbook at path: {:?}", playbook_path);
    
    // Determina il percorso del playbook
    let playbook = if playbook_path.is_dir() {
        // Cerca playbook con diverse estensioni
        let possible_playbooks = &[
            "playbook.yml", "playbook.yaml", 
            "main.yml", "main.yaml", 
            "site.yml", "site.yaml",
            "local.yml", "local.yaml",
            "install.yml", "install.yaml",
            "entrypoint.yml", "entrypoint.yaml"
        ];
        find_script_in_dir(playbook_path, possible_playbooks)?
    } else {
        // Usa direttamente il file se non è una directory
        playbook_path.to_path_buf()
    };

    info!("Using playbook: {:?}", playbook);

    // Verifica che il playbook esista
    if !playbook.exists() {
        return Err(anyhow!("Playbook not found: {:?}", playbook));
    }

    // Comandi di debug per verificare il contenuto del playbook
    info!("Playbook content preview:");
    if let Ok(content) = fs::read_to_string(&playbook) {
        for (i, line) in content.lines().take(5).enumerate() {
            info!("Line {}: {}", i + 1, line);
        }
    }

    // Esegui il playbook
    info!("Executing ansible-playbook with command: ansible-playbook -i localhost, --connection=local --tags={} {:?}", tag, playbook);
    unsafe {
        std::env::set_var("ANSIBLE_LOG_PATH", "/var/log/galatea/ansible.log");
        std::env::set_var("ANSIBLE_DISPLAY_ARGS_TO_STDOUT", "no");
        std::env::set_var("ANSIBLE_NO_LOG", "true");
        std::env::set_var("ANSIBLE_STDOUT_CALLBACK", "null");
    }
    let mut child = Command::new("ansible-playbook")
        .arg("-i")
        .arg("localhost,")
        .arg("--connection=local")
        .arg(format!("--tags={}", tag))
        .arg(&playbook)
        .current_dir(playbook.parent().unwrap_or(Path::new(".")))
        //.stdout(Stdio::inherit())
        //.stderr(Stdio::inherit())
        .spawn()
        .context(format!("Failed to execute ansible playbook: {:?}", playbook))?;

    // Attendi la terminazione del processo e verifica il codice di uscita
    let status = child.wait()
        .context(format!("Failed to wait for ansible playbook: {:?}", playbook))?;

    if !status.success() {
        return Err(anyhow!(
            "Ansible playbook failed with exit code: {}",
            status.code().unwrap_or(-1)
        ));
    }

    info!("Ansible playbook executed successfully");
    Ok(())
}

/// Cerca uno script all'interno di una directory
///
/// # Arguments
///
/// * `dir` - La directory in cui cercare
/// * `script_names` - I possibili nomi dello script da cercare
///
/// # Returns
///
/// Il percorso dello script, se trovato
fn find_script_in_dir(dir: &Path, script_names: &[&str]) -> Result<PathBuf> {
    // Verifica che la directory esista
    if !dir.exists() || !dir.is_dir() {
        return Err(anyhow!("Directory not found: {:?}", dir));
    }

    info!("Searching for scripts in directory: {:?}", dir);
    info!("Possible script names: {:?}", script_names);

    // Prova tutti i possibili nomi file
    for script_name in script_names {
        // Cerca lo script direttamente nella directory
        let direct_path = dir.join(script_name);
        info!("Checking for: {:?}, exists: {}", direct_path, direct_path.exists());
        if direct_path.exists() {
            return Ok(direct_path);
        }
    }

    // Elenco tutti i file nella directory per debug
    info!("Files in directory:");
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                info!("  {:?}", entry.path());
            }
        }
    }

    // Altrimenti, cerca in tutte le sottodirectory
    for entry in fs::read_dir(dir)
        .context(format!("Failed to read directory: {:?}", dir))? {

        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();

        if path.is_dir() {
            match find_script_in_dir(&path, script_names) {
                Ok(path) => return Ok(path),
                Err(_) => continue,
            }
        }
    }

    Err(anyhow!("No script found in directory {:?} with names {:?}", dir, script_names))
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

    let mut child = Command::new("sudo")
        .args(&["-S", "sh", "-c", command])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .context(format!("Failed to execute command with sudo: {}", command))?;

    // Attendi la terminazione del processo e verifica il codice di uscita
    let status = child.wait()
        .context(format!("Failed to wait for command with sudo: {}", command))?;

    if !status.success() {
        return Err(anyhow!(
            "Command with sudo failed with exit code: {}",
            status.code().unwrap_or(-1)
        ));
    }

    Ok(())
}