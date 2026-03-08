import { useState, useEffect, useCallback, useRef } from "preact/hooks";
import { open, save } from "@tauri-apps/plugin-dialog";
import { readTextFile, writeTextFile } from "@tauri-apps/plugin-fs";
import { invoke } from "@tauri-apps/api/core";
import {
	type StoredProfile,
	type DisplayPrefs,
	type TierColors,
	defaultTierColors,
	loadProfiles,
	saveProfiles,
	syncActiveProfile,
} from "../../store";
import type { EvalProfile } from "../../types";

const defaultDisplay: DisplayPrefs = {
	tierColors: { ...defaultTierColors },
	highlightWeights: true,
	dimIgnored: true,
};

export function ProfileSettings() {
	const [profiles, setProfiles] = useState<StoredProfile[]>([]);
	const [loaded, setLoaded] = useState(false);
	const [editing, setEditing] = useState<string | null>(null);

	const profilesRef = useRef(profiles);
	profilesRef.current = profiles;

	useEffect(() => {
		loadProfiles().then((p) => {
			setProfiles(p);
			setLoaded(true);
		});
	}, []);

	const persist = useCallback((next: StoredProfile[]) => {
		setProfiles(next);
		saveProfiles(next);
	}, []);

	const setActive = useCallback(
		(id: string) => {
			const next = profilesRef.current.map((p) => ({
				...p,
				active: p.id === id,
			}));
			persist(next);
			// Sync the newly active profile to the backend
			const active = next.find((p) => p.active);
			const json = active?.evalProfile
				? JSON.stringify(active.evalProfile)
				: "";
			invoke("set_active_profile", { profileJson: json });
		},
		[persist],
	);

	const addProfile = useCallback(() => {
		const id = String(Date.now());
		const next: StoredProfile[] = [
			...profilesRef.current,
			{
				id,
				name: "New Profile",
				active: false,
				evalProfile: null,
				display: { ...defaultDisplay },
			},
		];
		persist(next);
		setEditing(id);
	}, [persist]);

	const deleteProfile = useCallback(
		(id: string) => {
			const filtered = profilesRef.current.filter((p) => p.id !== id);
			if (filtered.length > 0 && !filtered.some((p) => p.active)) {
				filtered[0]!.active = true;
			}
			persist(filtered);
			syncActiveProfile();
		},
		[persist],
	);

	const duplicateProfile = useCallback(
		(id: string) => {
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
					evalProfile: source.evalProfile
						? { ...source.evalProfile }
						: null,
					display: {
						...source.display,
						tierColors: { ...source.display.tierColors },
					},
				},
			]);
		},
		[persist],
	);

	const importProfile = useCallback(async () => {
		const path = await open({
			filters: [{ name: "JSON", extensions: ["json"] }],
			multiple: false,
		});
		if (!path) return;
		try {
			const text = await readTextFile(path);
			const data = JSON.parse(text) as Partial<StoredProfile>;
			if (!data.name) throw new Error("Invalid profile: missing name");
			const imported: StoredProfile = {
				id: String(Date.now()),
				name: data.name,
				active: false,
				evalProfile: data.evalProfile ?? null,
				display: data.display ?? { ...defaultDisplay },
			};
			persist([...profilesRef.current, imported]);
		} catch (e) {
			console.error("Failed to import profile:", e);
		}
	}, [persist]);

	const exportProfile = useCallback(async (id: string) => {
		const profile = profilesRef.current.find((p) => p.id === id);
		if (!profile) return;
		const path = await save({
			defaultPath: `${profile.name.replace(/[^a-zA-Z0-9_-]/g, "_")}.json`,
			filters: [{ name: "JSON", extensions: ["json"] }],
		});
		if (!path) return;
		const { id: _id, active: _active, ...exportData } = profile;
		await writeTextFile(path, JSON.stringify(exportData, null, 2));
	}, []);

	const renameProfile = useCallback(
		(id: string, name: string) => {
			persist(
				profilesRef.current.map((p) =>
					p.id === id ? { ...p, name } : p,
				),
			);
		},
		[persist],
	);

	const updateProfile = useCallback(
		(id: string, patch: Partial<StoredProfile>) => {
			persist(
				profilesRef.current.map((p) =>
					p.id === id ? { ...p, ...patch } : p,
				),
			);
		},
		[persist],
	);

	if (!loaded) return null;

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
				<button
					type="button"
					class="btn btn-primary"
					onClick={addProfile}
				>
					+ New
				</button>
				<button type="button" class="btn" onClick={importProfile}>
					Import
				</button>
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
							title={
								profile.active
									? "Active profile"
									: "Set as active"
							}
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
								onClick={() => exportProfile(profile.id)}
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

