import { useState, useEffect, useCallback, useRef } from "preact/hooks";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { PhysicalSize } from "@tauri-apps/api/dpi";
import { ItemOverlay } from "./components/ItemOverlay";
import { mockItems } from "./mock-data";
import { loadGeneral, loadHotkeys } from "./store";

/** Resize the Tauri window to fit the rendered content.
 *  Scaling is handled via CSS `zoom` which affects layout, so
 *  getBoundingClientRect naturally reflects the zoomed dimensions. */
function useAutoResize(deps: unknown[]) {
	const ref = useRef<HTMLDivElement>(null);

	useEffect(() => {
		const el = ref.current;
		if (!el) return;

		requestAnimationFrame(() => {
			const rect = el.getBoundingClientRect();
			const win = getCurrentWebviewWindow();
			win.setSize(new PhysicalSize(
				Math.ceil(rect.width * window.devicePixelRatio),
				Math.ceil(rect.height * window.devicePixelRatio),
			));
		});
	}, deps);

	return ref;
}

export function App() {
	const [itemText, setItemText] = useState<string | null>(null);
	const [mockIndex, setMockIndex] = useState(0);
	const [showMock, setShowMock] = useState(true);
	const [overlayScale, setOverlayScale] = useState(100);

	const dismiss = useCallback(async () => {
		setItemText(null);
		await invoke("dismiss_overlay");
	}, []);

	const dismissKeyRef = useRef("Escape");

	useEffect(() => {
		// Load settings
		loadGeneral().then((s) => setOverlayScale(s.overlayScale));
		loadHotkeys().then((h) => {
			dismissKeyRef.current = h.dismissOverlay;
		});

		const unlistenCapture = listen<string>("item-captured", (event) => {
			// Reload settings each time overlay shows (picks up scale/hotkey changes)
			loadGeneral().then((s) => setOverlayScale(s.overlayScale));
			loadHotkeys().then((h) => {
				dismissKeyRef.current = h.dismissOverlay;
			});
			setItemText(event.payload);
			setShowMock(false);
		});

		const unlistenDismiss = listen("overlay-dismissed", () => {
			setItemText(null);
		});

		// Dismiss overlay on configured key (window-level, not global shortcut)
		const handleKeydown = (e: KeyboardEvent) => {
			const parts: string[] = [];
			if (e.ctrlKey) parts.push("Ctrl");
			if (e.shiftKey) parts.push("Shift");
			if (e.altKey) parts.push("Alt");
			let keyName = e.key;
			if (keyName === " ") keyName = "Space";
			else if (keyName.length === 1) keyName = keyName.toUpperCase();
			parts.push(keyName);
			const combo = parts.join("+");
			if (combo === dismissKeyRef.current) {
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

	// Auto-resize window to fit content
	const containerRef = useAutoResize([itemText, mockIndex, showMock, overlayScale]);
	const scaleStyle = overlayScale !== 100
		? { zoom: overlayScale / 100 }
		: undefined;

	// When we have raw clipboard text (real Ctrl+I capture), show it
	if (itemText && !showMock) {
		return (
			<div class="overlay-panel" ref={containerRef} style={scaleStyle}>
				<button type="button" class="dismiss-btn" onClick={dismiss}>
					&times;
				</button>
				<pre class="item-text">{itemText}</pre>
			</div>
		);
	}

	// Mock data mode: show styled item overlay
	const currentItem = mockItems[mockIndex];

	return (
		<div class="overlay-panel" ref={containerRef} style={scaleStyle}>
			<button
				type="button"
				class="dismiss-btn"
				onClick={() => {
					setShowMock(false);
					dismiss();
				}}
			>
				&times;
			</button>

			{/* Item selector for cycling mock items */}
			<div class="item-selector">
				{mockItems.map((item, i) => (
					<button
						key={item.name}
						type="button"
						class={i === mockIndex ? "active" : ""}
						onClick={() => {
							setMockIndex(i);
							setShowMock(true);
						}}
					>
						{item.name}
					</button>
				))}
			</div>

			{currentItem !== undefined && <ItemOverlay item={currentItem} />}
		</div>
	);
}
