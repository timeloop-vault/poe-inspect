use tauri::Emitter;

const UPDATER_STABLE: &str =
    "https://github.com/timeloop-vault/poe-inspect/releases/latest/download/latest.json";
// TODO: Beta channel requires GitHub Pages (paid plan for private repos).
// When enabled, deploy updater/beta.json to gh-pages and update this URL to:
// "https://timeloop-vault.github.io/poe-inspect/updater/beta.json"
const UPDATER_BETA: &str =
    "https://github.com/timeloop-vault/poe-inspect/releases/latest/download/latest.json";

fn updater_endpoint(channel: &str) -> &'static str {
    if channel == "beta" {
        UPDATER_BETA
    } else {
        UPDATER_STABLE
    }
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateInfo {
    version: String,
    date: Option<String>,
    body: Option<String>,
}

/// Check for application updates on the given channel ("stable" or "beta").
///
/// Returns update info (version, date, body) if an update is available,
/// or `null` if the app is up to date.
#[tauri::command]
pub(crate) async fn check_for_update(
    app: tauri::AppHandle,
    channel: String,
) -> Result<Option<UpdateInfo>, String> {
    use tauri_plugin_updater::UpdaterExt;

    let endpoint = updater_endpoint(&channel);
    eprintln!("[updater] Checking {channel} channel: {endpoint}");

    let updater = app
        .updater_builder()
        .endpoints(vec![endpoint.parse().unwrap()])
        .map_err(|e| e.to_string())?
        .build()
        .map_err(|e| e.to_string())?;

    match updater.check().await {
        Ok(Some(update)) => {
            eprintln!("[updater] Update available: {}", update.version);
            Ok(Some(UpdateInfo {
                version: update.version.clone(),
                date: update.date.map(|d| d.to_string()),
                body: update.body.clone(),
            }))
        }
        Ok(None) => {
            eprintln!("[updater] No update available");
            Ok(None)
        }
        Err(e) => {
            eprintln!("[updater] Check failed: {e}");
            Err(e.to_string())
        }
    }
}

/// Download and install an available update. Re-checks the given channel,
/// emits "update-progress" events during download, then installs.
#[tauri::command]
pub(crate) async fn download_and_install_update(
    app: tauri::AppHandle,
    channel: String,
) -> Result<(), String> {
    use tauri_plugin_updater::UpdaterExt;

    let endpoint = updater_endpoint(&channel);
    let updater = app
        .updater_builder()
        .endpoints(vec![endpoint.parse().unwrap()])
        .map_err(|e| e.to_string())?
        .build()
        .map_err(|e| e.to_string())?;

    let update = updater
        .check()
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No update available".to_string())?;

    eprintln!("[updater] Downloading {}", update.version);

    let app_handle = app.clone();
    let app_handle2 = app.clone();
    update
        .download_and_install(
            move |chunk_length, content_length| {
                let _ = app_handle.emit(
                    "update-progress",
                    serde_json::json!({
                        "event": "Progress",
                        "data": {
                            "chunkLength": chunk_length,
                            "contentLength": content_length,
                        }
                    }),
                );
            },
            move || {
                let _ = app_handle2.emit(
                    "update-progress",
                    serde_json::json!({ "event": "Finished", "data": {} }),
                );
            },
        )
        .await
        .map_err(|e| e.to_string())?;

    eprintln!("[updater] Update installed, restart required");
    Ok(())
}
