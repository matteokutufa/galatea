//! Gestione della configurazione per Galatea
//!
//! Questo modulo gestisce il caricamento e il salvataggio della configurazione dell'applicazione
//! utilizzando la libreria confucius.

use std::path::{Path, PathBuf};
use std::fs;
use anyhow::{Context, Result};
use confucius::{Config as ConfuciusConfig, ConfigValue};
use serde::{Serialize, Deserialize};
use log::{info, warn};

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

    /// Percorso del file di configurazione caricato
    #[serde(skip)]
    pub config_file_path: Option<PathBuf>,
}

impl Config {
    /// Crea una nuova configurazione con valori di default relativi alla directory dell'eseguibile
    pub fn default() -> Self {
        let base_dir = get_base_directory();

        Config {
            tasks_dir: base_dir.join("tasks").to_string_lossy().to_string(),
            stacks_dir: base_dir.join("stacks").to_string_lossy().to_string(),
            state_dir: base_dir.join("state").to_string_lossy().to_string(),
            download_timeout: 60,
            ui_theme: "default".to_string(),
            task_sources: Vec::new(),
            stack_sources: Vec::new(),
            config_file_path: None,
        }
    }

    /// Carica la configurazione da un file
    pub fn load(path: Option<&str>) -> Result<Self> {
        let mut conf = ConfuciusConfig::new("galatea");
        let mut config_loaded = false;
        let mut config_file_path = None;

        // Se è specificato un path esplicito, prova a caricare da lì
        if let Some(explicit_path) = path {
            info!("Tentativo di caricamento configurazione da: {}", explicit_path);
            if let Ok(_) = conf.load_from_file(Path::new(explicit_path)) {
                info!("Configurazione caricata con successo da: {}", explicit_path);
                config_loaded = true;
                config_file_path = Some(PathBuf::from(explicit_path));
            } else {
                warn!("Impossibile caricare la configurazione da: {}", explicit_path);
            }
        }

        // Se non è stato caricato da un path esplicito, usa la funzionalità di auto-ricerca di confucius
        if !config_loaded {
            info!("Ricerca automatica del file di configurazione");
            match conf.load() {
                Ok(_) => {
                    info!("Configurazione caricata con successo dai percorsi standard");
                    config_loaded = true;
                },
                Err(e) => {
                    warn!("Impossibile trovare o caricare la configurazione dai percorsi standard: {}", e);
                }
            }
        }

        // Se la configurazione non è stata trovata, crea e salva una configurazione di default
        if !config_loaded {
            info!("Creazione di una configurazione di default");
            let mut default_config = Config::default();

            // Crea e salva la configurazione di default
            let default_config_path = get_default_config_path();
            if let Some(parent) = default_config_path.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)
                        .context("Impossibile creare la directory per la configurazione di default")?;
                }
            }

            default_config.save(&default_config_path.to_string_lossy())?;
            info!("Configurazione di default salvata in: {:?}", default_config_path);

            default_config.config_file_path = Some(default_config_path);
            return Ok(default_config);
        }

        // Estrai i valori dalla configurazione caricata
        let tasks_dir = conf.get_string("general", "tasks_dir", None)
            .unwrap_or_else(|| {
                let base_dir = get_base_directory();
                base_dir.join("tasks").to_string_lossy().to_string()
            });

        let stacks_dir = conf.get_string("general", "stacks_dir", None)
            .unwrap_or_else(|| {
                let base_dir = get_base_directory();
                base_dir.join("stacks").to_string_lossy().to_string()
            });

        let state_dir = conf.get_string("general", "state_dir", None)
            .unwrap_or_else(|| {
                let base_dir = get_base_directory();
                base_dir.join("state").to_string_lossy().to_string()
            });

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
            config_file_path,
        };

        // Crea le directory se non esistono
        create_directories(&config)?;

        info!("Configurazione caricata con successo");
        Ok(config)
    }

    /// Salva la configurazione in un file
    pub fn save(&self, path: &str) -> Result<()> {
        info!("Salvataggio configurazione in: {}", path);

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

        // Assicurati che la directory esista
        if let Some(parent) = Path::new(path).parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .context(format!("Impossibile creare la directory per: {}", path))?;
            }
        }

        // Salva la configurazione
        conf.save_to_file(Path::new(path))
            .context(format!("Impossibile salvare la configurazione in: {}", path))?;

        info!("Configurazione salvata con successo");
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

/// Crea le directory necessarie basate sulla configurazione
fn create_directories(config: &Config) -> Result<()> {
    let dirs = [
        &config.tasks_dir,
        &config.stacks_dir,
        &config.state_dir,
    ];

    for dir in dirs.iter() {
        if !Path::new(dir).exists() {
            info!("Creazione directory: {}", dir);
            fs::create_dir_all(dir)
                .context(format!("Impossibile creare la directory: {}", dir))?;
        }
    }

    Ok(())
}

/// Ottiene la directory di base dell'applicazione
pub fn get_base_directory() -> PathBuf {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            return exe_dir.to_path_buf();
        }
    }

    // Fallback: utilizza la directory corrente
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Ottiene il percorso predefinito per il file di configurazione
pub fn get_default_config_path() -> PathBuf {
    get_base_directory().join("galatea.conf")
}

/// Crea un file di configurazione di esempio nella directory specificata
pub fn create_example_config(path: &Path) -> Result<()> {
    info!("Creazione configurazione di esempio in: {:?}", path);

    let default_config = Config::default();

    // Aggiungi alcuni valori di esempio
    let mut config = default_config.clone();
    config.add_task_source("https://example.com/tasks/security.zip");
    config.add_task_source("https://example.com/tasks/monitoring.zip");
    config.add_stack_source("https://example.com/stacks/web_server.zip");

    // Salva la configurazione di esempio
    config.save(&path.to_string_lossy())?;

    info!("File di configurazione di esempio creato con successo");
    Ok(())
}