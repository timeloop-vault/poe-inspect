import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Component } from "preact";
import { useCallback, useEffect, useRef, useState } from "preact/hooks";
import { type DisplaySettings, ItemOverlay } from "./components/ItemOverlay";
import { TradePanel } from "./components/TradePanel";
import { useTradeFilters } from "./hooks/useTradeFilters";
import { mockItems } from "./mock-data";
import {
	type QualityColors,
	type TradeSettings,
	defaultTrade,
	loadActiveQualityColors,
	loadGeneral,
	loadHotkeys,
	loadTrade,
	syncActiveProfile,
} from "./store";
import type { ItemPayload, TradeQueryConfig } from "./types";

/** Panel position — either left-anchored or right-anchored. */
type PanelPosition =
	| { anchor: "left"; left: number; top: number }
	| { anchor: "right"; right: number; top: number };

/** Calculate panel position within the fullscreen overlay.
 *  Handles cursor mode (offset from cursor, flip if overflow)
 *  and panel mode (beside PoE inventory/stash panel). */
function computePanelPosition(
	cursor: { x: number; y: number },
	mode: string,
	zoom: number,
): PanelPosition {
	const vpW = window.innerWidth;
	const vpH = window.innerHeight;
	const panelW = 440 * zoom;
	const panelEstH = 600 * zoom;

	if (mode === "panel") {
		// PoE's inventory/stash panel width is proportional to screen height.
		// Ratio 986/1600 is derived from PoE's UI layout (confirmed by Awakened Trade).
		// Do not change unless GGG changes the in-game panel sizing.
		const poePanelW = vpH * (986 / 1600);
		const midX = vpW / 2;

		if (cursor.x >= midX) {
			// Right side (inventory) — anchor panel's right edge to inventory's left edge
			return { anchor: "right", right: poePanelW, top: 0 };
		}
		// Left side (stash) — anchor panel's left edge to stash's right edge
		return { anchor: "left", left: poePanelW, top: 0 };
	}

	// Cursor mode: small offset from cursor, flip if overflow
	const offset = 10;
	let x = cursor.x + offset;
	let y = cursor.y + offset;

	if (x + panelW > vpW) x = cursor.x - offset - panelW;
	if (y + panelEstH > vpH) y = cursor.y - offset - panelEstH;

	x = Math.max(0, Math.min(x, vpW - panelW));
	y = Math.max(0, y);

	return { anchor: "left", left: x, top: y };
}

/** Catches render errors in the overlay content to prevent orphan DOM nodes. */
class OverlayErrorBoundary extends Component<
	{ children: preact.ComponentChildren },
	{ error: string | null }
> {
	state = { error: null as string | null };
	static getDerivedStateFromError(err: Error) {
		return { error: err.message };
	}
	componentDidCatch(err: Error) {
		console.error("[overlay] Render error caught:", err);
	}
	render() {
		if (this.state.error) {
			return (
				<div class="parse-error">
					<div class="parse-error-title">Overlay render error</div>
					<div class="parse-error-hint">{this.state.error}</div>
				</div>
			);
		}
		return this.props.children;
	}
}

