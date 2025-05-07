//! Gestione della configurazione per Galatea
//!
//! Questo modulo gestisce il caricamento e il salvataggio della configurazione dell'applicazione
//! utilizzando YAML.

use std::path::{Path, PathBuf};
use std::fs;
use anyhow::{Context, Result};
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

    /// Verifica se ci sono sorgenti configurate per task o stack
    pub fn has_sources(&self) -> bool {
        !self.task_sources.is_empty() || !self.stack_sources.is_empty()
    }

    /// Carica la configurazione da un file
    pub fn load(path: Option<&str>) -> Result<Self> {
        // Definisci i percorsi possibili da cui caricare la configurazione
        let config_paths = if let Some(explicit_path) = path {
            // Se è stato specificato un percorso, usa solo quello
            vec![PathBuf::from(explicit_path)]
        } else {
            // Altrimenti, cerca nei percorsi predefiniti
            vec![
                get_system_config_path(),  // /etc/galatea/galatea.yaml
                get_binary_config_path(),  // ./galatea.yaml
            ]
        };

        // Prova a caricare da ogni percorso nell'ordine specificato
        let mut config_loaded = false;
        let mut config = Config::default();
        let mut config_file_path = None;

        for config_path in config_paths {
            if config_path.exists() {
                info!("Tentativo di caricamento della configurazione da: {:?}", config_path);
                match fs::read_to_string(&config_path) {
                    Ok(yaml_content) => {
                        match serde_yaml::from_str::<Config>(&yaml_content) {
                            Ok(loaded_config) => {
                                config = loaded_config;
                                info!("Configurazione caricata da: {:?}", &config_path);
                                config_file_path = Some(config_path);
                                config_loaded = true;
                                break;
                            },
                            Err(e) => {
                                warn!("Errore nel parsing della configurazione YAML da {:?}: {}", config_path, e);
                            }
                        }
                    },
                    Err(e) => {
                        warn!("Impossibile leggere il file di configurazione {:?}: {}", config_path, e);
                    }
                }
            }
        }

        // Se la configurazione non è stata trovata, crea e salva una configurazione di default
        if !config_loaded {
            let default_config = Config::default();
            
            // Determina dove salvare la configurazione di default
            let default_config_path = get_binary_config_path();
            
            if let Err(e) = default_config.save(&default_config_path) {
                warn!("Impossibile salvare la configurazione di default in {:?}: {}", default_config_path, e);
                // Continuiamo comunque con la configurazione in memoria
            } else {
                info!("Creata configurazione di default in: {:?}", default_config_path);
                config_file_path = Some(default_config_path);
            }
            
            config = default_config;
        }

        // Imposta il percorso del file di configurazione
        config.config_file_path = config_file_path;

        // Crea le directory se non esistono
        create_directories(&config)?;

        Ok(config)
    }

    /// Salva la configurazione in un file
    pub fn save(&self, path: &PathBuf) -> Result<()> {
        // Assicurati che la directory esista
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .context(format!("Impossibile creare la directory per: {:?}", path))?;
            }
        }

        // Serializza la configurazione in YAML
        let yaml_content = serde_yaml::to_string(self)
            .context("Impossibile serializzare la configurazione in YAML")?;

        // Salva la configurazione
        fs::write(path, yaml_content)
            .context(format!("Impossibile salvare la configurazione in: {:?}", path))?;

        info!("Configurazione salvata in: {:?}", path);
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

/// Ottiene il percorso di configurazione nella directory dell'eseguibile
pub fn get_binary_config_path() -> PathBuf {
    get_base_directory().join("galatea.yaml")
}

/// Ottiene il percorso di configurazione di sistema
pub fn get_system_config_path() -> PathBuf {
    PathBuf::from("/etc/galatea/galatea.yaml")
}

/// Crea un file di configurazione di esempio nella directory specificata
pub fn create_example_config(path: &Path) -> Result<()> {
    // Assicurati che la directory esista
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            info!("Creazione directory: {:?}", parent);
            fs::create_dir_all(parent)
                .context(format!("Impossibile creare la directory per: {:?}", parent))?;
        }
    }

    let default_config = Config::default();

    // Aggiungi alcuni valori di esempio
    let mut config = default_config.clone();
    config.add_task_source("https://example.com/tasks/security.zip");
    config.add_task_source("https://example.com/tasks/monitoring.zip");
    config.add_stack_source("https://example.com/stacks/web_server.zip");

    // Serializza la configurazione in YAML
    let yaml_content = serde_yaml::to_string(&config)
        .context("Impossibile serializzare la configurazione di esempio in YAML")?;

    // Salva la configurazione di esempio
    fs::write(path, yaml_content)
        .context(format!("Impossibile salvare la configurazione di esempio in: {:?}", path))?;

    info!("Configurazione di esempio creata in: {:?}", path);
    Ok(())
}