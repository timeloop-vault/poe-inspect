import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "preact/hooks";
import {
	type TradeSettings as TradeSettingsType,
	defaultTrade,
	loadTrade,
	saveTrade,
} from "../../store";
import type { League, LeagueList } from "../../types";

export function TradeSettings() {
	const [settings, setSettings] = useState<TradeSettingsType>(defaultTrade);
	const [loaded, setLoaded] = useState(false);
	const [leagues, setLeagues] = useState<League[]>([]);
	const [privateLeagues, setPrivateLeagues] = useState<League[]>([]);
	const [leaguesLoading, setLeaguesLoading] = useState(false);
	const [leaguesError, setLeaguesError] = useState<string | null>(null);
	const [statsRefreshing, setStatsRefreshing] = useState(false);
	const [statsResult, setStatsResult] = useState<string | null>(null);

	useEffect(() => {
		loadTrade().then((s) => {
			setSettings(s);
			setLoaded(true);
		});
	}, []);

	// Fetch leagues on mount.
	useEffect(() => {
		fetchLeagues();
	}, []);

	const update = useCallback((patch: Partial<TradeSettingsType>) => {
		setSettings((prev) => {
			const next = { ...prev, ...patch };
			saveTrade(next);
			return next;
		});
	}, []);

	async function fetchLeagues() {
		setLeaguesLoading(true);
		setLeaguesError(null);
		try {
			const result = await invoke<LeagueList>("fetch_leagues");
			setLeagues(result.leagues);
			setPrivateLeagues(result.privateLeagues);
		} catch (e) {
			setLeaguesError(String(e));
		} finally {
			setLeaguesLoading(false);
		}
	}

	async function refreshStats() {
		setStatsRefreshing(true);
		setStatsResult(null);
		try {
			const matched = await invoke<number>("refresh_trade_stats");
			setStatsResult(`Index refreshed: ${matched} stats mapped`);
		} catch (e) {
			setStatsResult(`Error: ${e}`);
		} finally {
			setStatsRefreshing(false);
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
						Online only
						<div class="setting-description">Only show listings from players currently online.</div>
					</div>
					<Toggle checked={settings.onlineOnly} onChange={(v) => update({ onlineOnly: v })} />
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

function Toggle({ checked, onChange }: { checked: boolean; onChange: (v: boolean) => void }) {
	return (
		<label class="setting-toggle">
			<input
				type="checkbox"
				checked={checked}
				onChange={(e) => onChange((e.target as HTMLInputElement).checked)}
			/>
			<span class="toggle-track" />
		</label>
	);
}
