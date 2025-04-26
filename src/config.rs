//! Gestione della configurazione per Galatea
//!
//! Questo modulo gestisce il caricamento e il salvataggio della configurazione dell'applicazione
//! utilizzando la libreria confucius.

use std::path::{Path, PathBuf};
use std::fs;
use anyhow::{Context, Result};
use confucius::{Config as ConfuciusConfig, ConfigValue};
use serde::{Serialize, Deserialize};
use log::{info, warn, error};

/// Struttura principale di configurazione per Galatea
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Directory per i task
    pub tasks_dir: String,

    /// Directory per gli stack
    pub stacks_dir: String,

    /// Directory per lo stato dell'applicazione
    pub state_dir: String,

    /// Timeout per il download in secondi
    pub download_timeout: u64,

    /// Tema dell'interfaccia utente
    pub ui_theme: String,

    /// URL delle sorgenti dei task
    pub task_sources: Vec<String>,

    /// URL delle sorgenti degli stack
    pub stack_sources: Vec<String>,
}

impl Config {
    /// Crea una nuova configurazione con valori di default
    pub fn default() -> Self {
        Config {
            tasks_dir: "/opt/galatea/tasks".to_string(),
            stacks_dir: "/opt/galatea/stacks".to_string(),
            state_dir: "/opt/galatea/state".to_string(),
            download_timeout: 60,
            ui_theme: "default".to_string(),
            task_sources: Vec::new(),
            stack_sources: Vec::new(),
        }
    }

    /// Carica la configurazione da un file
    pub fn load(path: &str) -> Result<Self> {
        info!("Loading configuration from: {}", path);

        let mut conf = ConfuciusConfig::new("galatea");
        conf.load_from_file(Path::new(path))
            .context(format!("Failed to load configuration from {}", path))?;

        // Estrai i valori dalla configurazione
        let tasks_dir = conf.get_string("general", "tasks_dir", Some("/opt/galatea/tasks"))
            .unwrap_or_else(|| "/opt/galatea/tasks".to_string());

        let stacks_dir = conf.get_string("general", "stacks_dir", Some("/opt/galatea/stacks"))
            .unwrap_or_else(|| "/opt/galatea/stacks".to_string());

        let state_dir = conf.get_string("general", "state_dir", Some("/opt/galatea/state"))
            .unwrap_or_else(|| "/opt/galatea/state".to_string());

        let download_timeout = conf.get_integer("general", "download_timeout", Some(60))
            .unwrap_or(60) as u64;

        let ui_theme = conf.get_string("general", "ui_theme", Some("default"))
            .unwrap_or_else(|| "default".to_string());

        // Estrai le sorgenti dei task
        let mut task_sources = Vec::new();
        if let Some(sources) = conf.get_array("sources", "task_sources") {
            for source in sources {
                if let Some(url) = source.as_string() {
                    task_sources.push(url.clone());
                }
            }
        }

        // Estrai le sorgenti degli stack
        let mut stack_sources = Vec::new();
        if let Some(sources) = conf.get_array("sources", "stack_sources") {
            for source in sources {
                if let Some(url) = source.as_string() {
                    stack_sources.push(url.clone());
                }
            }
        }

        // Creazione della configurazione
        let config = Config {
            tasks_dir,
            stacks_dir,
            state_dir,
            download_timeout,
            ui_theme,
            task_sources,
            stack_sources,
        };

        info!("Configuration loaded successfully");
        Ok(config)
    }

    /// Salva la configurazione in un file
    pub fn save(&self, path: &str) -> Result<()> {
        info!("Saving configuration to: {}", path);

        let mut conf = ConfuciusConfig::new("galatea");

        // Configurazioni generali
        conf.set("general", "tasks_dir", ConfigValue::String(self.tasks_dir.clone()));
        conf.set("general", "stacks_dir", ConfigValue::String(self.stacks_dir.clone()));
        conf.set("general", "state_dir", ConfigValue::String(self.state_dir.clone()));
        conf.set("general", "download_timeout", ConfigValue::Integer(self.download_timeout as i64));
        conf.set("general", "ui_theme", ConfigValue::String(self.ui_theme.clone()));

        // Sorgenti dei task
        let task_sources: Vec<ConfigValue> = self.task_sources.iter()
            .map(|url| ConfigValue::String(url.clone()))
            .collect();
        conf.set("sources", "task_sources", ConfigValue::Array(task_sources));

        // Sorgenti degli stack
        let stack_sources: Vec<ConfigValue> = self.stack_sources.iter()
            .map(|url| ConfigValue::String(url.clone()))
            .collect();
        conf.set("sources", "stack_sources", ConfigValue::Array(stack_sources));

        // Salva la configurazione
        conf.save_to_file(Path::new(path))
            .context(format!("Failed to save configuration to {}", path))?;

        info!("Configuration saved successfully");
        Ok(())
    }

    /// Risolve un percorso relativo alle directory di configurazione
    pub fn resolve_path(&self, path: &str, base_dir: &str) -> PathBuf {
        let base = match base_dir {
            "tasks" => Path::new(&self.tasks_dir),
            "stacks" => Path::new(&self.stacks_dir),
            "state" => Path::new(&self.state_dir),
            _ => Path::new(base_dir),
        };

        base.join(path)
    }

    /// Aggiunge una nuova sorgente di task
    pub fn add_task_source(&mut self, url: &str) -> bool {
        if !self.task_sources.contains(&url.to_string()) {
            self.task_sources.push(url.to_string());
            true
        } else {
            false
        }
    }

    /// Aggiunge una nuova sorgente di stack
    pub fn add_stack_source(&mut self, url: &str) -> bool {
        if !self.stack_sources.contains(&url.to_string()) {
            self.stack_sources.push(url.to_string());
            true
        } else {
            false
        }
    }

    /// Rimuove una sorgente di task
    pub fn remove_task_source(&mut self, url: &str) -> bool {
        let len = self.task_sources.len();
        self.task_sources.retain(|u| u != url);
        self.task_sources.len() < len
    }

    /// Rimuove una sorgente di stack
    pub fn remove_stack_source(&mut self, url: &str) -> bool {
        let len = self.stack_sources.len();
        self.stack_sources.retain(|u| u != url);
        self.stack_sources.len() < len
    }
}