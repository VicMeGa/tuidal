use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;

use crate::app::{ApiCommand, ApiStatus, AppEvent, RepeatMode};

const BUS_NAME: &str = "org.mpris.MediaPlayer2.tuidal";
const OBJECT_PATH: &str = "/org/mpris/MediaPlayer2";

struct Shared {
    status: Arc<RwLock<ApiStatus>>,
    tx: UnboundedSender<AppEvent>,
}

// ── org.mpris.MediaPlayer2 (root interface) ─────────────────────────────────

#[allow(dead_code)]
struct MprisRoot(Shared);

#[zbus::interface(name = "org.mpris.MediaPlayer2")]
impl MprisRoot {
    #[zbus(property)]
    fn identity(&self) -> &str {
        "tuidal"
    }

    #[zbus(property)]
    fn desktop_entry(&self) -> &str {
        "tuidal"
    }

    #[zbus(property)]
    fn supported_uri_schemes(&self) -> Vec<&str> {
        vec!["http", "https"]
    }

    #[zbus(property)]
    fn supported_mime_types(&self) -> Vec<&str> {
        vec![
            "audio/flac",
            "audio/mpeg",
            "audio/ogg",
            "audio/wav",
            "audio/aac",
        ]
    }

    #[zbus(property)]
    fn has_track_list(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn can_quit(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_raise(&self) -> bool {
        false
    }

    #[zbus(property)]
    fn can_set_fullscreen(&self) -> bool {
        false
    }
}

// ── org.mpris.MediaPlayer2.Player ────────────────────────────────────────────

struct MprisPlayer(Shared);

impl MprisPlayer {
    fn playback_status_str(&self) -> String {
        match self.0.status.read().unwrap().state.as_str() {
            "playing" => "Playing",
            "paused" => "Paused",
            _ => "Stopped",
        }
        .to_string()
    }

    fn build_metadata(&self) -> HashMap<String, zbus::zvariant::Value<'static>> {
        let s = self.0.status.read().unwrap();
        let mut map = HashMap::new();

        if let Some(track_id) = s.track_id {
            let tid = format!("/com/tuidal/track/{}", track_id);
            if let Ok(path) = zbus::zvariant::ObjectPath::try_from(tid) {
                map.insert("mpris:trackid".into(), zbus::zvariant::Value::new(path));
            }
        } else if let Some(idx) = s.queue_index {
            let tid = format!("/com/tuidal/track/queue/{}", idx);
            if let Ok(path) = zbus::zvariant::ObjectPath::try_from(tid) {
                map.insert("mpris:trackid".into(), zbus::zvariant::Value::new(path));
            }
        }

        if let Some(ref title) = s.title {
            map.insert(
                "xesam:title".into(),
                zbus::zvariant::Value::new(title.clone()),
            );
        }
        if let Some(ref artist) = s.artist {
            map.insert(
                "xesam:artist".into(),
                zbus::zvariant::Value::new(vec![artist.clone()]),
            );
        }
        if let Some(ref album) = s.album {
            map.insert(
                "xesam:album".into(),
                zbus::zvariant::Value::new(album.clone()),
            );
        }
        if let Some(dur) = s.duration {
            map.insert(
                "mpris:length".into(),
                zbus::zvariant::Value::new((dur as i64) * 1_000_000),
            );
        }

        map
    }

    fn vol(&self) -> f64 {
        self.0.status.read().unwrap().volume as f64 / 100.0
    }

    fn elapsed_micros(&self) -> i64 {
        (self.0.status.read().unwrap().elapsed as i64) * 1_000_000
    }
}

#[zbus::interface(name = "org.mpris.MediaPlayer2.Player")]
impl MprisPlayer {
    #[zbus(property)]
    fn playback_status(&self) -> String {
        self.playback_status_str()
    }

    #[zbus(property)]
    fn loop_status(&self) -> String {
        match self.0.status.read().unwrap().repeat {
            RepeatMode::Off => "None",
            RepeatMode::One => "Track",
            RepeatMode::All => "Playlist",
        }
        .to_string()
    }

    #[zbus(property)]
    fn shuffle(&self) -> bool {
        self.0.status.read().unwrap().shuffle
    }

    #[zbus(property)]
    fn metadata(&self) -> HashMap<String, zbus::zvariant::Value<'static>> {
        self.build_metadata()
    }

    #[zbus(property)]
    fn volume(&self) -> f64 {
        self.vol()
    }

    #[zbus(property)]
    fn set_volume(&mut self, value: f64) {
        let vol = (value.clamp(0.0, 1.0) * 100.0) as u8;
        let _ = self.0.tx.send(AppEvent::ApiCmd(ApiCommand::VolumeSet(vol)));
    }

    #[zbus(property)]
    fn position(&self) -> i64 {
        self.elapsed_micros()
    }

    #[zbus(property)]
    fn minimum_rate(&self) -> f64 {
        1.0
    }

    #[zbus(property)]
    fn maximum_rate(&self) -> f64 {
        1.0
    }

    #[zbus(property)]
    fn rate(&self) -> f64 {
        1.0
    }

