import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState } from "preact/hooks";
import type { TradeFilters } from "../hooks/useTradeFilters";
import type { PriceCheckResult, TradeQueryConfig } from "../types";

interface TradePanelProps {
	/** Raw item text from clipboard (Ctrl+Alt+C). */
	itemText: string;
	/** Trade config from user settings. */
	config: TradeQueryConfig;
	/** Trade filter state from useTradeFilters hook. */
	filters: TradeFilters;
	/** Auto-fire price check on new item (trade mode). */
	autoSearch?: boolean;
}

type TradeState =
	| { status: "idle" }
	| { status: "loading" }
	| { status: "results"; result: PriceCheckResult }
	| { status: "empty" }
	| { status: "error"; message: string }
	| { status: "rate-limited"; retryAfterSecs: number };

/** Minimum cooldown (ms) between trade API operations. */
const COOLDOWN_MS = 2000;

export function TradePanel({ itemText, config, filters, autoSearch }: TradePanelProps) {
	const [state, setState] = useState<TradeState>({ status: "idle" });
	const [busy, setBusy] = useState(false);
	const [cooldown, setCooldown] = useState(false);
	const cooldownTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
	const pendingAutoSearch = useRef(false);

	const hasLeague = config.league.length > 0;
	const disabled = !hasLeague || busy || cooldown;

	// Clear cooldown timer on unmount.
	useEffect(() => {
		return () => {
			if (cooldownTimer.current) clearTimeout(cooldownTimer.current);
		};
	}, []);

	// Auto-search: mark pending when a new item arrives in trade mode.
	// Fire when not busy/cooldown. If another item arrives while queued,
	// it supersedes the previous (pendingAutoSearch is just a boolean flag,
	// and itemText is always the latest).
	useEffect(() => {
		if (autoSearch && itemText) {
			pendingAutoSearch.current = true;
		}
	}, [autoSearch, itemText]);

	/** Start a cooldown period after any trade operation. */
	const startCooldown = useCallback((ms: number) => {
		setCooldown(true);
		if (cooldownTimer.current) clearTimeout(cooldownTimer.current);
		cooldownTimer.current = setTimeout(() => setCooldown(false), ms);
	}, []);

	const priceCheck = useCallback(async () => {
		if (disabled) return;
		setBusy(true);
		setState({ status: "loading" });
		try {
			const result = await invoke<PriceCheckResult>("price_check", {
				itemText,
				config,
				filterConfig: filters.filterConfig,
			});
			if (result.total === 0 || result.prices.length === 0) {
				setState({ status: "empty" });
			} else {
				setState({ status: "results", result });
			}
			startCooldown(COOLDOWN_MS);
		} catch (e) {
			const msg = String(e);
			const rateLimitMatch = msg.match(/retry after (\d+)s/i);
			if (rateLimitMatch) {
				const secs = Number(rateLimitMatch[1]);
				setState({ status: "rate-limited", retryAfterSecs: secs });
				// Block for the full penalty duration.
				startCooldown(secs * 1000);
			} else {
				setState({ status: "error", message: msg });
				startCooldown(COOLDOWN_MS);
			}
		} finally {
			setBusy(false);
		}
	}, [itemText, config, filters.filterConfig, disabled, startCooldown]);

	// Fire queued auto-search when cooldown/busy clears.
	useEffect(() => {
		if (!pendingAutoSearch.current || busy || cooldown || !hasLeague) return;
		pendingAutoSearch.current = false;
		priceCheck();
	}, [busy, cooldown, hasLeague, priceCheck]);

	const openTrade = useCallback(async () => {
		if (disabled) return;
		setBusy(true);
		try {
			const url = await invoke<string>("trade_search_url", {
				itemText,
				config,
				filterConfig: filters.filterConfig,
			});
			await invoke("open_url", { url });
			startCooldown(COOLDOWN_MS);
		} catch (e) {
			console.error("Failed to open trade URL:", e);
			startCooldown(COOLDOWN_MS);
		} finally {
			setBusy(false);
		}
	}, [itemText, config, filters.filterConfig, disabled, startCooldown]);

	const disabledTitle = hasLeague ? undefined : "Set a league in Settings > Trade";

	return (
		<div class="trade-panel">
			<div class="trade-actions">
				<button
					type="button"
					class={`trade-action-btn ${filters.editMode ? "trade-action-active" : "trade-action-secondary"}`}
					onClick={filters.toggleEditMode}
					disabled={!hasLeague}
					title={hasLeague ? "Toggle search filter editing" : disabledTitle}
				>
					{filters.editMode ? "Done" : "Edit Search"}
				</button>
				<button
					type="button"
					class="trade-action-btn"
					onClick={priceCheck}
					disabled={disabled}
					title={disabledTitle}
				>
					{state.status === "loading" ? (
						<>
							<span class="trade-spinner" />
							Searching...
						</>
					) : (
						"Price Check"
					)}
				</button>
				<button
					type="button"
					class="trade-action-btn trade-action-secondary"
					onClick={openTrade}
					disabled={disabled}
					title={disabledTitle}
				>
					{busy && state.status !== "loading" ? (
						<>
							<span class="trade-spinner" />
							Opening...
						</>
					) : (
						"Open Trade"
					)}
				</button>
			</div>

			{state.status === "results" && <TradeResults result={state.result} />}
			{state.status === "empty" && <div class="trade-message">No listings found</div>}
			{state.status === "error" && (
				<div class="trade-message trade-message-error">
					{state.message}
					<button type="button" class="trade-retry-btn" onClick={priceCheck} disabled={disabled}>
						Retry
					</button>
				</div>
			)}
			{state.status === "rate-limited" && (
				<div class="trade-message trade-message-warn">
					Rate limited — retry in {state.retryAfterSecs}s
				</div>
			)}
		</div>
	);
}

function TradeResults({ result }: { result: PriceCheckResult }) {
	return (
		<div class="trade-results">
			<div class="trade-results-header">
				{result.total} listing{result.total !== 1 ? "s" : ""}
			</div>
			<div class="trade-price-list">
				{result.prices.map((p, i) => (
					// biome-ignore lint/suspicious/noArrayIndexKey: static price snapshot, never reordered
					<div class="trade-price-row" key={i}>
						<span class="trade-price-amount">{formatPrice(p.amount)}</span>
						<span class="trade-price-currency">{p.currency}</span>
					</div>
				))}
			</div>
		</div>
	);
}

function formatPrice(amount: number): string {
	if (Number.isInteger(amount)) return String(amount);
	return amount.toFixed(1);
}
