use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc::UnboundedSender;

use crate::app::{ApiCommand, ApiStatus, ApiTrack, AppEvent};
use crate::tidal::{FavAlbum, Playlist, Mix, Quality, TidalClient, Track};

pub const PORT: u16 = 7837;

#[derive(Clone)]
struct ApiState {
    tx:            UnboundedSender<AppEvent>,
    status:        Arc<RwLock<ApiStatus>>,
    tidal_script:  String,
    tidal_quality: Quality,
    tidal_python:  String,
}

impl ApiState {
    fn tidal(&self) -> TidalClient {
        TidalClient::with_path_and_quality(
            self.tidal_script.clone(),
            self.tidal_quality,
            self.tidal_python.clone(),
        )
    }
}

fn log(msg: &str) {
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/tuidal-api.log") {
        let _ = writeln!(f, "{}", msg);
    }
}

pub async fn start_server(
    tx:            UnboundedSender<AppEvent>,
    status:        Arc<RwLock<ApiStatus>>,
    tidal_script:  String,
    tidal_quality: Quality,
    tidal_python:  String,
) {
    log("start_server: task started");
    let state = ApiState { tx, status, tidal_script, tidal_quality, tidal_python };
    log("start_server: state created");

    let router = Router::new()
        .route("/status",        get(handle_status))
        .route("/play-pause",    post(handle_play_pause))
        .route("/next",          post(handle_next))
        .route("/previous",      post(handle_previous))
        .route("/volume-up",     post(handle_volume_up))
        .route("/volume-down",   post(handle_volume_down))
        .route("/volume",        post(handle_volume_set))
        .route("/seek-forward",  post(handle_seek_forward))
        .route("/seek-backward", post(handle_seek_backward))
        .route("/shuffle",       post(handle_shuffle))
        .route("/repeat",        post(handle_repeat))
        .route("/play-track",    post(handle_play_track))
        .route("/just-play",     post(handle_just_play))
        .route("/queue",         get(handle_queue))
        .route("/search",        get(handle_search));
    log("start_server: base routes registered");

    let router = router
        .route("/library",                 get(handle_library))
        .route("/library/favorites",       get(handle_library_favorites))
        .route("/library/favorite-albums", get(handle_library_fav_albums));
    log("start_server: library static routes registered");

    let router = router
        .route("/library/playlist/{uuid}", get(handle_library_playlist))
        .route("/library/mix/{id}",        get(handle_library_mix))
        .route("/library/album/{id}",      get(handle_library_album));
    log("start_server: library param routes registered");

    let router = router.with_state(state);
    log("start_server: with_state done");

    log(&format!("start_server: binding 127.0.0.1:{PORT}"));
    let listener = match tokio::net::TcpListener::bind(("127.0.0.1", PORT)).await {
        Ok(l)  => { log(&format!("start_server: bound OK on port {PORT}")); l }
        Err(e) => { log(&format!("start_server: bind FAILED: {e}")); return; }
    };
    log("start_server: calling axum::serve");
    if let Err(e) = axum::serve(listener, router).await {
        log(&format!("start_server: serve error: {e}"));
    }
}

// ── Status ────────────────────────────────────────────────────────────────────

