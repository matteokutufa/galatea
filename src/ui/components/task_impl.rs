// File: src/ui/components/task_impl.rs

use crate::task::{Task, ScriptType};
use crate::config::Config;
use crate::ui::components::selection::SelectableItem;
use crate::ui::components::selectable_view::Executable;
use anyhow::Result;

/// Implementazione del trait SelectableItem per i Task
impl SelectableItem for Task {
    /// Restituisce un marcatore di stato per i task
    fn get_status_marker(&self) -> &'static str {
        if self.installed {
            "[✓]"
        } else {
            "[ ]"
        }
    }
    
    /// Formatta il task per la visualizzazione nella lista
    fn format_for_list(&self) -> String {
        let status = self.get_status_marker();
        let task_type = format!("[{}]", self.script_type.get_letter());
        
        format!("{} {} {} - {}", status, task_type, self.name, self.description)
    }
    
    /// Formatta i dettagli del task
    fn format_details(&self) -> String {
        let mut details = format!("Nome: {}\n", self.name);
        details.push_str(&format!("Tipo: {} ({})\n", self.script_type.to_str(), 
                                 self.script_type.get_letter()));
        details.push_str(&format!("Descrizione: {}\n", self.description));
        details.push_str(&format!("URL: {}\n", self.url));
        details.push_str(&format!("Stato: {}\n", 
                                 if self.installed { "Installato" } else { "Non installato" }));

        if !self.dependencies.is_empty() {
            details.push_str(&format!("Dipendenze: {}\n", self.dependencies.join(", ")));
        }

        if !self.tags.is_empty() {
            details.push_str(&format!("Tag: {}\n", self.tags.join(", ")));
        }

        details.push_str(&format!("Richiede riavvio: {}\n", 
                                 if self.requires_reboot { "Sì" } else { "No" }));

        if let Some(cmd) = &self.cleanup_command {
            details.push_str(&format!("Comando di pulizia: {}\n", cmd));
        }

        if let Some(path) = &self.local_path {
            details.push_str(&format!("Percorso locale: {:?}\n", path));
        }
        
        details
    }
    
    /// Verifica se il task può essere installato
    fn can_install(&self) -> bool {
        !self.installed
    }
    
    /// Verifica se il task può essere disinstallato
    fn can_uninstall(&self) -> bool {
        self.installed
    }
    
    /// Verifica se il task può essere resettato
    fn can_reset(&self) -> bool {
        self.installed
    }
    
    /// Verifica se il task può essere rimediato
    fn can_remediate(&self) -> bool {
        self.installed
    }
}

/// Implementazione del trait Executable per i Task
impl Executable<Task> for Task {
    /// Implementazione dell'installazione del task
    fn install(&mut self, config: &Config) -> Result<()> {
        self.install(config)
    }
    
    /// Implementazione della disinstallazione del task
    fn uninstall(&mut self, config: &Config) -> Result<()> {
        self.uninstall(config)
    }
    
    /// Implementazione del reset del task
    fn reset(&mut self, config: &Config) -> Result<()> {
        self.reset(config)
    }
    
    /// Implementazione della remediazione del task
    fn remediate(&mut self, config: &Config) -> Result<()> {
        self.remediate(config)
    }
}
