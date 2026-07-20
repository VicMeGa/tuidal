/// tidal.rs — Llama a tidal.py como subproceso y parsea el JSON que devuelve.
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::sync::oneshot;

// ─── Calidades ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Quality {
    HiResLossless,
    Lossless,
    High,
}

impl Quality {
    pub fn as_api_str(&self) -> &'static str {
        match self {
            Quality::HiResLossless => "HI_RES_LOSSLESS",
            Quality::Lossless => "LOSSLESS",
            Quality::High => "HIGH",
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            Quality::HiResLossless => "HiRes FLAC 24bit",
            Quality::Lossless => "FLAC 16bit/44.1kHz",
            Quality::High => "AAC 320kbps",
        }
    }
}

// ─── Modelos ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Artist {
    pub id: u64,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Album {
    pub id: u64,
    pub title: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Track {
    pub id: u64,
    pub title: String,
    pub duration: u64,
    pub track_number: Option<u32>,
    pub artists: Vec<Artist>,
    pub album: Album,
    pub audio_quality: Option<String>,
    pub explicit: Option<bool>,
}

impl Track {
    pub fn artist_names(&self) -> String {
        self.artists
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn duration_str(&self) -> String {
        let m = self.duration / 60;
        let s = self.duration % 60;
        format!("{m}:{s:02}")
    }

    pub fn quality_icon(&self) -> &'static str {
        match self.audio_quality.as_deref() {
            Some("HI_RES_LOSSLESS") => "◈",
            Some("LOSSLESS") => "◆",
            _ => "◇",
        }
    }
}

// Modelo para álbumes de la colección
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FavAlbum {
    pub id: u64,
    pub title: String,
    pub number_of_tracks: u32,
    pub duration: u64,
    pub artists: Vec<Artist>,
    pub cover_url: Option<String>,
}

impl FavAlbum {
    pub fn artist_names(&self) -> String {
        self.artists
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[derive(Debug, Clone)]
pub struct StreamInfo {
    pub url: String,
    pub bit_depth: u32,
    pub sample_rate: u32,
    pub codec: String,
}

#[derive(Debug, Clone)]
pub struct CoverInfo {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub uuid: String,
    pub title: String,
    pub number_of_tracks: u32,
    pub duration: u64,
    #[serde(rename = "type")]
    pub playlist_type: String, // "USER", "EDITORIAL", "ARTIST"
    pub public_playlist: Option<bool>,
    pub image: Option<String>,
    pub square_image: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mix {
    pub id: String,
    pub title: String,
    pub sub_title: Option<String>,
}

// ─── Respuestas internas ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct LyricsResponse {
    pub lyrics: Option<String>,
    pub subtitles: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Lyrics {
    pub lines: Vec<(u64, String)>, // (timestamp_secs, text)
    pub plain: String,
    pub has_sync: bool,
}

impl Lyrics {
    pub fn from_response(resp: LyricsResponse) -> Self {
        let plain = resp.lyrics.unwrap_or_default();
        let has_sync = resp.subtitles.as_ref().map_or(false, |s| !s.is_empty());

        if has_sync {
            let lines = parse_lrc(&resp.subtitles.unwrap_or_default());
            Self {
                lines,
                plain,
                has_sync: true,
            }
        } else {
            Self {
                lines: Vec::new(),
                plain,
                has_sync: false,
            }
        }
    }

    pub fn current_line(&self, elapsed_secs: u64) -> usize {
        if !self.has_sync || self.lines.is_empty() {
            return 0;
        }
        // binary search for the last line with timestamp <= elapsed
        match self.lines.binary_search_by(|(ts, _)| ts.cmp(&elapsed_secs)) {
            Ok(i) => i,
            Err(0) => 0,
            Err(i) => i - 1,
        }
    }
}

fn parse_lrc(lrc: &str) -> Vec<(u64, String)> {
    let mut result = Vec::new();
    for line in lrc.lines() {
        let line = line.trim();
        if line.len() < 8 || !line.starts_with('[') {
            continue;
        }
        let close = line.find(']').unwrap_or(0);
        if close < 6 {
            continue;
        }
        let ts = &line[1..close];
        let text = line[close + 1..].trim().to_string();
        if let Some((min_str, rest)) = ts.split_once(':') {
            let min: u64 = min_str.parse().unwrap_or(0);
            let sec: f64 = rest.parse().unwrap_or(0.0);
            result.push((min * 60 + sec as u64, text));
        }
    }
    result
}

// ─── Cliente persistente (daemon) ─────────────────────────────────────────────

type RpcResult = Result<serde_json::Value>;

struct DaemonInner {
    pending: tokio::sync::Mutex<HashMap<u64, oneshot::Sender<RpcResult>>>,
    next_id: AtomicU64,
}

/// Cliente que mantiene un proceso Python persistente (--daemon).
/// Se comunica via JSON-RPC sobre stdin/stdout.
pub struct TidalDaemonClient {
    stdin: tokio::sync::Mutex<tokio::io::BufWriter<tokio::process::ChildStdin>>,
    inner: Arc<DaemonInner>,
    process: std::sync::Mutex<Option<tokio::process::Child>>,
    reader_handle: std::sync::Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl TidalDaemonClient {
    pub fn default_script_path() -> String {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("tidal.py")))
            .filter(|p| p.exists())
            .or_else(|| std::env::current_dir().ok().map(|d| d.join("tidal.py")))
            .filter(|p| p.exists())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "tidal.py".to_string())
    }

    pub async fn spawn(script_path: &str, python_path: &str, quality: &str) -> Result<Arc<Self>> {
        let mut child = tokio::process::Command::new(python_path)
            .arg(script_path)
            .arg("--daemon")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()?;

        let stdin = child.stdin.take().ok_or_else(|| anyhow!("No stdin"))?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow!("No stdout"))?;

        let inner = Arc::new(DaemonInner {
            pending: tokio::sync::Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
        });

        let mut writer = tokio::io::BufWriter::new(stdin);
        // Set initial quality
        let init_req =
            serde_json::json!({"id": 0, "method": "set_quality", "params": {"quality": quality}});
        let mut line = serde_json::to_string(&init_req)?;
        line.push('\n');
        writer.write_all(line.as_bytes()).await?;
        writer.flush().await?;

        // Read quality response
        let mut reader = tokio::io::BufReader::new(stdout);
        let mut resp_line = String::new();
        reader.read_line(&mut resp_line).await?;
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&resp_line) {
            if let Some(error) = val.get("error") {
                let msg = error.as_str().unwrap_or("unknown");
                return Err(anyhow!("Daemon init error: {msg}"));
            }
        }

        let inner_clone = inner.clone();
        let reader_handle = tokio::spawn(async move {
            let mut reader = reader;
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
                            if let Some(id) = val.get("id").and_then(|v| v.as_u64()) {
                                let mut pending = inner_clone.pending.lock().await;
                                if let Some(tx) = pending.remove(&id) {
                                    if let Some(error) = val.get("error") {
                                        let msg = error.as_str().unwrap_or("error desconocido");
                                        let _ = tx.send(Err(anyhow!("{}", msg)));
                                    } else if let Some(result) = val.get("result") {
                                        let _ = tx.send(Ok(result.clone()));
                                    } else {
                                        let _ = tx.send(Err(anyhow!("Respuesta inválida")));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        let client = Arc::new(Self {
            stdin: tokio::sync::Mutex::new(writer),
            inner,
            process: std::sync::Mutex::new(Some(child)),
            reader_handle: std::sync::Mutex::new(Some(reader_handle)),
        });

        Ok(client)
    }

    pub async fn call(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        let id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();

        {
            let mut pending = self.inner.pending.lock().await;
            pending.insert(id, tx);
        }

        let req = serde_json::json!({"id": id, "method": method, "params": params});
        let mut line = serde_json::to_string(&req)?;
        line.push('\n');

        {
            let mut stdin = self.stdin.lock().await;
            stdin.write_all(line.as_bytes()).await?;
            stdin.flush().await?;
        }

        match tokio::time::timeout(Duration::from_secs(30), rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(anyhow!("Request cancelled")),
            Err(_) => {
                let mut pending = self.inner.pending.lock().await;
                pending.remove(&id);
                Err(anyhow!("Timeout tras 30s"))
            }
        }
    }

    pub async fn shutdown(&self) {
        if let Ok(mut proc) = self.process.lock() {
            if let Some(mut child) = proc.take() {
                let _ = child.kill();
                // ponytail: no wait. child reaps on its own.
            }
        }
    }

    // ── Wrappers tipo-safe ──────────────────────────────────────────────────

    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<Track>> {
        let v = self
            .call(
                "search",
                serde_json::json!({"query": query, "limit": limit}),
            )
            .await?;
        serde_json::from_value(v).map_err(|e| anyhow!("JSON error: {e}"))
    }

    pub async fn get_stream_info(&self, track_id: u64, quality: &str) -> Result<StreamInfo> {
        let v = self
            .call(
                "stream",
                serde_json::json!({"track_id": track_id, "quality": quality}),
            )
            .await?;
        let bd = v.get("bit_depth").and_then(|x| x.as_u64()).unwrap_or(16) as u32;
        let sr = v
            .get("sample_rate")
            .and_then(|x| x.as_u64())
            .unwrap_or(44100) as u32;
        Ok(StreamInfo {
            url: v
                .get("url")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            codec: v
                .get("codec")
                .and_then(|x| x.as_str())
                .unwrap_or("flac")
                .to_string(),
            bit_depth: bd,
            sample_rate: sr,
        })
    }

    pub async fn get_cover(&self, track_id: u64) -> Result<CoverInfo> {
        let v = self
            .call("cover", serde_json::json!({"track_id": track_id}))
            .await?;
        Ok(CoverInfo {
            url: v
                .get("url")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
        })
    }

    pub async fn get_lyrics(&self, track_id: u64) -> Result<Lyrics> {
        let v = self
            .call("lyrics", serde_json::json!({"track_id": track_id}))
            .await?;
        let resp: LyricsResponse =
            serde_json::from_value(v).map_err(|e| anyhow!("JSON error: {e}"))?;
        Ok(Lyrics::from_response(resp))
    }

    pub async fn get_user_playlists(&self) -> Result<Vec<Playlist>> {
        let v = self.call("playlists", serde_json::json!({})).await?;
        serde_json::from_value(v).map_err(|e| anyhow!("JSON error: {e}"))
    }

    pub async fn get_playlist_tracks(&self, uuid: &str) -> Result<Vec<Track>> {
        let v = self
            .call("playlist_tracks", serde_json::json!({"uuid": uuid}))
            .await?;
        serde_json::from_value(v).map_err(|e| anyhow!("JSON error: {e}"))
    }

    pub async fn get_user_mixes(&self) -> Result<Vec<Mix>> {
        let v = self.call("mixes", serde_json::json!({})).await?;
        serde_json::from_value(v).map_err(|e| anyhow!("JSON error: {e}"))
    }

    pub async fn get_mix_tracks(&self, mix_id: &str) -> Result<Vec<Track>> {
        let v = self
            .call("mix_tracks", serde_json::json!({"mix_id": mix_id}))
            .await?;
        serde_json::from_value(v).map_err(|e| anyhow!("JSON error: {e}"))
    }

    pub async fn get_favorite_tracks(&self) -> Result<Vec<Track>> {
        let v = self.call("fav_tracks", serde_json::json!({})).await?;
        serde_json::from_value(v).map_err(|e| anyhow!("JSON error: {e}"))
    }

    pub async fn get_favorite_albums(&self) -> Result<Vec<FavAlbum>> {
        let v = self.call("fav_albums", serde_json::json!({})).await?;
        serde_json::from_value(v).map_err(|e| anyhow!("JSON error: {e}"))
    }

    pub async fn get_album_tracks(&self, album_id: u64) -> Result<Vec<Track>> {
        let v = self
            .call("album_tracks", serde_json::json!({"album_id": album_id}))
            .await?;
        serde_json::from_value(v).map_err(|e| anyhow!("JSON error: {e}"))
    }

    pub async fn start_device_auth(&self) -> Result<(String, String, String)> {
        let v = self.call("auth_start", serde_json::json!({})).await?;
        let url = v
            .get("url")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let code = v
            .get("code")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let device_code = v
            .get("device_code")
            .and_then(|x| x.as_str())
            .unwrap_or("pending")
            .to_string();
        Ok((device_code, code, url))
    }

    pub async fn poll_device_token(&self) -> Result<bool> {
        let v = self.call("auth_poll", serde_json::json!({})).await?;
        Ok(v.get("authenticated")
            .and_then(|x| x.as_bool())
            .unwrap_or(false))
    }

    pub async fn set_quality(&self, quality: &str) -> Result<()> {
        self.call("set_quality", serde_json::json!({"quality": quality}))
            .await?;
        Ok(())
    }
}

impl Drop for TidalDaemonClient {
    fn drop(&mut self) {
        if let Ok(mut handle) = self.reader_handle.lock() {
            if let Some(h) = handle.take() {
                h.abort();
            }
        }
        if let Ok(mut proc) = self.process.lock() {
            if let Some(mut child) = proc.take() {
                let _ = child.kill();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lrc_simple() {
        let lrc = "[00:12.00]Line 1\n[00:30.50]Line 2\n[01:05.00]Line 3\n";
        let parsed = parse_lrc(lrc);
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0], (12, "Line 1".to_string()));
        assert_eq!(parsed[1], (30, "Line 2".to_string()));
        assert_eq!(parsed[2], (65, "Line 3".to_string()));
    }

    #[test]
    fn test_parse_lrc_malformed_lines_skipped() {
        let lrc = "garbage\n[short]\n[00:12.00]valid\n";
        let parsed = parse_lrc(lrc);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0], (12, "valid".to_string()));
    }

    #[test]
    fn test_lyrics_current_line_synced() {
        let lrc = "[00:10.00]Ten\n[00:20.00]Twenty\n[00:30.00]Thirty\n";
        let resp = LyricsResponse {
            lyrics: Some("plain text".into()),
            subtitles: Some(lrc.into()),
        };
        let lyrics = Lyrics::from_response(resp);
        assert!(lyrics.has_sync);
        assert_eq!(lyrics.current_line(0), 0);
        assert_eq!(lyrics.current_line(10), 0);
        assert_eq!(lyrics.current_line(15), 0);
        assert_eq!(lyrics.current_line(20), 1);
        assert_eq!(lyrics.current_line(25), 1);
        assert_eq!(lyrics.current_line(30), 2);
        assert_eq!(lyrics.current_line(999), 2);
    }

    #[test]
    fn test_lyrics_current_line_no_sync() {
        let resp = LyricsResponse {
            lyrics: Some("plain text".into()),
            subtitles: None,
        };
        let lyrics = Lyrics::from_response(resp);
        assert!(!lyrics.has_sync);
        assert_eq!(lyrics.current_line(42), 0);
    }
}