async fn handle_status(State(s): State<ApiState>) -> Json<ApiStatus> {
    Json(s.status.read().unwrap().clone())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn send_cmd(s: &ApiState, cmd: ApiCommand) -> StatusCode {
    match s.tx.send(AppEvent::ApiCmd(cmd)) {
        Ok(_)  => StatusCode::NO_CONTENT,
        Err(_) => StatusCode::SERVICE_UNAVAILABLE,
    }
}

// ── Playback controls ─────────────────────────────────────────────────────────

async fn handle_play_pause(State(s): State<ApiState>) -> StatusCode {
    send_cmd(&s, ApiCommand::PlayPause)
}
async fn handle_next(State(s): State<ApiState>) -> StatusCode {
    send_cmd(&s, ApiCommand::Next)
}
async fn handle_previous(State(s): State<ApiState>) -> StatusCode {
    send_cmd(&s, ApiCommand::Prev)
}
async fn handle_volume_up(State(s): State<ApiState>) -> StatusCode {
    send_cmd(&s, ApiCommand::VolumeUp)
}
async fn handle_volume_down(State(s): State<ApiState>) -> StatusCode {
    send_cmd(&s, ApiCommand::VolumeDown)
}

#[derive(Deserialize)]
struct VolumeQuery { level: u8 }

async fn handle_volume_set(State(s): State<ApiState>, Query(q): Query<VolumeQuery>) -> StatusCode {
    send_cmd(&s, ApiCommand::VolumeSet(q.level))
}
async fn handle_seek_forward(State(s): State<ApiState>) -> StatusCode {
    send_cmd(&s, ApiCommand::SeekForward)
}
async fn handle_seek_backward(State(s): State<ApiState>) -> StatusCode {
    send_cmd(&s, ApiCommand::SeekBackward)
}
async fn handle_shuffle(State(s): State<ApiState>) -> StatusCode {
    send_cmd(&s, ApiCommand::ToggleShuffle)
}
async fn handle_repeat(State(s): State<ApiState>) -> StatusCode {
    send_cmd(&s, ApiCommand::CycleRepeat)
}

// ── Play track ────────────────────────────────────────────────────────────────

async fn handle_play_track(
    State(s): State<ApiState>,
    Json(track): Json<ApiTrack>,
) -> StatusCode {
    send_cmd(&s, ApiCommand::PlayTrack(track))
}

#[derive(Deserialize)]
struct JustPlayQuery { q: String }

async fn handle_just_play(
    State(s): State<ApiState>,
    Query(p): Query<JustPlayQuery>,
) -> StatusCode {
    match s.tidal().search(&p.q, 1).await {
        Ok(tracks) if !tracks.is_empty() => {
            let t = &tracks[0];
            let api_track = ApiTrack {
                id:       t.id,
                title:    t.title.clone(),
                artist:   t.artist_names(),
                album:    t.album.title.clone(),
                duration: t.duration,
            };
            send_cmd(&s, ApiCommand::PlayTrack(api_track))
        }
        _ => StatusCode::NOT_FOUND,
    }
}

// ── Queue ─────────────────────────────────────────────────────────────────────

async fn handle_queue(State(s): State<ApiState>) -> Json<serde_json::Value> {
    let st = s.status.read().unwrap();
    Json(serde_json::json!({
        "tracks":        st.queue,
        "current_index": st.queue_index,
    }))
}

// ── Search ────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct SearchQuery {
    q:                  String,
    #[serde(default = "default_limit")]
    limit:              usize,
}
fn default_limit() -> usize { 20 }

async fn handle_search(
    State(s): State<ApiState>,
    Query(p): Query<SearchQuery>,
) -> Result<Json<Vec<Track>>, StatusCode> {
    s.tidal().search(&p.q, p.limit).await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// ── Library ───────────────────────────────────────────────────────────────────

async fn handle_library(
    State(s): State<ApiState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let client    = s.tidal();
    let playlists = client.get_user_playlists().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mixes     = client.get_user_mixes().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "playlists": playlists, "mixes": mixes })))
}

async fn handle_library_favorites(
    State(s): State<ApiState>,
) -> Result<Json<Vec<Track>>, StatusCode> {
    s.tidal().get_favorite_tracks().await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn handle_library_fav_albums(
    State(s): State<ApiState>,
) -> Result<Json<Vec<FavAlbum>>, StatusCode> {
    s.tidal().get_favorite_albums().await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn handle_library_playlist(
    State(s): State<ApiState>,
    Path(uuid): Path<String>,
) -> Result<Json<Vec<Track>>, StatusCode> {
    s.tidal().get_playlist_tracks(&uuid).await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn handle_library_mix(
    State(s): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<Track>>, StatusCode> {
    s.tidal().get_mix_tracks(&id).await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn handle_library_album(
    State(s): State<ApiState>,
    Path(id): Path<u64>,
) -> Result<Json<Vec<Track>>, StatusCode> {
    s.tidal().get_album_tracks(id).await
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// Suppress unused-import warnings for types used only in return positions
const _: fn() = || {
    let _: Playlist;
    let _: Mix;
};
