import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "preact/hooks";
import {
	type TradeSettings as TradeSettingsType,
	defaultTrade,
	loadTrade,
	saveTrade,
} from "../../store";
import type { League, LeagueList } from "../../types";

interface IndexStatus {
	loaded: boolean;
	statCount: number;
	mappedCount: number;
}

interface ListingStatusOption {
	id: string;
	label: string;
}

export function TradeSettings() {
	const [settings, setSettings] = useState<TradeSettingsType>(defaultTrade);
	const [loaded, setLoaded] = useState(false);
	const [leagues, setLeagues] = useState<League[]>([]);
	const [privateLeagues, setPrivateLeagues] = useState<League[]>([]);
	const [leaguesLoading, setLeaguesLoading] = useState(false);
	const [leaguesError, setLeaguesError] = useState<string | null>(null);
	const [statsRefreshing, setStatsRefreshing] = useState(false);
	const [statsResult, setStatsResult] = useState<string | null>(null);
	const [indexStatus, setIndexStatus] = useState<IndexStatus | null>(null);
	const [listingStatuses, setListingStatuses] = useState<ListingStatusOption[]>([]);

	useEffect(() => {
		loadTrade().then((s) => {
			setSettings(s);
			setLoaded(true);
		});
	}, []);

	// Fetch leagues, listing statuses, + index status on mount.
	useEffect(() => {
		fetchLeagues();
		refreshIndexStatus();
		invoke<ListingStatusOption[]>("get_listing_statuses").then(setListingStatuses);
	}, []);

	const update = useCallback((patch: Partial<TradeSettingsType>) => {
		setSettings((prev) => {
			const next = { ...prev, ...patch };
			saveTrade(next);
			return next;
		});

		// Sync POESESSID to the Rust trade client when it changes.
		if ("poesessid" in patch) {
			invoke("set_trade_session", { poesessid: patch.poesessid ?? "" });
		}
	}, []);

	async function fetchLeagues() {
		if (leaguesLoading) return;
		setLeaguesLoading(true);
		setLeaguesError(null);
		try {
			const result = await invoke<LeagueList>("fetch_leagues");
			setLeagues(result.leagues);
			setPrivateLeagues(result.privateLeagues);

			// Auto-select first league if none saved.
			const firstLeague = result.leagues[0];
			setSettings((prev) => {
				if (prev.league === "" && firstLeague) {
					const next = { ...prev, league: firstLeague.id };
					saveTrade(next);
					return next;
				}
				return prev;
			});
		} catch (e) {
			setLeaguesError(String(e));
		} finally {
			// Keep disabled for 3s to prevent spamming GGG.
			setTimeout(() => setLeaguesLoading(false), 3000);
		}
	}

	async function refreshIndexStatus() {
		try {
			const status = await invoke<IndexStatus>("get_trade_index_status");
			setIndexStatus(status);
		} catch {
			// Ignore — command may not exist on older builds
		}
	}

	async function refreshStats() {
		if (statsRefreshing) return;
		setStatsRefreshing(true);
		setStatsResult(null);
		try {
			const matched = await invoke<number>("refresh_trade_stats");
			setStatsResult(`Index refreshed: ${matched} stats mapped`);
			refreshIndexStatus();
		} catch (e) {
			setStatsResult(`Error: ${e}`);
		} finally {
			// Keep disabled for 3s to prevent spamming GGG.
			setTimeout(() => setStatsRefreshing(false), 3000);
		}
	}

	if (!loaded) return null;

	const relaxPct = Math.round(settings.valueRelaxation * 100);

	return (
		<>
			<h2>Trade</h2>

			<div class="setting-group">
				<h3>League</h3>

				<div class="setting-row">
					<div class="setting-label">
						Active league
						<div class="setting-description">
							Select the league for trade searches. Fetched from pathofexile.com.
						</div>
					</div>
					<div class="setting-control">
						{leaguesLoading ? (
							<span class="trade-status">Loading leagues...</span>
						) : leaguesError ? (
							<span class="trade-status trade-error">{leaguesError}</span>
						) : (
							<select
								class="trade-select"
								value={settings.league}
								onChange={(e) => update({ league: (e.target as HTMLSelectElement).value })}
							>
								<option value="">— Select league —</option>
								{leagues.map((l) => (
									<option key={l.id} value={l.id}>
										{l.id}
									</option>
								))}
								{privateLeagues.length > 0 && (
									<optgroup label="Private Leagues">
										{privateLeagues.map((l) => (
											<option key={l.id} value={l.id}>
												{l.id}
											</option>
										))}
									</optgroup>
								)}
							</select>
						)}
						<button
							type="button"
							class="trade-btn trade-btn-small"
							onClick={fetchLeagues}
							disabled={leaguesLoading}
							title="Refresh league list"
						>
							&#x21bb;
						</button>
					</div>
				</div>
			</div>

			<div class="setting-group">
				<h3>Search</h3>

				<div class="setting-row">
					<div class="setting-label">
						Value relaxation
						<div class="setting-description">
							Search for items with at least this percentage of your roll values. Lower = broader
							search, more results.
						</div>
					</div>
					<div class="setting-slider">
						<input
							type="range"
							min={50}
							max={100}
							step={5}
							value={relaxPct}
							onInput={(e) =>
								update({
									valueRelaxation: Number((e.target as HTMLInputElement).value) / 100,
								})
							}
						/>
						<span class="slider-value">{relaxPct}%</span>
					</div>
				</div>

				<div class="setting-row">
					<div class="setting-label">
						Listing status
						<div class="setting-description">
							Filter trade results by seller availability. "Instant Buyout and In Person" matches
							the trade site default.
						</div>
					</div>
					<select
						class="setting-select"
						value={settings.listingStatus ?? "available"}
						onChange={(e) => update({ listingStatus: (e.target as HTMLSelectElement).value })}
					>
						{listingStatuses.map((s) => (
							<option key={s.id} value={s.id}>
								{s.label}
							</option>
						))}
					</select>
				</div>
			</div>

			<div class="setting-group">
				<h3>Authentication</h3>

				<div class="setting-row">
					<div class="setting-label">
						POESESSID
						<div class="setting-description">
							Session cookie from pathofexile.com. Enables accurate "online only" filtering and
							shows your own listings. Find it in your browser's cookies for pathofexile.com.
						</div>
					</div>
					<div class="setting-control">
						<input
							type="password"
							class="trade-input"
							placeholder="Paste your POESESSID"
							value={settings.poesessid}
							onInput={(e) => update({ poesessid: (e.target as HTMLInputElement).value })}
						/>
					</div>
				</div>
			</div>

			<div class="setting-group">
				<h3>Stats Index</h3>

				<div class="setting-row">
					<div class="setting-label">
						Trade stats index
						<div class="setting-description">
							Maps item stats to trade search filters. Refresh when a new league launches or if
							searches return unexpected results.
						</div>
					</div>
					<div class="setting-control">
						<button
							type="button"
							class="trade-btn"
							onClick={refreshStats}
							disabled={statsRefreshing}
						>
							{statsRefreshing ? "Refreshing..." : "Refresh Trade Stats"}
						</button>
					</div>
				</div>

				{indexStatus && (
					<div class="setting-row">
						<div class="setting-label" />
						<span class={`trade-status ${indexStatus.loaded ? "trade-success" : ""}`}>
							{indexStatus.loaded
								? `${indexStatus.statCount.toLocaleString()} stats loaded, ${indexStatus.mappedCount.toLocaleString()} GGPK IDs mapped`
								: "Not loaded — click Refresh Trade Stats"}
						</span>
					</div>
				)}

				{statsResult && (
					<div class="setting-row">
						<div class="setting-label" />
						<span
							class={`trade-status ${statsResult.startsWith("Error") ? "trade-error" : "trade-success"}`}
						>
							{statsResult}
						</span>
					</div>
				)}
			</div>
		</>
	);
}
