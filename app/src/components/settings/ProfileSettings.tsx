import { useState, useEffect, useCallback, useRef } from "preact/hooks";
import {
	type Profile,
	type Weight,
	type TierColors,
	defaultTierColors,
	loadProfiles,
	saveProfiles,
} from "../../store";

export function ProfileSettings() {
	const [profiles, setProfiles] = useState<Profile[]>([]);
	const [loaded, setLoaded] = useState(false);
	const [editing, setEditing] = useState<string | null>(null);

	// Keep a ref so callbacks always see latest profiles for saving
	const profilesRef = useRef(profiles);
	profilesRef.current = profiles;

	useEffect(() => {
		loadProfiles().then((p) => {
			setProfiles(p);
			setLoaded(true);
		});
	}, []);

	const persist = useCallback((next: Profile[]) => {
		setProfiles(next);
		saveProfiles(next);
	}, []);

	const setActive = useCallback((id: string) => {
		persist(profilesRef.current.map((p) => ({ ...p, active: p.id === id })));
	}, [persist]);

	const addProfile = useCallback(() => {
		const id = String(Date.now());
		const next: Profile[] = [
			...profilesRef.current,
			{
				id,
				name: "New Profile",
				active: false,
				modWeights: {},
				tierColors: { ...defaultTierColors },
				highlightWeights: true,
				dimIgnored: true,
			},
		];
		persist(next);
		setEditing(id);
	}, [persist]);

	const deleteProfile = useCallback((id: string) => {
		const filtered = profilesRef.current.filter((p) => p.id !== id);
		if (filtered.length > 0 && !filtered.some((p) => p.active)) {
			filtered[0]!.active = true;
		}
		persist(filtered);
	}, [persist]);

	const duplicateProfile = useCallback((id: string) => {
		const source = profilesRef.current.find((p) => p.id === id);
		if (!source) return;
		const newId = String(Date.now());
		persist([
			...profilesRef.current,
			{
				...source,
				id: newId,
				name: `${source.name} (copy)`,
				active: false,
				modWeights: { ...source.modWeights },
				tierColors: { ...source.tierColors },
			},
		]);
	}, [persist]);

	const renameProfile = useCallback((id: string, name: string) => {
		persist(profilesRef.current.map((p) => (p.id === id ? { ...p, name } : p)));
	}, [persist]);

	const updateProfile = useCallback((id: string, patch: Partial<Profile>) => {
		persist(profilesRef.current.map((p) => (p.id === id ? { ...p, ...patch } : p)));
	}, [persist]);

	if (!loaded) return null;

	// If editing a profile, show the profile editor
	if (editing !== null) {
		const profile = profiles.find((p) => p.id === editing);
		if (profile) {
			return (
				<ProfileEditor
					profile={profile}
					onBack={() => setEditing(null)}
					onRename={(name) => renameProfile(editing, name)}
					onUpdate={(patch) => updateProfile(editing, patch)}
				/>
			);
		}
	}

	return (
		<>
			<h2>Profiles</h2>

			<div class="profile-actions">
				<button type="button" class="btn btn-primary" onClick={addProfile}>
					+ New
				</button>
				<button type="button" class="btn">Import</button>
			</div>

			<div class="profile-list">
				{profiles.map((profile) => (
					<div
						key={profile.id}
						class={`profile-item ${profile.active ? "active" : ""}`}
					>
						<div
							class="profile-activate"
							onClick={() => setActive(profile.id)}
							title={profile.active ? "Active profile" : "Set as active"}
						>
							<span class="profile-star">
								{profile.active ? "\u2605" : "\u2606"}
							</span>
							<span class="profile-name">{profile.name}</span>
						</div>

						<div class="profile-item-actions">
							<button
								type="button"
								class="btn btn-small"
								onClick={() => setEditing(profile.id)}
							>
								Edit
							</button>
							<button
								type="button"
								class="btn btn-small"
								onClick={() => duplicateProfile(profile.id)}
								title="Duplicate"
							>
								Copy
							</button>
							<button
								type="button"
								class="btn btn-small"
								title="Export"
							>
								Export
							</button>
							{profiles.length > 1 && (
								<button
									type="button"
									class="btn btn-small"
									onClick={() => deleteProfile(profile.id)}
									title="Delete"
								>
									Del
								</button>
							)}
						</div>
					</div>
				))}
			</div>

			<div class="setting-description" style={{ marginTop: "12px" }}>
				{"\u2605"} = active profile used for item evaluation
			</div>
		</>
	);
}

