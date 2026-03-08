import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { readTextFile, writeTextFile } from "@tauri-apps/plugin-fs";
import { useCallback, useEffect, useRef, useState } from "preact/hooks";
import {
	type DisplayPrefs,
	type StoredProfile,
	type TierColors,
	defaultTierColors,
	loadProfiles,
	saveProfiles,
	syncActiveProfile,
} from "../../store";
import type { EvalProfile, PredicateSchema, ScoringRule } from "../../types";
import { PredicateEditor, defaultRule, getSchema } from "./PredicateEditor";

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
			const json = active?.evalProfile ? JSON.stringify(active.evalProfile) : "";
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
			const first = filtered[0];
			if (first && !filtered.some((p) => p.active)) {
				filtered[0] = { ...first, active: true };
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
					evalProfile: source.evalProfile ? { ...source.evalProfile } : null,
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

	const saveProfile = useCallback(
		(id: string, patch: Partial<StoredProfile>) => {
			persist(profilesRef.current.map((p) => (p.id === id ? { ...p, ...patch } : p)));
			syncActiveProfile();
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
					onSave={(patch) => saveProfile(editing, patch)}
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
				<button type="button" class="btn" onClick={importProfile}>
					Import
				</button>
			</div>

			<div class="profile-list">
				{profiles.map((profile) => (
					<div key={profile.id} class={`profile-item ${profile.active ? "active" : ""}`}>
						<button
							type="button"
							class="profile-activate"
							onClick={() => setActive(profile.id)}
							title={profile.active ? "Active profile" : "Set as active"}
						>
							<span class="profile-star">{profile.active ? "\u2605" : "\u2606"}</span>
							<span class="profile-name">{profile.name}</span>
						</button>

						<div class="profile-item-actions">
							<button type="button" class="btn btn-small" onClick={() => setEditing(profile.id)}>
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
	onSave,
}: {
	profile: StoredProfile;
	onBack: () => void;
	onSave: (patch: Partial<StoredProfile>) => void;
}) {
	const [tab, setTab] = useState<"scoring" | "display">("scoring");
	// Local draft — changes buffer here until Save
	const [draft, setDraft] = useState<Partial<StoredProfile>>({});
	const hasChanges = Object.keys(draft).length > 0;

	const currentName = (draft.name as string | undefined) ?? profile.name;
	const currentEval =
		"evalProfile" in draft ? (draft.evalProfile as EvalProfile | null) : profile.evalProfile;
	const currentDisplay = (draft.display as DisplayPrefs | undefined) ?? profile.display;

	const save = () => {
		if (hasChanges) onSave(draft);
		onBack();
	};

	const discard = () => {
		onBack();
	};

	return (
		<>
			<div
				style={{
					display: "flex",
					alignItems: "center",
					gap: "8px",
					marginBottom: "16px",
				}}
			>
				<button type="button" class="btn btn-small" onClick={discard}>
					&larr; Cancel
				</button>
				<input
					type="text"
					value={currentName}
					class="hotkey-display"
					style={{
						flex: 1,
						textAlign: "left",
						fontFamily: "inherit",
						fontSize: "16px",
						color: "var(--poe-header)",
					}}
					onInput={(e) => setDraft({ ...draft, name: (e.target as HTMLInputElement).value })}
				/>
				<button
					type="button"
					class={`btn btn-small ${hasChanges ? "btn-primary" : ""}`}
					onClick={save}
					disabled={!hasChanges}
				>
					Save
				</button>
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
					evalProfile={currentEval}
					onUpdate={(evalProfile) => setDraft({ ...draft, evalProfile })}
				/>
			)}
			{tab === "display" && (
				<DisplayTab
					display={currentDisplay}
					onUpdate={(display) => setDraft({ ...draft, display })}
				/>
			)}

			{hasChanges && (
				<div class="setting-description" style={{ marginTop: "12px", color: "var(--poe-accent)" }}>
					Unsaved changes — click Save to apply
				</div>
			)}
		</>
	);
}

/** Scoring Rules tab — view and edit eval profile rules */
function ScoringRulesTab({
	evalProfile,
	onUpdate,
}: {
	evalProfile: EvalProfile | null;
	onUpdate: (evalProfile: EvalProfile | null) => void;
}) {
	const [builtinProfile, setBuiltinProfile] = useState<EvalProfile | null>(null);
	const [schema, setSchema] = useState<PredicateSchema[]>([]);

	useEffect(() => {
		invoke<string | null>("get_default_profile").then((json) => {
			if (json) {
				try {
					setBuiltinProfile(JSON.parse(json) as EvalProfile);
				} catch {
					/* ignore */
				}
			}
		});
		getSchema().then(setSchema);
	}, []);

	if (evalProfile === null) {
		return (
			<DefaultProfileView
				builtinProfile={builtinProfile}
				onCustomize={() => {
					if (builtinProfile) {
						onUpdate({
							...builtinProfile,
							name: "Custom",
							description: "Customized from Generic",
						});
					}
				}}
			/>
		);
	}

	const updateScoring = (scoring: ScoringRule[]) => {
		onUpdate({ ...evalProfile, scoring });
	};

	return (
		<CustomProfileView
			scoring={evalProfile.scoring}
			schema={schema}
			onUpdateScoring={updateScoring}
			onReset={() => onUpdate(null)}
		/>
	);
}

/** Read-only view of the built-in Generic profile */
function DefaultProfileView({
	builtinProfile,
	onCustomize,
}: {
	builtinProfile: EvalProfile | null;
	onCustomize: () => void;
}) {
	return (
		<>
			<div class="setting-group">
				<div class="setting-description" style={{ marginBottom: "12px" }}>
					Using built-in <strong>Generic</strong> profile. Scores universally desirable stats (life,
					resistances, movement speed).
				</div>
				{builtinProfile && (
					<button type="button" class="btn" onClick={onCustomize}>
						Customize...
					</button>
				)}
			</div>

			{builtinProfile && (
				<div class="setting-group">
					<h3>{builtinProfile.scoring.length} Scoring Rules</h3>
					{builtinProfile.scoring.map((rule) => (
						<div key={rule.label} class="setting-row" style={{ padding: "4px 0" }}>
							<div class="setting-label" style={{ fontSize: "12px" }}>
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
								+{rule.weight}
							</span>
						</div>
					))}
				</div>
			)}
		</>
	);
}

/** Editable scoring rules list */
function CustomProfileView({
	scoring,
	schema,
	onUpdateScoring,
	onReset,
}: {
	scoring: ScoringRule[];
	schema: PredicateSchema[];
	onUpdateScoring: (scoring: ScoringRule[]) => void;
	onReset: () => void;
}) {
	const addRule = () => {
		const firstSchema = schema[0];
		if (!firstSchema) return;
		onUpdateScoring([
			...scoring,
			{ label: firstSchema.label, weight: 10, rule: defaultRule(firstSchema) },
		]);
	};

	return (
		<div class="setting-group">
			<div
				style={{
					display: "flex",
					justifyContent: "space-between",
					alignItems: "center",
					marginBottom: "12px",
				}}
			>
				<h3 style={{ margin: 0 }}>
					{scoring.length} Scoring Rule{scoring.length !== 1 ? "s" : ""}
				</h3>
				<div style={{ display: "flex", gap: "6px" }}>
					<button type="button" class="btn btn-small btn-primary" onClick={addRule}>
						+ Add Rule
					</button>
					<button
						type="button"
						class="btn btn-small"
						onClick={onReset}
						title="Reset to built-in Generic profile"
					>
						Reset
					</button>
				</div>
			</div>

			{scoring.map((rule, i) => (
				<ScoringRuleEditor
					// biome-ignore lint/suspicious/noArrayIndexKey: rules have no stable ID; only append/delete supported
					key={i}
					rule={rule}
					schema={schema}
					onChange={(updated) => {
						const next = [...scoring];
						next[i] = updated;
						onUpdateScoring(next);
					}}
					onDelete={() => onUpdateScoring(scoring.filter((_, j) => j !== i))}
				/>
			))}

			{scoring.length === 0 && (
				<div class="setting-description">No scoring rules. Click "+ Add Rule" to create one.</div>
			)}
		</div>
	);
}

/** Editor for a single scoring rule: label + weight + predicate. */
function ScoringRuleEditor({
	rule,
	schema,
	onChange,
	onDelete,
}: {
	rule: ScoringRule;
	schema: PredicateSchema[];
	onChange: (updated: ScoringRule) => void;
	onDelete: () => void;
}) {
	const [expanded, setExpanded] = useState(false);

	// Find the schema for this rule's predicate type
	const predType = rule.rule.rule_type === "Pred" ? (rule.rule as { type: string }).type : null;
	const predSchema = predType ? schema.find((s) => s.typeName === predType) : null;

	// Group schema by category for the type selector (computed once per render)
	const categories: Record<string, PredicateSchema[]> = {};
	for (const s of schema) {
		const list = categories[s.category] ?? [];
		list.push(s);
		categories[s.category] = list;
	}

	const handleTypeChange = (typeName: string) => {
		const newSchema = schema.find((s) => s.typeName === typeName);
		if (!newSchema) return;
		// Replace the entire rule — don't merge with old fields
		onChange({
			label: rule.label === predSchema?.label ? newSchema.label : rule.label,
			weight: rule.weight,
			rule: defaultRule(newSchema),
		});
	};

	return (
		<div class="scoring-rule">
			<div class="scoring-rule-header">
				<div class="scoring-rule-label">
					<input
						type="text"
						value={rule.label}
						onInput={(e) => onChange({ ...rule, label: (e.target as HTMLInputElement).value })}
						placeholder="Rule label..."
					/>
				</div>
				<div class="scoring-rule-weight">
					<span>pts</span>
					<input
						type="number"
						value={rule.weight}
						onInput={(e) =>
							onChange({
								...rule,
								weight: Number((e.target as HTMLInputElement).value),
							})
						}
					/>
				</div>
				<button
					type="button"
					class="btn btn-small"
					onClick={() => setExpanded(!expanded)}
					title={expanded ? "Collapse" : "Edit predicate"}
				>
					{expanded ? "\u25B2" : "\u25BC"}
				</button>
				<button
					type="button"
					class="btn btn-small"
					onClick={onDelete}
					title="Delete rule"
					style={{ color: "var(--tier-low)" }}
				>
					&times;
				</button>
			</div>

			{expanded && (
				<div class="scoring-rule-body">
					<select
						class="pred-type-select"
						value={predType ?? ""}
						onChange={(e) => handleTypeChange((e.target as HTMLSelectElement).value)}
					>
						{Object.entries(categories).map(([cat, schemas]) => (
							<optgroup key={cat} label={cat}>
								{schemas.map((s) => (
									<option key={s.typeName} value={s.typeName}>
										{s.label}
									</option>
								))}
							</optgroup>
						))}
					</select>

					{predSchema && (
						<>
							<div class="setting-description" style={{ marginBottom: "8px" }}>
								{predSchema.description}
							</div>
							<PredicateEditor
								rule={rule.rule}
								schema={predSchema}
								onChange={(newRule) => onChange({ ...rule, rule: newRule })}
							/>
						</>
					)}
				</div>
			)}
		</div>
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

				<ColorRow label="T1 (best)" color={tierColors.t1} onChange={(v) => updateColor("t1", v)} />
				<ColorRow label="T2-T3" color={tierColors.t2_3} onChange={(v) => updateColor("t2_3", v)} />
				<ColorRow label="T4-T5" color={tierColors.t4_5} onChange={(v) => updateColor("t4_5", v)} />
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
					<div class="setting-label">Highlight mods matching profile weights</div>
					<label class="setting-toggle">
						<input
							type="checkbox"
							checked={highlightWeights}
							onChange={(e) =>
								onUpdate({
									...display,
									highlightWeights: (e.target as HTMLInputElement).checked,
								})
							}
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
							onChange={(e) =>
								onUpdate({
									...display,
									dimIgnored: (e.target as HTMLInputElement).checked,
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
					onInput={(e) => onChange((e.target as HTMLInputElement).value)}
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
			<span style={{ fontSize: "10px", color: "var(--poe-text-dim)" }}>{pct}%</span>
		</div>
	);
}
