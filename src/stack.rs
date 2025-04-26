//! Gestione degli stack per Galatea
//!
//! Questo modulo definisce la struttura e le operazioni sugli stack, che sono
//! raccolte di task che possono essere eseguiti insieme.

use std::path::{Path, PathBuf};
use std::fs;
use std::collections::HashMap;
use anyhow::{Context, Result, anyhow};
use serde::{Serialize, Deserialize};
use log::{info, warn, error, debug};

use crate::config::Config;
use crate::task::{Task, load_tasks};

/// Definizione di uno stack
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stack {
    /// Nome dello stack
    pub name: String,

    /// Descrizione dello stack
    pub description: String,

    /// Lista dei task contenuti nello stack
    pub task_names: Vec<String>,

    /// Flag che indica se è richiesto il riavvio
    pub requires_reboot: bool,

    /// Tag per categorizzare lo stack
    pub tags: Vec<String>,

    /// Flag che indica se lo stack è completamente installato
    #[serde(skip)]
    pub fully_installed: bool,

    /// Flag che indica se lo stack è parzialmente installato
    #[serde(skip)]
    pub partially_installed: bool,
}

impl Stack {
    /// Crea un nuovo stack da un hashmap di valori
    pub fn from_hashmap(values: &HashMap<String, serde_yaml::Value>) -> Result<Self> {
        // Estrai i valori richiesti
        let name = values.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Stack missing 'name' field"))?
            .to_string();

        let description = values.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Estrai i nomi dei task
        let mut task_names = Vec::new();
        if let Some(tasks_value) = values.get("tasks") {
            if let Some(tasks_array) = tasks_value.as_sequence() {
                for task in tasks_array {
                    if let Some(task_str) = task.as_str() {
                        task_names.push(task_str.to_string());
                    }
                }
            }
        }

        // Estrai il flag requires_reboot
        let requires_reboot = values.get("requires_reboot")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

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

        Ok(Stack {
            name,
            description,
            task_names,
            requires_reboot,
            tags,
            fully_installed: false,
            partially_installed: false,
        })
    }

    /// Verifica lo stato di installazione dello stack
    pub fn check_installation_status(&mut self, tasks: &[Task]) -> Result<()> {
        let mut installed_count = 0;
        let total_tasks = self.task_names.len();

        if total_tasks == 0 {
            self.fully_installed = false;
            self.partially_installed = false;
            return Ok(());
        }

        // Conta quanti task sono installati
        for task_name in &self.task_names {
            if let Some(task) = tasks.iter().find(|t| &t.name == task_name) {
                if task.installed {
                    installed_count += 1;
                }
            }
        }

        // Aggiorna lo stato
        self.fully_installed = installed_count == total_tasks && total_tasks > 0;
        self.partially_installed = installed_count > 0 && installed_count < total_tasks;

        Ok(())
    }

    /// Installa tutti i task dello stack
    pub fn install(&mut self, config: &Config, all_tasks: &mut [Task]) -> Result<()> {
        info!("Installing stack: {}", self.name);

        let mut failed_tasks = Vec::new();

        // Installa ogni task dello stack
        for task_name in &self.task_names {
            if let Some(task) = all_tasks.iter_mut().find(|t| &t.name == task_name) {
                match task.install(config) {
                    Ok(_) => {
                        info!("Successfully installed task {} as part of stack {}", task_name, self.name);
                    },
                    Err(e) => {
                        error!("Failed to install task {} as part of stack {}: {}", task_name, self.name, e);
                        failed_tasks.push(task_name.clone());
                    }
                }
            } else {
                warn!("Task {} not found for stack {}", task_name, self.name);
                failed_tasks.push(task_name.clone());
            }
        }

        // Aggiorna lo stato
        self.check_installation_status(all_tasks)?;

        // Se ci sono stati fallimenti, restituisci un errore
        if !failed_tasks.is_empty() {
            return Err(anyhow!(
                "Failed to install {} out of {} tasks in stack {}: {:?}",
                failed_tasks.len(),
                self.task_names.len(),
                self.name,
                failed_tasks
            ));
        }

        info!("Stack {} installed successfully", self.name);

        Ok(())
    }

    /// Disinstalla tutti i task dello stack
    pub fn uninstall(&mut self, config: &Config, all_tasks: &mut [Task]) -> Result<()> {
        info!("Uninstalling stack: {}", self.name);

        let mut failed_tasks = Vec::new();

        // Disinstalla ogni task dello stack in ordine inverso
        for task_name in self.task_names.iter().rev() {
            if let Some(task) = all_tasks.iter_mut().find(|t| &t.name == task_name) {
                match task.uninstall(config) {
                    Ok(_) => {
                        info!("Successfully uninstalled task {} as part of stack {}", task_name, self.name);
                    },
                    Err(e) => {
                        error!("Failed to uninstall task {} as part of stack {}: {}", task_name, self.name, e);
                        failed_tasks.push(task_name.clone());
                    }
                }
            } else {
                warn!("Task {} not found for stack {}", task_name, self.name);
                failed_tasks.push(task_name.clone());
            }
        }

        // Aggiorna lo stato
        self.check_installation_status(all_tasks)?;

        // Se ci sono stati fallimenti, restituisci un errore
        if !failed_tasks.is_empty() {
            return Err(anyhow!(
                "Failed to uninstall {} out of {} tasks in stack {}: {:?}",
                failed_tasks.len(),
                self.task_names.len(),
                self.name,
                failed_tasks
            ));
        }

        info!("Stack {} uninstalled successfully", self.name);

        Ok(())
    }

