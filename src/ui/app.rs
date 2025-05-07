//! Applicazione TUI principale
//!
//! Questo modulo gestisce l'interfaccia utente testuale principale dell'applicazione.

use std::sync::{Arc, Mutex};
use std::path::PathBuf;

use anyhow::{Result, anyhow};

use cursive::Cursive;
use cursive::views::{Dialog, TextView, LinearLayout, SelectView, DummyView, Panel, EditView};
use cursive::view::Scrollable;
use cursive::traits::*;
use cursive::align::HAlign;

use crate::config::{Config, get_binary_config_path};
use crate::task::{Task, load_tasks, ScriptType};
use crate::stack::{Stack, load_stacks};
use crate::ui::theme;
use crate::ui::task_view;
use crate::ui::stack_view;

// In `ui/app.rs`
pub struct App;

/// Avvia l'applicazione TUI
pub fn run_app(config: Config) -> Result<()> {
    // Crea l'oggetto Cursive per la TUI
    let mut siv = cursive::default();

    // Imposta il tema
    let theme = theme::get_theme(&config.ui_theme);
    siv.set_theme(theme);

    // Carica i task e gli stack
    let tasks = load_tasks(&config)?;
    let stacks = load_stacks(&config, &tasks)?;

    // Condividi i dati tra i thread
    let config = Arc::new(Mutex::new(config));
    let tasks = Arc::new(Mutex::new(tasks));
    let stacks = Arc::new(Mutex::new(stacks));

    // Crea la schermata principale
    create_main_screen(&mut siv, Arc::clone(&config), Arc::clone(&tasks), Arc::clone(&stacks))?;

    // Esegui il loop principale
    siv.run();

    Ok(())
}

/// Crea la schermata principale dell'applicazione
fn create_main_screen(siv: &mut Cursive, config: Arc<Mutex<Config>>, tasks: Arc<Mutex<Vec<Task>>>, stacks: Arc<Mutex<Vec<Stack>>>) -> Result<()> {
    // Mostra il titolo dell'applicazione
    let title = TextView::new("GALATEA")
        .h_align(HAlign::Center)
        .with_name("title");

    // Mostra una descrizione
    let description = TextView::new("Strumento di installazione e configurazione server e workstation")
        .h_align(HAlign::Center)
        .with_name("description");

    // Ottieni statistiche
    let stats = get_statistics(&tasks, &stacks)?;
    let stats_view = TextView::new(stats)
        .with_name("stats");

    // Crea il menu principale
    let mut main_menu = SelectView::new()
        .h_align(HAlign::Center)
        .autojump();

    // Aggiungi le voci di menu
    main_menu.add_item("Gestione Task", "tasks");
    main_menu.add_item("Gestione Stack", "stacks");
    main_menu.add_item("Impostazioni", "settings");
    main_menu.add_item("Informazioni", "about");
    main_menu.add_item("Esci", "quit");

    // Gestisci la selezione del menu
    let config_clone = Arc::clone(&config);
    let tasks_clone = Arc::clone(&tasks);
    let stacks_clone = Arc::clone(&stacks);

    main_menu.set_on_submit(move |s, item: &str| {
        match item {
            "tasks" => {
                let result = task_view::create_task_view(s, Arc::clone(&config_clone), Arc::clone(&tasks_clone));
                if let Err(e) = result {
                    s.add_layer(Dialog::info(format!("Errore durante il caricamento della vista dei task: {}", e)));
                }
            },
            "stacks" => {
                let result = stack_view::create_stack_view(s, Arc::clone(&config_clone), Arc::clone(&stacks_clone), Arc::clone(&tasks_clone));
                if let Err(e) = result {
                    s.add_layer(Dialog::info(format!("Errore durante il caricamento della vista degli stack: {}", e)));
                }
            },
            "settings" => {
                create_settings_screen(s, Arc::clone(&config_clone));
            },
            "about" => {
                s.add_layer(Dialog::info(
                    "Galatea v0.1.0\n\n\
                    Strumento di installazione e configurazione server e workstation\n\n\
                    Basato su Rust con interfaccia TUI gestita da cursive."
                ).title("Informazioni"));
            },
            "quit" => {
                s.add_layer(Dialog::around(TextView::new("Sei sicuro di voler uscire?"))
                    .title("Conferma uscita")
                    .button("No", |s| { s.pop_layer(); })
                    .button("Sì", |s| s.quit()));
            },
            _ => s.add_layer(Dialog::info(format!("Opzione non implementata: {}", item))),
        }
    });

    // Layout principale
    let layout = LinearLayout::vertical()
        .child(title)
        .child(DummyView.fixed_height(1))
        .child(description)
        .child(DummyView.fixed_height(1))
        .child(Panel::new(stats_view)
            .title("Statistiche"))
        .child(DummyView.fixed_height(1))
        .child(Panel::new(main_menu.scrollable())
            .title("Menu principale"));

    // Aggiungi la vista alla UI
    siv.add_layer(Dialog::around(layout)
        .title("Galatea")
        .button("Quit", |s| {
            s.add_layer(Dialog::around(TextView::new("Sei sicuro di voler uscire?"))
                .title("Conferma uscita")
                .button("No", |s| { s.pop_layer(); })
                .button("Sì", |s| s.quit()));
        }));

    Ok(())
}

