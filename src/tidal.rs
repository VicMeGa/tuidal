/// tidal.rs — Llama a tidal.py como subproceso y parsea el JSON que devuelve.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::io::BufRead;
use std::process::{Command, Stdio};

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

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Artist {
    pub id:   u64,
    pub name: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Album {
    pub id:    u64,
    pub title: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
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

// Modelo para álbumes de la colección
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FavAlbum {
    pub id:               u64,
    pub title:            String,
    pub number_of_tracks: u32,
    pub duration:         u64,
    pub artists:          Vec<Artist>,
    pub cover_url:        Option<String>,
}

impl FavAlbum {
    pub fn artist_names(&self) -> String {
        self.artists.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", ")
    }
}

#[derive(Debug, Clone)]
pub struct StreamInfo {
    pub url:         String,
    pub bit_depth:   u32,
    pub sample_rate: u32,
    pub codec:       String,
}

#[derive(Debug, Clone)]
pub struct CoverInfo {
    pub url: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub uuid:             String,
    pub title:            String,
    pub number_of_tracks: u32,
    pub duration:         u64,
    #[serde(rename = "type")]
    pub playlist_type:    String,   // "USER", "EDITORIAL", "ARTIST"
    pub public_playlist:  Option<bool>,
    pub image:            Option<String>,
    pub square_image:     Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mix {
    pub id:    String,
    pub title: String,
    pub sub_title: Option<String>,
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
    error:       Option<String>,
}

#[derive(Deserialize)]
struct CoverResp {
    url:   Option<String>,
    error: Option<String>,
}

// ─── Cliente ──────────────────────────────────────────────────────────────────

pub struct TidalClient {
    pub quality:     Quality,
    pub script_path: String,
    pub python_path: String,
}

impl TidalClient {
    pub fn new() -> Self {
        let script_path = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("tidal.py")))
            .filter(|p| p.exists())
            .or_else(|| std::env::current_dir().ok().map(|d| d.join("tidal.py")))
            .filter(|p| p.exists())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "tidal.py".to_string());

        let python_path = std::env::var("TUIDAL_PYTHON_PATH").unwrap_or_else(|_| "python3".to_string());

        Self { quality: Quality::Lossless, script_path, python_path }
    }

    pub fn with_path_and_quality(script_path: String, quality: Quality, python_path: String) -> Self {
        Self { quality, script_path, python_path }
    }

    fn run(&self, args: &[&str]) -> Result<String> {
        let output = Command::new(&self.python_path)
            .arg(&self.script_path)
            .args(args)
            .output()
            .map_err(|e| anyhow!(
                "No se pudo ejecutar python3: {e}\n¿Está tidal.py en '{}'?",
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

   pub async fn get_favorite_tracks(&self) -> Result<Vec<Track>> {
        let stdout = self.run(&["fav_tracks"])?;
        let tracks: Vec<Track> = serde_json::from_str(&stdout)
            .map_err(|e| anyhow!("JSON error: {e}\noutput: {stdout}"))?;
        Ok(tracks)
    }

    pub async fn get_favorite_albums(&self) -> Result<Vec<FavAlbum>> {
        let stdout = self.run(&["fav_albums"])?;
        let albums: Vec<FavAlbum> = serde_json::from_str(&stdout)
            .map_err(|e| anyhow!("JSON error: {e}\noutput: {stdout}"))?;
        Ok(albums)
    }

    pub async fn get_album_tracks(&self, album_id: u64) -> Result<Vec<Track>> {
        let stdout = self.run(&["album_tracks", &album_id.to_string()])?;
        let tracks: Vec<Track> = serde_json::from_str(&stdout)
            .map_err(|e| anyhow!("JSON error: {e}\noutput: {stdout}"))?;
        Ok(tracks)
    }

    // ── Auth ──────────────────────────────────────────────────────────────────

    pub async fn load_session(&self) -> Result<()> {
        let stdout = self.run(&["auth", "poll"])?;
        let resp: AuthPollResp = serde_json::from_str(&stdout)
            .map_err(|e| anyhow!("JSON error: {e}\noutput: {stdout}"))?;
        if resp.authenticated.unwrap_or(false) { Ok(()) }
        else { Err(anyhow!("Sin sesión activa")) }
    }

    pub async fn start_device_auth(&self) -> Result<(String, String, String)> {
        let script_path = self.script_path.clone();
        let python_path = self.python_path.clone();
        let stdout = tokio::task::spawn_blocking(move || {
            let mut child = Command::new(&python_path)
                .arg(&script_path)
                .args(["auth", "start"])
                .stdin(Stdio::null())
                .stderr(Stdio::null())
                .stdout(Stdio::piped())
                .spawn()
                .map_err(|e| anyhow!("Error lanzando python3: {e}"))?;
            // Leer solo la primera línea (JSON con URL), luego dejar el proceso
            // corriendo en background — el thread de Python sigue esperando la
            // autorización y escribe el resultado en POLL_FILE.
            let reader = std::io::BufReader::new(child.stdout.take().unwrap());
            let first_line = reader.lines()
                .next()
                .ok_or_else(|| anyhow!("tidal.py no produjo output"))?
                .map_err(|e| anyhow!("Error leyendo output: {e}"))?;
            Ok::<String, anyhow::Error>(first_line)
        }).await??;

        let resp: AuthStartResp = serde_json::from_str(&stdout)
            .map_err(|e| anyhow!("JSON error: {e}\noutput: {stdout}"))?;
        if let Some(e) = resp.error { return Err(anyhow!("{e}")); }
        let url  = resp.url.ok_or_else(|| anyhow!("Sin URL de auth"))?;
        let code = resp.code.unwrap_or_default();
        Ok(("pending".to_string(), code, url))
    }

    pub async fn poll_device_token(&self, _device_code: &str) -> Result<bool> {
        let stdout = self.run(&["auth", "poll"])?;
        let resp: AuthPollResp = serde_json::from_str(&stdout)
            .map_err(|e| anyhow!("JSON error: {e}\noutput: {stdout}"))?;
        if !resp.error.is_empty() { eprintln!("[auth poll] {}", resp.error); }
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
        if let Some(e) = resp.error { return Err(anyhow!("{e}")); }
        Ok(StreamInfo {
            url:         resp.url.ok_or_else(|| anyhow!("Sin URL de stream"))?,
            codec:       resp.codec.unwrap_or_else(|| "flac".into()),
            bit_depth:   resp.bit_depth.unwrap_or(16),
            sample_rate: resp.sample_rate.unwrap_or(44100),
        })
    }

    pub async fn get_cover(&self, track_id: u64) -> Result<CoverInfo> {
        let id_str = track_id.to_string();
        let stdout = self.run(&["cover", &id_str])?;
        let resp: CoverResp = serde_json::from_str(&stdout)
            .map_err(|e| anyhow!("JSON error: {e}\noutput: {stdout}"))?;
        if let Some(e) = resp.error { return Err(anyhow!("{e}")); }
        Ok(CoverInfo {
            url: resp.url.unwrap_or_default(),
        })
    }
    pub async fn get_user_playlists(&self) -> Result<Vec<Playlist>> {
        let stdout = self.run(&["playlists"])?;
        let playlists: Vec<Playlist> = serde_json::from_str(&stdout)
            .map_err(|e| anyhow!("JSON error: {e}\noutput: {stdout}"))?;
        Ok(playlists)
    }

    pub async fn get_playlist_tracks(&self, uuid: &str) -> Result<Vec<Track>> {
        let stdout = self.run(&["playlist_tracks", uuid])?;
        let tracks: Vec<Track> = serde_json::from_str(&stdout)
            .map_err(|e| anyhow!("JSON error: {e}\noutput: {stdout}"))?;
        Ok(tracks)
    }

    pub async fn get_user_mixes(&self) -> Result<Vec<Mix>> {
        let stdout = self.run(&["mixes"])?;
        let mixes: Vec<Mix> = serde_json::from_str(&stdout)
            .map_err(|e| anyhow!("JSON error: {e}\noutput: {stdout}"))?;
        Ok(mixes)
    }

    pub async fn get_mix_tracks(&self, mix_id: &str) -> Result<Vec<Track>> {
        let stdout = self.run(&["mix_tracks", mix_id])?;
        let tracks: Vec<Track> = serde_json::from_str(&stdout)
            .map_err(|e| anyhow!("JSON error: {e}\noutput: {stdout}"))?;
        Ok(tracks)
    }
}