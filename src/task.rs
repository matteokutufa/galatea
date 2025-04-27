//! Gestione dei task per Galatea
//!
//! Questo modulo definisce la struttura e le operazioni sui task, che sono
//! elementi atomici che possono essere eseguiti (script bash o playbook ansible).

use std::path::{Path, PathBuf};
use std::fs;
use std::collections::HashMap;
use anyhow::{Context, Result, anyhow};
use serde::{Serialize, Deserialize};
use log::{info, warn, error, debug};

use crate::config::Config;
use crate::downloader;
use crate::executor;

/// Tipi di script supportati
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScriptType {
    /// Script Bash
    Bash,
    /// Playbook Ansible
    Ansible,
    /// Mix di entrambi
    Mixed,
}

impl ScriptType {
    /// Converte una stringa nel tipo di script corrispondente
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "bash" | "b" => Ok(ScriptType::Bash),
            "ansible" | "a" => Ok(ScriptType::Ansible),
            "mixed" | "m" => Ok(ScriptType::Mixed),
            _ => Err(anyhow!("Unknown script type: {}", s)),
        }
    }

    /// Converte il tipo di script in una stringa
    pub fn to_str(&self) -> &'static str {
        match self {
            ScriptType::Bash => "bash",
            ScriptType::Ansible => "ansible",
            ScriptType::Mixed => "mixed",
        }
    }

    /// Restituisce la lettera identificativa del tipo di script
    pub fn get_letter(&self) -> char {
        match self {
            ScriptType::Bash => 'B',
            ScriptType::Ansible => 'A',
            ScriptType::Mixed => 'M',
        }
    }
}

/// Definizione di un task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Nome del task
    pub name: String,

    /// Tipo di script (Bash, Ansible, Mixed)
    pub script_type: ScriptType,

    /// Descrizione del task
    pub description: String,

    /// URL da cui scaricare il task
    pub url: String,

    /// Comando per la pulizia/disinstallazione
    pub cleanup_command: Option<String>,

    /// Dipendenze (altri task che devono essere eseguiti prima)
    pub dependencies: Vec<String>,

    /// Tag per categorizzare il task
    pub tags: Vec<String>,

    /// Flag che indica se è richiesto il riavvio
    pub requires_reboot: bool,

    /// Percorso locale dove è stato scaricato il task (calcolato a runtime)
    #[serde(skip)]
    pub local_path: Option<PathBuf>,

    /// Flag che indica se il task è installato
    #[serde(skip)]
    pub installed: bool,
}

impl Task {
    /// Crea un nuovo task da un hashmap di valori
    pub fn from_hashmap(values: &HashMap<String, serde_yaml::Value>) -> Result<Self> {
        // Estrai i valori richiesti
        let name = values.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Task missing 'name' field"))?
            .to_string();

        let type_str = values.get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Task missing 'type' field"))?;

        let script_type = ScriptType::from_str(type_str)
            .context(format!("Invalid script type for task {}: {}", name, type_str))?;

        let description = values.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let url = values.get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Task missing 'url' field"))?
            .to_string();

        let cleanup_command = values.get("cleanup_command")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Estrai le dipendenze
        let mut dependencies = Vec::new();
        if let Some(deps) = values.get("dependencies") {
            if let Some(deps_array) = deps.as_sequence() {
                for dep in deps_array {
                    if let Some(dep_str) = dep.as_str() {
                        dependencies.push(dep_str.to_string());
                    }
                }
            }
        }

        // Estrai i tag
        let mut tags = Vec::new();
        if let Some(tag_values) = values.get("tags") {
            if let Some(tag_array) = tag_values.as_sequence() {
                for tag in tag_array {
                    if let Some(tag_str) = tag.as_str() {
                        tags.push(tag_str.to_string());
                    }
                }
            }
        }

        // Estrai il flag requires_reboot
        let requires_reboot = values.get("requires_reboot")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Ok(Task {
            name,
            script_type,
            description,
            url,
            cleanup_command,
            dependencies,
            tags,
            requires_reboot,
            local_path: None,
            installed: false,
        })
    }

