mod cache;
mod configedit;
mod discover;
mod history;
mod mounts;
mod parse;
mod purge;
mod rc;
mod rclone;
mod schedule;
mod setup;
mod transfers;
mod wizard;

use cache::CacheReport;
use configedit::{ProviderInfo, RemoteConfig, RemoteWrite};
use discover::RcloneInstance;
use history::HistoryEvent;
use mounts::MountProfile;
use rc::{RcProbe, RcTarget};
use rclone::RcloneInfo;
use schedule::BandwidthSchedule;
use transfers::ObservedTransfers;
use wizard::RemoteDraft;

#[tauri::command]
fn rclone_info() -> RcloneInfo {
    rclone::rclone_info()
}

#[tauri::command]
async fn install_rclone() -> Result<RcloneInfo, String> {
    tauri::async_runtime::spawn_blocking(rclone::install_rclone)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
fn list_remotes() -> Result<Vec<String>, String> {
    rclone::list_remotes()
}

#[tauri::command]
fn list_instances() -> Vec<RcloneInstance> {
    discover::list_instances()
}

#[tauri::command]
fn stop_instance(pid: u32) -> Result<(), String> {
    discover::stop_pid(pid)
}

#[tauri::command]
async fn probe_rc(targets: Vec<RcTarget>) -> Vec<RcProbe> {
    rc::probe_many(targets).await
}

#[tauri::command]
fn observe_transfers() -> ObservedTransfers {
    transfers::observe_transfers()
}

#[tauri::command]
fn analyze_cache() -> CacheReport {
    cache::analyze_cache()
}

#[tauri::command]
fn list_mounts() -> Result<Vec<MountProfile>, String> {
    mounts::load_profiles()
}

#[tauri::command]
fn save_mount(profile: MountProfile) -> Result<MountProfile, String> {
    mounts::upsert_profile(profile)
}

#[tauri::command]
fn delete_mount(id: String) -> Result<(), String> {
    mounts::delete_profile(&id)
}

#[tauri::command]
async fn apply_signed_update(app: tauri::AppHandle, manifest_url: String) -> Result<(), String> {
    use tauri_plugin_updater::UpdaterExt;
    let url = manifest_url
        .parse()
        .map_err(|_| "The update address is not valid.".to_string())?;
    let updater = app
        .updater_builder()
        .endpoints(vec![url])
        .map_err(|e| e.to_string())?
        .build()
        .map_err(|e| e.to_string())?;
    let update = updater.check().await.map_err(|e| e.to_string())?;
    let Some(update) = update else {
        return Ok(());
    };
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn start_mount(id: String) -> Result<u32, String> {
    mounts::start_mount(&id)
}

#[tauri::command]
fn preview_mount(profile: MountProfile) -> Result<String, String> {
    mounts::preview_command(profile)
}

#[tauri::command]
fn cache_advice() -> mounts::CacheAdvice {
    mounts::cache_advice()
}

#[tauri::command]
fn list_history() -> Vec<HistoryEvent> {
    history::list()
}

#[tauri::command]
fn note_failures(messages: Vec<String>) -> Vec<HistoryEvent> {
    history::note_failures(&messages);
    history::list()
}

#[tauri::command]
fn load_bandwidth() -> BandwidthSchedule {
    schedule::load()
}

#[tauri::command]
fn save_bandwidth(schedule: BandwidthSchedule) -> Result<BandwidthSchedule, String> {
    schedule::save(schedule)
}

#[tauri::command]
async fn apply_bandwidth() -> Result<String, String> {
    let saved = schedule::load();
    if !saved.enabled {
        return Ok("Schedule is off.".into());
    }
    let rate = schedule::current_rate(&saved).unwrap_or_else(|| "off".into());
    let mut applied = 0u32;
    for inst in discover::list_instances() {
        if let Some(addr) = inst.rc_addr {
            if rc::set_bwlimit(&addr, &rate).await.is_ok() {
                applied += 1;
            }
        }
    }
    Ok(format!("{rate} on {applied} running rclone processes."))
}

#[tauri::command]
fn create_remote(draft: RemoteDraft) -> Result<String, String> {
    wizard::create_remote(draft)
}

#[tauri::command]
async fn remote_providers() -> Result<Vec<ProviderInfo>, String> {
    tauri::async_runtime::spawn_blocking(configedit::providers)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn remote_configs() -> Result<Vec<RemoteConfig>, String> {
    tauri::async_runtime::spawn_blocking(configedit::dump_remotes)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn save_remote(body: RemoteWrite) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || configedit::save_remote(body))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn delete_remote(name: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || configedit::delete_remote(name))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn reconnect_remote(name: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || configedit::reconnect_remote(name))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn preview_cache_clear(
    app: tauri::AppHandle,
    mode: String,
    older_than_days: u32,
) -> Result<purge::CachePreview, String> {
    tauri::async_runtime::spawn_blocking(move || purge::preview(app, mode, older_than_days))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn apply_cache_clear(app: tauri::AppHandle) -> Result<purge::CachePreview, String> {
    tauri::async_runtime::spawn_blocking(move || purge::apply(app))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
fn cancel_cache_clear() {
    purge::cancel();
}

#[tauri::command]
fn setup_status() -> setup::SetupState {
    setup::status()
}

#[tauri::command]
fn complete_setup(autostart: bool) -> Result<setup::SetupState, String> {
    setup::complete(autostart)
}

static QUITTING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn notify_still_running() {
    std::thread::spawn(|| {
        let script = r#"
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$n = New-Object System.Windows.Forms.NotifyIcon
$n.Icon = [System.Drawing.SystemIcons]::Information
$n.Visible = $true
$n.BalloonTipTitle = 'LiClone is still open'
$n.BalloonTipText = 'It is in the tray. Click the tray icon to bring it back.'
$n.ShowBalloonTip(5000)
Start-Sleep -Seconds 6
$n.Dispose()
"#;
        let mut cmd = rclone::hidden_command("powershell");
        cmd.args(["-NoProfile", "-NonInteractive", "-Command", script]);
        let _ = cmd.output();
    });
}

fn install_tray(app: &tauri::App) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
    use tauri::Manager;

    let show = MenuItem::with_id(app, "show", "Show LiClone", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;
    let mut builder = TrayIconBuilder::new()
        .tooltip("Wroom LiClone")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                }
            }
            "quit" => {
                QUITTING.store(true, std::sync::atomic::Ordering::SeqCst);
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if let Some(window) = tray.app_handle().get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.unminimize();
                    let _ = window.set_focus();
                }
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    let tray = builder.build(app)?;
    let handle = app.handle().clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(30));
        let Some(observed) = transfers::cached_observation() else {
            continue;
        };
        let waiting = observed.transferring.len();
        let tip = if waiting == 0 {
            "Wroom LiClone — nothing waiting to upload".to_string()
        } else {
            format!("Wroom LiClone — {waiting} on disk, not on the cloud yet")
        };
        if let Some(icon) = handle.tray_by_id(tray.id()) {
            let _ = icon.set_tooltip(Some(tip));
        } else {
            break;
        }
    });
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            use tauri::Manager;
            install_tray(app)?;
            setup::refresh_tray_launch();
            let tray_launch = std::env::args().any(|arg| arg == "--tray");
            if let Some(window) = app.get_webview_window("main") {
                if !tray_launch {
                    let _ = window.show();
                }
                let handle = app.handle().clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        if QUITTING.load(std::sync::atomic::Ordering::SeqCst) {
                            return;
                        }
                        api.prevent_close();
                        if let Some(window) = handle.get_webview_window("main") {
                            let _ = window.hide();
                        }
                        notify_still_running();
                    }
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            rclone_info,
            apply_signed_update,
            install_rclone,
            list_remotes,
            list_instances,
            stop_instance,
            probe_rc,
            observe_transfers,
            analyze_cache,
            list_mounts,
            save_mount,
            delete_mount,
            start_mount,
            preview_mount,
            cache_advice,
            list_history,
            note_failures,
            load_bandwidth,
            save_bandwidth,
            apply_bandwidth,
            create_remote,
            remote_providers,
            remote_configs,
            save_remote,
            delete_remote,
            reconnect_remote,
            preview_cache_clear,
            apply_cache_clear,
            cancel_cache_clear,
            setup_status,
            complete_setup
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
