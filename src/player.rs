use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

// ponytail: socket único por PID, evita colisión entre instancias
fn socket_path() -> String {
    format!("/tmp/tuidal-mpv-{}.sock", std::process::id())
}

pub struct Player {
    process: Option<Child>,
    pub state: PlayerState,
    pub current: Option<TrackInfo>,
    pub volume: u8,
    pub elapsed: Duration,
    last_tick: Option<Instant>,
    pos_skip: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlayerState {
    Stopped,
    Playing,
    Paused,
}

#[derive(Debug, Clone)]
pub struct TrackInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: u64,
    pub bit_depth: u32,
    pub sample_rate: u32,
    pub codec: String,
}

impl Player {
    pub fn new() -> Self {
        Self {
            process: None,
            state: PlayerState::Stopped,
            current: None,
            volume: 85,
            elapsed: Duration::ZERO,
            last_tick: None,
            pos_skip: 0,
        }
    }

    pub fn play(&mut self, url: &str, info: TrackInfo) {
        self.stop();
        self.current = Some(info);
        self.elapsed = Duration::ZERO;
        self.last_tick = Some(Instant::now());
        self.pos_skip = 0;

        // Eliminar socket anterior si quedó huérfano
        let _ = std::fs::remove_file(socket_path());

        let mut mpv_args = vec![
            "--no-video".to_string(),
            "--really-quiet".to_string(),
            format!("--input-ipc-server={}", socket_path()),
            format!("--volume={}", self.volume),
        ];
        #[cfg(target_os = "linux")]
        mpv_args.push("--audio-device=alsa/default".to_string());
        mpv_args.push(url.to_string());

        let child = Command::new("mpv")
            .args(&mpv_args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        match child {
            Ok(c) => {
                self.process = Some(c);
                self.state = PlayerState::Playing;
            }
            Err(_) => {
                // fallback: ffplay (no tiene IPC pero al menos reproduce)
                let child2 = Command::new("ffplay")
                    .args(["-nodisp", "-autoexit", "-loglevel", "quiet", url])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn();
                if let Ok(c) = child2 {
                    self.process = Some(c);
                    self.state = PlayerState::Playing;
                }
            }
        }
    }

    pub fn stop(&mut self) {
        // Pedir a mpv que salga limpiamente antes de kill
        self.ipc_cmd(r#"{"command":["quit"]}"#);
        // ponytail: removed blocking thread::sleep.

        if let Some(mut child) = self.process.take() {
            let _ = child.kill();
        }
        let _ = std::fs::remove_file(socket_path());
        self.state = PlayerState::Stopped;
        self.elapsed = Duration::ZERO;
        self.last_tick = None;
        self.pos_skip = 0;
    }

    pub fn toggle_pause(&mut self) {
        match self.state {
            PlayerState::Playing => {
                self.ipc_cmd(r#"{"command":["set_property","pause",true]}"#);
                self.state = PlayerState::Paused;
                self.last_tick = None;
            }
            PlayerState::Paused => {
                self.ipc_cmd(r#"{"command":["set_property","pause",false]}"#);
                self.state = PlayerState::Playing;
                self.last_tick = Some(Instant::now());
            }
            PlayerState::Stopped => {}
        }
    }

    pub fn set_volume(&mut self, v: u8) {
        self.volume = v.min(100);
        self.ipc_cmd(&format!(
            r#"{{"command":["set_property","volume",{}]}}"#,
            self.volume
        ));
    }

    pub fn volume_up(&mut self) {
        self.volume = (self.volume + 5).min(100);
        self.ipc_cmd(&format!(
            r#"{{"command":["set_property","volume",{}]}}"#,
            self.volume
        ));
    }

    pub fn volume_down(&mut self) {
        self.volume = self.volume.saturating_sub(5);
        self.ipc_cmd(&format!(
            r#"{{"command":["set_property","volume",{}]}}"#,
            self.volume
        ));
    }

    pub fn seek_forward(&mut self) {
        // Seek real en mpv + actualizar contador visual
        self.ipc_cmd(r#"{"command":["seek",10,"relative"]}"#);
        if let Some(info) = &self.current {
            let max = Duration::from_secs(info.duration);
            self.elapsed = (self.elapsed + Duration::from_secs(10)).min(max);
        }
    }

    pub fn seek_backward(&mut self) {
        self.ipc_cmd(r#"{"command":["seek",-10,"relative"]}"#);
        self.elapsed = self.elapsed.saturating_sub(Duration::from_secs(10));
    }

    pub fn seek_relative(&mut self, secs: i64) {
        self.ipc_cmd(&format!(r#"{{"command":["seek",{},"relative"]}}"#, secs));
        if secs >= 0 {
            self.elapsed = self
                .elapsed
                .saturating_add(Duration::from_secs(secs as u64));
        } else {
            self.elapsed = self
                .elapsed
                .saturating_sub(Duration::from_secs((-secs) as u64));
        }
        if let Some(info) = &self.current {
            self.elapsed = self.elapsed.min(Duration::from_secs(info.duration));
        }
    }

    pub fn seek_absolute(&mut self, secs: u64) {
        self.ipc_cmd(&format!(r#"{{"command":["seek",{},"absolute"]}}"#, secs));
        self.elapsed = Duration::from_secs(secs);
        if let Some(info) = &self.current {
            self.elapsed = self.elapsed.min(Duration::from_secs(info.duration));
        }
    }

    /// Envía un comando JSON al socket IPC de mpv (fire-and-forget)
    fn ipc_cmd(&self, json: &str) {
        let msg = format!("{json}\n");
        for _ in 0..3 {
            if let Ok(mut stream) = UnixStream::connect(socket_path()) {
                let _ = stream.write_all(msg.as_bytes());
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    pub fn tick(&mut self) {
        if let Some(ref mut child) = self.process {
            if let Ok(Some(_)) = child.try_wait() {
                self.process = None;
                self.state = PlayerState::Stopped;
                self.last_tick = None;
                self.pos_skip = 0;
                let _ = std::fs::remove_file(socket_path());
            }
        }

        if self.state == PlayerState::Playing {
            self.pos_skip += 1;
            // ponytail: query mpv every ~1s (20 ticks at 50ms) instead of every tick
            if self.pos_skip % 20 == 0 {
                if let Some(dur) = self.query_time_pos() {
                    self.elapsed = dur;
                }
            } else if let Some(last) = self.last_tick {
                self.elapsed += Instant::now().duration_since(last);
                if let Some(info) = &self.current {
                    self.elapsed = self.elapsed.min(Duration::from_secs(info.duration));
                }
            }
            self.last_tick = Some(Instant::now());
        }
    }

    fn query_time_pos(&self) -> Option<Duration> {
        let mut stream = UnixStream::connect(socket_path()).ok()?;
        let _ = stream.set_read_timeout(Some(Duration::from_millis(5)));
        let cmd = b"{\"command\":[\"get_property\",\"time-pos\"]}\n";
        let _ = stream.write_all(cmd);
        let mut buf = [0u8; 512];
        let n = stream.read(&mut buf).ok()?;
        if n == 0 {
            return Some(Duration::ZERO);
        }
        let resp = std::str::from_utf8(&buf[..n]).ok()?;
        let val: serde_json::Value = serde_json::from_str(resp).ok()?;
        let data = val.get("data")?;
        if data.is_null() {
            return Some(Duration::ZERO);
        }
        let pos = data.as_f64()?;
        mpv_pos_to_duration(pos)
    }

    pub fn progress(&self) -> f64 {
        if let Some(info) = &self.current {
            if info.duration > 0 {
                return (self.elapsed.as_secs_f64() / info.duration as f64).min(1.0);
            }
        }
        0.0
    }

    pub fn elapsed_str(&self) -> String {
        let s = self.elapsed.as_secs();
        format!("{}:{:02}", s / 60, s % 60)
    }
}

pub(crate) fn mpv_pos_to_duration(pos: f64) -> Option<Duration> {
    if pos.is_nan() || pos.is_infinite() || pos < 0.0 {
        return None;
    }
    Some(Duration::from_secs_f64(pos))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elapsed_str_zero() {
        let p = Player::new();
        assert_eq!(p.elapsed_str(), "0:00");
    }

    #[test]
    fn test_elapsed_str_format() {
        let mut p = Player::new();
        p.elapsed = Duration::from_secs(65);
        assert_eq!(p.elapsed_str(), "1:05");
        p.elapsed = Duration::from_secs(3661);
        assert_eq!(p.elapsed_str(), "61:01");
    }

    #[test]
    fn test_progress_no_track() {
        let p = Player::new();
        assert_eq!(p.progress(), 0.0);
    }

    #[test]
    fn test_progress_with_track() {
        let mut p = Player::new();
        p.current = Some(TrackInfo {
            title: "Test".into(),
            artist: "Artist".into(),
            album: "Album".into(),
            duration: 200,
            bit_depth: 16,
            sample_rate: 44100,
            codec: "flac".into(),
        });
        p.elapsed = Duration::from_secs(50);
        let prog = p.progress();
        assert!((prog - 0.25).abs() < 0.001);
    }

    #[test]
    fn test_progress_clamped() {
        let mut p = Player::new();
        p.current = Some(TrackInfo {
            title: "Test".into(),
            artist: "Artist".into(),
            album: "Album".into(),
            duration: 100,
            bit_depth: 16,
            sample_rate: 44100,
            codec: "flac".into(),
        });
        p.elapsed = Duration::from_secs(999);
        assert!((p.progress() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_mpv_pos_to_duration_nan_returns_none() {
        assert!(mpv_pos_to_duration(f64::NAN).is_none());
    }

    #[test]
    fn test_mpv_pos_to_duration_infinite_returns_none() {
        assert!(mpv_pos_to_duration(f64::INFINITY).is_none());
        assert!(mpv_pos_to_duration(f64::NEG_INFINITY).is_none());
    }

    #[test]
    fn test_mpv_pos_to_duration_negative_returns_none() {
        assert!(mpv_pos_to_duration(-1.0).is_none());
        assert!(mpv_pos_to_duration(-0.001).is_none());
    }

    #[test]
    fn test_mpv_pos_to_duration_valid_returns_some() {
        let d = mpv_pos_to_duration(0.0).expect("0.0 should be valid");
        assert_eq!(d, Duration::ZERO);

        let d = mpv_pos_to_duration(42.5).expect("42.5 should be valid");
        assert_eq!(d, Duration::from_secs_f64(42.5));
    }
}