    /// Verifica se il task è installato
    pub fn check_installed(&mut self, config: &Config) -> Result<bool> {
        let state_file = config.resolve_path(&format!("{}.state", self.name), "state");

        if state_file.exists() {
            let content = fs::read_to_string(&state_file)
                .context(format!("Failed to read state file for task {}", self.name))?;

            // Se il file esiste e contiene "installed", il task è installato
            self.installed = content.trim() == "installed";
        } else {
            self.installed = false;
        }

        Ok(self.installed)
    }

    /// Installa il task
    pub fn install(&mut self, config: &Config) -> Result<()> {
        info!("Installing task: {}", self.name);

        // Scarica il task se necessario
        self.download(config)?;

        // Controlla se ci sono dipendenze mancanti
        if !self.dependencies.is_empty() {
            warn!("Task {} has dependencies that need to be installed first", self.name);
            // In un'implementazione reale, qui si potrebbe risolvere le dipendenze
            // Per ora, avvisiamo solo e procediamo
        }

        // Esegui il task
        let local_path = self.local_path.as_ref()
            .ok_or_else(|| anyhow!("Task not downloaded: {}", self.name))?;

        match self.script_type {
            ScriptType::Bash => {
                executor::run_bash_script(local_path, &["install"])
                    .context(format!("Failed to run bash install script for task {}", self.name))?;
            },
            ScriptType::Ansible => {
                executor::run_ansible_playbook(local_path, "install")
                    .context(format!("Failed to run ansible playbook for task {}", self.name))?;
            },
            ScriptType::Mixed => {
                // Per i task mixed, prova prima ansible e poi bash se necessario
                if let Err(e) = executor::run_ansible_playbook(local_path, "install") {
                    warn!("Ansible playbook failed for mixed task {}, trying bash: {}", self.name, e);
                    executor::run_bash_script(local_path, &["install"])
                        .context(format!("Both ansible and bash failed for mixed task {}", self.name))?;
                }
            }
        }

        // Segna come installato
        let state_file = config.resolve_path(&format!("{}.state", self.name), "state");
        fs::write(&state_file, "installed")
            .context(format!("Failed to write state file for task {}", self.name))?;

        self.installed = true;
        info!("Task {} installed successfully", self.name);

        Ok(())
    }

    /// Disinstalla il task
    pub fn uninstall(&mut self, config: &Config) -> Result<()> {
        info!("Uninstalling task: {}", self.name);

        // Verifica che il task sia installato
        if !self.check_installed(config)? {
            return Err(anyhow!("Task is not installed: {}", self.name));
        }

        // Scarica il task se necessario
        self.download(config)?;

        // Esegui il comando di cleanup
        let local_path = self.local_path.as_ref()
            .ok_or_else(|| anyhow!("Task not downloaded: {}", self.name))?;

        match self.script_type {
            ScriptType::Bash => {
                if let Some(cmd) = &self.cleanup_command {
                    executor::run_command(cmd)
                        .context(format!("Failed to run cleanup command for task {}", self.name))?;
                } else {
                    executor::run_bash_script(local_path, &["uninstall"])
                        .context(format!("Failed to run bash uninstall script for task {}", self.name))?;
                }
            },
            ScriptType::Ansible => {
                if let Some(cmd) = &self.cleanup_command {
                    executor::run_command(cmd)
                        .context(format!("Failed to run cleanup command for task {}", self.name))?;
                } else {
                    executor::run_ansible_playbook(local_path, "uninstall")
                        .context(format!("Failed to run ansible uninstall playbook for task {}", self.name))?;
                }
            },
            ScriptType::Mixed => {
                if let Some(cmd) = &self.cleanup_command {
                    executor::run_command(cmd)
                        .context(format!("Failed to run cleanup command for task {}", self.name))?;
                } else {
                    // Per i task mixed, prova prima ansible e poi bash se necessario
                    if let Err(e) = executor::run_ansible_playbook(local_path, "uninstall") {
                        warn!("Ansible playbook failed for mixed task {}, trying bash: {}", self.name, e);
                        executor::run_bash_script(local_path, &["uninstall"])
                            .context(format!("Both ansible and bash failed for mixed task {}", self.name))?;
                    }
                }
            }
        }

        // Rimuovi il file di stato
        let state_file = config.resolve_path(&format!("{}.state", self.name), "state");
        if state_file.exists() {
            fs::remove_file(&state_file)
                .context(format!("Failed to remove state file for task {}", self.name))?;
        }

        self.installed = false;
        info!("Task {} uninstalled successfully", self.name);

        Ok(())
    }

