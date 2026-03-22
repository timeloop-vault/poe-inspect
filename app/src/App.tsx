import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Component } from "preact";
import { useCallback, useEffect, useRef, useState } from "preact/hooks";
import { CompactPill, CompactPillNA } from "./components/CompactPill";
import { type DisplaySettings, ItemOverlay, type ProfileSummary } from "./components/ItemOverlay";
import { TradePanel } from "./components/TradePanel";
import { useTradeFilters } from "./hooks/useTradeFilters";
import { mockItems } from "./mock-data";
import {
	type DangerLevel,
	type MapDangerConfig,
	type MarketplaceSettings,
	type QualityColors,
	type StoredProfile,
	type TradeSettings,
	defaultMarketplace,
	defaultTrade,
	loadActiveQualityColors,
	loadGeneral,
	loadHotkeys,
	loadMarketplace,
	loadProfiles,
	loadTrade,
	saveProfiles,
	syncActiveProfile,
} from "./store";

// Marketplace/RQE is experimental — only available in dev builds
const rqe = import.meta.env.DEV ? await import("./rqe") : null;
import type { ItemPayload, TradeQueryConfig } from "./types";

/** Panel position — either left-anchored or right-anchored. */
type PanelPosition =
	| { anchor: "left"; left: number; top: number }
	| { anchor: "right"; right: number; top: number };

/** Calculate panel position within the fullscreen overlay.
 *  Handles cursor mode (offset from cursor, flip if overflow)
 *  and panel mode (beside PoE inventory/stash panel).
 *  When `panelSize` is provided (measured from DOM), uses actual dimensions
 *  instead of estimates for accurate overflow detection. */
