use actix_web::body::BoxBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::error::ErrorUnauthorized;
use actix_web::http::header;
use actix_web::middleware::{from_fn, Next};
use actix_web::{web, App, Error, HttpResponse, HttpServer, Responder};
use mpris::{PlaybackStatus, Player, PlayerFinder};
use serde::Serialize;
use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, PlatformConfig};
use std::env;
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
    // Which player is being controlled (identity)
    controlled_player: Option<String>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let token = get_api_token();
    let token_data = web::Data::new(token);

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
        .attach(|evt: MediaControlEvent| println!("media key: {evt:?}"))
        .unwrap();

    // 2) Set some initial metadata & playback state
    let initial_meta: MediaMetadata<'static> = MediaMetadata {
        title: Some("Souvlaki Space Station"),
        artist: Some("Slowdive"),
        album: Some("Souvlaki"),
        ..Default::default()
    };
    let initial_pb = MediaPlayback::Paused { progress: None };

    controls.set_metadata(initial_meta.clone()).unwrap();
    controls.set_playback(initial_pb.clone()).unwrap();

    // 3) Wrap everything in Arcs+Mutex for sharing across Actix handlers
    // let state = web::Data::new(AppState {
    //     controls: Arc::new(Mutex::new(controls)),
    //     copy_meta: Arc::new(Mutex::new(initial_meta)),
    //     copy_playback: Arc::new(Mutex::new(initial_pb)),
    // });

    let shared_state = web::Data::new(AppState {
        controls: Arc::new(Mutex::new(controls)),
        copy_meta: Arc::new(Mutex::new(initial_meta)),
        copy_playback: Arc::new(Mutex::new(initial_pb)),
    });

    // let token_data = web::Data::new(token.clone());

    // 4) Spin up the HTTP server
    HttpServer::new(move || {
        App::new()
            .app_data(token_data.clone())
            .wrap(from_fn(auth_middleware))
            .app_data(shared_state.clone())
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

/// Read the token from an env var
fn get_api_token() -> String {
    env::var("MEDIA_CONTROL_API_TOKEN").expect("must set MEDIA_CONTROL_API_TOKEN")
}

/// Read the preferred player from env var, defaulting to "chromium"
fn get_preferred_player() -> String {
    env::var("MEDIA_CONTROL_PREFERRED_PLAYER")
        .unwrap_or_else(|_| "chromium".to_string())
        .to_lowercase()
}

/// This middleware will run *before* every handler.
async fn auth_middleware(
    req: ServiceRequest,
    next: Next<BoxBody>, // <-- note BoxBody here
) -> Result<ServiceResponse<BoxBody>, Error> {
    // Grab expected token from app data
    let expected = req
        .app_data::<web::Data<String>>()
        .map(|d| d.get_ref().clone())
        .unwrap_or_default();

    // Check for `Authorization: Bearer <token>`
    let authorized = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .map(|val| val == format!("Bearer {expected}"))
        .unwrap_or(false);

    if authorized {
        // forward to the actual handler
        let res: ServiceResponse<BoxBody> = next.call(req).await?;
        Ok(res)
    } else {
        // short-circuit with 401
        Err(ErrorUnauthorized("Invalid or missing API token"))
    }
}

/// Helper: find the best MPRIS player to control, prioritizing the preferred player.
fn find_player() -> Option<Player> {
    let pf = PlayerFinder::new().ok()?;
    let all = pf.find_all().ok()?;
    let preferred_player = get_preferred_player();

    // Filter out our own "My Player" service
    let external_players: Vec<_> = all
        .into_iter()
        .filter(|p| p.identity() != "My Player")
        .collect();

    if external_players.is_empty() {
        println!("No external MPRIS players found");
        return None;
    }

    // Find the best player based on priority
    let mut selected_player = None;
    let mut selection_reason = String::new();

    // First priority: Look for the preferred player (default: chromium)
    for player in &external_players {
        if player.identity().to_lowercase().contains(&preferred_player) {
            selected_player = Some(player);
            selection_reason = format!(
                "Found preferred player '{}': {}",
                preferred_player,
                player.identity()
            );
            break;
        }
    }

    // If preferred is "chromium" and not found, try "chrome" as fallback
    if selected_player.is_none() && preferred_player == "chromium" {
        for player in &external_players {
            if player.identity().to_lowercase().contains("chrome") {
                selected_player = Some(player);
                selection_reason = format!(
                    "Found Chrome player as Chromium fallback: {}",
                    player.identity()
                );
                break;
            }
        }
    }

    // Final fallback: Use the first available player
    if selected_player.is_none() {
        if let Some(fallback) = external_players.first() {
            selected_player = Some(fallback);
            selection_reason = format!(
                "Using fallback player (preferred '{}' not found): {}",
                preferred_player,
                fallback.identity()
            );
        }
    }

    if let Some(player) = selected_player {
        println!("{selection_reason}");
        // Find the index and return the owned player
        let identity = player.identity().to_string();
        return external_players
            .into_iter()
            .find(|p| p.identity() == identity);
    }

    None
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
                eprintln!("Failed to get playback status: {e}");
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
        Ok(s) => HttpResponse::InternalServerError().body(format!("pactl exited with {s}")),
        Err(e) => HttpResponse::InternalServerError().body(format!("failed to launch pactl: {e}")),
    }
}

/// POST /volume_down — lower the system volume by 5%
async fn volume_down(_state: web::Data<AppState>) -> impl Responder {
    let status = Command::new("pactl")
        .args(["set-sink-volume", "@DEFAULT_SINK@", "-5%"])
        .status();

    match status {
        Ok(s) if s.success() => HttpResponse::Ok().body("system volume -5%"),
        Ok(s) => HttpResponse::InternalServerError().body(format!("pactl exited with {s}")),
        Err(e) => HttpResponse::InternalServerError().body(format!("failed to launch pactl: {e}")),
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
    // Ask the other player and get its identity
    let player = find_player();
    let other_pb = player
        .as_ref()
        .and_then(|p| p.get_playback_status().ok())
        .map(|s| format!("{s:?}"));
    let controlled_player = player.as_ref().map(|p| p.identity().to_string());

    // Read your last‐set title
    let title = {
        let md = state.copy_meta.lock().unwrap();
        md.title.as_ref().map(|cow| cow.to_string())
    };

    let resp = Status {
        our_playback: our_pb,
        other_playback: other_pb,
        title,
        controlled_player,
    };
    HttpResponse::Ok().json(resp)
}
