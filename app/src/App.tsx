import { useState, useEffect, useCallback } from "preact/hooks";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

export function App() {
	const [itemText, setItemText] = useState<string | null>(null);

	const dismiss = useCallback(async () => {
		setItemText(null);
		await invoke("dismiss_overlay");
	}, []);

	useEffect(() => {
		const unlistenCapture = listen<string>("item-captured", (event) => {
			setItemText(event.payload);
		});

		const unlistenDismiss = listen("overlay-dismissed", () => {
			setItemText(null);
		});

		// Dismiss on Escape key
		const handleKeydown = (e: KeyboardEvent) => {
			if (e.key === "Escape") {
				dismiss();
			}
		};
		document.addEventListener("keydown", handleKeydown);

		// DEV: Show immediately so we can see it's alive.
		// TODO: In production, keep hidden until first Ctrl+I hotkey press.
		getCurrentWebviewWindow().show();

		return () => {
			unlistenCapture.then((fn) => fn());
			unlistenDismiss.then((fn) => fn());
			document.removeEventListener("keydown", handleKeydown);
		};
	}, [dismiss]);

	if (!itemText) {
		return (
			<div class="overlay-panel overlay-idle">
				<div class="idle-message">
					<h3>PoE Inspect</h3>
					<p>
						Press <kbd>Ctrl+I</kbd> over an item in PoE
					</p>
				</div>
			</div>
		);
	}

	return (
		<div class="overlay-panel">
			<button type="button" class="dismiss-btn" onClick={dismiss}>
				&times;
			</button>
			<pre class="item-text">{itemText}</pre>
		</div>
	);
}
