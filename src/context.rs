use std::env;
use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityKind {
    Coding,
    MusicPlaying,
    MediaVideo,
    Gaming,
    Browsing,
    Chatting,
    Idle,
    General,
}

#[derive(Debug, Clone)]
pub struct WindowLedge {
    pub address: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub class: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaSource {
    SpotifyMusic,
    YouTubeVideo,
    OtherMusic,
    OtherVideo,
}

#[derive(Debug, Clone)]
pub struct ScreenContext {
    pub screen_w: i32,
    pub screen_h: i32,
    pub reserved_top: i32,
    pub active_workspace_id: i64,
    pub active_class: String,
    pub active_title: String,
    pub activity: ActivityKind,
    pub music_playing: bool,
    pub video_playing: bool,
    pub media_source: Option<MediaSource>,
    pub media_label: Option<String>,
    pub cursor_x: i32,
    pub cursor_y: i32,
    pub cursor_speed: f32,
    pub user_idle_secs: f32,
    pub ledges: Vec<WindowLedge>,
    pub last_event_seq: u64,
    pub last_event_kind: Option<ScreenEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScreenEvent {
    WindowChanged { class: String, title: String },
    WindowOpened,
    WorkspaceChanged,
    MusicStarted(String),
    MusicStopped,
    VideoStarted(String),
    VideoStopped,
}

impl Default for ScreenContext {
    fn default() -> Self {
        Self {
            screen_w: 1536,
            screen_h: 960,
            reserved_top: 34,
            active_workspace_id: 1,
            active_class: String::new(),
            active_title: String::new(),
            activity: ActivityKind::General,
            music_playing: false,
            video_playing: false,
            media_source: None,
            media_label: None,
            cursor_x: 768,
            cursor_y: 480,
            cursor_speed: 0.0,
            user_idle_secs: 0.0,
            ledges: Vec::new(),
            last_event_seq: 0,
            last_event_kind: None,
        }
    }
}

#[derive(Debug, Deserialize)]
struct HyprMonitor {
    width: i32,
    height: i32,
    scale: f32,
    focused: bool,
    reserved: [i32; 4],
    #[serde(rename = "activeWorkspace")]
    active_workspace: HyprWorkspaceRef,
}

#[derive(Debug, Deserialize)]
struct HyprWorkspaceRef {
    id: i64,
}

#[derive(Debug, Deserialize)]
struct HyprClient {
    address: String,
    mapped: bool,
    hidden: bool,
    at: [i32; 2],
    size: [i32; 2],
    workspace: HyprWorkspaceRef,
    class: String,
    fullscreen: i32,
}

#[derive(Debug, Deserialize)]
struct HyprActiveWindow {
    #[serde(default)]
    class: String,
    #[serde(default)]
    title: String,
}

pub fn spawn_context_watcher() -> Arc<Mutex<ScreenContext>> {
    let shared = Arc::new(Mutex::new(ScreenContext::default()));

    // Thread 1: Hyprland socket2 listener for instant window/workspace reaction events
    let socket_shared = Arc::clone(&shared);
    thread::spawn(move || {
        run_socket2_listener(socket_shared);
    });

    // Thread 2: Geometry, cursor, and MPRIS poller
    let poll_shared = Arc::clone(&shared);
    thread::spawn(move || {
        run_poller(poll_shared);
    });

    shared
}

fn hypr_socket2_path() -> Option<PathBuf> {
    let runtime = env::var_os("XDG_RUNTIME_DIR")?;
    let sig = env::var_os("HYPRLAND_INSTANCE_SIGNATURE")?;
    Some(PathBuf::from(runtime).join("hypr").join(sig).join(".socket2.sock"))
}

fn run_socket2_listener(shared: Arc<Mutex<ScreenContext>>) {
    loop {
        if let Some(path) = hypr_socket2_path() {
            if let Ok(stream) = UnixStream::connect(&path) {
                let reader = BufReader::new(stream);
                for line in reader.lines().map_while(Result::ok) {
                    handle_socket2_line(&line, &shared);
                }
            }
        }
        thread::sleep(Duration::from_secs(2));
    }
}

fn handle_socket2_line(line: &str, shared: &Arc<Mutex<ScreenContext>>) {
    let Some((event, payload)) = line.split_once(">>") else {
        return;
    };
    let mut ctx = shared.lock().unwrap();
    match event {
        "activewindow" => {
            if let Some((class, title)) = payload.split_once(',') {
                if class != ctx.active_class || title != ctx.active_title {
                    ctx.active_class = class.to_string();
                    ctx.active_title = title.to_string();
                    ctx.user_idle_secs = 0.0;
                    ctx.last_event_seq += 1;
                    ctx.last_event_kind = Some(ScreenEvent::WindowChanged {
                        class: ctx.active_class.clone(),
                        title: ctx.active_title.clone(),
                    });
                }
            }
        }
        "openwindow" => {
            ctx.user_idle_secs = 0.0;
            ctx.last_event_seq += 1;
            ctx.last_event_kind = Some(ScreenEvent::WindowOpened);
        }
        "workspace" | "workspacev2" => {
            ctx.user_idle_secs = 0.0;
            ctx.last_event_seq += 1;
            ctx.last_event_kind = Some(ScreenEvent::WorkspaceChanged);
        }
        _ => {}
    }
}

fn run_poller(shared: Arc<Mutex<ScreenContext>>) {
    let mut last_cursor = (0i32, 0i32);
    let mut last_cursor_move = Instant::now();
    let mut last_media_check = Instant::now() - Duration::from_secs(10);
    let mut last_clients_check = Instant::now() - Duration::from_secs(10);
    let mut prev_music = false;
    let mut prev_video = false;

    loop {
        let now = Instant::now();

        // Fast cursor poll (~12 Hz) for smooth cursor following & drag tracking
        if let Some((cx, cy)) = query_cursorpos() {
            let dx = (cx - last_cursor.0) as f32;
            let dy = (cy - last_cursor.1) as f32;
            let dist = (dx * dx + dy * dy).sqrt();
            let dt = now.duration_since(last_cursor_move).as_secs_f32().max(0.05);
            if dist > 2.0 {
                last_cursor_move = now;
            }
            let mut ctx = shared.lock().unwrap();
            ctx.cursor_x = cx;
            ctx.cursor_y = cy;
            ctx.cursor_speed = if dist > 2.0 { dist / dt } else { 0.0 };
            ctx.user_idle_secs = now.duration_since(last_cursor_move).as_secs_f32();
            last_cursor = (cx, cy);
        }

        // Window & monitor geometry poll (~4 Hz)
        if now.duration_since(last_clients_check) >= Duration::from_millis(250) {
            last_clients_check = now;
            let mon = query_focused_monitor();
            let clients = query_clients();
            let active = query_active_window();

            let mut ctx = shared.lock().unwrap();
            if let Some(m) = mon {
                let scale = if m.scale > 0.1 { m.scale } else { 1.0 };
                ctx.screen_w = (m.width as f32 / scale).round() as i32;
                ctx.screen_h = (m.height as f32 / scale).round() as i32;
                ctx.reserved_top = m.reserved[1];
                ctx.active_workspace_id = m.active_workspace.id;
            }

            if let Some(act) = active {
                if !act.class.is_empty() {
                    ctx.active_class = act.class;
                    ctx.active_title = act.title;
                }
            }

            let ws_id = ctx.active_workspace_id;
            let reserved_top = ctx.reserved_top;
            ctx.ledges = clients
                .into_iter()
                .filter(|c| {
                    c.mapped
                        && !c.hidden
                        && c.fullscreen == 0
                        && c.workspace.id == ws_id
                        && c.size[0] >= 160
                        && c.at[1] >= reserved_top + 24
                })
                .map(|c| WindowLedge {
                    address: c.address,
                    x: c.at[0],
                    y: c.at[1],
                    width: c.size[0],
                    class: c.class,
                })
                .collect();

            ctx.activity = classify_activity(
                &ctx.active_class,
                &ctx.active_title,
                ctx.music_playing,
                ctx.video_playing,
                ctx.user_idle_secs,
            );
        }

        // MPRIS media poll (~0.7 Hz) distinguishing Spotify music vs YouTube videos
        if now.duration_since(last_media_check) >= Duration::from_millis(1400) {
            last_media_check = now;
            let (active_class, active_title) = {
                let ctx = shared.lock().unwrap();
                (ctx.active_class.clone(), ctx.active_title.clone())
            };
            let media = query_media_players(&active_class, &active_title);
            let mut ctx = shared.lock().unwrap();
            ctx.music_playing = media.music_playing;
            ctx.video_playing = media.video_playing;
            ctx.media_source = media.primary_source;
            ctx.media_label = media.label.clone();

            if media.music_playing && !prev_music {
                ctx.last_event_seq += 1;
                ctx.last_event_kind = Some(ScreenEvent::MusicStarted(
                    media.music_label.unwrap_or_else(|| "Spotify".to_string()),
                ));
            } else if media.video_playing && !prev_video {
                ctx.last_event_seq += 1;
                ctx.last_event_kind = Some(ScreenEvent::VideoStarted(
                    media.video_label.unwrap_or_else(|| "YouTube".to_string()),
                ));
            } else if !media.music_playing && prev_music {
                ctx.last_event_seq += 1;
                ctx.last_event_kind = Some(ScreenEvent::MusicStopped);
            } else if !media.video_playing && prev_video {
                ctx.last_event_seq += 1;
                ctx.last_event_kind = Some(ScreenEvent::VideoStopped);
            }
            prev_music = media.music_playing;
            prev_video = media.video_playing;
            ctx.activity = classify_activity(
                &ctx.active_class,
                &ctx.active_title,
                ctx.music_playing,
                ctx.video_playing,
                ctx.user_idle_secs,
            );
        }

        // ~40 Hz cursor poll for smooth dragging & proportional flick velocity
        thread::sleep(Duration::from_millis(25));
    }
}

fn is_gaming_window(c: &str, t: &str) -> bool {
    c.contains("steam")
        || c.contains("gamescope")
        || c.contains("lutris")
        || c.contains("heroic")
        || c.contains("prism")
        || c.contains("minecraft")
        || c.contains("godot")
        || c.contains("retroarch")
        || c.contains("pcsx2")
        || c.contains("dolphin-emu")
        || c.contains("ryujinx")
        || c.contains("bottles")
        || c.contains("proton")
        || t.contains(".exe")
        || t.contains("albion")
        || t.contains("elden")
}

pub fn classify_activity(
    class: &str,
    title: &str,
    music_playing: bool,
    video_playing: bool,
    idle_secs: f32,
) -> ActivityKind {
    let c = class.to_lowercase();
    let t = title.to_lowercase();
    let gaming = is_gaming_window(&c, &t);

    if gaming {
        return ActivityKind::Gaming;
    }

    if idle_secs > 45.0 && !music_playing && !video_playing {
        return ActivityKind::Idle;
    }

    if t.contains("youtube")
        || t.contains("twitch")
        || t.contains("netflix")
        || t.contains("crunchyroll")
        || t.contains("bilibili")
        || c.contains("mpv")
        || c.contains("vlc")
    {
        return ActivityKind::MediaVideo;
    }

    if video_playing && !music_playing {
        return ActivityKind::MediaVideo;
    }

    if music_playing || c.contains("spotify") {
        return ActivityKind::MusicPlaying;
    }

    if c.contains("kitty")
        || c.contains("alacritty")
        || c.contains("wezterm")
        || c.contains("foot")
        || c.contains("ghostty")
        || c.contains("code")
        || c.contains("cursor")
        || c.contains("zed")
        || c.contains("jetbrains")
        || t.contains("nvim")
        || t.contains("vim")
        || t.contains("cargo")
        || t.contains(".rs")
        || t.contains("agy")
    {
        return ActivityKind::Coding;
    }

    if c.contains("discord")
        || c.contains("vesktop")
        || c.contains("telegram")
        || c.contains("slack")
        || c.contains("element")
    {
        return ActivityKind::Chatting;
    }

    if c.contains("brave")
        || c.contains("firefox")
        || c.contains("chromium")
        || c.contains("zen")
        || c.contains("vivaldi")
    {
        return ActivityKind::Browsing;
    }

    ActivityKind::General
}

fn query_cursorpos() -> Option<(i32, i32)> {
    let out = Command::new("hyprctl").arg("cursorpos").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout);
    let (x, y) = s.trim().split_once(',')?;
    Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
}

fn query_focused_monitor() -> Option<HyprMonitor> {
    let out = Command::new("hyprctl").args(["-j", "monitors"]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let mons: Vec<HyprMonitor> = serde_json::from_slice(&out.stdout).ok()?;
    mons.into_iter().find(|m| m.focused)
}

fn query_clients() -> Vec<HyprClient> {
    let Ok(out) = Command::new("hyprctl").args(["-j", "clients"]).output() else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    serde_json::from_slice(&out.stdout).unwrap_or_default()
}

fn query_active_window() -> Option<HyprActiveWindow> {
    let out = Command::new("hyprctl").args(["-j", "activewindow"]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    serde_json::from_slice(&out.stdout).ok()
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct MediaSnapshot {
    pub music_playing: bool,
    pub video_playing: bool,
    pub primary_source: Option<MediaSource>,
    pub label: Option<String>,
    pub music_label: Option<String>,
    pub video_label: Option<String>,
}

pub fn classify_playerctl_line(
    line: &str,
    active_class: &str,
    active_title: &str,
) -> Option<(MediaSource, bool, String)> {
    let mut parts = line.splitn(5, '\t');
    let player = parts.next()?.trim().to_lowercase();
    let status = parts.next()?.trim();
    let url = parts.next().unwrap_or("").trim().to_lowercase();
    let artist = parts.next().unwrap_or("").trim();
    let title = parts.next().unwrap_or("").trim();

    let playing = status.eq_ignore_ascii_case("Playing");
    let label = match (artist.is_empty(), title.is_empty()) {
        (false, false) => format!("{artist} - {title}"),
        (true, false) => title.to_string(),
        (false, true) => artist.to_string(),
        (true, true) => player.clone(),
    };

    let source = if player.contains("spotify")
        || player.contains("ncspot")
        || url.contains("open.spotify.com")
        || url.starts_with("spotify:")
    {
        MediaSource::SpotifyMusic
    } else if url.contains("music.youtube.com")
        || player.contains("mpd")
        || player.contains("rhythmbox")
        || player.contains("clementine")
        || player.contains("elisa")
        || player.contains("audacious")
    {
        MediaSource::OtherMusic
    } else if url.contains("youtube.com")
        || url.contains("youtu.be")
        || ((player.contains("brave")
            || player.contains("chromium")
            || player.contains("firefox")
            || player.contains("zen")
            || player.contains("vivaldi")
            || player.contains("edge"))
            && (active_title.to_lowercase().contains("youtube")
                || active_class.to_lowercase().contains(&player)
                || !url.contains("spotify")))
    {
        MediaSource::YouTubeVideo
    } else {
        MediaSource::OtherVideo
    };

    Some((source, playing, label))
}

pub fn parse_media_players_output(
    output: &str,
    active_class: &str,
    active_title: &str,
) -> MediaSnapshot {
    let mut snap = MediaSnapshot::default();
    for line in output.lines() {
        let Some((source, playing, label)) =
            classify_playerctl_line(line, active_class, active_title)
        else {
            continue;
        };
        if !playing {
            continue;
        }
        match source {
            MediaSource::SpotifyMusic | MediaSource::OtherMusic => {
                if !snap.music_playing || source == MediaSource::SpotifyMusic {
                    snap.music_playing = true;
                    snap.music_label = Some(label.clone());
                    if snap.primary_source.is_none() || source == MediaSource::SpotifyMusic {
                        snap.primary_source = Some(source);
                        snap.label = Some(label);
                    }
                }
            }
            MediaSource::YouTubeVideo | MediaSource::OtherVideo => {
                if !snap.video_playing || source == MediaSource::YouTubeVideo {
                    snap.video_playing = true;
                    snap.video_label = Some(label.clone());
                    if !snap.music_playing
                        || active_title.to_lowercase().contains("youtube")
                    {
                        snap.primary_source = Some(source);
                        snap.label = Some(label);
                    }
                }
            }
        }
    }
    snap
}

fn query_media_players(active_class: &str, active_title: &str) -> MediaSnapshot {
    let Ok(out) = Command::new("playerctl")
        .args([
            "-a",
            "metadata",
            "--format",
            "{{playerName}}\t{{status}}\t{{xesam:url}}\t{{artist}}\t{{title}}",
        ])
        .output()
    else {
        return MediaSnapshot::default();
    };
    if !out.status.success() {
        return MediaSnapshot::default();
    }
    let text = String::from_utf8_lossy(&out.stdout);
    parse_media_players_output(&text, active_class, active_title)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_activities_accurately() {
        assert_eq!(
            classify_activity("kitty", "nvim src/main.rs", false, false, 1.0),
            ActivityKind::Coding
        );
        assert_eq!(
            classify_activity("Spotify", "Color Your Night", true, false, 2.0),
            ActivityKind::MusicPlaying
        );
        assert_eq!(
            classify_activity("brave-browser", "YouTube - Rust", false, true, 1.0),
            ActivityKind::MediaVideo
        );
        assert_eq!(
            classify_activity("kitty", "bash", false, true, 1.0),
            ActivityKind::MediaVideo
        );
        assert_eq!(
            classify_activity("discord", "General", false, false, 1.0),
            ActivityKind::Chatting
        );
        assert_eq!(
            classify_activity("kitty", "bash", false, false, 60.0),
            ActivityKind::Idle
        );
        assert_eq!(
            classify_activity("Spotify", "Track", true, false, 120.0),
            ActivityKind::MusicPlaying
        );
        assert_eq!(
            classify_activity("steam_app_1245620", "ELDEN RING", true, false, 90.0),
            ActivityKind::Gaming
        );
        assert_eq!(
            classify_activity("gamescope", "Hollow Knight", false, false, 120.0),
            ActivityKind::Gaming
        );
    }

    #[test]
    fn distinguishes_spotify_music_from_youtube_videos() {
        let spotify_only =
            "brave\tPaused\thttps://www.youtube.com/watch?v=123\tCreator\tAlbion Guide\n\
             spotify\tPlaying\thttps://open.spotify.com/track/abc\tWu-Tang Clan\tC.R.E.A.M.";
        let snap1 = parse_media_players_output(spotify_only, "kitty", "agy");
        assert!(snap1.music_playing);
        assert!(!snap1.video_playing);
        assert_eq!(snap1.primary_source, Some(MediaSource::SpotifyMusic));

        let yt_only =
            "spotify\tPaused\thttps://open.spotify.com/track/abc\tWu-Tang Clan\tC.R.E.A.M.\n\
             brave\tPlaying\thttps://www.youtube.com/watch?v=123\tJosh Strife Hayes\tAlbion Review";
        let snap2 = parse_media_players_output(yt_only, "brave-browser", "YouTube");
        assert!(!snap2.music_playing);
        assert!(snap2.video_playing);
        assert_eq!(snap2.primary_source, Some(MediaSource::YouTubeVideo));

        let yt_music =
            "brave\tPlaying\thttps://music.youtube.com/watch?v=xyz\tArtist\tBGM";
        let snap3 = parse_media_players_output(yt_music, "brave-browser", "YouTube Music");
        assert!(snap3.music_playing);
        assert!(!snap3.video_playing);
        assert_eq!(snap3.primary_source, Some(MediaSource::OtherMusic));
    }
}