export function App() {
	const [itemText, setItemText] = useState<string | null>(null);
	const [evaluatedItem, setEvaluatedItem] = useState<ItemPayload | null>(null);
	const [parseError, setParseError] = useState<{ error: string; rawText: string } | null>(null);
	const [mockIndex, setMockIndex] = useState(0);
	const [showMock, setShowMock] = useState(true);
	const [overlayScale, setOverlayScale] = useState(100);
	const [overlayMode, setOverlayMode] = useState("cursor");
	const [displaySettings, setDisplaySettings] = useState<DisplaySettings>({
		showRollBars: true,
		showTierBadges: true,
		showTypeBadges: true,
		showOpenAffixes: true,
		showStatIds: false,
	});
	const [qualityColors, setQualityColors] = useState<QualityColors | null>(null);
	const [cursorPos, setCursorPos] = useState<{ x: number; y: number }>({
		x: 200,
		y: 100,
	});
	const [tradeSettings, setTradeSettings] = useState<TradeSettings>(defaultTrade);

	const dismiss = useCallback(async () => {
		setItemText(null);
		setEvaluatedItem(null);
		setParseError(null);
		setShowMock(false);
		await invoke("dismiss_overlay");
	}, []);

	const dismissKeyRef = useRef("Escape");

	useEffect(() => {
		// Load settings
		const reloadSettings = () => {
			loadGeneral().then((s) => {
				setOverlayScale(s.overlayScale);
				setOverlayMode(s.overlayPosition);
				setDisplaySettings({
					showRollBars: s.showRollBars,
					showTierBadges: s.showTierBadges,
					showTypeBadges: s.showTypeBadges,
					showOpenAffixes: s.showOpenAffixes,
					showStatIds: s.showStatIds,
				});
			});
			loadHotkeys().then((h) => {
				dismissKeyRef.current = h.dismissOverlay;
			});
			loadActiveQualityColors().then(setQualityColors);
			loadTrade().then(setTradeSettings);
			syncActiveProfile();
		};
		reloadSettings();

		const unlistenPosition = listen<{ x: number; y: number }>("overlay-position", (event) => {
			setCursorPos(event.payload);
		});

		const unlistenEvaluated = listen<ItemPayload>("item-evaluated", (event) => {
			reloadSettings();
			setEvaluatedItem(event.payload);
			setItemText(null);
			setParseError(null);
			setShowMock(false);
		});

		const unlistenCapture = listen<string>("item-captured", (event) => {
			reloadSettings();
			setItemText(event.payload);
			setEvaluatedItem(null);
			setParseError(null);
			setShowMock(false);
		});

		const unlistenParseFailed = listen<{ error: string; rawText: string }>(
			"item-parse-failed",
			(event) => {
				reloadSettings();
				setParseError(event.payload);
				setEvaluatedItem(null);
				setItemText(null);
				setShowMock(false);
			},
		);

		const unlistenDismiss = listen("overlay-dismissed", () => {
			setItemText(null);
			setEvaluatedItem(null);
			setParseError(null);
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

		return () => {
			unlistenPosition.then((fn) => fn());
			unlistenEvaluated.then((fn) => fn());
			unlistenCapture.then((fn) => fn());
			unlistenParseFailed.then((fn) => fn());
			unlistenDismiss.then((fn) => fn());
			unlistenDebug.then((fn) => fn());
			document.removeEventListener("keydown", handleKeydown);
		};
	}, [dismiss]);

	const zoom = overlayScale / 100;
	const pos = computePanelPosition(cursorPos, overlayMode, zoom);

	// Build style object: scale + tier color CSS custom properties + absolute position
	// Use transform instead of CSS zoom — zoom affects layout and breaks
	// absolute positioning within the fullscreen backdrop.
	const panelStyle: Record<string, string | number> = {
		top: `${pos.top}px`,
	};
	if (pos.anchor === "right") {
		panelStyle.right = `${pos.right}px`;
	} else {
		panelStyle.left = `${pos.left}px`;
	}
	if (zoom !== 1) {
		panelStyle.transform = `scale(${zoom})`;
		panelStyle.transformOrigin = pos.anchor === "right" ? "top right" : "top left";
	}
	if (qualityColors) {
		panelStyle["--quality-best"] = qualityColors.best;
		panelStyle["--quality-good"] = qualityColors.good;
		panelStyle["--quality-mid"] = qualityColors.mid;
		panelStyle["--quality-low"] = qualityColors.low;
	}

	// Determine content to display
	let content: preact.ComponentChildren = null;
	let showDismiss = true;

	const tradeConfig: TradeQueryConfig = {
		league: tradeSettings.league,
		valueRelaxation: tradeSettings.valueRelaxation,
		usePseudoStats: false,
		onlineOnly: tradeSettings.onlineOnly,
	};

	const tradeFilters = useTradeFilters(evaluatedItem?.rawText ?? "", tradeConfig);

	// Trade panel goes on the opposite side from the screen edge:
	// right-anchored overlay → trade on left; left-anchored → trade on right.
	const tradeSide = pos.anchor === "right" ? "left" : "right";

	if (evaluatedItem && !showMock) {
		const tradeEditProps = tradeFilters.editMode
			? {
					mappedStats: tradeFilters.mappedStats,
					isStatEnabled: tradeFilters.isStatEnabled,
					getStatMin: tradeFilters.getStatMin,
					toggleStat: tradeFilters.toggleStat,
					setStatMin: tradeFilters.setStatMin,
				}
			: undefined;

		const itemCard = (
			<div class="overlay-item-col">
				<ItemOverlay
					item={evaluatedItem.item}
					eval={evaluatedItem.eval}
					display={displaySettings}
					tradeEdit={tradeEditProps}
				/>
			</div>
		);
		const tradeCol = (
			<div class="overlay-trade-col">
				<TradePanel
					itemText={evaluatedItem.rawText}
					config={tradeConfig}
					filters={tradeFilters}
					baseType={evaluatedItem.item.header.baseType}
					itemClass={evaluatedItem.item.header.itemClass}
				/>
			</div>
		);
		content = (
			<div class="overlay-columns">
				{tradeSide === "left" ? (
					<>
						{tradeCol}
						{itemCard}
					</>
				) : (
					<>
						{itemCard}
						{tradeCol}
					</>
				)}
			</div>
		);
	} else if (parseError && !showMock) {
		content = (
			<div class="overlay-single">
				<div class="parse-error">
					<div class="parse-error-title">Item not supported yet</div>
					<div class="parse-error-hint">
						This item type can't be parsed. Copy the item text (Ctrl+Alt+C) and report it to help us
						add support.
					</div>
					<details class="parse-error-details">
						<summary>Raw item text</summary>
						<pre>{parseError.rawText}</pre>
					</details>
				</div>
			</div>
		);
	} else if (itemText && !showMock) {
		content = (
			<div class="overlay-single">
				<pre class="item-text">{itemText}</pre>
			</div>
		);
	} else if (showMock) {
		const currentItem = mockItems[mockIndex];
		showDismiss = true;
		content = (
			<div class="overlay-single">
				{/* Item selector for cycling mock items */}
				<div class="item-selector">
					{mockItems.map((m, i) => (
						<button
							key={m.item.header.name ?? m.item.header.baseType}
							type="button"
							class={i === mockIndex ? "active" : ""}
							onClick={() => {
								setMockIndex(i);
								setShowMock(true);
							}}
						>
							{m.item.header.name ?? m.item.header.baseType}
						</button>
					))}
				</div>
				{currentItem !== undefined && (
					<ItemOverlay item={currentItem.item} eval={currentItem.eval} display={displaySettings} />
				)}
			</div>
		);
	}

	return (
		// biome-ignore lint/a11y/useKeyWithClickEvents: backdrop is mouse-only, keyboard dismiss handled via document keydown listener
		<div
			class="overlay-backdrop"
			onClick={(e) => {
				// Click on backdrop (not on panel) = dismiss
				if (e.target === e.currentTarget) dismiss();
			}}
		>
			{content && (
				<div class="overlay-panel" style={panelStyle}>
					{showDismiss && (
						<button
							type="button"
							class="dismiss-btn"
							onClick={() => {
								if (showMock) setShowMock(false);
								dismiss();
							}}
						>
							&times;
						</button>
					)}
					<OverlayErrorBoundary>{content}</OverlayErrorBoundary>
				</div>
			)}
		</div>
	);
}
