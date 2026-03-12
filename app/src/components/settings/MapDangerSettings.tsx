import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useMemo, useRef, useState } from "preact/hooks";
import {
	type DangerLevel,
	type StoredProfile,
	loadProfiles,
	saveProfiles,
	syncActiveProfile,
} from "../../store";

interface MapModTemplate {
	template: string;
	statIds: string[];
}

/** Danger levels in cycle order for radio selection. */
const DANGER_LEVELS: { value: DangerLevel | null; label: string; cls: string }[] = [
	{ value: null, label: "—", cls: "danger-radio-unclassified" },
	{ value: "deadly", label: "Deadly", cls: "danger-radio-deadly" },
	{ value: "warning", label: "Warning", cls: "danger-radio-warning" },
	{ value: "good", label: "Safe", cls: "danger-radio-good" },
];

export function MapDangerSettings() {
	const [templates, setTemplates] = useState<MapModTemplate[]>([]);
	const [profiles, setProfiles] = useState<StoredProfile[]>([]);
	const [selectedProfileId, setSelectedProfileId] = useState<string | null>(null);
	const [search, setSearch] = useState("");
	const [loaded, setLoaded] = useState(false);
	const profilesRef = useRef<StoredProfile[]>([]);

	useEffect(() => {
		Promise.all([invoke<MapModTemplate[]>("get_map_mod_templates"), loadProfiles()]).then(
			([t, p]) => {
				setTemplates(t);
				setProfiles(p);
				profilesRef.current = p;
				const primary = p.find((pr) => pr.role === "primary");
				setSelectedProfileId(primary?.id ?? p[0]?.id ?? null);
				setLoaded(true);
			},
		);
	}, []);

	const selectedProfile = profiles.find((p) => p.id === selectedProfileId);
	const mapDanger = selectedProfile?.mapDanger ?? {};

	const persist = useCallback((next: StoredProfile[]) => {
		profilesRef.current = next;
		setProfiles(next);
		saveProfiles(next);
		syncActiveProfile(next);
	}, []);

	const setDanger = useCallback(
		(template: string, level: DangerLevel | null) => {
			if (!selectedProfileId) return;
			const next = profilesRef.current.map((p) => {
				if (p.id !== selectedProfileId) return p;
				const updated = { ...p.mapDanger };
				if (level === null) {
					delete updated[template];
				} else {
					updated[template] = level;
				}
				return { ...p, mapDanger: updated };
			});
			persist(next);
		},
		[selectedProfileId, persist],
	);

	const filtered = useMemo(() => {
		if (!search.trim()) return templates;
		const q = search.toLowerCase();
		return templates.filter((t) => t.template.toLowerCase().includes(q));
	}, [templates, search]);

	// Stats
	const classifiedCount = Object.keys(mapDanger).length;
	const totalCount = templates.length;

	if (!loaded) return null;

	return (
		<>
			<h2>Map Danger</h2>

			<div class="setting-group">
				<h3>Profile</h3>
				<div class="setting-row">
					<div class="setting-label">
						Classify map mods for
						<div class="setting-description">Each profile has its own danger classifications</div>
					</div>
					<select
						class="danger-profile-select"
						value={selectedProfileId ?? ""}
						onChange={(e) => setSelectedProfileId((e.target as HTMLSelectElement).value)}
					>
						{profiles.map((p) => (
							<option key={p.id} value={p.id}>
								{p.name}
								{p.role === "primary" ? " (primary)" : ""}
							</option>
						))}
					</select>
				</div>
			</div>

			<div class="setting-group">
				<h3>
					Map Mods
					<span class="danger-stats">
						{classifiedCount}/{totalCount} classified
					</span>
				</h3>

				<div class="danger-search-row">
					<input
						type="text"
						class="danger-search"
						placeholder="Search map mods..."
						value={search}
						onInput={(e) => setSearch((e.target as HTMLInputElement).value)}
					/>
					{search && (
						<button type="button" class="danger-search-clear" onClick={() => setSearch("")}>
							&times;
						</button>
					)}
				</div>

				<div class="danger-list">
					{filtered.map((t) => {
						const level = mapDanger[t.template] ?? null;
						return (
							<div key={t.template} class="danger-row">
								<div class="danger-template">{t.template}</div>
								<div class="danger-radios">
									{DANGER_LEVELS.map((dl) => (
										<button
											key={dl.label}
											type="button"
											class={`danger-radio ${dl.cls} ${level === dl.value ? "active" : ""}`}
											onClick={() => setDanger(t.template, dl.value)}
											title={dl.label}
										>
											{dl.label}
										</button>
									))}
								</div>
							</div>
						);
					})}
					{filtered.length === 0 && <div class="danger-empty">No map mods match "{search}"</div>}
				</div>
			</div>
		</>
	);
}
