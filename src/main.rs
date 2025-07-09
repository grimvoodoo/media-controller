use actix_web::{App, HttpResponse, HttpServer, Responder, web};
use mpris::{PlaybackStatus, Player, PlayerFinder};
use serde::Serialize;
use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, PlatformConfig};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Application state, shared between handlers.
struct AppState {
    // Your MPRIS *publisher* ("My Player")
    controls: Arc<Mutex<MediaControls>>,
    // Your own copy of what metadata you last set
    copy_meta: Arc<Mutex<MediaMetadata<'static>>>,
    // Your own copy of what playback state you last set
    copy_playback: Arc<Mutex<MediaPlayback>>,
}

/// JSON view returned by GET /status
#[derive(Serialize)]
struct Status {
    // What *you* last told the system (Playing/Paused)
    our_playback: String,
    // What the *other* active player reports (if any)
    other_playback: Option<String>,
    // What title you last set
    title: Option<String>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // On Linux/macOS we don't need an HWND; on Windows you'd supply it here.
    #[cfg(not(target_os = "windows"))]
    let hwnd = None;
    #[cfg(target_os = "windows")]
    let hwnd = Some(/* your HWND here */);

    // 1) Initialize your own MPRIS service (so desktop UIs see "My Player")
    let config = PlatformConfig {
        dbus_name: "my_player",
        display_name: "My Player",
        hwnd,
    };
    let mut controls = MediaControls::new(config).expect("failed to init MediaControls");

    // Optional: log any hardware key events
    controls
        .attach(|evt: MediaControlEvent| println!("media key: {:?}", evt))
        .unwrap();

    // 2) Set some initial metadata & playback state
    let initial_meta: MediaMetadata<'static> = MediaMetadata {
        title: Some("Souvlaki Space Station".into()),
        artist: Some("Slowdive".into()),
        album: Some("Souvlaki".into()),
        ..Default::default()
    };
    let initial_pb = MediaPlayback::Paused { progress: None };

    controls.set_metadata(initial_meta.clone()).unwrap();
    controls.set_playback(initial_pb.clone()).unwrap();

    // 3) Wrap everything in Arcs+Mutex for sharing across Actix handlers
    let state = web::Data::new(AppState {
        controls: Arc::new(Mutex::new(controls)),
        copy_meta: Arc::new(Mutex::new(initial_meta)),
        copy_playback: Arc::new(Mutex::new(initial_pb)),
    });

    // 4) Spin up the HTTP server
    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .route("/play", web::post().to(play))
            .route("/pause", web::post().to(pause))
            .route("/toggle", web::post().to(toggle))
            .route("/volume_up", web::post().to(volume_up))
            .route("/volume_down", web::post().to(volume_down))
            .route("/next", web::post().to(next_track))
            .route("/previous", web::post().to(prev_track))
            .route("/seek_forward", web::post().to(seek_forward))
            .route("/seek_backward", web::post().to(seek_backward))
            .route("/status", web::get().to(status))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}

/// Helper: find the *currently active* MPRIS player, if any.
fn find_player() -> Option<Player> {
    let pf = PlayerFinder::new().ok()?;
    let all = pf.find_all().ok()?;
    all.into_iter().find(|p| p.identity() != "My Player")
}

/// POST /play — update *your* MPRIS state and tell the active player to play
async fn play(state: web::Data<AppState>) -> impl Responder {
    // 1) Update your own publisher state
    {
        let mut ctrls = state.controls.lock().unwrap();
        let mut pb = state.copy_playback.lock().unwrap();
        *pb = MediaPlayback::Playing { progress: None };
        ctrls.set_playback(pb.clone()).unwrap();
    }
    // 2) Tell any other active player to play
    if let Some(p) = find_player() {
        let _ = p.play();
    }
    HttpResponse::Ok().body("playing")
}

/// POST /pause — same pattern for pause
async fn pause(state: web::Data<AppState>) -> impl Responder {
    {
        let mut ctrls = state.controls.lock().unwrap();
        let mut pb = state.copy_playback.lock().unwrap();
        *pb = MediaPlayback::Paused { progress: None };
        ctrls.set_playback(pb.clone()).unwrap();
    }
    if let Some(p) = find_player() {
        let _ = p.pause();
    }
    HttpResponse::Ok().body("paused")
}

