use std::io::Write;
use std::os::unix::net::UnixStream;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

// Ruta del socket IPC de mpv — única por proceso
const SOCKET_PATH: &str = "/tmp/tuidal-mpv.sock";

pub struct Player {
    process:    Option<Child>,
    pub state:  PlayerState,
    pub current: Option<TrackInfo>,
    pub volume:  u8,
    pub elapsed: Duration,
    last_tick:   Option<Instant>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlayerState {
    Stopped,
    Playing,
    Paused,
}

#[derive(Debug, Clone)]
pub struct TrackInfo {
    pub title:       String,
    pub artist:      String,
    pub album:       String,
    pub duration:    u64,
    pub bit_depth:   u32,
    pub sample_rate: u32,
    pub codec:       String,
}

impl Player {
    pub fn new() -> Self {
        Self {
            process:   None,
            state:     PlayerState::Stopped,
            current:   None,
            volume:    85,
            elapsed:   Duration::ZERO,
            last_tick: None,
        }
    }

    pub fn play(&mut self, url: &str, info: TrackInfo) {
        self.stop();
        self.current   = Some(info);
        self.elapsed   = Duration::ZERO;
        self.last_tick = Some(Instant::now());

        // Eliminar socket anterior si quedó huérfano
        let _ = std::fs::remove_file(SOCKET_PATH);

        let mut mpv_args = vec![
            "--no-video".to_string(),
            "--really-quiet".to_string(),
            format!("--input-ipc-server={SOCKET_PATH}"),
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
                self.state   = PlayerState::Playing;
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
                    self.state   = PlayerState::Playing;
                }
            }
        }
    }

    pub fn stop(&mut self) {
        // Pedir a mpv que salga limpiamente antes de kill
        self.ipc_cmd(r#"{"command":["quit"]}"#);
        std::thread::sleep(Duration::from_millis(50));

        if let Some(mut child) = self.process.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = std::fs::remove_file(SOCKET_PATH);
        self.state     = PlayerState::Stopped;
        self.elapsed   = Duration::ZERO;
        self.last_tick = None;
    }

    pub fn toggle_pause(&mut self) {
        match self.state {
            PlayerState::Playing => {
                self.ipc_cmd(r#"{"command":["set_property","pause",true]}"#);
                self.state     = PlayerState::Paused;
                self.last_tick = None;
            }
            PlayerState::Paused => {
                self.ipc_cmd(r#"{"command":["set_property","pause",false]}"#);
                self.state     = PlayerState::Playing;
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

    /// Envía un comando JSON al socket IPC de mpv (fire-and-forget)
    fn ipc_cmd(&self, json: &str) {
        if let Ok(mut stream) = UnixStream::connect(SOCKET_PATH) {
            let msg = format!("{json}\n");
            let _ = stream.write_all(msg.as_bytes());
        }
        // Si falla (socket no listo todavía) simplemente ignoramos
    }

    pub fn tick(&mut self) {
        if self.state == PlayerState::Playing {
            if let Some(last) = self.last_tick {
                self.elapsed += last.elapsed();
                self.last_tick = Some(Instant::now());
            }

            // Verificar si el proceso terminó
            if let Some(ref mut child) = self.process {
                if let Ok(Some(_)) = child.try_wait() {
                    self.process   = None;
                    self.state     = PlayerState::Stopped;
                    self.last_tick = None;
                    let _ = std::fs::remove_file(SOCKET_PATH);
                }
            }
        }
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