/** Profile editor with Mod Weights and Display sub-tabs */
function ProfileEditor({
	profile,
	onBack,
	onRename,
	onUpdate,
}: {
	profile: Profile;
	onBack: () => void;
	onRename: (name: string) => void;
	onUpdate: (patch: Partial<Profile>) => void;
}) {
	const [tab, setTab] = useState<"weights" | "display">("weights");
	const [name, setName] = useState(profile.name);

	return (
		<>
			<div style={{ display: "flex", alignItems: "center", gap: "12px", marginBottom: "16px" }}>
				<button type="button" class="btn btn-small" onClick={onBack}>
					&larr; Back
				</button>
				<input
					type="text"
					value={name}
					class="hotkey-display"
					style={{ flex: 1, textAlign: "left", fontFamily: "inherit", fontSize: "16px", color: "var(--poe-header)" }}
					onInput={(e) => {
						const val = (e.target as HTMLInputElement).value;
						setName(val);
						onRename(val);
					}}
				/>
			</div>

			<div style={{ display: "flex", gap: "4px", marginBottom: "20px" }}>
				<button
					type="button"
					class={`btn ${tab === "weights" ? "btn-primary" : ""}`}
					onClick={() => setTab("weights")}
				>
					Mod Weights
				</button>
				<button
					type="button"
					class={`btn ${tab === "display" ? "btn-primary" : ""}`}
					onClick={() => setTab("display")}
				>
					Display
				</button>
			</div>

			{tab === "weights" && (
				<ModWeightsTab
					modWeights={profile.modWeights}
					onUpdate={(modWeights) => onUpdate({ modWeights })}
				/>
			)}
			{tab === "display" && (
				<DisplayTab
					tierColors={profile.tierColors}
					highlightWeights={profile.highlightWeights}
					dimIgnored={profile.dimIgnored}
					onUpdate={onUpdate}
				/>
			)}
		</>
	);
}

/** Mod weights sub-tab — searchable mod list with weight sliders */
function ModWeightsTab({
	modWeights,
	onUpdate,
}: {
	modWeights: Record<string, Weight>;
	onUpdate: (modWeights: Record<string, Weight>) => void;
}) {
	const [search, setSearch] = useState("");

	// Mock mod categories with a few representative mods each
	const categories = [
		{
			name: "Life & Defence",
			mods: [
				"+# to maximum Life",
				"+#% to Armour",
				"+# to maximum Energy Shield",
				"+# to Evasion Rating",
			],
		},
		{
			name: "Resistances",
			mods: [
				"+#% to Fire Resistance",
				"+#% to Cold Resistance",
				"+#% to Lightning Resistance",
				"+#% to Chaos Resistance",
				"+#% to all Elemental Resistances",
			],
		},
		{
			name: "Damage",
			mods: [
				"#% increased Physical Damage",
				"Adds # to # Physical Damage",
				"#% increased Spell Damage",
				"+#% to Critical Strike Multiplier",
			],
		},
		{
			name: "Speed",
			mods: [
				"#% increased Movement Speed",
				"#% increased Attack Speed",
				"#% increased Cast Speed",
			],
		},
	];

	const lowerSearch = search.toLowerCase();

	return (
		<>
			<input
				type="text"
				placeholder="Search mods..."
				value={search}
				class="hotkey-display"
				style={{
					width: "100%",
					textAlign: "left",
					marginBottom: "16px",
					fontFamily: "inherit",
				}}
				onInput={(e) =>
					setSearch((e.target as HTMLInputElement).value)
				}
			/>

			{categories.map((cat) => {
				const filteredMods = cat.mods.filter((m) =>
					m.toLowerCase().includes(lowerSearch),
				);
				if (filteredMods.length === 0) return null;
				return (
					<div class="setting-group" key={cat.name}>
						<h3>{cat.name}</h3>
						{filteredMods.map((mod) => (
							<ModWeightRow
								key={mod}
								mod={mod}
								weight={modWeights[mod] ?? "Medium"}
								onChange={(w) => onUpdate({ ...modWeights, [mod]: w })}
							/>
						))}
					</div>
				);
			})}
		</>
	);
}

const weightLabels: Weight[] = ["Ignore", "Low", "Medium", "High", "Critical"];

const weightColors: Record<Weight, string> = {
	Ignore: "var(--poe-text-dim)",
	Low: "var(--tier-low)",
	Medium: "var(--tier-4-5)",
	High: "var(--tier-2-3)",
	Critical: "var(--tier-1)",
};

function ModWeightRow({ mod, weight, onChange }: { mod: string; weight: Weight; onChange: (w: Weight) => void }) {
	return (
		<div class="setting-row">
			<div class="setting-label" style={{ fontSize: "12px" }}>
				{mod}
			</div>
			<div class="setting-slider">
				<input
					type="range"
					min={0}
					max={4}
					step={1}
					value={weightLabels.indexOf(weight)}
					onInput={(e) => {
						const idx = Number((e.target as HTMLInputElement).value);
						onChange(weightLabels[idx]!);
					}}
					style={{ width: "80px" }}
				/>
				<span
					class="slider-value"
					style={{ color: weightColors[weight], minWidth: "52px", fontSize: "11px" }}
				>
					{weight}
				</span>
			</div>
		</div>
	);
}