/** Profile editor with Scoring Rules and Display sub-tabs */
function ProfileEditor({
	profile,
	onBack,
	onRename,
	onUpdate,
}: {
	profile: StoredProfile;
	onBack: () => void;
	onRename: (name: string) => void;
	onUpdate: (patch: Partial<StoredProfile>) => void;
}) {
	const [tab, setTab] = useState<"scoring" | "display">("scoring");
	const [name, setName] = useState(profile.name);

	return (
		<>
			<div
				style={{
					display: "flex",
					alignItems: "center",
					gap: "12px",
					marginBottom: "16px",
				}}
			>
				<button type="button" class="btn btn-small" onClick={onBack}>
					&larr; Back
				</button>
				<input
					type="text"
					value={name}
					class="hotkey-display"
					style={{
						flex: 1,
						textAlign: "left",
						fontFamily: "inherit",
						fontSize: "16px",
						color: "var(--poe-header)",
					}}
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
					class={`btn ${tab === "scoring" ? "btn-primary" : ""}`}
					onClick={() => setTab("scoring")}
				>
					Scoring Rules
				</button>
				<button
					type="button"
					class={`btn ${tab === "display" ? "btn-primary" : ""}`}
					onClick={() => setTab("display")}
				>
					Display
				</button>
			</div>

			{tab === "scoring" && (
				<ScoringRulesTab
					evalProfile={profile.evalProfile}
					onUpdate={(evalProfile) => onUpdate({ evalProfile })}
				/>
			)}
			{tab === "display" && (
				<DisplayTab
					display={profile.display}
					onUpdate={(display) => onUpdate({ display })}
				/>
			)}
		</>
	);
}

/** Scoring Rules tab — shows current eval profile rules */
function ScoringRulesTab({
	evalProfile,
	onUpdate,
}: {
	evalProfile: EvalProfile | null;
	onUpdate: (evalProfile: EvalProfile | null) => void;
}) {
	const [defaultProfile, setDefaultProfile] = useState<EvalProfile | null>(
		null,
	);

	// Load the built-in default profile for display and "customize" action
	useEffect(() => {
		invoke<string | null>("get_default_profile").then((json) => {
			if (json) {
				try {
					setDefaultProfile(JSON.parse(json) as EvalProfile);
				} catch {
					/* ignore parse errors */
				}
			}
		});
	}, []);

	// What to display: custom profile or the built-in default
	const displayProfile = evalProfile ?? defaultProfile;
	const isDefault = evalProfile === null;

	return (
		<>
			{isDefault && (
				<div class="setting-group">
					<div
						class="setting-description"
						style={{ marginBottom: "12px" }}
					>
						Using built-in <strong>Generic</strong> profile. Scores
						universally desirable stats (life, resistances, movement
						speed).
					</div>
					{defaultProfile && (
						<button
							type="button"
							class="btn"
							onClick={() =>
								onUpdate({
									...defaultProfile,
									name: "Custom",
									description: "Customized from Generic",
								})
							}
						>
							Customize...
						</button>
					)}
				</div>
			)}

			{!isDefault && (
				<div class="setting-group">
					<div
						style={{
							display: "flex",
							justifyContent: "space-between",
							alignItems: "center",
							marginBottom: "8px",
						}}
					>
						<span
							style={{
								fontSize: "12px",
								color: "var(--poe-text-dim)",
							}}
						>
							Custom scoring profile
						</span>
						<button
							type="button"
							class="btn btn-small"
							onClick={() => onUpdate(null)}
							title="Reset to built-in Generic profile"
						>
							Reset to Default
						</button>
					</div>
				</div>
			)}

			{displayProfile && (
				<div class="setting-group">
					<h3>
						{displayProfile.scoring.length} Scoring Rule
						{displayProfile.scoring.length !== 1 ? "s" : ""}
					</h3>
					{displayProfile.filter && (
						<div
							style={{
								fontSize: "11px",
								color: "var(--poe-text-dim)",
								marginBottom: "8px",
							}}
						>
							Filter: only applies to matching items
						</div>
					)}
					{displayProfile.scoring.map((rule, i) => (
						<div
							key={`${rule.label}-${i}`}
							class="setting-row"
							style={{ padding: "4px 0" }}
						>
							<div
								class="setting-label"
								style={{ fontSize: "12px" }}
							>
								{rule.label}
							</div>
							<span
								style={{
									fontSize: "12px",
									color: "var(--tier-2-3)",
									fontWeight: "bold",
									minWidth: "36px",
									textAlign: "right",
								}}
							>
								{rule.weight > 0 ? "+" : ""}
								{rule.weight}
							</span>
						</div>
					))}

					{!isDefault && (
						<div
							class="setting-description"
							style={{ marginTop: "12px" }}
						>
							Rule editing coming soon — use Import/Export for
							now.
						</div>
					)}
				</div>
			)}
		</>
	);
}

