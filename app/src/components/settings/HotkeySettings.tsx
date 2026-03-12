import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState } from "preact/hooks";
import {
	type HotkeySettings as HotkeySettingsType,
	defaultHotkeys,
	loadHotkeys,
	saveHotkeys,
} from "../../store";

/** Sync hotkey settings to the Rust backend for global shortcut registration. */
async function syncHotkeysToBackend(settings: HotkeySettingsType) {
	await invoke("update_hotkeys", {
		inspectItem: settings.inspectItem.toLowerCase().replace(/\+/g, "+"),
		dismissOverlay: settings.dismissOverlay.toLowerCase().replace(/\+/g, "+"),
		openSettings: settings.openSettings.toLowerCase().replace(/\+/g, "+"),
		cycleProfile: settings.cycleProfile.toLowerCase().replace(/\+/g, "+"),
	});
}

const hotkeyFields: { label: string; key: keyof HotkeySettingsType }[] = [
	{ label: "Inspect Item", key: "inspectItem" },
	{ label: "Dismiss Overlay", key: "dismissOverlay" },
	{ label: "Open Settings", key: "openSettings" },
	{ label: "Cycle Profile", key: "cycleProfile" },
];

function findConflict(
	settings: HotkeySettingsType,
	key: keyof HotkeySettingsType,
	combo: string,
): string | null {
	for (const field of hotkeyFields) {
		if (field.key !== key && settings[field.key] === combo) {
			return field.label;
		}
	}
	return null;
}

export function HotkeySettings() {
	const [settings, setSettings] = useState<HotkeySettingsType>(defaultHotkeys);
	const [loaded, setLoaded] = useState(false);
	const [capturing, setCapturing] = useState<string | null>(null);
	const [conflict, setConflict] = useState<{ key: string; message: string } | null>(null);
	const settingsRef = useRef(settings);
	settingsRef.current = settings;
	const capturingRef = useRef<string | null>(null);

	useEffect(() => {
		loadHotkeys().then((s) => {
			setSettings(s);
			setLoaded(true);
			syncHotkeysToBackend(s);
		});
	}, []);

	const startCapture = useCallback((key: string) => {
		capturingRef.current = key;
		setCapturing(key);
		setConflict(null);
		invoke("pause_hotkeys");
	}, []);

	const cancelCapture = useCallback(() => {
		capturingRef.current = null;
		setCapturing(null);
		invoke("resume_hotkeys");
	}, []);

	const handleKeyDown = useCallback((e: KeyboardEvent, key: keyof HotkeySettingsType) => {
		// Guard against stale events (e.g. enigo keystrokes arriving late)
		if (capturingRef.current !== key) return;

		e.preventDefault();
		e.stopPropagation();

		// Ignore lone modifier keys
		if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) return;

		const parts: string[] = [];
		if (e.ctrlKey) parts.push("Ctrl");
		if (e.shiftKey) parts.push("Shift");
		if (e.altKey) parts.push("Alt");

		// Map key names
		let keyName = e.key;
		if (keyName === " ") keyName = "Space";
		else if (keyName === "Escape") keyName = "Escape";
		else if (keyName.length === 1) keyName = keyName.toUpperCase();
		parts.push(keyName);

		const combo = parts.join("+");
		const conflictWith = findConflict(settingsRef.current, key, combo);
		if (conflictWith) {
			setConflict({ key, message: `"${combo}" is already used by ${conflictWith}` });
			capturingRef.current = null;
			setCapturing(null);
			invoke("resume_hotkeys");
			return;
		}

		setConflict(null);
		const next = { ...settingsRef.current, [key]: combo };
		setSettings(next);
		saveHotkeys(next);
		syncHotkeysToBackend(next);
		capturingRef.current = null;
		setCapturing(null);
	}, []);

	const resetHotkey = useCallback((key: keyof HotkeySettingsType) => {
		const next = { ...settingsRef.current, [key]: defaultHotkeys[key] };
		setSettings(next);
		saveHotkeys(next);
		syncHotkeysToBackend(next);
		setConflict(null);
	}, []);

	if (!loaded) return null;

	return (
		<>
			<h2>Hotkeys</h2>

			<div class="setting-group">
				<h3>Key Bindings</h3>

				{hotkeyFields.map((field) => (
					<div class="setting-row" key={field.key}>
						<div class="setting-label">{field.label}</div>
						<div class="hotkey-input">
							<button
								type="button"
								class={`hotkey-display ${capturing === field.key ? "capturing" : ""}`}
								onClick={() => {
									if (capturing !== field.key) startCapture(field.key);
								}}
								onKeyDown={(e) => {
									if (capturing === field.key) {
										handleKeyDown(e as unknown as KeyboardEvent, field.key);
									} else if (e.key === "Enter" || e.key === " ") {
										startCapture(field.key);
									}
								}}
							>
								{capturing === field.key ? "Press keys..." : settings[field.key]}
							</button>
							{capturing === field.key ? (
								<button type="button" class="hotkey-reset" onClick={cancelCapture}>
									Cancel
								</button>
							) : (
								settings[field.key] !== defaultHotkeys[field.key] && (
									<button type="button" class="hotkey-reset" onClick={() => resetHotkey(field.key)}>
										Reset
									</button>
								)
							)}
						</div>
					</div>
				))}

				{conflict && (
					<div class="setting-description" style={{ marginTop: "12px", color: "#e04040" }}>
						{conflict.message}
					</div>
				)}

				<div class="setting-description" style={{ marginTop: conflict ? "4px" : "12px" }}>
					Click a hotkey to rebind it. Global shortcuts are paused during capture.
				</div>
			</div>
		</>
	);
}
