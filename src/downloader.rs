//! Modulo per il download e l'estrazione dei file
//!
//! Questo modulo fornisce funzionalità per scaricare file da URL e
//! estrarre archivi nei formati supportati (zip, tar.gz, tgz).

use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::{copy};
use std::time::Duration;
use anyhow::{Context, Result, anyhow};
use log::{info, warn, debug};
use reqwest::blocking::Client;
use zip::ZipArchive;
use tar::Archive;
use flate2::read::GzDecoder;



/// Scarica un file da un URL in una directory specifica
///
/// # Arguments
///
/// * `url` - L'URL da cui scaricare il file
/// * `dir` - La directory di destinazione
/// * `timeout_secs` - Il timeout in secondi per la richiesta
///
/// # Returns
///
/// Il percorso del file scaricato
pub fn download_file(url: &str, dir: &Path, timeout_secs: u64) -> Result<PathBuf> {
    // Crea la directory se non esiste
    if !dir.exists() {
        fs::create_dir_all(dir).context("Failed to create download directory")?;
    }

    // Ottieni il nome del file dall'URL
    let filename = url.split('/').last()
        .ok_or_else(|| anyhow!("Invalid URL: {}", url))?;

    let file_path = dir.join(filename);

    // Crea un client HTTP con timeout
    let client = Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .context("Failed to create HTTP client")?;

    // Effettua la richiesta
    info!("Downloading {} to {:?}", url, file_path);
    let mut response = client.get(url)
        .send()
        .context(format!("Failed to download file from {}", url))?;

    // Verifica che la richiesta sia andata a buon fine
    if !response.status().is_success() {
        return Err(anyhow!("HTTP error: {}", response.status()));
    }

    // Crea il file di destinazione
    let mut file = File::create(&file_path)
        .context(format!("Failed to create file: {:?}", file_path))?;

    // Copia il contenuto della risposta nel file
    copy(&mut response, &mut file)
        .context("Failed to write file content")?;

    debug!("File downloaded to {:?}", file_path);

    Ok(file_path)
}

/// Scarica un file di configurazione da un URL
///
/// # Arguments
///
/// * `url` - L'URL da cui scaricare il file
/// * `dir` - La directory di destinazione
/// * `timeout_secs` - Il timeout in secondi per la richiesta
///
/// # Returns
///
/// Il percorso del file scaricato
pub fn download_config_file(url: &str, dir: &str, timeout_secs: u64) -> Result<PathBuf> {
    download_file(url, Path::new(dir), timeout_secs)
}

/// Estrae un archivio in una directory specificata
///
/// # Arguments
///
/// * `archive_path` - Il percorso dell'archivio
/// * `extract_dir` - La directory in cui estrarre l'archivio
///
/// # Returns
///
/// Il percorso della directory in cui è stato estratto l'archivio
pub fn extract_archive(archive_path: &Path, extract_dir: &Path) -> Result<PathBuf> {
    // Crea la directory di estrazione se non esiste
    if !extract_dir.exists() {
        fs::create_dir_all(extract_dir).context("Failed to create extraction directory")?;
    }

    let file_name = archive_path.file_name()
        .ok_or_else(|| anyhow!("Invalid archive path"))?
        .to_string_lossy();

    info!("Extracting {:?} to {:?}", archive_path, extract_dir);

    // Estrai in base al tipo di archivio
    if file_name.ends_with(".zip") {
        extract_zip(archive_path, extract_dir)?;
    } else if file_name.ends_with(".tar.gz") || file_name.ends_with(".tgz") {
        extract_tar_gz(archive_path, extract_dir)?;
    } else {
        // Non è un archivio supportato, copialo semplicemente
        let dest_path = extract_dir.join(file_name.to_string());
        fs::copy(archive_path, &dest_path)
            .context(format!("Failed to copy file to {:?}", dest_path))?;
    }

    Ok(extract_dir.to_path_buf())
}

/// Estrae un archivio ZIP
///
/// # Arguments
///
/// * `archive_path` - Il percorso dell'archivio ZIP
/// * `extract_dir` - La directory in cui estrarre l'archivio
///
/// # Returns
///
/// `Ok(())` in caso di successo, altrimenti un errore
fn extract_zip(archive_path: &Path, extract_dir: &Path) -> Result<()> {
    debug!("Extracting ZIP archive: {:?}", archive_path);

    let file = File::open(archive_path)
        .context(format!("Failed to open ZIP file: {:?}", archive_path))?;

    let mut archive = ZipArchive::new(file)
        .context(format!("Failed to parse ZIP file: {:?}", archive_path))?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)
            .context(format!("Failed to read file at index {} in ZIP", i))?;

        let file_path = file.enclosed_name()
            .ok_or_else(|| anyhow!("Invalid file path in ZIP"))?;

        let output_path = extract_dir.join(file_path);

        // Crea le directory necessarie
        if let Some(parent) = output_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .context(format!("Failed to create directory: {:?}", parent))?;
            }
        }

        if file.is_dir() {
            // Crea la directory se non esiste
            if !output_path.exists() {
                fs::create_dir_all(&output_path)
                    .context(format!("Failed to create directory: {:?}", output_path))?;
            }
        } else {
            // Crea il file
            let mut output_file = File::create(&output_path)
                .context(format!("Failed to create file: {:?}", output_path))?;

            // Copia il contenuto
            copy(&mut file, &mut output_file)
                .context(format!("Failed to write file: {:?}", output_path))?;

            // Imposta i permessi di esecuzione per script
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;

                if output_path.extension().map_or(false, |ext| ext == "sh") {
                    let mut perms = fs::metadata(&output_path)
                        .context(format!("Failed to get file permissions: {:?}", output_path))?
                        .permissions();

                    perms.set_mode(0o755); // rwx r-x r-x

                    fs::set_permissions(&output_path, perms)
                        .context(format!("Failed to set file permissions: {:?}", output_path))?;
                }
            }
        }
    }

    debug!("ZIP extraction completed");
    Ok(())
}

