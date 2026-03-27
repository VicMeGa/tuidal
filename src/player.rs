use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

pub struct Player {
    process:     Option<Child>,
    pub state:   PlayerState,
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

        let child = Command::new("mpv")
            .args([
                "--no-video",
                "--really-quiet",
                &format!("--volume={}", self.volume),
                url,
            ])
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
                self.try_fallback(url);
            }
        }
    }

    fn try_fallback(&mut self, url: &str) {
        let child = Command::new("ffplay")
            .args(["-nodisp", "-autoexit", "-loglevel", "quiet", url])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        if let Ok(c) = child {
            self.process = Some(c);
            self.state   = PlayerState::Playing;
        }
    }

    pub fn stop(&mut self) {
        if let Some(mut child) = self.process.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.state     = PlayerState::Stopped;
        self.elapsed   = Duration::ZERO;
        self.last_tick = None;
    }

    pub fn toggle_pause(&mut self) {
        match self.state {
            PlayerState::Playing => {
                if let Some(ref mut child) = self.process {
                    #[cfg(unix)]
                    unsafe { libc::kill(child.id() as libc::pid_t, libc::SIGSTOP); }
                }
                self.state     = PlayerState::Paused;
                self.last_tick = None;
            }
            PlayerState::Paused => {
                if let Some(ref mut child) = self.process {
                    #[cfg(unix)]
                    unsafe { libc::kill(child.id() as libc::pid_t, libc::SIGCONT); }
                }
                self.state     = PlayerState::Playing;
                self.last_tick = Some(Instant::now());
            }
            PlayerState::Stopped => {}
        }
    }

    pub fn volume_up(&mut self) {
        self.volume = (self.volume + 5).min(100);
        self.set_mpv_volume();
    }

    pub fn volume_down(&mut self) {
        self.volume = self.volume.saturating_sub(5);
        self.set_mpv_volume();
    }

    /// Envía el volumen actualizado a mpv via stdin no funciona sin IPC,
    /// pero al menos actualiza el valor para la próxima canción.
    fn set_mpv_volume(&self) {
        // Sin IPC socket el volumen solo aplica al siguiente play().
        // Para control en tiempo real habría que agregar --input-ipc-server.
    }

    pub fn seek_forward(&mut self) {
        if let Some(info) = &self.current {
            let max = Duration::from_secs(info.duration);
            self.elapsed = (self.elapsed + Duration::from_secs(10)).min(max);
        }
    }

    pub fn seek_backward(&mut self) {
        self.elapsed = self.elapsed.saturating_sub(Duration::from_secs(10));
    }

    pub fn tick(&mut self) {
        if self.state == PlayerState::Playing {
            if let Some(last) = self.last_tick {
                self.elapsed  += last.elapsed();
                self.last_tick = Some(Instant::now());
            }

            // Verificar si mpv terminó
            if let Some(ref mut child) = self.process {
                if let Ok(Some(_)) = child.try_wait() {
                    self.process   = None;
                    self.state     = PlayerState::Stopped;
                    self.last_tick = None;
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