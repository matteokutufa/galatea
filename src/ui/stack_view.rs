// File: src/ui/stack_view.rs (refactorizzato)

//! Visualizzazione e gestione degli stack nell'interfaccia utente
//!
//! Questo modulo fornisce la visualizzazione e l'interazione con gli stack.

use std::sync::{Arc, Mutex};
use anyhow::Result;

use cursive::Cursive;

use crate::config::Config;
use crate::task::Task;
use crate::stack::Stack;
use crate::ui::components::selection;
use crate::ui::components::selectable_view;
use crate::ui::components::stack_impl::StackWithTasks;

/// Crea la vista per la gestione degli stack
pub fn create_stack_view(siv: &mut Cursive, config: Arc<Mutex<Config>>, stacks: Arc<Mutex<Vec<Stack>>>, tasks: Arc<Mutex<Vec<Task>>>) -> Result<()> {
    // Crea StackWithTasks che contiene sia lo stack che i tasks necessari
    let stacks_with_tasks = {
        let stacks_guard = stacks.lock().map_err(|_| anyhow::anyhow!("Failed to lock stacks"))?;
        
        let stacks_vec: Vec<StackWithTasks> = stacks_guard.iter().cloned()
            .map(|stack| StackWithTasks::new(stack, Arc::clone(&tasks)))
            .collect();
        
        Arc::new(Mutex::new(stacks_vec))
    };
    
    // Inizializza la selezione condivisa
    let selection = selection::new_shared_selection::<StackWithTasks>();
    
    // Crea la vista selezionabile per gli stack
    selectable_view::create_selectable_view(
        siv,
        config,
        stacks_with_tasks,
        selection,
        "Gestione Stack",
        true, // Gli stack possono essere modificati
    )
}