/// Estrae un archivio TAR.GZ
///
/// # Arguments
///
/// * `archive_path` - Il percorso dell'archivio TAR.GZ
/// * `extract_dir` - La directory in cui estrarre l'archivio
///
/// # Returns
///
/// `Ok(())` in caso di successo, altrimenti un errore
fn extract_tar_gz(archive_path: &Path, extract_dir: &Path) -> Result<()> {
    debug!("Extracting TAR.GZ archive: {:?}", archive_path);

    let file = File::open(archive_path)
        .context(format!("Failed to open TAR.GZ file: {:?}", archive_path))?;

    let gz = GzDecoder::new(file);
    let mut archive = Archive::new(gz);

    archive.unpack(extract_dir)
        .context(format!("Failed to extract TAR.GZ file: {:?}", archive_path))?;

    debug!("TAR.GZ extraction completed");
    Ok(())
}

/// Scarica ed estrae un file o un archivio
///
/// # Arguments
///
/// * `url` - L'URL da cui scaricare
/// * `extract_dir` - La directory in cui estrarre
/// * `timeout_secs` - Il timeout in secondi per la richiesta
///
/// # Returns
///
/// Il percorso della directory in cui è stato estratto il file o l'archivio
/// Scarica e decomprime solo se è un archivio, altrimenti copia il file
pub fn download_and_extract(url: &str, extract_dir: &Path, timeout_secs: u64) -> Result<PathBuf> {
    info!("Starting download_and_extract for URL: {}", url);
    info!("Extract directory: {:?}", extract_dir);

    // Crea una directory temporanea per il download
    let temp_dir = extract_dir.join("temp");
    if !temp_dir.exists() {
        info!("Creating temp directory: {:?}", temp_dir);
        fs::create_dir_all(&temp_dir).context("Failed to create temp directory")?;
    }

    // Scarica il file
    info!("Downloading file...");
    let downloaded_file = download_file(url, &temp_dir, timeout_secs)?;
    info!("File downloaded to: {:?}", downloaded_file);

    // Verifica se il file è un archivio
    let file_name = downloaded_file.file_name()
        .ok_or_else(|| anyhow!("Invalid file path"))?
        .to_string_lossy();
    info!("Downloaded file name: {}", file_name);

    // Se il file ha estensione .conf, copialo direttamente nella directory di destinazione
    if file_name.ends_with(".conf") {
        let dest_path = extract_dir.join(file_name.to_string());
        info!("Copying config file from {:?} to: {:?}", downloaded_file, dest_path);

        // Assicurati che la directory di destinazione esista
        if let Some(parent) = dest_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .context(format!("Failed to create directory: {:?}", parent))?;
            }
        }

        fs::copy(&downloaded_file, &dest_path)
            .context(format!("Failed to copy config file to {:?}", dest_path))?;

        // Rimuovi il file scaricato nella directory temporanea
        if downloaded_file.exists() {
            if let Err(e) = fs::remove_file(&downloaded_file) {
                warn!("Failed to remove temporary file {:?}: {}", downloaded_file, e);
            }
        }

        // Rimuovi la directory temporanea se è vuota
        if temp_dir.exists() {
            if let Ok(entries) = fs::read_dir(&temp_dir) {
                if entries.count() == 0 {
                    if let Err(e) = fs::remove_dir(&temp_dir) {
                        warn!("Failed to remove empty temporary directory {:?}: {}", temp_dir, e);
                    }
                }
            }
        }

        info!("Config file successfully copied to: {:?}", dest_path);
        return Ok(dest_path);
    }

    // Se è un archivio, estrai nella directory principale (non in temp)
    info!("Extracting archive...");
    let extracted_dir = extract_archive(&downloaded_file, extract_dir)?;
    info!("Archive extracted to: {:?}", extracted_dir);

    // Rimuovi il file scaricato nella directory temporanea
    if downloaded_file.exists() {
        if let Err(e) = fs::remove_file(&downloaded_file) {
            warn!("Failed to remove temporary file {:?}: {}", downloaded_file, e);
        }
    }

    // Rimuovi la directory temporanea se è vuota
    if temp_dir.exists() {
        if let Ok(entries) = fs::read_dir(&temp_dir) {
            if entries.count() == 0 {
                if let Err(e) = fs::remove_dir(&temp_dir) {
                    warn!("Failed to remove empty temporary directory {:?}: {}", temp_dir, e);
                }
            }
        }
    }

    Ok(extracted_dir)
}


/// Legge un file e restituisce il suo contenuto come stringa
///
/// # Arguments
///
/// * `path` - Il percorso del file da leggere
///
/// # Returns
///
/// Il contenuto del file come stringa
pub fn read_file_to_string(path: &Path) -> Result<String> {
    fs::read_to_string(path)
        .context(format!("Failed to read file: {:?}", path))
}

/// Scrive una stringa in un file
///
/// # Arguments
///
/// * `path` - Il percorso del file da scrivere
/// * `content` - Il contenuto da scrivere
///
/// # Returns
///
/// `Ok(())` in caso di successo, altrimenti un errore
pub fn write_string_to_file(path: &Path, content: &str) -> Result<()> {
    // Crea la directory padre se necessario
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .context(format!("Failed to create directory: {:?}", parent))?;
        }
    }

    fs::write(path, content)
        .context(format!("Failed to write file: {:?}", path))
}