    /// Reset di tutti i task dello stack
    pub fn reset(&mut self, config: &Config, all_tasks: &mut [Task]) -> Result<()> {
        info!("Resetting stack: {}", self.name);

        let mut failed_tasks = Vec::new();

        // Resetta ogni task dello stack
        for task_name in &self.task_names {
            if let Some(task) = all_tasks.iter_mut().find(|t| &t.name == task_name) {
                match task.reset(config) {
                    Ok(_) => {
                        info!("Successfully reset task {} as part of stack {}", task_name, self.name);
                    },
                    Err(e) => {
                        error!("Failed to reset task {} as part of stack {}: {}", task_name, self.name, e);
                        failed_tasks.push(task_name.clone());
                    }
                }
            } else {
                warn!("Task {} not found for stack {}", task_name, self.name);
                failed_tasks.push(task_name.clone());
            }
        }

        // Se ci sono stati fallimenti, restituisci un errore
        if !failed_tasks.is_empty() {
            return Err(anyhow!(
                "Failed to reset {} out of {} tasks in stack {}: {:?}",
                failed_tasks.len(),
                self.task_names.len(),
                self.name,
                failed_tasks
            ));
        }

        info!("Stack {} reset successfully", self.name);

        Ok(())
    }

    /// Riavvia i servizi di tutti i task dello stack
    pub fn remediate(&mut self, config: &Config, all_tasks: &mut [Task]) -> Result<()> {
        info!("Remediating stack: {}", self.name);

        let mut failed_tasks = Vec::new();

        // Riavvia i servizi di ogni task dello stack
        for task_name in &self.task_names {
            if let Some(task) = all_tasks.iter_mut().find(|t| &t.name == task_name) {
                match task.remediate(config) {
                    Ok(_) => {
                        info!("Successfully remediated task {} as part of stack {}", task_name, self.name);
                    },
                    Err(e) => {
                        error!("Failed to remediate task {} as part of stack {}: {}", task_name, self.name, e);
                        failed_tasks.push(task_name.clone());
                    }
                }
            } else {
                warn!("Task {} not found for stack {}", task_name, self.name);
                failed_tasks.push(task_name.clone());
            }
        }

        // Se ci sono stati fallimenti, restituisci un errore
        if !failed_tasks.is_empty() {
            return Err(anyhow!(
                "Failed to remediate {} out of {} tasks in stack {}: {:?}",
                failed_tasks.len(),
                self.task_names.len(),
                self.name,
                failed_tasks
            ));
        }

        info!("Stack {} remediated successfully", self.name);

        Ok(())
    }
}

/// Carica gli stack da tutti i file di configurazione disponibili
pub fn load_stacks(config: &Config, tasks: &[Task]) -> Result<Vec<Stack>> {
    info!("Loading stacks from configuration files");

    let mut stacks = Vec::new();
    let stacks_dir = Path::new(&config.stacks_dir);

    // Verifica che la directory esista
    if !stacks_dir.exists() {
        warn!("Stacks directory does not exist: {}", config.stacks_dir);
        return Ok(stacks);
    }

    // Leggi tutti i file di configurazione (con estensione .conf)
    for entry in fs::read_dir(stacks_dir)
        .context(format!("Failed to read stacks directory: {}", config.stacks_dir))? {

        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();

        // Controlla che sia un file con estensione .conf
        if path.is_file() && path.extension().map_or(false, |ext| ext == "conf") {
            debug!("Loading stacks from file: {:?}", path);

            // Leggi il contenuto del file
            let content = fs::read_to_string(&path)
                .context(format!("Failed to read stack config file: {:?}", path))?;

            // Analizza il contenuto come YAML
            let yaml: serde_yaml::Value = serde_yaml::from_str(&content)
                .context(format!("Failed to parse YAML from file: {:?}", path))?;

            // Estrai la lista degli stack
            if let Some(stack_list) = yaml.get("stacks") {
                if let Some(stack_array) = stack_list.as_sequence() {
                    for stack_value in stack_array {
                        if let Some(stack_map) = stack_value.as_mapping() {
                            // Converti il mapping di YAML in HashMap
                            let mut stack_hash = HashMap::new();
                            for (key, value) in stack_map {
                                if let Some(key_str) = key.as_str() {
                                    stack_hash.insert(key_str.to_string(), value.clone());
                                }
                            }

                            // Crea un nuovo stack
                            match Stack::from_hashmap(&stack_hash) {
                                Ok(mut stack) => {
                                    // Verifica lo stato di installazione
                                    if let Err(e) = stack.check_installation_status(tasks) {
                                        warn!("Failed to check installation status for stack {}: {}", stack.name, e);
                                    }

                                    stacks.push(stack);
                                },
                                Err(e) => {
                                    warn!("Failed to parse stack from {:?}: {}", path, e);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    info!("Loaded {} stacks", stacks.len());
    Ok(stacks)
}