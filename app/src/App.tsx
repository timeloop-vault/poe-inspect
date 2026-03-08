import { useState, useEffect, useCallback, useRef } from "preact/hooks";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { PhysicalSize } from "@tauri-apps/api/dpi";
import { ItemOverlay, type DisplaySettings } from "./components/ItemOverlay";
import { mockItems } from "./mock-data";
import { loadGeneral, loadHotkeys } from "./store";

/** Resize the Tauri window to fit the rendered content.
 *  CSS `zoom` reduces available CSS pixels (parent_width / zoom), so we
 *  first expand the transparent window to give content enough room to
 *  lay out at its natural max-width, then measure and shrink to fit. */
function useAutoResize(deps: unknown[], zoom = 1) {
	const ref = useRef<HTMLDivElement>(null);

	useEffect(() => {
		const el = ref.current;
		if (!el) return;

		let cancelled = false;
		const win = getCurrentWebviewWindow();
		const dpr = window.devicePixelRatio;

		// Expand window so the panel's max-width (420px + padding) isn't
		// starved by zoom.  The window is transparent so the flash is invisible.
		const maxCssWidth = 500;
		const generous = Math.ceil(maxCssWidth * zoom * dpr);

		win.setSize(new PhysicalSize(generous, generous)).then(() => {
			if (cancelled) return;
			// Double-rAF: first ensures the browser processes the resize
			// event, second ensures layout has reflowed with the new size.
			requestAnimationFrame(() => {
				if (cancelled) return;
				requestAnimationFrame(() => {
					if (cancelled) return;
					const rect = el.getBoundingClientRect();
					win.setSize(new PhysicalSize(
						Math.ceil(rect.width * dpr),
						Math.ceil(rect.height * dpr),
					));
				});
			});
		});

		return () => { cancelled = true; };
	}, deps);

	return ref;
}

export function App() {
	const [itemText, setItemText] = useState<string | null>(null);
	const [mockIndex, setMockIndex] = useState(0);
	const [showMock, setShowMock] = useState(true);
	const [overlayScale, setOverlayScale] = useState(100);
	const [displaySettings, setDisplaySettings] = useState<DisplaySettings>({
		showRollBars: true,
		showTierBadges: true,
		showTypeBadges: true,
		showOpenAffixes: true,
	});

	const dismiss = useCallback(async () => {
		setItemText(null);
		await invoke("dismiss_overlay");
	}, []);

	const dismissKeyRef = useRef("Escape");

	useEffect(() => {
		// Load settings
		const reloadSettings = () => {
			loadGeneral().then((s) => {
				setOverlayScale(s.overlayScale);
				setDisplaySettings({
					showRollBars: s.showRollBars,
					showTierBadges: s.showTierBadges,
					showTypeBadges: s.showTypeBadges,
					showOpenAffixes: s.showOpenAffixes,
				});
			});
			loadHotkeys().then((h) => {
				dismissKeyRef.current = h.dismissOverlay;
			});
		};
		reloadSettings();

		const unlistenCapture = listen<string>("item-captured", (event) => {
			reloadSettings();
			setItemText(event.payload);
			setShowMock(false);
		});

		const unlistenDismiss = listen("overlay-dismissed", () => {
			setItemText(null);
		});

		const unlistenDebug = listen("show-debug-overlay", () => {
			reloadSettings();
			setShowMock(true);
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
			unlistenDebug.then((fn) => fn());
			document.removeEventListener("keydown", handleKeydown);
		};
	}, [dismiss]);

	// Auto-resize window to fit content
	const zoom = overlayScale / 100;
	const containerRef = useAutoResize([itemText, mockIndex, showMock, overlayScale], zoom);
	const scaleStyle = zoom !== 1 ? { zoom } : undefined;

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

			{currentItem !== undefined && <ItemOverlay item={currentItem} display={displaySettings} />}
		</div>
	);
}
