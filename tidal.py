#!/usr/bin/env python3
"""
tidal.py — Subproceso intermediario entre Rust y Tidal via tidalapi.

Uso:
  python3 tidal.py auth start
  python3 tidal.py auth poll
  python3 tidal.py search "Daft Punk"
  python3 tidal.py stream 12345
  python3 tidal.py stream 12345 HI_RES_LOSSLESS
"""

import json
import sys
import threading
from pathlib import Path

import tidalapi

SESSION_FILE = Path.home() / ".config" / "tidal-tui" / "tidalapi_session.json"
POLL_FILE    = Path.home() / ".config" / "tidal-tui" / "oauth_pending.json"

def out(data):
    print(json.dumps(data, ensure_ascii=False))
    sys.stdout.flush()

def err(msg: str):
    print(msg, file=sys.stderr)
    out({"error": msg})

def make_session(quality=None) -> tidalapi.Session:
    config = tidalapi.Config(quality=quality) if quality else tidalapi.Config()
    return tidalapi.Session(config)

def load_session(session: tidalapi.Session) -> bool:
    try:
        session.load_session_from_file(SESSION_FILE)
        return session.check_login()
    except Exception:
        return False

def save_session(session: tidalapi.Session):
    SESSION_FILE.parent.mkdir(parents=True, exist_ok=True)
    session.save_session_to_file(SESSION_FILE)

# ─── Comandos ─────────────────────────────────────────────────────────────────

def cmd_auth_start():
    session = make_session()
    POLL_FILE.parent.mkdir(parents=True, exist_ok=True)

    # login_oauth() → (LinkLogin, Future)
    link_login, future = session.login_oauth()

    url  = str(link_login.verification_uri_complete)
    code = str(link_login.user_code)

    # Marcar como pendiente
    POLL_FILE.write_text(json.dumps({"done": False}))

    # Imprimir URL para que Rust la muestre
    out({"url": url, "code": code})

    # Esperar en hilo a que el usuario autorice
    def wait_auth():
        try:
            future.result()  # bloquea hasta autorización
            save_session(session)
            POLL_FILE.write_text(json.dumps({"done": True}))
        except Exception as e:
            POLL_FILE.write_text(json.dumps({"done": False, "error": str(e)}))

    t = threading.Thread(target=wait_auth, daemon=False)
    t.start()
    t.join()  # el proceso espera hasta que el usuario autorice

def cmd_auth_poll():
    session = make_session()

    if load_session(session):
        out({"authenticated": True})
        return

    if not POLL_FILE.exists():
        out({"authenticated": False, "pending": False})
        return

    try:
        state = json.loads(POLL_FILE.read_text())
        if state.get("done"):
            POLL_FILE.unlink(missing_ok=True)
            out({"authenticated": True})
        else:
            out({"authenticated": False, "pending": True, "error": state.get("error", "")})
    except Exception as e:
        out({"authenticated": False, "error": str(e)})

def cmd_search(query: str, limit: int = 20):
    session = make_session()
    if not load_session(session):
        err("No autenticado")
        return

    try:
        results = session.search(query, [tidalapi.Track], limit=limit)
        tracks  = results.get("tracks", []) or []
        out([_track_dict(t) for t in tracks[:limit]])
    except Exception as e:
        err(str(e))

def cmd_stream(track_id: int, quality_str: str = "LOSSLESS"):
    quality_map = {
        "HI_RES_LOSSLESS": tidalapi.Quality.hi_res_lossless,
        "LOSSLESS":        tidalapi.Quality.high_lossless,
        "HIGH":            tidalapi.Quality.low_320k,
    }

    fallback_chain = {
        "HI_RES_LOSSLESS": ["HI_RES_LOSSLESS", "LOSSLESS", "HIGH"],
        "LOSSLESS":        ["LOSSLESS", "HIGH"],
        "HIGH":            ["HIGH"],
    }

    last_error = ""
    for q_str in fallback_chain.get(quality_str, ["LOSSLESS", "HIGH"]):
        # Crear sesión nueva con la calidad correcta en cada intento
        session = make_session(quality=quality_map[q_str])
        if not load_session(session):
            err("No autenticado")
            return
        try:
            track = session.track(track_id)
            url   = track.get_url()
            out({
                "url":         url,
                "codec":       "flac" if q_str in ("HI_RES_LOSSLESS", "LOSSLESS") else "aac",
                "bit_depth":   24 if q_str == "HI_RES_LOSSLESS" else 16,
                "sample_rate": 96000 if q_str == "HI_RES_LOSSLESS" else 44100,
                "mime_type":   "audio/flac" if q_str in ("HI_RES_LOSSLESS", "LOSSLESS") else "audio/aac",
                "quality":     q_str,
            })
            return
        except Exception as e:
            last_error = str(e)
            continue

    err(f"No se pudo obtener stream en ninguna calidad: {last_error}")

# ─── Helpers ──────────────────────────────────────────────────────────────────

def _track_dict(t: tidalapi.Track) -> dict:
    return {
        "id":            t.id,
        "title":         t.name,
        "duration":      t.duration,
        "track_number":  getattr(t, "track_num", None),
        "artists":       [{"id": a.id, "name": a.name} for a in t.artists],
        "album":         {"id": t.album.id, "title": t.album.name},
        "audio_quality": str(getattr(t, "audio_quality", "") or ""),
        "explicit":      getattr(t, "explicit", False),
    }

# ─── Charge album cover ───────────────────────────────────────────────────────
def cmd_cover(track_id: int):
    session = make_session()
    if not load_session(session):
        err("No autenticado")
        return
    try:
        track = session.track(track_id)
        url   = track.album.image(320)
        out({"url": url, "title": track.name, "artist": track.artist.name, "album": track.album.name})
    except Exception as e:
        err(str(e))


# ─── Entry point ──────────────────────────────────────────────────────────────

if __name__ == "__main__":
    args = sys.argv[1:]

    if not args:
        err("Uso: tidal.py <auth start|auth poll|search <query>|stream <id> [quality]>")
        sys.exit(1)

    match args:
        case ["auth", "start"]:
            cmd_auth_start()
        case ["auth", "poll"]:
            cmd_auth_poll()
        case ["search", query]:
            cmd_search(query)
        case ["search", query, limit]:
            cmd_search(query, int(limit))
        case ["stream", track_id]:
            cmd_stream(int(track_id))
        case ["stream", track_id, quality]:
            cmd_stream(int(track_id), quality)
        case ["cover", track_id]:
            cmd_cover(int(track_id))
        case _:
            err(f"Comando desconocido: {args}")
            sys.exit(1)