// File: src/ui/components/stack_impl.rs

use crate::stack::Stack;
use crate::task::Task;
use crate::config::Config;
use crate::ui::components::selection::SelectableItem;
use crate::ui::components::selectable_view::Executable;
use anyhow::Result;
use std::sync::{Arc, Mutex};

/// Implementazione del trait SelectableItem per gli Stack
impl SelectableItem for Stack {
    /// Restituisce un marcatore di stato per gli stack
    fn get_status_marker(&self) -> &'static str {
        if self.fully_installed {
            "[✓]"
        } else if self.partially_installed {
            "[!]"
        } else {
            "[ ]"
        }
    }
    
    /// Formatta lo stack per la visualizzazione nella lista
    fn format_for_list(&self) -> String {
        let status = self.get_status_marker();
        format!("{} {} - {}", status, self.name, self.description)
    }
    
    /// Formatta i dettagli dello stack
    fn format_details(&self) -> String {
        let mut details = format!("Nome: {}\n", self.name);
        details.push_str(&format!("Descrizione: {}\n", self.description));
        details.push_str(&format!("Stato: {}\n",
                                 if self.fully_installed {
                                     "Completamente installato"
                                 } else if self.partially_installed {
                                     "Parzialmente installato"
                                 } else {
                                     "Non installato"
                                 }));

        if !self.tags.is_empty() {
            details.push_str(&format!("Tag: {}\n", self.tags.join(", ")));
        }

        details.push_str(&format!("Richiede riavvio: {}\n", 
                                 if self.requires_reboot { "Sì" } else { "No" }));

        // Aggiungi l'elenco dei task inclusi
        details.push_str("\nTask inclusi:\n");
        for task_name in &self.task_names {
            details.push_str(&format!("  - {}\n", task_name));
        }
        
        details
    }
    
    /// Verifica se lo stack può essere installato
    fn can_install(&self) -> bool {
        !self.fully_installed
    }
    
    /// Verifica se lo stack può essere disinstallato
    fn can_uninstall(&self) -> bool {
        self.fully_installed || self.partially_installed
    }
    
    /// Verifica se lo stack può essere resettato
    fn can_reset(&self) -> bool {
        self.fully_installed || self.partially_installed
    }
    
    /// Verifica se lo stack può essere rimediato
    fn can_remediate(&self) -> bool {
        self.fully_installed || self.partially_installed
    }
}

// Implementazione per gli Stack richiede un riferimento ai Task
// Questa versione accetta tasks come parametro quando necessario
impl Stack {
    /// Implementazione dell'installazione che accetta tasks come parametro
    pub fn install_with_tasks(&mut self, config: &Config, tasks: &mut [Task]) -> Result<()> {
        self.install(config, tasks)
    }
    
    /// Implementazione della disinstallazione che accetta tasks come parametro
    pub fn uninstall_with_tasks(&mut self, config: &Config, tasks: &mut [Task]) -> Result<()> {
        self.uninstall(config, tasks)
    }
    
    /// Implementazione del reset che accetta tasks come parametro
    pub fn reset_with_tasks(&mut self, config: &Config, tasks: &mut [Task]) -> Result<()> {
        self.reset(config, tasks)
    }
    
    /// Implementazione della remediazione che accetta tasks come parametro
    pub fn remediate_with_tasks(&mut self, config: &Config, tasks: &mut [Task]) -> Result<()> {
        self.remediate(config, tasks)
    }
}

#[derive(Clone)]
/// Versione di Stack che include i tasks per poter implementare Executable
pub struct StackWithTasks {
    /// Lo stack originale
    pub stack: Stack,
    /// Riferimento ai tasks
    pub tasks: Arc<Mutex<Vec<Task>>>,
}

impl StackWithTasks {
    /// Crea un nuovo StackWithTasks
    pub fn new(stack: Stack, tasks: Arc<Mutex<Vec<Task>>>) -> Self {
        StackWithTasks { stack, tasks }
    }
}

/// Implementazione di Display per StackWithTasks
impl std::fmt::Display for StackWithTasks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.stack)
    }
}

/// Implementazione di SelectableItem per StackWithTasks (delega a Stack)
impl SelectableItem for StackWithTasks {
    fn get_status_marker(&self) -> &'static str {
        self.stack.get_status_marker()
    }
    
    fn format_for_list(&self) -> String {
        self.stack.format_for_list()
    }
    
    fn format_details(&self) -> String {
        let mut details = self.stack.format_details();
        
        // Aggiungiamo informazioni sui task installati con stato
        if let Ok(tasks_guard) = self.tasks.lock() {
            let task_details = format!("\nDettagli task:\n");
            details.push_str(&task_details);
            
            for task_name in &self.stack.task_names {
                if let Some(task) = tasks_guard.iter().find(|t| &t.name == task_name) {
                    let status = if task.installed { "[✓]" } else { "[ ]" };
                    details.push_str(&format!("  {} {}\n", status, task_name));
                } else {
                    details.push_str(&format!("  [?] {} (non trovato)\n", task_name));
                }
            }
        }
        
        details
    }
    
    fn can_install(&self) -> bool {
        self.stack.can_install()
    }
    
    fn can_uninstall(&self) -> bool {
        self.stack.can_uninstall()
    }
    
    fn can_reset(&self) -> bool {
        self.stack.can_reset()
    }
    
    fn can_remediate(&self) -> bool {
        self.stack.can_remediate()
    }
}

/// Implementazione del trait Executable per StackWithTasks
impl Executable<StackWithTasks> for StackWithTasks {
    /// Implementazione dell'installazione dello stack
    fn install(&mut self, config: &Config) -> Result<()> {
        let mut tasks_guard = self.tasks.lock().map_err(|_| anyhow::anyhow!("Failed to lock tasks"))?;
        self.stack.install_with_tasks(config, &mut tasks_guard)
    }
    
    /// Implementazione della disinstallazione dello stack
    fn uninstall(&mut self, config: &Config) -> Result<()> {
        let mut tasks_guard = self.tasks.lock().map_err(|_| anyhow::anyhow!("Failed to lock tasks"))?;
        self.stack.uninstall_with_tasks(config, &mut tasks_guard)
    }
    
    /// Implementazione del reset dello stack
    fn reset(&mut self, config: &Config) -> Result<()> {
        let mut tasks_guard = self.tasks.lock().map_err(|_| anyhow::anyhow!("Failed to lock tasks"))?;
        self.stack.reset_with_tasks(config, &mut tasks_guard)
    }
    
    /// Implementazione della remediazione dello stack
    fn remediate(&mut self, config: &Config) -> Result<()> {
        let mut tasks_guard = self.tasks.lock().map_err(|_| anyhow::anyhow!("Failed to lock tasks"))?;
        self.stack.remediate_with_tasks(config, &mut tasks_guard)
    }
}
