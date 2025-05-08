// File: src/ui/task_view.rs (refactorizzato)

//! Visualizzazione e gestione dei task nell'interfaccia utente
//!
//! Questo modulo fornisce la visualizzazione e l'interazione con i task.

use std::sync::{Arc, Mutex};
use anyhow::Result;

use cursive::Cursive;

use crate::config::Config;
use crate::task::Task;
use crate::ui::components::selection;
use crate::ui::components::selectable_view;

/// Crea la vista per la gestione dei task
pub fn create_task_view(siv: &mut Cursive, config: Arc<Mutex<Config>>, tasks: Arc<Mutex<Vec<Task>>>) -> Result<()> {
    // Inizializza la selezione condivisa
    let selection = selection::new_shared_selection::<Task>();
    
    // Crea la vista selezionabile per i task
    selectable_view::create_selectable_view(
        siv,
        config,
        tasks,
        selection,
        "Gestione Task",
        true, // I task possono essere modificati (installati/disinstallati)
    )
}