function computePanelPosition(
	cursor: { x: number; y: number },
	mode: string,
	zoom: number,
	panelSize?: { width: number; height: number },
): PanelPosition {
	const vpW = window.innerWidth;
	const vpH = window.innerHeight;
	const panelW = panelSize ? panelSize.width : 440 * zoom;
	const panelH = panelSize ? panelSize.height : 600 * zoom;

	if (mode === "panel") {
		// PoE's inventory/stash panel width is proportional to screen height.
		// Ratio 986/1600 is derived from PoE's UI layout (confirmed by Awakened Trade).
		// Do not change unless GGG changes the in-game panel sizing.
		const poePanelW = vpH * (986 / 1600);

		if (cursor.x < poePanelW) {
			// Cursor is in the stash area (left edge) — anchor beside stash
			return { anchor: "left", left: poePanelW, top: 0 };
		}
		if (cursor.x >= vpW - poePanelW) {
			// Cursor is in the inventory area (right edge) — anchor beside inventory
			return { anchor: "right", right: poePanelW, top: 0 };
		}
		// Middle of screen (ritual, vendors, etc.) — anchor beside inventory
		return { anchor: "right", right: poePanelW, top: 0 };
	}

	// Cursor mode: position near cursor, avoiding overlap with the cursor icon.
	// Cursor hotspot is top-left; icon is ~32px tall, ~16px wide.
	// X: prefer right of cursor (+20 to clear icon width), flip left if no room (-6 gap).
	// Y: prefer above cursor (-6 gap above hotspot), flip below if at top of screen (+24 to clear icon).
	const cursorClearX = 20;
	const cursorClearY = 24;
	const gap = 6;

	let x = cursor.x + cursorClearX;
	if (x + panelW > vpW) x = cursor.x - gap - panelW;

	let y = cursor.y - gap - panelH;
	if (y < 0) y = cursor.y + cursorClearY;

	x = Math.max(0, Math.min(x, vpW - panelW));
	y = Math.max(0, Math.min(y, vpH - panelH));

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
	const [panelReady, setPanelReady] = useState(false);
	const [parseError, setParseError] = useState<{ error: string; rawText: string } | null>(null);
	const [mockIndex, setMockIndex] = useState(0);
	const [showMock, setShowMock] = useState(import.meta.env.DEV);
	const [overlayScale, setOverlayScale] = useState(100);
	const [overlayMode, setOverlayMode] = useState("cursor");
	const [compactMode, setCompactMode] = useState("cursor");
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
	const [mapDanger, setMapDanger] = useState<MapDangerConfig>({});
	const [profileSummaries, setProfileSummaries] = useState<ProfileSummary[]>([]);
	const [inspectMode, setInspectMode] = useState<"full" | "compact" | "trade" | null>(null);
	const [compactFading, setCompactFading] = useState(false);
	const [demandResult, setDemandResult] = useState<{
		count: number;
		matches: { id: number; owner: string | null }[];
	} | null>(null);
	const [marketplaceSettings, setMarketplaceSettings] =
		useState<MarketplaceSettings>(defaultMarketplace);
	const [panelSize, setPanelSize] = useState<{ width: number; height: number } | null>(null);
	const compactTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
	const panelRef = useRef<HTMLDivElement>(null);
	const profilesRef = useRef<StoredProfile[]>([]);
	const startupToastShown = useRef(false);

	const showProfileToast = useCallback((profile: StoredProfile) => {
		invoke("show_toast", { profileName: profile.name, color: profile.watchColor ?? "" });
	}, []);

	/** Update derived state from the profiles ref. */
	const syncProfileState = useCallback((profiles: StoredProfile[]) => {
		profilesRef.current = profiles;
		const primary = profiles.find((p) => p.role === "primary");
		setMapDanger(primary?.mapDanger ?? {});
		setProfileSummaries(
			profiles.map((p) => ({ id: p.id, name: p.name, role: p.role, watchColor: p.watchColor })),
		);
	}, []);

	const dismiss = useCallback(async () => {
		setItemText(null);
		setEvaluatedItem(null);
		setParseError(null);
		setShowMock(false);
		setInspectMode(null);
		setCompactFading(false);
		setDemandResult(null);
		if (compactTimerRef.current) {
			clearTimeout(compactTimerRef.current);
			compactTimerRef.current = null;
		}
		await invoke("dismiss_overlay");
	}, []);

	const handleDangerChange = useCallback(
		(template: string, level: DangerLevel | null) => {
			const profiles = profilesRef.current;
			const primaryIdx = profiles.findIndex((p) => p.role === "primary");
			const primary = primaryIdx >= 0 ? profiles[primaryIdx] : undefined;
			if (!primary) return;
			const updated = { ...primary.mapDanger };
			if (level === null) {
				delete updated[template];
			} else {
				updated[template] = level;
			}
			const newProfiles = [...profiles];
			newProfiles[primaryIdx] = { ...primary, mapDanger: updated };
			syncProfileState(newProfiles);
			saveProfiles(newProfiles);
		},
		[syncProfileState],
	);

	const handleSwitchProfile = useCallback(
		(profileId: string) => {
			const profiles = profilesRef.current;
			const next = profiles.map((p) => {
				if (p.id === profileId) return { ...p, role: "primary" as const };
				if (p.role === "primary") return { ...p, role: "off" as const };
				return p;
			});
			syncProfileState(next);
			saveProfiles(next);
			syncActiveProfile(next);
			loadActiveQualityColors().then(setQualityColors);
			const switched = next.find((p) => p.id === profileId);
			if (switched) showProfileToast(switched);
		},
		[syncProfileState, showProfileToast],
	);

	const dismissKeyRef = useRef("Escape");

	useEffect(() => {
		// Load settings
		const reloadSettings = () => {
			loadGeneral().then((s) => {
				setOverlayScale(s.overlayScale);
				setOverlayMode(s.overlayPosition);
				setCompactMode(s.compactPosition ?? "cursor");
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
			loadMarketplace().then(setMarketplaceSettings);
			loadProfiles().then((profiles) => {
				syncProfileState(profiles);
				if (!startupToastShown.current) {
					const primary = profiles.find((p) => p.role === "primary");
					if (primary) {
						startupToastShown.current = true;
						// Delay startup toast to avoid stealing focus from PoE
						setTimeout(() => showProfileToast(primary), 2000);
					}
				}
			});
			syncActiveProfile();
		};
		reloadSettings();

		const unlistenPosition = listen<{ x: number; y: number }>("overlay-position", (event) => {
			setCursorPos(event.payload);
		});

		const unlistenInspectMode = listen<string>("inspect-mode", (event) => {
			const mode = event.payload as "full" | "compact" | "trade";
			setCompactFading(false);
			// Clear any existing compact auto-dismiss timer
			if (compactTimerRef.current) {
				clearTimeout(compactTimerRef.current);
				compactTimerRef.current = null;
			}
			setInspectMode(mode);
		});

		const unlistenEvaluated = listen<ItemPayload>("item-evaluated", (event) => {
			reloadSettings();
			setPanelReady(false);
			setPanelSize(null);
			setEvaluatedItem(event.payload);
			setItemText(null);
			setParseError(null);
			setShowMock(false);
		});

		const unlistenCapture = listen<string>("item-captured", (event) => {
			reloadSettings();
			setPanelReady(false);
			setPanelSize(null);
			setItemText(event.payload);
			setEvaluatedItem(null);
			setParseError(null);
			setShowMock(false);
		});

		const unlistenParseFailed = listen<{ error: string; rawText: string }>(
			"item-parse-failed",
			(event) => {
				reloadSettings();
				setPanelReady(false);
				setPanelSize(null);
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
			setPanelReady(false);
			setPanelSize(null);
			setShowMock(true);
		});

		const unlistenCycleProfile = listen("cycle-profile", async () => {
			// Reload profiles from store — local state may be stale
			const fresh = await loadProfiles();
			syncProfileState(fresh);
			if (fresh.length < 2) return;
			const primaryIdx = fresh.findIndex((p) => p.role === "primary");
			const nextIdx = (primaryIdx + 1) % fresh.length;
			const next = fresh[nextIdx];
			if (next) handleSwitchProfile(next.id);
		});

		const unlistenSwitchProfile = listen<string>("switch-profile", async (event) => {
			// Reload profiles from store — local state may be stale
			const fresh = await loadProfiles();
			syncProfileState(fresh);
			handleSwitchProfile(event.payload);
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
			unlistenInspectMode.then((fn) => fn());
			unlistenEvaluated.then((fn) => fn());
			unlistenCapture.then((fn) => fn());
			unlistenParseFailed.then((fn) => fn());
			unlistenDismiss.then((fn) => fn());
			unlistenDebug.then((fn) => fn());
			unlistenCycleProfile.then((fn) => fn());
			unlistenSwitchProfile.then((fn) => fn());
			document.removeEventListener("keydown", handleKeydown);
		};
	}, [dismiss, showProfileToast, syncProfileState, handleSwitchProfile]);

	// When new content arrives (panelReady=false), wait for the browser to
	// complete layout before making the panel visible. This prevents a flash
	// of partially-laid-out content that leaves ghost pixels on the transparent
	// overlay surface (WebKitGTK does not fully clear the backing store between
	// rapid repaints of different-sized content at different positions).
	useEffect(() => {
		if (panelReady) return;
		const id = requestAnimationFrame(() => {
			requestAnimationFrame(() => {
				// Measure actual panel size before making visible
				if (panelRef.current) {
					const rect = panelRef.current.getBoundingClientRect();
					setPanelSize({ width: rect.width, height: rect.height });
				}
				setPanelReady(true);
			});
		});
		return () => cancelAnimationFrame(id);
	}, [panelReady]);

	// Async RQE demand check — runs when a new evaluated item arrives (dev only)
	useEffect(() => {
		if (!rqe || !evaluatedItem) return;
		setDemandResult(null);
		rqe.checkDemand(evaluatedItem.item, marketplaceSettings).then((result) => {
			if (result && result.count > 0) {
				setDemandResult(result);
			}
		});
	}, [evaluatedItem, marketplaceSettings]);

	// Auto-dismiss compact mode after 2.5s (fade out at 2.2s, dismiss at 2.5s)
	const hasCompactContent = inspectMode === "compact" && (evaluatedItem || parseError);
	useEffect(() => {
		if (!hasCompactContent) return;
		const fadeTimer = setTimeout(() => setCompactFading(true), 2200);
		compactTimerRef.current = setTimeout(() => {
			dismiss();
		}, 2500);
		return () => {
			clearTimeout(fadeTimer);
			if (compactTimerRef.current) {
				clearTimeout(compactTimerRef.current);
				compactTimerRef.current = null;
			}
		};
	}, [hasCompactContent, dismiss]);

	const zoom = overlayScale / 100;
	const isCompact = inspectMode === "compact";
	const activeMode = isCompact ? compactMode : overlayMode;
	const pos = computePanelPosition(cursorPos, activeMode, zoom, panelSize ?? undefined);

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
	if (!panelReady) {
		panelStyle.visibility = "hidden";
	}

	// Determine content to display
	let content: preact.ComponentChildren = null;
	let showDismiss = true;

	const tradeConfig: TradeQueryConfig = {
		league: tradeSettings.league,
		valueRelaxation: tradeSettings.valueRelaxation,
		listingStatus: tradeSettings.listingStatus ?? "available",
		searchDefaults: tradeSettings.searchDefaults,
	};

	const tradeFilters = useTradeFilters(
		evaluatedItem?.rawText ?? "",
		tradeConfig,
		inspectMode === "trade",
	);

	// Trade panel goes on the opposite side from the screen edge:
	// right-anchored overlay → trade on left; left-anchored → trade on right.
	const tradeSide = pos.anchor === "right" ? "left" : "right";

	if (evaluatedItem && !showMock && inspectMode === "compact") {
		// Compact mode: small pill near cursor, no dismiss button, no backdrop click
		showDismiss = false;
		content = (
			<div class={compactFading ? "compact-pill-fading" : ""}>
				<CompactPill
					item={evaluatedItem}
					profiles={profileSummaries}
					mapDanger={mapDanger}
					demandCount={demandResult?.count}
					demandColor={marketplaceSettings.badgeColor}
				/>
			</div>
		);
	} else if (evaluatedItem && !showMock) {
		const tradeEditProps = tradeFilters.editMode
			? {
					mappedStats: tradeFilters.mappedStats,
					isStatEnabled: tradeFilters.isStatEnabled,
					getStatMin: tradeFilters.getStatMin,
					getStatMax: tradeFilters.getStatMax,
					toggleStat: tradeFilters.toggleStat,
					setStatMin: tradeFilters.setStatMin,
					setStatMax: tradeFilters.setStatMax,
					filterMap: tradeFilters.filterMap,
					filterOverrides: tradeFilters.filterOverrides,
					onFilterOverride: tradeFilters.onFilterOverride,
					rarityFilter: tradeFilters.rarityFilter,
					typeScopeOptions: tradeFilters.editSchema?.typeScope.options ?? [],
					typeScope: tradeFilters.typeScope,
					setTypeScope: tradeFilters.setTypeScope,
				}
			: undefined;

		const itemCard = (
			<div class="overlay-item-col">
				<ItemOverlay
					item={evaluatedItem.item}
					eval={evaluatedItem.eval}
					display={displaySettings}
					tradeEdit={tradeEditProps}
					mapDanger={mapDanger}
					onDangerChange={handleDangerChange}
					profiles={profileSummaries}
					onSwitchProfile={handleSwitchProfile}
				/>
				{demandResult != null &&
					demandResult.count > 0 &&
					(() => {
						const owners = demandResult.matches
							.map((m) => m.owner)
							.filter((o): o is string => o != null)
							.filter((v, i, a) => a.indexOf(v) === i);
						const shown = owners.slice(0, 3);
						const remaining = owners.length - shown.length;

						return (
							<div class="demand-badge-wrap">
								<div
									class="demand-badge"
									style={{ "--demand-color": marketplaceSettings.badgeColor }}
								>
									{demandResult.count}
								</div>
								<div class="demand-tooltip">
									<div class="demand-tooltip-header">
										{demandResult.count} want {demandResult.count === 1 ? "list" : "lists"} match
									</div>
									{shown.map((owner) => (
										<div key={owner} class="demand-tooltip-row">
											{owner}
										</div>
									))}
									{remaining > 0 && <div class="demand-tooltip-more">+{remaining} more</div>}
								</div>
							</div>
						);
					})()}
			</div>
		);
		const tradeCol = (
			<div class="overlay-trade-col">
				<TradePanel itemText={evaluatedItem.rawText} config={tradeConfig} filters={tradeFilters} />
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
	} else if (parseError && !showMock && inspectMode === "compact") {
		// Compact mode: show N/A pill for parse errors
		showDismiss = false;
		content = (
			<div class={compactFading ? "compact-pill-fading" : ""}>
				<CompactPillNA />
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
			class={`overlay-backdrop ${inspectMode === "compact" ? "compact-backdrop" : ""}`}
			onClick={(e) => {
				// Click on backdrop (not on panel) = dismiss (not in compact mode — it's click-through)
				if (inspectMode !== "compact" && e.target === e.currentTarget) dismiss();
			}}
		>
			{content && (
				<div ref={panelRef} class="overlay-panel" style={panelStyle}>
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