/// Crea la schermata delle impostazioni
fn create_settings_screen(siv: &mut Cursive, config: Arc<Mutex<Config>>) {
    // Ottieni la configurazione attuale
    let config_guard = config.lock().unwrap();

    // Crea una vista per la configurazione
    let mut content = String::new();

    content.push_str(&format!("Directory task: {}\n", config_guard.tasks_dir));
    content.push_str(&format!("Directory stack: {}\n", config_guard.stacks_dir));
    content.push_str(&format!("Directory stato: {}\n", config_guard.state_dir));
    content.push_str(&format!("Timeout download: {} sec\n", config_guard.download_timeout));
    content.push_str(&format!("Tema UI: {}\n", config_guard.ui_theme));
    content.push_str("\nSorgenti Task:\n");

    if config_guard.task_sources.is_empty() {
        content.push_str("  Nessuna sorgente di task configurata\n");
    } else {
        for (i, url) in config_guard.task_sources.iter().enumerate() {
            content.push_str(&format!("  {}. {}\n", i + 1, url));
        }
    }

    content.push_str("\nSorgenti Stack:\n");
    if config_guard.stack_sources.is_empty() {
        content.push_str("  Nessuna sorgente di stack configurata\n");
    } else {
        for (i, url) in config_guard.stack_sources.iter().enumerate() {
            content.push_str(&format!("  {}. {}\n", i + 1, url));
        }
    }

    // Lista dei temi disponibili
    content.push_str("\nTemi disponibili:\n");
    for theme_name in theme::get_available_themes() {
        content.push_str(&format!("  - {}\n", theme_name));
    }

    // Informazioni sulla configurazione
    if let Some(config_path) = &config_guard.config_file_path {
        content.push_str(&format!("\nFile di configurazione: {:?}\n", config_path));
    } else {
        content.push_str("\nFile di configurazione: usando valori predefiniti\n");
    }

    // Rilascia il lock prima di procedere
    drop(config_guard);

    // Aggiungi la vista alla UI
    siv.add_layer(Dialog::around(TextView::new(content).scrollable())
        .title("Impostazioni")
        .button("Cambia tema", {
            let config = Arc::clone(&config);
            move |s| {
                // Crea una vista per selezionare il tema
                let mut theme_select = SelectView::new();

                // Aggiungi i temi disponibili
                for theme_name in theme::get_available_themes() {
                    theme_select.add_item(theme_name.clone(), theme_name);
                }

                // Gestisci la selezione del tema
                let config_clone = Arc::clone(&config);
                theme_select.set_on_submit(move |s, theme_name: &str| {
                    // Aggiorna la configurazione
                    {
                        let mut config_guard = config_clone.lock().unwrap();
                        config_guard.ui_theme = theme_name.to_string();

                        // Salva la configurazione aggiornata
                        if let Some(config_path) = &config_guard.config_file_path {
                            match config_guard.save(config_path) {
                                Ok(_) => {},
                                Err(e) => {
                                    s.add_layer(Dialog::info(format!("Errore nel salvataggio della configurazione: {}", e)));
                                    return;
                                }
                            }
                        }
                    }

                    // Imposta il nuovo tema
                    let new_theme = theme::get_theme(theme_name);
                    s.set_theme(new_theme);

                    // Notifica l'utente
                    s.add_layer(Dialog::info(format!("Tema cambiato a: {}", theme_name)));
                    s.pop_layer();
                });

                // Mostra la vista di selezione del tema
                s.add_layer(Dialog::around(theme_select.scrollable())
                    .title("Seleziona tema")
                    .button("Cancel", |s| { s.pop_layer(); }));
            }
        })
        .button("Aggiungi sorgente Task", {
            let config = Arc::clone(&config);
            move |s| {
                s.add_layer(Dialog::around(
                    LinearLayout::vertical()
                        .child(TextView::new("Inserisci l'URL della sorgente:"))
                        .child(DummyView.fixed_height(1))
                        .child(EditView::new()
                            .with_name("url_input")
                            .fixed_width(50))
                ).title("Aggiungi sorgente Task")
                    .button("Cancel", |s| { s.pop_layer(); })
                    .button("OK", {
                        let config = Arc::clone(&config);
                        move |s| {
                            let url = s.call_on_name("url_input", |view: &mut EditView| {
                                view.get_content()
                            }).unwrap().to_string();

                            if url.is_empty() {
                                s.add_layer(Dialog::info("L'URL non può essere vuoto"));
                                return;
                            }

                            // Aggiungi la sorgente e salva la configurazione
                            {
                                let mut config_guard = config.lock().unwrap();
                                if config_guard.add_task_source(&url) {
                                    // Salva la configurazione aggiornata
                                    if let Some(config_path) = &config_guard.config_file_path {
                                        match config_guard.save(config_path) {
                                            Ok(_) => {
                                                s.pop_layer();
                                                s.add_layer(Dialog::info(format!("Sorgente Task aggiunta: {}", url)));
                                            },
                                            Err(e) => {
                                                s.add_layer(Dialog::info(format!("Errore nel salvataggio della configurazione: {}", e)));
                                            }
                                        }
                                    } else {
                                        s.pop_layer();
                                        s.add_layer(Dialog::info(format!("Sorgente Task aggiunta: {}", url)));
                                    }
                                } else {
                                    s.add_layer(Dialog::info(format!("La sorgente {} esiste già", url)));
                                }
                            }
                        }
                    }));
            }
        })
        .button("Aggiungi sorgente Stack", {
            let config = Arc::clone(&config);
            move |s| {
                s.add_layer(Dialog::around(
                    LinearLayout::vertical()
                        .child(TextView::new("Inserisci l'URL della sorgente:"))
                        .child(DummyView.fixed_height(1))
                        .child(EditView::new()
                            .with_name("url_input")
                            .fixed_width(50))
                ).title("Aggiungi sorgente Stack")
                    .button("Cancel", |s| { s.pop_layer(); })
                    .button("OK", {
                        let config = Arc::clone(&config);
                        move |s| {
                            let url = s.call_on_name("url_input", |view: &mut EditView| {
                                view.get_content()
                            }).unwrap().to_string();

                            if url.is_empty() {
                                s.add_layer(Dialog::info("L'URL non può essere vuoto"));
                                return;
                            }

                            // Aggiungi la sorgente e salva la configurazione
                            {
                                let mut config_guard = config.lock().unwrap();
                                if config_guard.add_stack_source(&url) {
                                    // Salva la configurazione aggiornata
                                    if let Some(config_path) = &config_guard.config_file_path {
                                        match config_guard.save(config_path) {
                                            Ok(_) => {
                                                s.pop_layer();
                                                s.add_layer(Dialog::info(format!("Sorgente Stack aggiunta: {}", url)));
                                            },
                                            Err(e) => {
                                                s.add_layer(Dialog::info(format!("Errore nel salvataggio della configurazione: {}", e)));
                                            }
                                        }
                                    } else {
                                        s.pop_layer();
                                        s.add_layer(Dialog::info(format!("Sorgente Stack aggiunta: {}", url)));
                                    }
                                } else {
                                    s.add_layer(Dialog::info(format!("La sorgente {} esiste già", url)));
                                }
                            }
                        }
                    }));
            }
        })
        .button("Salva configurazione", {
            let config = Arc::clone(&config);
            move |s| {
                // Pre-popola con il percorso attuale
                let initial_path = {
                    let config_guard = config.lock().unwrap();
                    config_guard.config_file_path
                        .as_ref()
                        .map_or_else(
                            || get_binary_config_path().to_string_lossy().to_string(),
                            |p| p.to_string_lossy().to_string()
                        )
                };

                // Crea un EditView con il contenuto iniziale
                let edit_view = EditView::new()
                    .content(initial_path)
                    .with_name("path_input")
                    .fixed_width(50);

                s.add_layer(Dialog::around(
                    LinearLayout::vertical()
                        .child(TextView::new("Inserisci il percorso del file di configurazione:"))
                        .child(DummyView.fixed_height(1))
                        .child(edit_view)
                ).title("Salva configurazione")
                    .button("Cancel", |s| { s.pop_layer(); })
                    .button("OK", {
                        let config = Arc::clone(&config);
                        move |s| {
                            let path = s.call_on_name("path_input", |view: &mut EditView| {
                                view.get_content()
                            }).unwrap().to_string();

                            if path.is_empty() {
                                s.add_layer(Dialog::info("Il percorso non può essere vuoto"));
                                return;
                            }

                            // Salva la configurazione
                            {
                                let mut config_guard = config.lock().unwrap();
                                match config_guard.save(&PathBuf::from(&path)) {
                                    Ok(_) => {
                                        // Aggiorna il percorso nella configurazione
                                        config_guard.config_file_path = Some(PathBuf::from(&path));
                                        s.pop_layer();
                                        s.add_layer(Dialog::info(format!("Configurazione salvata in: {}", path)));
                                    },
                                    Err(e) => {
                                        s.add_layer(Dialog::info(format!("Errore nel salvataggio della configurazione: {}", e)));
                                    }
                                }
                            }
                        }
                    }));
            }
        })
        .button("Back", |s| { s.pop_layer(); }));
}

