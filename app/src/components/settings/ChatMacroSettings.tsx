import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState } from "preact/hooks";
import {
	type ChatMacro,
	type HotkeySettings,
	defaultHotkeys,
	loadChatMacros,
	loadHotkeys,
	saveChatMacros,
} from "../../store";

/** Sync macros to the Rust backend for global shortcut registration. */
async function syncMacrosToBackend(macros: ChatMacro[]) {
	const cleaned = macros
		.filter((m) => m.hotkey && m.command)
		.map((m) => ({
			hotkey: m.hotkey.toLowerCase(),
			command: m.command,
			send: m.send,
		}));
	await invoke("update_chat_macros", { macrosJson: JSON.stringify(cleaned) });
}

/** Check if a hotkey combo conflicts with core hotkeys or other macros. */
function findConflict(
	combo: string,
	currentId: string,
	macros: ChatMacro[],
	coreHotkeys: HotkeySettings,
): string | null {
	const lc = combo.toLowerCase();
	for (const [label, key] of [
		["Inspect Item", coreHotkeys.inspectItem],
		["Dismiss Overlay", coreHotkeys.dismissOverlay],
		["Open Settings", coreHotkeys.openSettings],
		["Cycle Profile", coreHotkeys.cycleProfile],
	] as const) {
		if (key.toLowerCase() === lc) return label;
	}
	for (const m of macros) {
		if (m.id !== currentId && m.hotkey.toLowerCase() === lc) {
			return `macro "${m.command}"`;
		}
	}
	return null;
}

export function ChatMacroSettings() {
	const [macros, setMacros] = useState<ChatMacro[]>([]);
	const [coreHotkeys, setCoreHotkeys] = useState<HotkeySettings>(defaultHotkeys);
	const [loaded, setLoaded] = useState(false);
	const [capturing, setCapturing] = useState<string | null>(null);
	const [conflict, setConflict] = useState<{ id: string; message: string } | null>(null);
	const macrosRef = useRef(macros);
	macrosRef.current = macros;
	const capturingRef = useRef<string | null>(null);

	useEffect(() => {
		Promise.all([loadChatMacros(), loadHotkeys()]).then(([m, h]) => {
			setMacros(m);
			setCoreHotkeys(h);
			setLoaded(true);
		});
	}, []);

	const save = useCallback((next: ChatMacro[]) => {
		setMacros(next);
		saveChatMacros(next);
		syncMacrosToBackend(next);
	}, []);

	const addMacro = useCallback(() => {
		save([...macrosRef.current, { id: String(Date.now()), hotkey: "", command: "", send: true }]);
	}, [save]);

	const removeMacro = useCallback(
		(id: string) => {
			save(macrosRef.current.filter((m) => m.id !== id));
			if (conflict?.id === id) setConflict(null);
		},
		[save, conflict],
	);

	const updateMacro = useCallback(
		(id: string, patch: Partial<ChatMacro>) => {
			save(macrosRef.current.map((m) => (m.id === id ? { ...m, ...patch } : m)));
		},
		[save],
	);

	const startCapture = useCallback((id: string) => {
		capturingRef.current = id;
		setCapturing(id);
		setConflict(null);
		invoke("pause_hotkeys");
	}, []);

	const cancelCapture = useCallback(() => {
		capturingRef.current = null;
		setCapturing(null);
		invoke("resume_hotkeys");
	}, []);

	const handleKeyDown = useCallback(
		(e: KeyboardEvent, id: string) => {
			if (capturingRef.current !== id) return;
			e.preventDefault();
			e.stopPropagation();

			if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) return;

			const parts: string[] = [];
			if (e.ctrlKey) parts.push("Ctrl");
			if (e.shiftKey) parts.push("Shift");
			if (e.altKey) parts.push("Alt");

			let keyName = e.key;
			if (keyName === " ") keyName = "Space";
			else if (keyName.length === 1) keyName = keyName.toUpperCase();
			parts.push(keyName);

			const combo = parts.join("+");
			const conflictWith = findConflict(combo, id, macrosRef.current, coreHotkeys);
			if (conflictWith) {
				setConflict({ id, message: `"${combo}" is already used by ${conflictWith}` });
				capturingRef.current = null;
				setCapturing(null);
				invoke("resume_hotkeys");
				return;
			}

			setConflict(null);
			updateMacro(id, { hotkey: combo });
			capturingRef.current = null;
			setCapturing(null);
		},
		[coreHotkeys, updateMacro],
	);

	if (!loaded) return null;

	return (
		<>
			<h2>Chat Macros</h2>

			<div class="setting-group">
				<h3>Macros</h3>
				<div class="setting-description" style={{ marginBottom: "12px" }}>
					Bind hotkeys to chat commands. When triggered, the command is pasted into PoE's chat.
					Commands starting with / are sent immediately by default.
				</div>

				{macros.map((macro) => (
					<div class="macro-row" key={macro.id}>
						<button
							type="button"
							class={`hotkey-display ${capturing === macro.id ? "capturing" : ""}`}
							onClick={() => {
								if (capturing !== macro.id) startCapture(macro.id);
							}}
							onKeyDown={(e) => {
								if (capturing === macro.id) {
									handleKeyDown(e as unknown as KeyboardEvent, macro.id);
								} else if (e.key === "Enter" || e.key === " ") {
									startCapture(macro.id);
								}
							}}
						>
							{capturing === macro.id ? "Press keys..." : macro.hotkey || "Click to bind"}
						</button>

						{capturing === macro.id ? (
							<button type="button" class="hotkey-reset" onClick={cancelCapture}>
								Cancel
							</button>
						) : null}

						<input
							type="text"
							class="macro-command-input"
							placeholder="/hideout"
							value={macro.command}
							onInput={(e) =>
								updateMacro(macro.id, {
									command: (e.target as HTMLInputElement).value,
								})
							}
						/>

						<label class="macro-send-label" title="Auto-send (press Enter after paste)">
							<input
								type="checkbox"
								checked={macro.send}
								onChange={(e) =>
									updateMacro(macro.id, {
										send: (e.target as HTMLInputElement).checked,
									})
								}
							/>
							Send
						</label>

						<button
							type="button"
							class="btn btn-small btn-danger"
							onClick={() => removeMacro(macro.id)}
							title="Remove macro"
						>
							&times;
						</button>
					</div>
				))}

				{conflict && (
					<div class="setting-description" style={{ marginTop: "8px", color: "#e04040" }}>
						{conflict.message}
					</div>
				)}

				<button type="button" class="btn btn-small" style={{ marginTop: "8px" }} onClick={addMacro}>
					+ Add Macro
				</button>
			</div>
		</>
	);
}