/// POST /toggle
/// If the external player is playing, pause it; otherwise play it.
/// Also update your own MPRIS service to match.
async fn toggle(state: web::Data<AppState>) -> impl Responder {
    // 1) Find the first real player
    if let Some(player) = find_player() {
        // 2) Query its status
        match player.get_playback_status() {
            Ok(PlaybackStatus::Playing) => {
                // pause external
                let _ = player.pause();
                // update ours
                let mut ctrls = state.controls.lock().unwrap();
                let mut pb = state.copy_playback.lock().unwrap();
                *pb = MediaPlayback::Paused { progress: None };
                ctrls.set_playback(pb.clone()).unwrap();
                HttpResponse::Ok().body("paused")
            }
            Ok(_) => {
                // play external
                let _ = player.play();
                // update ours
                let mut ctrls = state.controls.lock().unwrap();
                let mut pb = state.copy_playback.lock().unwrap();
                *pb = MediaPlayback::Playing { progress: None };
                ctrls.set_playback(pb.clone()).unwrap();
                HttpResponse::Ok().body("playing")
            }
            Err(e) => {
                eprintln!("Failed to get playback status: {}", e);
                HttpResponse::InternalServerError().body("couldn't read status")
            }
        }
    } else {
        // no external player found → just play
        let mut ctrls = state.controls.lock().unwrap();
        let mut pb = state.copy_playback.lock().unwrap();
        *pb = MediaPlayback::Playing { progress: None };
        ctrls.set_playback(pb.clone()).unwrap();
        HttpResponse::Ok().body("playing (no external player)")
    }
}

/// POST /volume_up — bump the system volume by 5%
async fn volume_up(_state: web::Data<AppState>) -> impl Responder {
    let status = Command::new("pactl")
        .args(["set-sink-volume", "@DEFAULT_SINK@", "+5%"])
        .status();

    match status {
        Ok(s) if s.success() => HttpResponse::Ok().body("system volume +5%"),
        Ok(s) => HttpResponse::InternalServerError().body(format!("pactl exited with {}", s)),
        Err(e) => {
            HttpResponse::InternalServerError().body(format!("failed to launch pactl: {}", e))
        }
    }
}

/// POST /volume_down — lower the system volume by 5%
async fn volume_down(_state: web::Data<AppState>) -> impl Responder {
    let status = Command::new("pactl")
        .args(["set-sink-volume", "@DEFAULT_SINK@", "-5%"])
        .status();

    match status {
        Ok(s) if s.success() => HttpResponse::Ok().body("system volume -5%"),
        Ok(s) => HttpResponse::InternalServerError().body(format!("pactl exited with {}", s)),
        Err(e) => {
            HttpResponse::InternalServerError().body(format!("failed to launch pactl: {}", e))
        }
    }
}

/// POST /next – skip to next track
async fn next_track(_state: web::Data<AppState>) -> impl Responder {
    if let Some(p) = find_player() {
        let _ = p.next(); // whole‐track skip :contentReference[oaicite:2]{index=2}
        HttpResponse::Ok().body("skipped to next track")
    } else {
        HttpResponse::NotFound().body("no external player found")
    }
}

/// POST /previous – skip to previous track
async fn prev_track(_state: web::Data<AppState>) -> impl Responder {
    if let Some(p) = find_player() {
        let _ = p.previous(); // whole‐track skip :contentReference[oaicite:3]{index=3}
        HttpResponse::Ok().body("skipped to previous track")
    } else {
        HttpResponse::NotFound().body("no external player found")
    }
}

/// POST /seek_forward – move forward 30 s within the current track
async fn seek_forward(_state: web::Data<AppState>) -> impl Responder {
    if let Some(p) = find_player() {
        if p.can_seek().unwrap() {
            let _ = p.seek_forwards(&Duration::from_secs(30)); // 30 s jump :contentReference[oaicite:4]{index=4}
            HttpResponse::Ok().body("seeked forward 30s")
        } else {
            HttpResponse::BadRequest().body("player cannot seek")
        }
    } else {
        HttpResponse::NotFound().body("no external player found")
    }
}

/// POST /seek_backward – move back 30 s within the current track
async fn seek_backward(_state: web::Data<AppState>) -> impl Responder {
    if let Some(p) = find_player() {
        if p.can_seek().unwrap() {
            let _ = p.seek_backwards(&Duration::from_secs(30)); // 30 s jump :contentReference[oaicite:5]{index=5}
            HttpResponse::Ok().body("seeked backward 30s")
        } else {
            HttpResponse::BadRequest().body("player cannot seek")
        }
    } else {
        HttpResponse::NotFound().body("no external player found")
    }
}

/// GET /status — report both your MPRIS state and the system's active player state
async fn status(state: web::Data<AppState>) -> impl Responder {
    // Read your last‐set playback
    let our_pb = {
        let pb = state.copy_playback.lock().unwrap();
        format!("{pb:?}")
    };
    // Ask the other player
    let other_pb = find_player()
        .and_then(|p| p.get_playback_status().ok())
        .map(|s| format!("{s:?}"));
    // Read your last‐set title
    let title = {
        let md = state.copy_meta.lock().unwrap();
        md.title.as_ref().map(|cow| cow.to_string())
    };

    let resp = Status {
        our_playback: our_pb,
        other_playback: other_pb,
        title,
    };
    HttpResponse::Ok().json(resp)
}