    /// Reset del task alle impostazioni iniziali
    pub fn reset(&mut self, config: &Config) -> Result<()> {
        info!("Resetting task: {}", self.name);

        // Verifica che il task sia installato
        if !self.check_installed(config)? {
            return Err(anyhow!("Task is not installed: {}", self.name));
        }

        // Scarica il task se necessario
        self.download(config)?;

        // Esegui il comando di reset
        let local_path = self.local_path.as_ref()
            .ok_or_else(|| anyhow!("Task not downloaded: {}", self.name))?;

        match self.script_type {
            ScriptType::Bash => {
                executor::run_bash_script(local_path, &["reset"])
                    .context(format!("Failed to run bash reset script for task {}", self.name))?;
            },
            ScriptType::Ansible => {
                executor::run_ansible_playbook(local_path, "reset")
                    .context(format!("Failed to run ansible reset playbook for task {}", self.name))?;
            },
            ScriptType::Mixed => {
                // Per i task mixed, prova prima ansible e poi bash se necessario
                if let Err(e) = executor::run_ansible_playbook(local_path, "reset") {
                    warn!("Ansible playbook failed for mixed task {}, trying bash: {}", self.name, e);
                    executor::run_bash_script(local_path, &["reset"])
                        .context(format!("Both ansible and bash failed for mixed task {}", self.name))?;
                }
            }
        }

        info!("Task {} reset successfully", self.name);

        Ok(())
    }

    /// Riavvia i servizi del task
    pub fn remediate(&mut self, config: &Config) -> Result<()> {
        info!("Remediating task: {}", self.name);

        // Verifica che il task sia installato
        if !self.check_installed(config)? {
            return Err(anyhow!("Task is not installed: {}", self.name));
        }

        // Scarica il task se necessario
        self.download(config)?;

        // Esegui il comando di remediation
        let local_path = self.local_path.as_ref()
            .ok_or_else(|| anyhow!("Task not downloaded: {}", self.name))?;

        match self.script_type {
            ScriptType::Bash => {
                executor::run_bash_script(local_path, &["remediate"])
                    .context(format!("Failed to run bash remediate script for task {}", self.name))?;
            },
            ScriptType::Ansible => {
                executor::run_ansible_playbook(local_path, "remediate")
                    .context(format!("Failed to run ansible remediate playbook for task {}", self.name))?;
            },
            ScriptType::Mixed => {
                // Per i task mixed, prova prima ansible e poi bash se necessario
                if let Err(e) = executor::run_ansible_playbook(local_path, "remediate") {
                    warn!("Ansible playbook failed for mixed task {}, trying bash: {}", self.name, e);
                    executor::run_bash_script(local_path, &["remediate"])
                        .context(format!("Both ansible and bash failed for mixed task {}", self.name))?;
                }
            }
        }

        info!("Task {} remediated successfully", self.name);

        Ok(())
    }

    /// Scarica il task e lo estrae nella directory appropriata
    pub fn download(&mut self, config: &Config) -> Result<PathBuf> {
        // Se il task è già stato scaricato, restituisci il percorso
        if let Some(path) = &self.local_path {
            if path.exists() {
                return Ok(path.clone());
            }
        }

        info!("Downloading task: {} from {}", self.name, self.url);

        // Crea il percorso di destinazione
        let task_dir = config.resolve_path(&self.name, "tasks");

        // Scarica e/o estrai il task
        let downloaded_path = downloader::download_and_extract(
            &self.url,
            &task_dir,
            config.download_timeout,
        ).context(format!("Failed to download task: {}", self.name))?;

        self.local_path = Some(downloaded_path.clone());

        info!("Task {} downloaded successfully to {:?}", self.name, downloaded_path);

        Ok(downloaded_path)
    }
}

