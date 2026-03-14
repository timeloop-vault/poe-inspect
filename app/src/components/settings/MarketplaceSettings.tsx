import { useCallback, useEffect, useState } from "preact/hooks";
import {
	type MarketplaceSettings as MarketplaceConfig,
	defaultMarketplace,
	isValidAccountName,
	loadMarketplace,
	normalizeAccountName,
	saveMarketplace,
} from "../../store";
import { QueryEditor } from "./QueryEditor";

// --- Types for RQE server responses ---

interface HealthResponse {
	status: string;
	query_count: number;
	node_count: number;
}

interface StoredQuery {
	id: number;
	conditions: unknown[];
	labels: string[];
	owner: string | null;
}

// --- Sub-components ---

function LoginGate({
	onLogin,
}: {
	onLogin: (settings: MarketplaceConfig) => void;
}) {
	const [accountName, setAccountName] = useState("");
	const [serverUrl, setServerUrl] = useState(defaultMarketplace.serverUrl);
	const [apiKey, setApiKey] = useState("");
	const [error, setError] = useState<string | null>(null);
	const [connecting, setConnecting] = useState(false);

	const handleConnect = useCallback(async () => {
		setError(null);

		if (!isValidAccountName(accountName)) {
			setError("Invalid format. Use Name#0000 (e.g., Stefan#1234)");
			return;
		}

		const url = serverUrl.replace(/\/+$/, "");
		if (!url) {
			setError("Server URL is required");
			return;
		}

		setConnecting(true);
		try {
			const resp = await fetch(`${url}/health`);
			if (!resp.ok) {
				setError(`Server returned ${resp.status}`);
				return;
			}
			const health: HealthResponse = await resp.json();
			if (health.status !== "ok") {
				setError("Server health check failed");
				return;
			}

			const settings: MarketplaceConfig = {
				accountName: normalizeAccountName(accountName),
				serverUrl: url,
				apiKey: apiKey || null,
				enabled: true,
				badgeColor: "#f1c40f",
			};
			await saveMarketplace(settings);
			onLogin(settings);
		} catch {
			setError("Cannot reach server. Is rqe-server running?");
		} finally {
			setConnecting(false);
		}
	}, [accountName, serverUrl, apiKey, onLogin]);

	return (
		<div>
			<h2>Demand Marketplace</h2>
			<p class="setting-description" style={{ marginBottom: 16 }}>
				Register items you're looking for and get notified when they appear. Connect to the reverse
				query engine to manage your want lists.
			</p>

			<div class="setting-group">
				<h3>Account</h3>
				<label class="setting-row">
					<span class="setting-label">PoE Account Name</span>
					<input
						type="text"
						class="setting-select"
						placeholder="PlayerName#1234"
						value={accountName}
						onInput={(e) => setAccountName((e.target as HTMLInputElement).value)}
						onKeyDown={(e) => e.key === "Enter" && handleConnect()}
						style={{ width: 240 }}
					/>
				</label>
				<p class="setting-description">
					Format: Name#0000. Use your real PoE account name so your data carries over when we switch
					to GGG OAuth.
				</p>
			</div>

			<div class="setting-group">
				<h3>Server</h3>
				<label class="setting-row">
					<span class="setting-label">Server URL</span>
					<input
						type="text"
						class="setting-select"
						value={serverUrl}
						onInput={(e) => setServerUrl((e.target as HTMLInputElement).value)}
						style={{ width: 300 }}
					/>
				</label>
				<label class="setting-row">
					<span class="setting-label">API Key (optional)</span>
					<input
						type="password"
						class="setting-select"
						placeholder="Leave empty if auth is disabled"
						value={apiKey}
						onInput={(e) => setApiKey((e.target as HTMLInputElement).value)}
						style={{ width: 300 }}
					/>
				</label>
			</div>

			{error && <p style={{ color: "#e04040", marginTop: 8 }}>{error}</p>}

			<button
				type="button"
				class="btn btn-primary"
				onClick={handleConnect}
				disabled={connecting}
				style={{ marginTop: 12 }}
			>
				{connecting ? "Connecting..." : "Connect"}
			</button>
		</div>
	);
}

function AccountBar({
	settings,
	health,
	onLogout,
}: {
	settings: MarketplaceConfig;
	health: HealthResponse | null;
	onLogout: () => void;
}) {
	const connected = health !== null;
	return (
		<div
			class="setting-group"
			style={{
				display: "flex",
				alignItems: "center",
				gap: 12,
				padding: "8px 12px",
			}}
		>
			<span
				style={{
					width: 8,
					height: 8,
					borderRadius: "50%",
					background: connected ? "#2ecc71" : "#e04040",
					flexShrink: 0,
				}}
			/>
			<div style={{ flex: 1 }}>
				<strong>{settings.accountName}</strong>
				{health ? (
					<span class="setting-description" style={{ marginLeft: 8 }}>
						{health.query_count} want {health.query_count === 1 ? "list" : "lists"}
					</span>
				) : (
					<span style={{ marginLeft: 8, color: "#e04040", fontSize: 12 }}>Server unreachable</span>
				)}
			</div>
			<button type="button" class="btn btn-small" onClick={onLogout}>
				Disconnect
			</button>
		</div>
	);
}

