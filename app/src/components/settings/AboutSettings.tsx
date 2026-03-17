import { invoke } from "@tauri-apps/api/core";
import { relaunch } from "@tauri-apps/plugin-process";
import { check } from "@tauri-apps/plugin-updater";
import { useCallback, useState } from "preact/hooks";

interface UpdateInfo {
	version: string;
	date: string | null;
	body: string | null;
}

type UpdateState =
	| { status: "idle" }
	| { status: "checking" }
	| { status: "available"; info: UpdateInfo }
	| { status: "downloading"; percent: number }
	| { status: "ready" }
	| { status: "up-to-date" }
	| { status: "error"; message: string };

export function AboutSettings() {
	const [state, setState] = useState<UpdateState>({ status: "idle" });

	const checkForUpdate = useCallback(async () => {
		setState({ status: "checking" });
		try {
			const info = await invoke<UpdateInfo | null>("check_for_update");
			if (info) {
				setState({ status: "available", info });
			} else {
				setState({ status: "up-to-date" });
			}
		} catch (e) {
			setState({ status: "error", message: String(e) });
		}
	}, []);

	const downloadAndInstall = useCallback(async () => {
		setState({ status: "downloading", percent: 0 });
		try {
			const update = await check();
			if (!update) {
				setState({ status: "up-to-date" });
				return;
			}
			let totalBytes = 0;
			let downloadedBytes = 0;
			await update.downloadAndInstall((progress) => {
				if (progress.event === "Started" && progress.data.contentLength) {
					totalBytes = progress.data.contentLength;
				} else if (progress.event === "Progress") {
					downloadedBytes += progress.data.chunkLength;
					const pct = totalBytes > 0 ? Math.round((downloadedBytes / totalBytes) * 100) : 0;
					setState({ status: "downloading", percent: pct });
				} else if (progress.event === "Finished") {
					setState({ status: "ready" });
				}
			});
			setState({ status: "ready" });
		} catch (e) {
			setState({ status: "error", message: String(e) });
		}
	}, []);

	const doRelaunch = useCallback(async () => {
		await relaunch();
	}, []);

	return (
		<>
			<h2>About</h2>

			<div class="setting-group">
				<h3>PoE Inspect</h3>
				<div class="setting-row">
					<div class="setting-label">Version</div>
					<div class="setting-value">0.1.0</div>
				</div>
				<div class="setting-description">Real-time item evaluation overlay for Path of Exile.</div>
			</div>

			<div class="setting-group">
				<h3>Updates</h3>

				{state.status === "idle" && (
					<button type="button" class="setting-btn" onClick={checkForUpdate}>
						Check for Updates
					</button>
				)}

				{state.status === "checking" && (
					<div class="setting-description">Checking for updates...</div>
				)}

				{state.status === "up-to-date" && (
					<>
						<div class="setting-description" style={{ color: "#5a5" }}>
							You are on the latest version.
						</div>
						<button
							type="button"
							class="setting-btn"
							style={{ marginTop: "8px" }}
							onClick={checkForUpdate}
						>
							Check Again
						</button>
					</>
				)}

				{state.status === "available" && (
					<>
						<div class="setting-description">
							Update available: <strong>{state.info.version}</strong>
						</div>
						{state.info.body && (
							<div class="setting-description" style={{ marginTop: "4px", opacity: 0.8 }}>
								{state.info.body}
							</div>
						)}
						<button
							type="button"
							class="setting-btn"
							style={{ marginTop: "8px" }}
							onClick={downloadAndInstall}
						>
							Download &amp; Install
						</button>
					</>
				)}

				{state.status === "downloading" && (
					<div class="setting-description">Downloading... {state.percent}%</div>
				)}

				{state.status === "ready" && (
					<>
						<div class="setting-description" style={{ color: "#5a5" }}>
							Update installed. Restart to apply.
						</div>
						<button
							type="button"
							class="setting-btn"
							style={{ marginTop: "8px" }}
							onClick={doRelaunch}
						>
							Restart Now
						</button>
					</>
				)}

				{state.status === "error" && (
					<>
						<div class="setting-description" style={{ color: "#e04040" }}>
							Update check failed: {state.message}
						</div>
						<button
							type="button"
							class="setting-btn"
							style={{ marginTop: "8px" }}
							onClick={checkForUpdate}
						>
							Retry
						</button>
					</>
				)}
			</div>
		</>
	);
}
