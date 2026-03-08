import { useState, useEffect, useCallback, useRef } from "preact/hooks";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { PhysicalSize } from "@tauri-apps/api/dpi";
import { ItemOverlay } from "./components/ItemOverlay";
import { mockItems } from "./mock-data";

/** Resize the Tauri window to fit the rendered content */
function useAutoResize(deps: unknown[]) {
	const ref = useRef<HTMLDivElement>(null);

	useEffect(() => {
		const el = ref.current;
		if (!el) return;

		// Wait one frame for layout to settle
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

	const dismiss = useCallback(async () => {
		setItemText(null);
		await invoke("dismiss_overlay");
	}, []);

	useEffect(() => {
		const unlistenCapture = listen<string>("item-captured", (event) => {
			setItemText(event.payload);
			setShowMock(false);
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

	// Auto-resize window to fit content
	const containerRef = useAutoResize([itemText, mockIndex, showMock]);

	// When we have raw clipboard text (real Ctrl+I capture), show it
	if (itemText && !showMock) {
		return (
			<div class="overlay-panel" ref={containerRef}>
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
		<div class="overlay-panel" ref={containerRef}>
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
