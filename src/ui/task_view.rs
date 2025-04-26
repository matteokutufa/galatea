//! Visualizzazione e gestione dei task nell'interfaccia utente
//!
//! Questo modulo fornisce la visualizzazione e l'interazione con i task.

use std::sync::{Arc, Mutex};

use anyhow::{Result, anyhow};

use cursive::Cursive;
use cursive::views::{Dialog, SelectView, TextView, LinearLayout, DummyView, Panel, TextContent, Button};
use cursive::view::Scrollable;
use cursive::traits::*;
use cursive::align::HAlign;

use crate::config::Config;
use crate::task::Task;

/// Crea la vista per la gestione dei task
pub fn create_task_view(siv: &mut Cursive, config: Arc<Mutex<Config>>, tasks: Arc<Mutex<Vec<Task>>>) -> Result<()> {
    let tasks_clone = Arc::clone(&tasks);

    // Ottiene i task dal mutex
    let tasks_guard = tasks.lock().map_err(|_| anyhow!("Failed to lock tasks mutex"))?;

    // Crea la vista per selezionare i task
    let mut select_view = SelectView::new()
        .h_align(HAlign::Left)
        .autojump();

    // Popola la vista con i task
    for (idx, task) in tasks_guard.iter().enumerate() {
        // Aggiungi il task alla vista
        let status = if task.installed { "[✓]" } else { "[ ]" };
        let task_type = format!("[{}]", task.script_type.get_letter());

        let task_line = format!("{} {} {} - {}",
                                status,
                                task_type,
                                task.name,
                                task.description
        );

        select_view.add_item(task_line, idx);
    }

    // Rilascia il lock prima di creare le closure
    drop(tasks_guard);

    // Descrizione dettagliata del task selezionato
    let task_detail = TextContent::new("Seleziona un task per vedere i dettagli");
    let task_detail_view = TextView::new_with_content(task_detail.clone())
        .scrollable();

    // Gestisci la selezione dei task
    let tasks_clone2 = Arc::clone(&tasks_clone);
    select_view.set_on_select(move |_siv, idx| {
        if let Ok(tasks_guard) = tasks_clone2.lock() {
            if let Some(task) = tasks_guard.get(*idx) {
                // Crea una descrizione dettagliata del task
                let mut details = format!("Nome: {}\n", task.name);
                details.push_str(&format!("Tipo: {} ({})\n", task.script_type.to_str(), task.script_type.get_letter()));
                details.push_str(&format!("Descrizione: {}\n", task.description));
                details.push_str(&format!("URL: {}\n", task.url));
                details.push_str(&format!("Stato: {}\n", if task.installed { "Installato" } else { "Non installato" }));

                if !task.dependencies.is_empty() {
                    details.push_str(&format!("Dipendenze: {}\n", task.dependencies.join(", ")));
                }

                if !task.tags.is_empty() {
                    details.push_str(&format!("Tag: {}\n", task.tags.join(", ")));
                }

                details.push_str(&format!("Richiede riavvio: {}\n", if task.requires_reboot { "Sì" } else { "No" }));

                if let Some(cmd) = &task.cleanup_command {
                    details.push_str(&format!("Comando di pulizia: {}\n", cmd));
                }

                if let Some(path) = &task.local_path {
                    details.push_str(&format!("Percorso locale: {:?}\n", path));
                }

                // Aggiorna la vista dei dettagli
                task_detail.set_content(details);
            }
        }
    });

    // Funzione per aggiornare la vista
    let update_task_view = {
        let tasks = Arc::clone(&tasks);
        let select_view_cb = siv.cb_sink().clone();

        move || {
            if let Ok(tasks_guard) = tasks.lock() {
                // Crea una copia dei dati necessari
                let task_data: Vec<(bool, String, String, String)> = tasks_guard.iter()
                    .map(|t| (t.installed, t.script_type.get_letter().to_string(), t.name.clone(), t.description.clone()))
                    .collect();

                // Invia i dati al callback
                select_view_cb.send(Box::new(move |s: &mut Cursive| {
                    s.call_on_name("task_list", |view: &mut SelectView<usize>| {
                        view.clear();

                        for (idx, (installed, type_letter, name, description)) in task_data.iter().enumerate() {
                            let status = if *installed { "[✓]" } else { "[ ]" };
                            let task_type = format!("[{}]", type_letter);

                            let task_line = format!("{} {} {} - {}", status, task_type, name, description);
                            view.add_item(task_line, idx);
                        }
                    });
                })).unwrap();
            }
        }
    };

    // Crea i bottoni per le azioni
    let config_clone = Arc::clone(&config);
    let tasks_clone = Arc::clone(&tasks);
    let update_clone = update_task_view.clone();

    let install_button = {
        let tasks = Arc::clone(&tasks_clone);
        let config = Arc::clone(&config_clone);
        let update_fn = update_clone.clone();

        Button::new("Install", move |s| {
            let idx = match s.call_on_name("task_list", |view: &mut SelectView<usize>| view.selected_id()) {
                Some(Some(idx)) => idx,
                _ => return,
            };

            // Ottieni il task selezionato
            if let Ok(mut tasks_guard) = tasks.lock() {
                let task = match tasks_guard.get_mut(idx) {
                    Some(task) => task,
                    None => return,
                };

                // Verifica che il task non sia già installato
                if task.installed {
                    s.add_layer(Dialog::info("Il task è già installato"));
                    return;
                }

                // Installa il task
                if let Ok(config_guard) = config.lock() {
                    match task.install(&config_guard) {
                        Ok(_) => {
                            s.add_layer(Dialog::info(format!("Task {} installato con successo", task.name)));
                            update_fn();
                        },
                        Err(e) => {
                            s.add_layer(Dialog::info(format!("Errore durante l'installazione del task: {}", e)));
                        }
                    }
                }
            }
        })
    };

    let uninstall_button = {
        let tasks = Arc::clone(&tasks_clone);
        let config = Arc::clone(&config_clone);
        let update_fn = update_clone.clone();

        Button::new("Uninstall", move |s| {
            let idx = match s.call_on_name("task_list", |view: &mut SelectView<usize>| view.selected_id()) {
                Some(Some(idx)) => idx,
                _ => return,
            };

            // Ottieni il task selezionato
            if let Ok(mut tasks_guard) = tasks.lock() {
                let task = match tasks_guard.get_mut(idx) {
                    Some(task) => task,
                    None => return,
                };

                // Verifica che il task sia installato
                if !task.installed {
                    s.add_layer(Dialog::info("Il task non è installato"));
                    return;
                }

                // Disinstalla il task
                if let Ok(config_guard) = config.lock() {
                    match task.uninstall(&config_guard) {
                        Ok(_) => {
                            s.add_layer(Dialog::info(format!("Task {} disinstallato con successo", task.name)));
                            update_fn();
                        },
                        Err(e) => {
                            s.add_layer(Dialog::info(format!("Errore durante la disinstallazione del task: {}", e)));
                        }
                    }
                }
            }
        })
    };

    let reset_button = {
        let tasks = Arc::clone(&tasks_clone);
        let config = Arc::clone(&config_clone);

        Button::new("Reset", move |s| {
            let idx = match s.call_on_name("task_list", |view: &mut SelectView<usize>| view.selected_id()) {
                Some(Some(idx)) => idx,
                _ => return,
            };

            // Ottieni il task selezionato
            if let Ok(mut tasks_guard) = tasks.lock() {
                let task = match tasks_guard.get_mut(idx) {
                    Some(task) => task,
                    None => return,
                };

                // Verifica che il task sia installato
                if !task.installed {
                    s.add_layer(Dialog::info("Il task non è installato"));
                    return;
                }

                // Reset del task
                if let Ok(config_guard) = config.lock() {
                    match task.reset(&config_guard) {
                        Ok(_) => {
                            s.add_layer(Dialog::info(format!("Reset del task {} completato con successo", task.name)));
                        },
                        Err(e) => {
                            s.add_layer(Dialog::info(format!("Errore durante il reset del task: {}", e)));
                        }
                    }
                }
            }
        })
    };

    let remediate_button = {
        let tasks = Arc::clone(&tasks_clone);
        let config = Arc::clone(&config_clone);

        Button::new("Remediate", move |s| {
            let idx = match s.call_on_name("task_list", |view: &mut SelectView<usize>| view.selected_id()) {
                Some(Some(idx)) => idx,
                _ => return,
            };

            // Ottieni il task selezionato
            if let Ok(mut tasks_guard) = tasks.lock() {
                let task = match tasks_guard.get_mut(idx) {
                    Some(task) => task,
                    None => return,
                };

                // Verifica che il task sia installato
                if !task.installed {
                    s.add_layer(Dialog::info("Il task non è installato"));
                    return;
                }

                // Riavvia i servizi del task
                if let Ok(config_guard) = config.lock() {
                    match task.remediate(&config_guard) {
                        Ok(_) => {
                            s.add_layer(Dialog::info(format!("Remediation del task {} completata con successo", task.name)));
                        },
                        Err(e) => {
                            s.add_layer(Dialog::info(format!("Errore durante la remediation del task: {}", e)));
                        }
                    }
                }
            }
        })
    };

    // Crea la barra dei pulsanti
    let buttons = LinearLayout::horizontal()
        .child(install_button)
        .child(DummyView.fixed_width(1))
        .child(uninstall_button)
        .child(DummyView.fixed_width(1))
        .child(reset_button)
        .child(DummyView.fixed_width(1))
        .child(remediate_button);

    // Layout principale
    let layout = LinearLayout::vertical()
        .child(Panel::new(select_view.with_name("task_list").scrollable())
            .title("Task disponibili"))
        .child(DummyView.fixed_height(1))
        .child(Panel::new(task_detail_view)
            .title("Dettagli del task"))
        .child(DummyView.fixed_height(1))
        .child(buttons);

    // Aggiungi la vista alla UI
    siv.add_layer(Dialog::around(layout)
        .title("Gestione Task")
        .button("Back", |s| {
            s.pop_layer();
        }));

    Ok(())
}