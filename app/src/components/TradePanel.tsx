import { invoke } from "@tauri-apps/api/core";
import { useCallback, useState } from "preact/hooks";
import type { TradeFilters } from "../hooks/useTradeFilters";
import type { PriceCheckResult, TradeQueryConfig, TypeSearchScope } from "../types";

interface TradePanelProps {
	/** Raw item text from clipboard (Ctrl+Alt+C). */
	itemText: string;
	/** Trade config from user settings. */
	config: TradeQueryConfig;
	/** Trade filter state from useTradeFilters hook. */
	filters: TradeFilters;
	/** Base type from item header (for breadcrumb display). */
	baseType: string;
	/** Item class from item header (e.g., "Wands"). */
	itemClass: string;
}

type TradeState =
	| { status: "idle" }
	| { status: "loading" }
	| { status: "results"; result: PriceCheckResult }
	| { status: "empty" }
	| { status: "error"; message: string }
	| { status: "rate-limited"; retryAfterSecs: number };

const scopeLabels: Record<TypeSearchScope, string> = {
	baseType: "Base Type",
	itemClass: "Item Class",
	any: "Any",
};

const scopeOrder: TypeSearchScope[] = ["baseType", "itemClass", "any"];

export function TradePanel({ itemText, config, filters, baseType, itemClass }: TradePanelProps) {
	const [state, setState] = useState<TradeState>({ status: "idle" });
	const [urlLoading, setUrlLoading] = useState(false);

	const hasLeague = config.league.length > 0;

	const priceCheck = useCallback(async () => {
		if (!hasLeague) return;
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
		} catch (e) {
			const msg = String(e);
			const rateLimitMatch = msg.match(/retry after (\d+)s/i);
			if (rateLimitMatch) {
				setState({
					status: "rate-limited",
					retryAfterSecs: Number(rateLimitMatch[1]),
				});
			} else {
				setState({ status: "error", message: msg });
			}
		}
	}, [itemText, config, hasLeague, filters.filterConfig]);

	const openTrade = useCallback(async () => {
		if (!hasLeague) return;
		setUrlLoading(true);
		try {
			const url = await invoke<string>("trade_search_url", {
				itemText,
				config,
				filterConfig: filters.filterConfig,
			});
			await invoke("open_url", { url });
		} catch (e) {
			console.error("Failed to open trade URL:", e);
		} finally {
			setUrlLoading(false);
		}
	}, [itemText, config, hasLeague, filters.filterConfig]);

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
					disabled={!hasLeague || state.status === "loading"}
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
					disabled={!hasLeague || urlLoading}
					title={disabledTitle}
				>
					{urlLoading ? (
						<>
							<span class="trade-spinner" />
							Opening...
						</>
					) : (
						"Open Trade"
					)}
				</button>
			</div>

			{/* Type scope breadcrumb — shown in edit mode */}
			{filters.editMode && (
				<div class="trade-type-scope">
					{scopeOrder.map((scope) => {
						const label =
							scope === "baseType" ? baseType : scope === "itemClass" ? itemClass : "Any";
						return (
							<button
								key={scope}
								type="button"
								class={`scope-btn ${filters.typeScope === scope ? "scope-active" : ""}`}
								onClick={() => filters.setTypeScope(scope)}
								title={scopeLabels[scope]}
							>
								{label}
							</button>
						);
					})}
				</div>
			)}

			{state.status === "results" && <TradeResults result={state.result} />}
			{state.status === "empty" && <div class="trade-message">No listings found</div>}
			{state.status === "error" && (
				<div class="trade-message trade-message-error">
					{state.message}
					<button type="button" class="trade-retry-btn" onClick={priceCheck}>
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