/// Carica i task da tutti i file di configurazione disponibili
pub fn load_tasks(config: &Config) -> Result<Vec<Task>> {
    info!("Loading tasks from configuration files");

    let mut tasks = Vec::new();
    let tasks_dir = Path::new(&config.tasks_dir);

    // Verifica che la directory esista
    if !tasks_dir.exists() {
        info!("Tasks directory does not exist: {}, creating it", config.tasks_dir);
        fs::create_dir_all(tasks_dir).context(format!("Failed to create tasks directory: {}", config.tasks_dir))?;
        return Ok(tasks);
    }

    // Crea una configurazione di task di esempio se non ci sono file .conf
    let conf_files = fs::read_dir(tasks_dir)
        .context(format!("Failed to read tasks directory: {}", config.tasks_dir))?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.path().is_file() &&
                entry.path().extension().map_or(false, |ext| ext == "conf")
        })
        .count();

    if conf_files == 0 {
        info!("No task configuration files found, creating an example");
        create_example_task_config(tasks_dir)?;
    }

    // Leggi tutti i file di configurazione (con estensione .conf)
    for entry in fs::read_dir(tasks_dir)
        .context(format!("Failed to read tasks directory: {}", config.tasks_dir))? {

        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();

        // Controlla che sia un file con estensione .conf
        if path.is_file() && path.extension().map_or(false, |ext| ext == "conf") {
            debug!("Loading tasks from file: {:?}", path);

            // Leggi il contenuto del file
            let content = fs::read_to_string(&path)
                .context(format!("Failed to read task config file: {:?}", path))?;

            // Analizza il contenuto come YAML
            let yaml: serde_yaml::Value = serde_yaml::from_str(&content)
                .context(format!("Failed to parse YAML from file: {:?}", path))?;

            // Estrai la lista dei task
            if let Some(task_list) = yaml.get("tasks") {
                if let Some(task_array) = task_list.as_sequence() {
                    for task_value in task_array {
                        if let Some(task_map) = task_value.as_mapping() {
                            // Converti il mapping di YAML in HashMap
                            let mut task_hash = HashMap::new();
                            for (key, value) in task_map {
                                if let Some(key_str) = key.as_str() {
                                    task_hash.insert(key_str.to_string(), value.clone());
                                }
                            }

                            // Crea un nuovo task
                            match Task::from_hashmap(&task_hash) {
                                Ok(mut task) => {
                                    // Verifica se il task è già installato
                                    if let Err(e) = task.check_installed(config) {
                                        warn!("Failed to check if task {} is installed: {}", task.name, e);
                                    }

                                    tasks.push(task);
                                },
                                Err(e) => {
                                    warn!("Failed to parse task from {:?}: {}", path, e);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    info!("Loaded {} tasks", tasks.len());
    Ok(tasks)
}

/// Crea un file di configurazione di task di esempio
fn create_example_task_config(tasks_dir: &Path) -> Result<()> {
    let example_file_path = tasks_dir.join("example_tasks.conf");

    let example_content = r#"# Esempio di configurazione dei task
# Questo file contiene definizioni di task di esempio

tasks:
  - name: example_bash_task
    type: bash
    description: "Un task bash di esempio che installa un pacchetto"
    url: "https://example.com/tasks/bash_task.tgz"
    requires_reboot: false
    tags:
      - example
      - bash

  - name: example_ansible_task
    type: ansible
    description: "Un task ansible di esempio che configura un servizio"
    url: "https://example.com/tasks/ansible_task.zip"
    cleanup_command: "systemctl stop example_service"
    requires_reboot: true
    tags:
      - example
      - ansible
      - service

  - name: example_mixed_task
    type: mixed
    description: "Un task misto di esempio che può usare sia bash che ansible"
    url: "https://example.com/tasks/mixed_task.tar.gz"
    dependencies:
      - example_bash_task
    tags:
      - example
      - mixed
"#;

    fs::write(&example_file_path, example_content)
        .context(format!("Failed to write example task config file: {:?}", example_file_path))?;

    info!("Created example task configuration file: {:?}", example_file_path);
    Ok(())
}