function QueryList({
	queries,
	onRefresh,
	onEdit,
	onDelete,
}: {
	queries: StoredQuery[];
	onRefresh: () => void;
	onEdit: (query: StoredQuery) => void;
	onDelete: (id: number) => void;
}) {
	if (queries.length === 0) {
		return (
			<div class="setting-group">
				<h3>Your Want Lists</h3>
				<p class="setting-description">No want lists yet. Use the CLI to add queries:</p>
				<code
					style={{
						display: "block",
						background: "rgba(0,0,0,0.3)",
						padding: 8,
						borderRadius: 4,
						fontSize: 12,
						marginTop: 4,
					}}
				>
					rqe-cli add-query your-query.json
				</code>
			</div>
		);
	}

	return (
		<div class="setting-group">
			<div
				style={{
					display: "flex",
					justifyContent: "space-between",
					alignItems: "center",
				}}
			>
				<h3>Your Want Lists</h3>
				<button type="button" class="btn btn-small" onClick={onRefresh}>
					Refresh
				</button>
			</div>
			{queries.map((q) => (
				<div
					key={q.id}
					class="setting-row"
					style={{
						justifyContent: "space-between",
						padding: "6px 0",
						borderBottom: "1px solid var(--poe-border)",
					}}
				>
					<div>
						<strong style={{ color: "var(--poe-text)" }}>{q.labels[0] || `Query #${q.id}`}</strong>
						<span class="setting-description" style={{ marginLeft: 8 }}>
							{q.conditions.length} condition
							{q.conditions.length !== 1 ? "s" : ""}
						</span>
						{q.labels.length > 1 && (
							<span class="setting-description" style={{ marginLeft: 8 }}>
								{q.labels.slice(1).join(", ")}
							</span>
						)}
					</div>
					<div style={{ display: "flex", gap: 4 }}>
						<button type="button" class="btn btn-small" onClick={() => onEdit(q)}>
							Edit
						</button>
						<button type="button" class="btn btn-small btn-danger" onClick={() => onDelete(q.id)}>
							Delete
						</button>
					</div>
				</div>
			))}
		</div>
	);
}

// --- Main component ---

export function MarketplaceSettings() {
	const [settings, setSettings] = useState<MarketplaceConfig | null>(null);
	const [health, setHealth] = useState<HealthResponse | null>(null);
	const [queries, setQueries] = useState<StoredQuery[]>([]);
	const [loaded, setLoaded] = useState(false);
	const [editing, setEditing] = useState<StoredQuery | "new" | null>(null);

	// Load saved settings on mount
	useEffect(() => {
		loadMarketplace().then((s) => {
			setSettings(s);
			setLoaded(true);
		});
	}, []);

	// Fetch health + queries when logged in
	const refreshData = useCallback(async (s: MarketplaceConfig) => {
		if (!s.accountName) return;
		try {
			const url = s.serverUrl;
			const healthResp = await fetch(`${url}/health`);
			if (healthResp.ok) {
				setHealth(await healthResp.json());
			} else {
				setHealth(null);
			}

			const queryResp = await fetch(`${url}/queries?owner=${encodeURIComponent(s.accountName)}`);
			if (queryResp.ok) {
				setQueries(await queryResp.json());
			}
		} catch {
			setHealth(null);
		}
	}, []);

	// Auto-refresh: on login, on returning from editor, and on mount
	useEffect(() => {
		if (settings?.accountName && editing === null) {
			refreshData(settings);
		}
	}, [settings, editing, refreshData]);

	const handleLogin = useCallback(
		(s: MarketplaceConfig) => {
			setSettings(s);
			refreshData(s);
		},
		[refreshData],
	);

	const handleLogout = useCallback(async () => {
		const cleared: MarketplaceConfig = { ...defaultMarketplace };
		await saveMarketplace(cleared);
		setSettings(cleared);
		setHealth(null);
		setQueries([]);
	}, []);

	const handleDelete = useCallback(
		async (id: number) => {
			if (!settings) return;
			const headers: Record<string, string> = {};
			if (settings.apiKey) {
				headers["X-API-Key"] = settings.apiKey;
			}
			try {
				await fetch(`${settings.serverUrl}/queries/${id}`, {
					method: "DELETE",
					headers,
				});
				setQueries((prev) => prev.filter((q) => q.id !== id));
				// Refresh health to update counts
				const healthResp = await fetch(`${settings.serverUrl}/health`);
				if (healthResp.ok) {
					setHealth(await healthResp.json());
				}
			} catch {
				setHealth(null); // Mark as disconnected
			}
		},
		[settings],
	);

	if (!loaded) return null;

	// Not logged in → show login gate
	if (!settings?.accountName) {
		return <LoginGate onLogin={handleLogin} />;
	}

	// Logged in → show marketplace panel
	if (editing !== null) {
		return (
			<div>
				<h2>Demand Marketplace</h2>
				<AccountBar settings={settings} health={health} onLogout={handleLogout} />
				<QueryEditor
					settings={settings}
					editingQuery={editing !== "new" ? editing : null}
					onSave={() => {
						setEditing(null);
						refreshData(settings);
					}}
					onCancel={() => setEditing(null)}
				/>
			</div>
		);
	}

	return (
		<div>
			<h2>Demand Marketplace</h2>
			<AccountBar settings={settings} health={health} onLogout={handleLogout} />
			<QueryList
				queries={queries}
				onRefresh={() => refreshData(settings)}
				onEdit={(q) => setEditing(q)}
				onDelete={handleDelete}
			/>
			<button
				type="button"
				class="btn btn-primary"
				onClick={() => setEditing("new")}
				style={{ marginTop: 12 }}
			>
				+ Add Want List
			</button>
		</div>
	);
}