/** Display sub-tab — tier colors and preview */
function DisplayTab({
	tierColors,
	highlightWeights,
	dimIgnored,
	onUpdate,
}: {
	tierColors: TierColors;
	highlightWeights: boolean;
	dimIgnored: boolean;
	onUpdate: (patch: Partial<Profile>) => void;
}) {
	const updateColor = (key: keyof TierColors, value: string) => {
		onUpdate({ tierColors: { ...tierColors, [key]: value } });
	};

	return (
		<>
			<div class="setting-group">
				<h3>Tier Colors</h3>

				<ColorRow label="T1 (best)" color={tierColors.t1} onChange={(v) => updateColor("t1", v)} />
				<ColorRow label="T2-T3" color={tierColors.t2_3} onChange={(v) => updateColor("t2_3", v)} />
				<ColorRow label="T4-T5" color={tierColors.t4_5} onChange={(v) => updateColor("t4_5", v)} />
				<ColorRow label="T6+ (low)" color={tierColors.low} onChange={(v) => updateColor("low", v)} />
			</div>

			<div class="setting-group">
				<h3>Preview</h3>
				<div
					style={{
						background: "var(--poe-bg)",
						border: "1px solid var(--poe-border)",
						borderRadius: "4px",
						padding: "8px 12px",
					}}
				>
					<PreviewLine tier="T1" label="P" text="+88 to maximum Life" pct={95} color={tierColors.t1} />
					<PreviewLine tier="T3" label="S" text="+31% Cold Resistance" pct={50} color={tierColors.t2_3} />
					<PreviewLine tier="T5" label="P" text="+12% Spell Damage" pct={20} color={tierColors.t4_5} />
					<PreviewLine tier="T8" label="S" text="+14 to Dexterity" pct={25} color={tierColors.low} />
				</div>
			</div>

			<div class="setting-group">
				<h3>Options</h3>
				<div class="setting-row">
					<div class="setting-label">Highlight mods matching profile weights</div>
					<label class="setting-toggle">
						<input
							type="checkbox"
							checked={highlightWeights}
							onChange={(e) => onUpdate({ highlightWeights: (e.target as HTMLInputElement).checked })}
						/>
						<span class="toggle-track" />
					</label>
				</div>
				<div class="setting-row">
					<div class="setting-label">Dim mods with weight = Ignore</div>
					<label class="setting-toggle">
						<input
							type="checkbox"
							checked={dimIgnored}
							onChange={(e) => onUpdate({ dimIgnored: (e.target as HTMLInputElement).checked })}
						/>
						<span class="toggle-track" />
					</label>
				</div>
			</div>
		</>
	);
}

function ColorRow({ label, color, onChange }: { label: string; color: string; onChange: (v: string) => void }) {
	return (
		<div class="setting-row">
			<div class="setting-label">{label}</div>
			<div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
				<input
					type="color"
					value={color}
					onInput={(e) => onChange((e.target as HTMLInputElement).value)}
					style={{ width: "28px", height: "28px", border: "none", background: "none", cursor: "pointer" }}
				/>
				<span style={{ fontFamily: "Consolas, monospace", fontSize: "11px", color: "var(--poe-text-dim)" }}>
					{color}
				</span>
			</div>
		</div>
	);
}

function PreviewLine({ tier, label, text, pct, color }: {
	tier: string; label: string; text: string; pct: number; color: string;
}) {
	return (
		<div style={{ display: "flex", alignItems: "center", gap: "6px", padding: "3px 0", fontSize: "13px" }}>
			<span
				style={{
					fontSize: "10px",
					fontWeight: "bold",
					padding: "0 4px",
					borderRadius: "2px",
					border: `1px solid ${color}40`,
					background: `${color}20`,
					color,
					minWidth: "22px",
					textAlign: "center",
				}}
			>
				{tier}
			</span>
			<span
				style={{
					fontSize: "9px",
					fontWeight: "bold",
					padding: "0 3px",
					borderRadius: "2px",
					opacity: 0.7,
					color: label === "P" ? "#6cc" : "#c6c",
					border: `1px solid ${label === "P" ? "rgba(102,204,204,0.3)" : "rgba(204,102,204,0.3)"}`,
				}}
			>
				{label}
			</span>
			<span style={{ flex: 1, color }}>{text}</span>
			<span style={{ fontSize: "10px", color: "var(--poe-text-dim)" }}>{pct}%</span>
		</div>
	);
}
