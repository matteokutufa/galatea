//! Gestione dei temi per l'interfaccia utente (TUI)
//!
//! Questo modulo fornisce la personalizzazione dell'aspetto dell'interfaccia utente.

use cursive::theme::{BorderStyle, Palette, PaletteColor, Theme, Color, BaseColor};
use cursive::theme;
use std::collections::HashMap;

/// Ottiene un tema di default per l'applicazione
pub fn default_theme() -> Theme {
    // Clona il tema di default di cursive
    let mut theme = cursive::theme::Theme::default();

    // Personalizza la palette di colori
    let mut palette = Palette::default();

    // Imposta i colori del testo e dello sfondo
    palette[PaletteColor::Background] = Color::TerminalDefault;
    palette[PaletteColor::View] = Color::TerminalDefault;
    palette[PaletteColor::Primary] = Color::Dark(BaseColor::White);

    // Personalizza il colore dei bordi
    palette[PaletteColor::TitlePrimary] = Color::Dark(BaseColor::Green);
    palette[PaletteColor::Secondary] = Color::Dark(BaseColor::Blue);
    palette[PaletteColor::Highlight] = Color::Dark(BaseColor::Green);
    palette[PaletteColor::HighlightInactive] = Color::Dark(BaseColor::Blue);

    // Imposta lo stile dei bordi
    theme.borders = BorderStyle::Simple;

    // Imposta la palette personalizzata
    theme.palette = palette;

    theme
}

/// Tema dark mode
pub fn dark_theme() -> Theme {
    let mut theme = cursive::theme::Theme::default();

    let mut palette = Palette::default();

    // Colori scuri per lo sfondo
    palette[PaletteColor::Background] = Color::Dark(BaseColor::Black);
    palette[PaletteColor::View] = Color::Dark(BaseColor::Black);
    palette[PaletteColor::Primary] = Color::Light(BaseColor::White);

    // Colori per i bordi e gli elementi evidenziati
    palette[PaletteColor::TitlePrimary] = Color::Light(BaseColor::Green);
    palette[PaletteColor::Secondary] = Color::Light(BaseColor::Blue);
    palette[PaletteColor::Highlight] = Color::Light(BaseColor::Green);
    palette[PaletteColor::HighlightInactive] = Color::Light(BaseColor::Blue);

    // Imposta lo stile dei bordi
    theme.borders = BorderStyle::Simple;

    // Imposta la palette personalizzata
    theme.palette = palette;

    theme
}

/// Tema high contrast
pub fn high_contrast_theme() -> Theme {
    let mut theme = cursive::theme::Theme::default();

    let mut palette = Palette::default();

    // Colori ad alto contrasto
    palette[PaletteColor::Background] = Color::Dark(BaseColor::Black);
    palette[PaletteColor::View] = Color::Dark(BaseColor::Black);
    palette[PaletteColor::Primary] = Color::Light(BaseColor::White);

    // Colori per i bordi e gli elementi evidenziati
    palette[PaletteColor::TitlePrimary] = Color::Light(BaseColor::Yellow);
    palette[PaletteColor::Secondary] = Color::Light(BaseColor::White);
    palette[PaletteColor::Highlight] = Color::Light(BaseColor::Yellow);
    palette[PaletteColor::HighlightInactive] = Color::Light(BaseColor::White);

    // Imposta lo stile dei bordi
    theme.borders = BorderStyle::Outset;

    // Imposta la palette personalizzata
    theme.palette = palette;

    theme
}

/// Ottiene un tema in base al nome
pub fn get_theme(name: &str) -> Theme {
    match name.to_lowercase().as_str() {
        "dark" => dark_theme(),
        "high_contrast" => high_contrast_theme(),
        _ => default_theme(),
    }
}

/// Ottiene la lista dei temi disponibili
pub fn get_available_themes() -> Vec<String> {
    vec![
        "default".to_string(),
        "dark".to_string(),
        "high_contrast".to_string(),
    ]
}