/// Ottiene le statistiche sui task e gli stack
fn get_statistics(tasks: &Arc<Mutex<Vec<Task>>>, stacks: &Arc<Mutex<Vec<Stack>>>) -> Result<String> {
    // Ottieni i lock sui mutex
    let tasks_guard = tasks.lock().map_err(|_| anyhow!("Failed to lock tasks mutex"))?;
    let stacks_guard = stacks.lock().map_err(|_| anyhow!("Failed to lock stacks mutex"))?;

    // Calcola le statistiche
    let total_tasks = tasks_guard.len();
    let installed_tasks = tasks_guard.iter().filter(|t| t.installed).count();

    let total_stacks = stacks_guard.len();
    let fully_installed_stacks = stacks_guard.iter().filter(|s| s.fully_installed).count();
    let partially_installed_stacks = stacks_guard.iter().filter(|s| s.partially_installed).count();

    // Task per tipo
    let bash_tasks = tasks_guard.iter().filter(|t| t.script_type == ScriptType::Bash).count();
    let ansible_tasks = tasks_guard.iter().filter(|t| t.script_type == ScriptType::Ansible).count();
    let mixed_tasks = tasks_guard.iter().filter(|t| t.script_type == ScriptType::Mixed).count();

    // Formatta le statistiche
    let mut stats = String::new();

    stats.push_str(&format!("Task totali: {} (installati: {})\n", total_tasks, installed_tasks));
    stats.push_str(&format!("Stack totali: {} (installati: {}, parziali: {})\n",
                            total_stacks, fully_installed_stacks, partially_installed_stacks));
    stats.push_str(&format!("Task per tipo: Bash: {}, Ansible: {}, Misti: {}\n",
                            bash_tasks, ansible_tasks, mixed_tasks));

    // Aggiungi informazioni sul sistema
    stats.push_str(&format!("Sistema operativo: {}\n", crate::utils::get_os_name()));
    stats.push_str(&format!("Eseguito come root: {}\n", if crate::utils::is_running_as_root() { "Sì" } else { "No" }));
    stats.push_str(&format!("Ansible disponibile: {}\n",
                            if crate::executor::is_ansible_available() { "Sì" } else { "No" }));

    Ok(stats)
}