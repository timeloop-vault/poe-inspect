import { useState, useCallback } from "preact/hooks";

interface HotkeyConfig {
	label: string;
	default: string;
	current: string;
}

const defaultHotkeys: HotkeyConfig[] = [
	{ label: "Inspect Item", default: "Ctrl+I", current: "Ctrl+I" },
	{ label: "Dismiss Overlay", default: "Escape", current: "Escape" },
	{ label: "Open Settings", default: "Ctrl+Shift+I", current: "Ctrl+Shift+I" },
];

export function HotkeySettings() {
	const [hotkeys, setHotkeys] = useState(defaultHotkeys);
	const [capturing, setCapturing] = useState<number | null>(null);

	const startCapture = useCallback((index: number) => {
		setCapturing(index);
	}, []);

	const handleKeyDown = useCallback(
		(e: KeyboardEvent, index: number) => {
			e.preventDefault();
			e.stopPropagation();

			// Ignore lone modifier keys
			if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) return;

			const parts: string[] = [];
			if (e.ctrlKey) parts.push("Ctrl");
			if (e.shiftKey) parts.push("Shift");
			if (e.altKey) parts.push("Alt");

			if (e.key === "Escape" && parts.length === 0) {
				// Bare Escape cancels capture
				setCapturing(null);
				return;
			}

			// Map key names
			let keyName = e.key;
			if (keyName === " ") keyName = "Space";
			else if (keyName.length === 1) keyName = keyName.toUpperCase();
			parts.push(keyName);

			const combo = parts.join("+");
			setHotkeys((prev) =>
				prev.map((h, i) => (i === index ? { ...h, current: combo } : h)),
			);
			setCapturing(null);
		},
		[],
	);

	const resetHotkey = useCallback((index: number) => {
		setHotkeys((prev) =>
			prev.map((h, i) =>
				i === index ? { ...h, current: h.default } : h,
			),
		);
	}, []);

	return (
		<>
			<h2>Hotkeys</h2>

			<div class="setting-group">
				<h3>Key Bindings</h3>

				{hotkeys.map((hotkey, i) => (
					<div class="setting-row" key={hotkey.label}>
						<div class="setting-label">{hotkey.label}</div>
						<div class="hotkey-input">
							<div
								class={`hotkey-display ${capturing === i ? "capturing" : ""}`}
								tabIndex={0}
								onClick={() => startCapture(i)}
								onKeyDown={(e) => {
									if (capturing === i) {
										handleKeyDown(e as unknown as KeyboardEvent, i);
									} else if (e.key === "Enter" || e.key === " ") {
										startCapture(i);
									}
								}}
							>
								{capturing === i ? "Press keys..." : hotkey.current}
							</div>
							{hotkey.current !== hotkey.default && (
								<button
									type="button"
									class="hotkey-reset"
									onClick={() => resetHotkey(i)}
								>
									Reset
								</button>
							)}
						</div>
					</div>
				))}

				<div
					class="setting-description"
					style={{ marginTop: "12px" }}
				>
					Click a hotkey to change it. Press Escape to cancel.
				</div>
			</div>
		</>
	);
}
