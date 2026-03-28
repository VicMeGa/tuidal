/// tidal.rs — Llama a tidal.py como subproceso y parsea el JSON que devuelve.
/// tidal.py debe estar en el mismo directorio que el binario, o en el directorio actual.

use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::process::Command;

// ─── Calidades ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Quality {
    HiResLossless,
    Lossless,
    High,
}

impl Quality {
    pub fn as_api_str(&self) -> &'static str {
        match self {
            Quality::HiResLossless => "HI_RES_LOSSLESS",
            Quality::Lossless      => "LOSSLESS",
            Quality::High          => "HIGH",
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            Quality::HiResLossless => "HiRes FLAC 24bit",
            Quality::Lossless      => "FLAC 16bit/44.1kHz",
            Quality::High          => "AAC 320kbps",
        }
    }
}

// ─── Modelos ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct Artist {
    pub id:   u64,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Album {
    pub id:    u64,
    pub title: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Track {
    pub id:            u64,
    pub title:         String,
    pub duration:      u64,
    pub track_number:  Option<u32>,
    pub artists:       Vec<Artist>,
    pub album:         Album,
    pub audio_quality: Option<String>,
    pub explicit:      Option<bool>,
}

impl Track {
    pub fn artist_names(&self) -> String {
        self.artists.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", ")
    }

    pub fn duration_str(&self) -> String {
        let m = self.duration / 60;
        let s = self.duration % 60;
        format!("{m}:{s:02}")
    }

    pub fn quality_icon(&self) -> &'static str {
        match self.audio_quality.as_deref() {
            Some("HI_RES_LOSSLESS") => "◈",
            Some("LOSSLESS")        => "◆",
            _                       => "◇",
        }
    }
}

#[derive(Debug, Clone)]
pub struct StreamInfo {
    pub url:         String,
    pub mime_type:   String,
    pub bit_depth:   u32,
    pub sample_rate: u32,
    pub codec:       String,
}

// ─── Respuestas internas ──────────────────────────────────────────────────────

#[derive(Deserialize)]
struct AuthStartResp {
    url:   Option<String>,
    code:  Option<String>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct AuthPollResp {
    authenticated: Option<bool>,
    #[serde(default)]
    error: String,
}

#[derive(Deserialize)]
struct StreamResp {
    url:         Option<String>,
    codec:       Option<String>,
    bit_depth:   Option<u32>,
    sample_rate: Option<u32>,
    mime_type:   Option<String>,
    error:       Option<String>,
}

// ─── Cliente ──────────────────────────────────────────────────────────────────

pub struct TidalClient {
    pub quality:     Quality,
    pub script_path: String,
}

impl TidalClient {
    pub fn new() -> Self {
        // Busca tidal.py junto al ejecutable, luego en el directorio actual
        let script_path = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("tidal.py")))
            .filter(|p| p.exists())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "tidal.py".to_string());

        Self { quality: Quality::Lossless, script_path }
    }

    /// Constructor para usar desde tareas en background
    pub fn with_path_and_quality(script_path: String, quality: Quality) -> Self {
        Self { quality, script_path }
    }

    /// Ejecuta tidal.py con los args dados y devuelve el stdout.
    fn run(&self, args: &[&str]) -> Result<String> {
        let output = Command::new("/home/victor/mi_python/bin/python3")
            .arg(&self.script_path)
            .args(args)
            .output()
            .map_err(|e| anyhow!(
                "No se pudo ejecutar python3: {e}\n¿Está instalado tidalapi? ¿Está tidal.py en '{}'?",
                self.script_path
            ))?;

        if !output.stderr.is_empty() {
            eprintln!("[tidal.py] {}", String::from_utf8_lossy(&output.stderr).trim());
        }

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if stdout.is_empty() {
            return Err(anyhow!("tidal.py no produjo output"));
        }
        Ok(stdout)
    }

    // ── Auth ──────────────────────────────────────────────────────────────────

    pub async fn load_session(&self) -> Result<()> {
        let stdout = self.run(&["auth", "poll"])?;
        let resp: AuthPollResp = serde_json::from_str(&stdout)
            .map_err(|e| anyhow!("JSON error: {e}\noutput: {stdout}"))?;

        if resp.authenticated.unwrap_or(false) {
            Ok(())
        } else {
            Err(anyhow!("Sin sesión activa"))
        }
    }

    /// Inicia Device Flow. El script bloquea hasta que el usuario autorice,
    /// así que lo corremos en spawn_blocking para no congelar la TUI.
    pub async fn start_device_auth(&self) -> Result<(String, String, String)> {
        let script_path = self.script_path.clone();

        let stdout = tokio::task::spawn_blocking(move || {
            Command::new("/home/victor/mi_python/bin/python3")
                .arg(&script_path)
                .args(["auth", "start"])
                .output()
                .map_err(|e| anyhow!("Error lanzando python3: {e}"))
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        })
        .await??;

        let resp: AuthStartResp = serde_json::from_str(&stdout)
            .map_err(|e| anyhow!("JSON error: {e}\noutput: {stdout}"))?;

        if let Some(e) = resp.error {
            return Err(anyhow!("{e}"));
        }

        let url  = resp.url.ok_or_else(|| anyhow!("Sin URL de auth"))?;
        let code = resp.code.unwrap_or_default();

        Ok(("pending".to_string(), code, url))
    }

    pub async fn poll_device_token(&self, _device_code: &str) -> Result<bool> {
        let stdout = self.run(&["auth", "poll"])?;
        let resp: AuthPollResp = serde_json::from_str(&stdout)
            .map_err(|e| anyhow!("JSON error: {e}\noutput: {stdout}"))?;

        if !resp.error.is_empty() {
            eprintln!("[auth poll] {}", resp.error);
        }

        Ok(resp.authenticated.unwrap_or(false))
    }

    // ── API ───────────────────────────────────────────────────────────────────

    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<Track>> {
        let limit_str = limit.to_string();
        let stdout = self.run(&["search", query, &limit_str])?;
        let tracks: Vec<Track> = serde_json::from_str(&stdout)
            .map_err(|e| anyhow!("JSON error: {e}\noutput: {stdout}"))?;
        Ok(tracks)
    }

    pub async fn get_stream_info(&self, track_id: u64) -> Result<StreamInfo> {
        let id_str = track_id.to_string();
        let stdout = self.run(&["stream", &id_str, self.quality.as_api_str()])?;
        let resp: StreamResp = serde_json::from_str(&stdout)
            .map_err(|e| anyhow!("JSON error: {e}\noutput: {stdout}"))?;

        if let Some(e) = resp.error {
            return Err(anyhow!("{e}"));
        }

        Ok(StreamInfo {
            url:         resp.url.ok_or_else(|| anyhow!("Sin URL de stream"))?,
            codec:       resp.codec.unwrap_or_else(|| "flac".into()),
            bit_depth:   resp.bit_depth.unwrap_or(16),
            sample_rate: resp.sample_rate.unwrap_or(44100),
            mime_type:   resp.mime_type.unwrap_or_else(|| "audio/flac".into()),
        })
    }
}