    #[zbus(property)]
    fn can_go_next(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_go_previous(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_play(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_pause(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_seek(&self) -> bool {
        true
    }

    #[zbus(property)]
    fn can_control(&self) -> bool {
        true
    }

    fn next(&self) {
        let _ = self.0.tx.send(AppEvent::ApiCmd(ApiCommand::Next));
    }

    fn previous(&self) {
        let _ = self.0.tx.send(AppEvent::ApiCmd(ApiCommand::Prev));
    }

    fn pause(&self) {
        if self.0.status.read().unwrap().state == "playing" {
            let _ = self.0.tx.send(AppEvent::ApiCmd(ApiCommand::PlayPause));
        }
    }

    fn play_pause(&self) {
        let _ = self.0.tx.send(AppEvent::ApiCmd(ApiCommand::PlayPause));
    }

    fn play(&self) {
        let state = self.0.status.read().unwrap().state.clone();
        if state == "paused" {
            let _ = self.0.tx.send(AppEvent::ApiCmd(ApiCommand::PlayPause));
        } else if state == "stopped" {
            let s = self.0.status.read().unwrap();
            if let Some(i) = s.queue_index {
                if i < s.queue.len() {
                    let track = &s.queue[i];
                    let _ = self.0.tx.send(AppEvent::ApiCmd(ApiCommand::PlayTrack(
                        crate::app::ApiTrack {
                            id: track.id,
                            title: track.title.clone(),
                            artist: track.artist_names(),
                            album: track.album.title.clone(),
                            duration: track.duration,
                        },
                    )));
                }
            }
        }
    }

    fn stop(&self) {
        let _ = self.0.tx.send(AppEvent::ApiCmd(ApiCommand::Stop));
    }

    fn seek(&self, offset: i64) {
        let secs = offset / 1_000_000;
        let _ = self.0.tx.send(AppEvent::ApiCmd(ApiCommand::Seek(secs)));
    }

    fn set_position(&self, _track_id: zbus::zvariant::ObjectPath<'_>, position: i64) {
        let secs = (position / 1_000_000) as u64;
        let _ = self
            .0
            .tx
            .send(AppEvent::ApiCmd(ApiCommand::SetPosition(secs)));
    }

    fn open_uri(&self, _uri: String) {}
}

// ── Server entry point ───────────────────────────────────────────────────────

pub async fn start_mpris_server(status: Arc<RwLock<ApiStatus>>, tx: UnboundedSender<AppEvent>) {
    let Ok(conn) = zbus::Connection::session().await else {
        return;
    };

    if conn
        .object_server()
        .at(
            OBJECT_PATH,
            MprisRoot(Shared {
                status: status.clone(),
                tx: tx.clone(),
            }),
        )
        .await
        .is_err()
    {
        return;
    }

    if conn
        .object_server()
        .at(
            OBJECT_PATH,
            MprisPlayer(Shared {
                status: status.clone(),
                tx: tx.clone(),
            }),
        )
        .await
        .is_err()
    {
        return;
    }

    let _ = conn.request_name(BUS_NAME).await;

    let Ok(ctxt) = zbus::object_server::SignalEmitter::new(&conn, OBJECT_PATH) else {
        return;
    };

    let player = MprisPlayer(Shared {
        status: status.clone(),
        tx: tx.clone(),
    });

    let mut prev_state = String::new();
    let mut prev_title: Option<String> = None;
    let mut prev_artist: Option<String> = None;
    let mut prev_album: Option<String> = None;
    let mut prev_duration: Option<u64> = None;
    let mut prev_track_id: Option<u64> = None;
    let mut prev_volume = 0u8;
    let mut prev_repeat = RepeatMode::Off;
    let mut prev_shuffle = false;

    let mut interval = tokio::time::interval(Duration::from_millis(500));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        interval.tick().await;

        let (state, volume, repeat, shuffle, title, artist, album, duration, track_id) = {
            let cur = status.read().unwrap();
            (
                cur.state.clone(),
                cur.volume,
                cur.repeat.clone(),
                cur.shuffle,
                cur.title.clone(),
                cur.artist.clone(),
                cur.album.clone(),
                cur.duration,
                cur.track_id,
            )
        };

        if state != prev_state {
            prev_state = state;
            let _ = player.playback_status_changed(&ctxt).await;
        }

        if volume != prev_volume {
            prev_volume = volume;
            let _ = player.volume_changed(&ctxt).await;
        }

        if repeat != prev_repeat {
            prev_repeat = repeat;
            let _ = player.loop_status_changed(&ctxt).await;
        }

        if shuffle != prev_shuffle {
            prev_shuffle = shuffle;
            let _ = player.shuffle_changed(&ctxt).await;
        }

        if title != prev_title
            || artist != prev_artist
            || album != prev_album
            || duration != prev_duration
            || track_id != prev_track_id
        {
            prev_title = title;
            prev_artist = artist;
            prev_album = album;
            prev_duration = duration;
            prev_track_id = track_id;
            let _ = player.metadata_changed(&ctxt).await;
        }
    }
}