/** Display sub-tab — tier colors and preview */
function DisplayTab({
	display,
	onUpdate,
}: {
	display: DisplayPrefs;
	onUpdate: (display: DisplayPrefs) => void;
}) {
	const { tierColors, highlightWeights, dimIgnored } = display;

	const updateColor = (key: keyof TierColors, value: string) => {
		onUpdate({
			...display,
			tierColors: { ...tierColors, [key]: value },
		});
	};

	return (
		<>
			<div class="setting-group">
				<h3>Tier Colors</h3>

				<ColorRow
					label="T1 (best)"
					color={tierColors.t1}
					onChange={(v) => updateColor("t1", v)}
				/>
				<ColorRow
					label="T2-T3"
					color={tierColors.t2_3}
					onChange={(v) => updateColor("t2_3", v)}
				/>
				<ColorRow
					label="T4-T5"
					color={tierColors.t4_5}
					onChange={(v) => updateColor("t4_5", v)}
				/>
				<ColorRow
					label="T6+ (low)"
					color={tierColors.low}
					onChange={(v) => updateColor("low", v)}
				/>
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
					<PreviewLine
						tier="T1"
						label="P"
						text="+88 to maximum Life"
						pct={95}
						color={tierColors.t1}
					/>
					<PreviewLine
						tier="T3"
						label="S"
						text="+31% Cold Resistance"
						pct={50}
						color={tierColors.t2_3}
					/>
					<PreviewLine
						tier="T5"
						label="P"
						text="+12% Spell Damage"
						pct={20}
						color={tierColors.t4_5}
					/>
					<PreviewLine
						tier="T8"
						label="S"
						text="+14 to Dexterity"
						pct={25}
						color={tierColors.low}
					/>
				</div>
			</div>

			<div class="setting-group">
				<h3>Options</h3>
				<div class="setting-row">
					<div class="setting-label">
						Highlight mods matching profile weights
					</div>
					<label class="setting-toggle">
						<input
							type="checkbox"
							checked={highlightWeights}
							onChange={(e) =>
								onUpdate({
									...display,
									highlightWeights: (
										e.target as HTMLInputElement
									).checked,
								})
							}
						/>
						<span class="toggle-track" />
					</label>
				</div>
				<div class="setting-row">
					<div class="setting-label">
						Dim mods with weight = Ignore
					</div>
					<label class="setting-toggle">
						<input
							type="checkbox"
							checked={dimIgnored}
							onChange={(e) =>
								onUpdate({
									...display,
									dimIgnored: (e.target as HTMLInputElement)
										.checked,
								})
							}
						/>
						<span class="toggle-track" />
					</label>
				</div>
			</div>
		</>
	);
}

function ColorRow({
	label,
	color,
	onChange,
}: {
	label: string;
	color: string;
	onChange: (v: string) => void;
}) {
	return (
		<div class="setting-row">
			<div class="setting-label">{label}</div>
			<div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
				<input
					type="color"
					value={color}
					onInput={(e) =>
						onChange((e.target as HTMLInputElement).value)
					}
					style={{
						width: "28px",
						height: "28px",
						border: "none",
						background: "none",
						cursor: "pointer",
					}}
				/>
				<span
					style={{
						fontFamily: "Consolas, monospace",
						fontSize: "11px",
						color: "var(--poe-text-dim)",
					}}
				>
					{color}
				</span>
			</div>
		</div>
	);
}

function PreviewLine({
	tier,
	label,
	text,
	pct,
	color,
}: {
	tier: string;
	label: string;
	text: string;
	pct: number;
	color: string;
}) {
	return (
		<div
			style={{
				display: "flex",
				alignItems: "center",
				gap: "6px",
				padding: "3px 0",
				fontSize: "13px",
			}}
		>
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
			<span
				style={{ fontSize: "10px", color: "var(--poe-text-dim)" }}
			>
				{pct}%
			</span>
		</div>
	);
}
