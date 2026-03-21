import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { relaunch } from "@tauri-apps/plugin-process";
import { useCallback, useEffect, useState } from "preact/hooks";
import { type UpdateChannel, loadGeneral } from "../../store";

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

async function getChannel(): Promise<UpdateChannel> {
	const settings = await loadGeneral();
	return settings.updateChannel ?? "stable";
}

export function AboutSettings() {
	const [state, setState] = useState<UpdateState>({ status: "idle" });
	const [appVersion, setAppVersion] = useState("...");

	useEffect(() => {
		getVersion().then(setAppVersion);
	}, []);

	const checkForUpdate = useCallback(async () => {
		setState({ status: "checking" });
		try {
			const channel = await getChannel();
			const info = await invoke<UpdateInfo | null>("check_for_update", { channel });
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
			let totalBytes = 0;
			let downloadedBytes = 0;

			const unlisten = await listen<{ event: string; data: Record<string, number> }>(
				"update-progress",
				(event) => {
					const { event: kind, data } = event.payload;
					if (kind === "Progress" && data.chunkLength != null) {
						downloadedBytes += data.chunkLength;
						if (data.contentLength != null) totalBytes = data.contentLength;
						const pct = totalBytes > 0 ? Math.round((downloadedBytes / totalBytes) * 100) : 0;
						setState({ status: "downloading", percent: pct });
					} else if (kind === "Finished") {
						setState({ status: "ready" });
					}
				},
			);

			const channel = await getChannel();
			await invoke("download_and_install_update", { channel });
			unlisten();
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
					<div class="setting-value">{appVersion}</div>
				</div>
				<div class="setting-description">Real-time item evaluation overlay for Path of Exile.</div>
			</div>

			<div class="setting-group">
				<h3>Updates</h3>

				{state.status === "idle" && (
					<button type="button" class="btn" onClick={checkForUpdate}>
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
						<button type="button" class="btn" style={{ marginTop: "8px" }} onClick={checkForUpdate}>
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
							class="btn"
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
						<button type="button" class="btn" style={{ marginTop: "8px" }} onClick={doRelaunch}>
							Restart Now
						</button>
					</>
				)}

				{state.status === "error" && (
					<>
						<div class="setting-description" style={{ color: "#e04040" }}>
							Update check failed: {state.message}
						</div>
						<button type="button" class="btn" style={{ marginTop: "8px" }} onClick={checkForUpdate}>
							Retry
						</button>
					</>
				)}
			</div>
		</>
	);
}
