use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

use crate::app::AppEvent;
use crate::tidal::{TidalDaemonClient, Track};

pub fn search(tidal: Arc<TidalDaemonClient>, tx: UnboundedSender<AppEvent>, query: String) {
    tokio::spawn(async move {
        let result = tidal.search(&query, 20).await.map_err(|e| e.to_string());
        let _ = tx.send(AppEvent::SearchDone(result));
    });
}

pub fn stream(
    tidal: Arc<TidalDaemonClient>,
    tx: UnboundedSender<AppEvent>,
    quality: String,
    track: Track,
    queue_index: usize,
    generation: u64,
) {
    tokio::spawn(async move {
        match tidal.get_stream_info(track.id, &quality).await {
            Ok(info) => {
                let _ = tx.send(AppEvent::StreamReady {
                    track,
                    info,
                    queue_index,
                    generation,
                });
            }
            Err(e) => {
                let _ = tx.send(AppEvent::StreamError {
                    error: e.to_string(),
                    generation,
                });
            }
        }
    });
}

pub fn lyrics(tidal: Arc<TidalDaemonClient>, tx: UnboundedSender<AppEvent>, track_id: u64) {
    tokio::spawn(async move {
        match tidal.get_lyrics(track_id).await {
            Ok(l) => {
                let _ = tx.send(AppEvent::LyricsReady(l));
            }
            Err(_) => {
                let _ = tx.send(AppEvent::LyricsError);
            }
        }
    });
}

pub fn cover(tidal: Arc<TidalDaemonClient>, tx: UnboundedSender<AppEvent>, track_id: u64) {
    tokio::spawn(async move {
        let cover_info = match tidal.get_cover(track_id).await {
            Ok(c) => c,
            Err(_) => {
                let _ = tx.send(AppEvent::CoverError);
                return;
            }
        };
        let image_bytes = match reqwest::get(&cover_info.url).await {
            Ok(r) => match r.bytes().await {
                Ok(b) => b,
                Err(_) => {
                    let _ = tx.send(AppEvent::CoverError);
                    return;
                }
            },
            Err(_) => {
                let _ = tx.send(AppEvent::CoverError);
                return;
            }
        };
        match image::load_from_memory(&image_bytes) {
            Ok(img) => {
                let _ = tx.send(AppEvent::CoverReady {
                    info: cover_info,
                    image: img,
                });
            }
            Err(_) => {
                let _ = tx.send(AppEvent::CoverError);
            }
        }
    });
}

pub fn start_auth(tidal: Arc<TidalDaemonClient>, tx: UnboundedSender<AppEvent>) {
    tokio::spawn(async move {
        match tidal.start_device_auth().await {
            Ok((device_code, user_code, url)) => {
                let _ = tx.send(AppEvent::AuthStarted {
                    url,
                    code: user_code,
                    device_code,
                });
            }
            Err(e) => {
                let _ = tx.send(AppEvent::AuthError(e.to_string()));
            }
        }
    });
}

pub fn poll_auth(tidal: Arc<TidalDaemonClient>, tx: UnboundedSender<AppEvent>) {
    tokio::spawn(async move {
        match tidal.poll_device_token().await {
            Ok(true) => {
                let _ = tx.send(AppEvent::AuthDone);
            }
            Ok(false) => {}
            Err(e) => {
                let _ = tx.send(AppEvent::StatusMsg(format!("Error poll: {e}")));
            }
        }
    });
}

pub fn library(tidal: Arc<TidalDaemonClient>, tx: UnboundedSender<AppEvent>) {
    tokio::spawn(async move {
        let playlists = tidal.get_user_playlists().await.unwrap_or_default();
        let mixes = tidal.get_user_mixes().await.unwrap_or_default();
        let _ = tx.send(AppEvent::LibraryLoaded { playlists, mixes });
    });
}

pub fn playlist_tracks(tidal: Arc<TidalDaemonClient>, tx: UnboundedSender<AppEvent>, uuid: String) {
    tokio::spawn(async move {
        match tidal.get_playlist_tracks(&uuid).await {
            Ok(tracks) => {
                let _ = tx.send(AppEvent::PlaylistTracksLoaded(tracks));
            }
            Err(e) => {
                let _ = tx.send(AppEvent::StreamError {
                    error: e.to_string(),
                    generation: 0,
                });
            }
        }
    });
}

pub fn mix_tracks(tidal: Arc<TidalDaemonClient>, tx: UnboundedSender<AppEvent>, mix_id: String) {
    tokio::spawn(async move {
        match tidal.get_mix_tracks(&mix_id).await {
            Ok(tracks) => {
                let _ = tx.send(AppEvent::PlaylistTracksLoaded(tracks));
            }
            Err(e) => {
                let _ = tx.send(AppEvent::StreamError {
                    error: e.to_string(),
                    generation: 0,
                });
            }
        }
    });
}

pub fn fav_tracks(tidal: Arc<TidalDaemonClient>, tx: UnboundedSender<AppEvent>) {
    tokio::spawn(async move {
        match tidal.get_favorite_tracks().await {
            Ok(t) => {
                let _ = tx.send(AppEvent::FavTracksLoaded(t));
            }
            Err(e) => {
                let _ = tx.send(AppEvent::StreamError {
                    error: e.to_string(),
                    generation: 0,
                });
            }
        }
    });
}

pub fn fav_albums(tidal: Arc<TidalDaemonClient>, tx: UnboundedSender<AppEvent>) {
    tokio::spawn(async move {
        match tidal.get_favorite_albums().await {
            Ok(a) => {
                let _ = tx.send(AppEvent::FavAlbumsLoaded(a));
            }
            Err(e) => {
                let _ = tx.send(AppEvent::StreamError {
                    error: e.to_string(),
                    generation: 0,
                });
            }
        }
    });
}

pub fn album_tracks(tidal: Arc<TidalDaemonClient>, tx: UnboundedSender<AppEvent>, album_id: u64) {
    tokio::spawn(async move {
        match tidal.get_album_tracks(album_id).await {
            Ok(tracks) => {
                let _ = tx.send(AppEvent::PlaylistTracksLoaded(tracks));
            }
            Err(e) => {
                let _ = tx.send(AppEvent::StreamError {
                    error: e.to_string(),
                    generation: 0,
                });
            }
        }
    });
}

pub fn set_quality(tidal: Arc<TidalDaemonClient>, quality: String) {
    tokio::spawn(async move {
        let _ = tidal.set_quality(&quality).await;
    });
}
