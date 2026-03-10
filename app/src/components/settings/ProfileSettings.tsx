import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { readTextFile, writeTextFile } from "@tauri-apps/plugin-fs";
import { useCallback, useEffect, useRef, useState } from "preact/hooks";
import {
	type DisplayPrefs,
	type ProfileRole,
	type QualityColors,
	type StoredProfile,
	WATCH_COLORS,
	WEIGHT_VALUES,
	type WeightLevel,
	defaultDisplay,
	loadProfiles,
	mergeModWeightsIntoScoring,
	saveProfiles,
	syncActiveProfile,
} from "../../store";
import {
	type EvalProfile,
	type PredicateSchema,
	type Rule,
	type ScoringRule,
	isCompoundRule,
	isPredRule,
} from "../../types";
import { PredicateEditor, defaultCompoundRule, defaultRule, getSchema } from "./PredicateEditor";

const WEIGHT_LEVELS: WeightLevel[] = ["low", "medium", "high", "critical"];

const LEVEL_LABELS: Record<WeightLevel, string> = {
	low: "Low",
	medium: "Med",
	high: "High",
	critical: "Crit",
};

/** Map a numeric weight to the closest discrete level. */
function weightToLevel(weight: number): WeightLevel {
	if (weight >= 75) return "critical";
	if (weight >= 32) return "high";
	if (weight >= 10) return "medium";
	return "low";
}

/** Check if a weight matches an exact level value. */
function weightMatchesLevel(weight: number): WeightLevel | null {
	for (const l of WEIGHT_LEVELS) {
		if (WEIGHT_VALUES[l] === weight) return l;
	}
	return null;
}

/** Check if a ScoringRule is a simple stat weight (HasStatId predicate). */
function isStatRule(rule: ScoringRule): boolean {
	return isPredRule(rule.rule) && (rule.rule as Record<string, unknown>).type === "HasStatId";
}

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

	const setRole = useCallback(
		(id: string, role: ProfileRole) => {
			const next = profilesRef.current.map((p) => {
				if (p.id === id) return { ...p, role };
				// Only one primary allowed — demote the old primary
				if (role === "primary" && p.role === "primary") return { ...p, role: "off" as const };
				return p;
			});
			persist(next);
			syncActiveProfile(next);
		},
		[persist],
	);

	const setWatchColor = useCallback(
		(id: string, color: string) => {
			const next = profilesRef.current.map((p) => (p.id === id ? { ...p, watchColor: color } : p));
			persist(next);
			syncActiveProfile(next);
		},
		[persist],
	);

	const addProfile = useCallback(() => {
		const id = String(Date.now());
		// Auto-assign the first unused watch color
		const usedColors = new Set(profilesRef.current.map((p) => p.watchColor));
		const color = WATCH_COLORS.find((c) => !usedColors.has(c)) ?? WATCH_COLORS[0];
		const next: StoredProfile[] = [
			...profilesRef.current,
			{
				id,
				name: "New Profile",
				role: "off",
				watchColor: color,
				evalProfile: null,
				modWeights: [],
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
			if (first && !filtered.some((p) => p.role === "primary")) {
				filtered[0] = { ...first, role: "primary" as const };
			}
			persist(filtered);
			syncActiveProfile(filtered);
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
					...structuredClone(source),
					id: newId,
					name: `${source.name} (copy)`,
					role: "off" as const,
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
			const imported = mergeModWeightsIntoScoring({
				id: String(Date.now()),
				name: data.name,
				role: "off",
				watchColor: WATCH_COLORS[0],
				evalProfile: data.evalProfile ?? null,
				modWeights: data.modWeights ?? [],
				display: data.display ?? { ...defaultDisplay },
			});
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
		const { id: _id, role: _role, watchColor: _wc, ...exportData } = profile;
		await writeTextFile(path, JSON.stringify(exportData, null, 2));
	}, []);

	const saveProfile = useCallback(
		(id: string, patch: Partial<StoredProfile>) => {
			const next = profilesRef.current.map((p) => (p.id === id ? { ...p, ...patch } : p));
			persist(next);
			syncActiveProfile(next);
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

			<div class="setting-description" style={{ marginTop: "6px", marginBottom: "6px" }}>
				{"\u2605"} Primary = scored in overlay &nbsp; {"\u25CF"} Watching = background indicator
			</div>

			<ProfileWarnings profiles={profiles} />

			<div class="profile-list">
				{profiles.map((profile) => (
					<div
						key={profile.id}
						class={`profile-item ${profile.role === "primary" ? "active" : ""}`}
					>
						<div class="profile-role-area">
							<select
								class="profile-role-select"
								value={profile.role}
								onChange={(e) =>
									setRole(profile.id, (e.target as HTMLSelectElement).value as ProfileRole)
								}
								title="Profile role"
							>
								<option value="primary">{"\u2605"} Primary</option>
								<option value="watching">{"\u25CF"} Watching</option>
								<option value="off">Off</option>
							</select>
							{profile.role === "watching" && (
								<WatchColorPicker
									color={profile.watchColor}
									onChange={(c) => setWatchColor(profile.id, c)}
								/>
							)}
						</div>
						<span class="profile-name">{profile.name}</span>

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
		</>
	);
}

// ── Profile Editor ───────────────────────────────────────────────────────

/** Profile editor with Scoring and Display sub-tabs */
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
	const [draft, setDraft] = useState<Partial<StoredProfile>>({});
	const [builtinProfile, setBuiltinProfile] = useState<EvalProfile | null>(null);
	const [schema, setSchema] = useState<PredicateSchema[]>([]);

	const hasChanges = Object.keys(draft).length > 0;

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

	const currentName = (draft.name as string | undefined) ?? profile.name;
	const currentEval =
		"evalProfile" in draft ? (draft.evalProfile as EvalProfile | null) : profile.evalProfile;
	const currentDisplay = (draft.display as DisplayPrefs | undefined) ?? profile.display;

	const save = () => {
		if (hasChanges) onSave(draft);
		setDraft({});
	};

	const back = () => {
		onBack();
	};

	/** Customize from the built-in profile — clone all rules into evalProfile. */
	const handleCustomize = () => {
		if (!builtinProfile) return;
		setDraft({
			...draft,
			evalProfile: {
				...structuredClone(builtinProfile),
				name: "Custom",
				description: "Customized from Generic",
			},
		});
	};

	const updateEvalScoring = (scoring: ScoringRule[]) => {
		if (!currentEval) return;
		setDraft({ ...draft, evalProfile: { ...currentEval, scoring } });
	};

	const resetToDefault = () => {
		setDraft({ ...draft, evalProfile: null });
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
				<button type="button" class="btn btn-small" onClick={back}>
					{hasChanges ? "Discard" : "\u2190 Back"}
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
					class={`btn btn-small ${hasChanges ? "btn-save-pulse" : ""}`}
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
					Scoring
				</button>
				<button
					type="button"
					class={`btn ${tab === "display" ? "btn-primary" : ""}`}
					onClick={() => setTab("display")}
				>
					Display
				</button>
			</div>

			{tab === "scoring" && currentEval === null && (
				<DefaultProfileView builtinProfile={builtinProfile} onCustomize={handleCustomize} />
			)}
			{tab === "scoring" && currentEval !== null && (
				<CustomProfileView
					scoring={currentEval.scoring}
					originalScoring={profile.evalProfile?.scoring ?? null}
					schema={schema}
					onUpdateScoring={updateEvalScoring}
					onReset={resetToDefault}
				/>
			)}

			{tab === "display" && (
				<DisplayTab
					display={currentDisplay}
					onUpdate={(display) => setDraft({ ...draft, display })}
				/>
			)}

			{hasChanges && (
				<div class="unsaved-bar">
					<button type="button" class="btn btn-small" onClick={back}>
						Discard
					</button>
					<span>Unsaved changes</span>
					<button type="button" class="btn btn-small btn-save-pulse" onClick={save}>
						Save
					</button>
				</div>
			)}
		</>
	);
}

// ── Scoring Rules ────────────────────────────────────────────────────────

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
					{builtinProfile.scoring.map((rule) => {
						const level = weightMatchesLevel(rule.weight);
						return (
							<div key={rule.label} class="setting-row" style={{ padding: "4px 0" }}>
								<div class="setting-label" style={{ fontSize: "12px" }}>
									{rule.label}
								</div>
								<span
									style={{
										fontSize: "11px",
										color: "var(--poe-text-dim)",
										minWidth: "48px",
										textAlign: "right",
									}}
								>
									{level ? LEVEL_LABELS[level] : `${rule.weight} pts`}
								</span>
							</div>
						);
					})}
				</div>
			)}
		</>
	);
}

/** Editable scoring rules list with stat search, drag-and-drop reordering. */
function CustomProfileView({
	scoring,
	originalScoring,
	schema,
	onUpdateScoring,
	onReset,
}: {
	scoring: ScoringRule[];
	originalScoring: ScoringRule[] | null;
	schema: PredicateSchema[];
	onUpdateScoring: (scoring: ScoringRule[]) => void;
	onReset: () => void;
}) {
	const [dragFrom, setDragFrom] = useState<number | null>(null);
	const [dragOver, setDragOver] = useState<number | null>(null);

	const addRule = () => {
		const firstSchema = schema[0];
		if (!firstSchema) return;
		onUpdateScoring([
			...scoring,
			{ label: firstSchema.label, weight: 15, rule: defaultRule(firstSchema) },
		]);
	};

	const addGroup = () => {
		const firstSchema = schema[0];
		if (!firstSchema) return;
		onUpdateScoring([
			...scoring,
			{ label: "Rule Group", weight: 15, rule: defaultCompoundRule(firstSchema) },
		]);
	};

	const handleDrop = (targetIndex: number) => {
		if (dragFrom === null || dragFrom === targetIndex) return;
		const next = [...scoring];
		const moved = next[dragFrom];
		if (!moved) return;
		next.splice(dragFrom, 1);
		next.splice(targetIndex, 0, moved);
		onUpdateScoring(next);
	};

	const handleChange = (i: number, updated: ScoringRule) => {
		const next = [...scoring];
		next[i] = updated;
		onUpdateScoring(next);
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
				<h3 style={{ margin: 0 }}>Scoring ({scoring.length})</h3>
				<div style={{ display: "flex", gap: "6px" }}>
					<button type="button" class="btn btn-small btn-primary" onClick={addRule}>
						+ Rule
					</button>
					<button type="button" class="btn btn-small btn-primary" onClick={addGroup}>
						+ Group
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

			{/* Scoring rules with drag-and-drop reordering */}
			{scoring.map((rule, i) => {
				const orig = originalScoring ? originalScoring[i] : undefined;
				const modified = !orig || JSON.stringify(rule) !== JSON.stringify(orig);
				const isDragging = dragFrom === i;
				const isDropTarget = dragOver === i && dragFrom !== null && dragFrom !== i;
				return (
					<div
						// biome-ignore lint/suspicious/noArrayIndexKey: rules have no stable ID
						key={i}
						class={`scoring-rule-slot${isDragging ? " dragging" : ""}${isDropTarget ? " drop-target" : ""}`}
						draggable={true}
						onDragStart={(e) => {
							setDragFrom(i);
							const dt = (e as DragEvent).dataTransfer;
							if (dt) {
								dt.effectAllowed = "move";
								dt.setData("text/plain", String(i));
							}
						}}
						onDragOver={(e) => {
							e.preventDefault();
							const dt = (e as DragEvent).dataTransfer;
							if (dt) dt.dropEffect = "move";
							if (dragFrom !== null && dragFrom !== i) setDragOver(i);
						}}
						onDragEnter={(e) => {
							e.preventDefault();
						}}
						onDrop={(e) => {
							e.preventDefault();
							handleDrop(i);
						}}
						onDragEnd={() => {
							setDragFrom(null);
							setDragOver(null);
						}}
					>
						{isStatRule(rule) ? (
							<CompactStatRow
								rule={rule}
								modified={modified}
								onChange={(updated) => handleChange(i, updated)}
								onDelete={() => onUpdateScoring(scoring.filter((_, j) => j !== i))}
							/>
						) : (
							<ScoringRuleEditor
								rule={rule}
								schema={schema}
								modified={modified}
								onChange={(updated) => handleChange(i, updated)}
								onDelete={() => onUpdateScoring(scoring.filter((_, j) => j !== i))}
							/>
						)}
					</div>
				);
			})}

			{scoring.length === 0 && (
				<div class="setting-description">
					No scoring rules. Search for stats above or click "+ Rule" to add one.
				</div>
			)}
		</div>
	);
}

// ── Predicate Row (reusable: single Pred rule with type selector) ─────

/** Group schemas by category (for type selector dropdown). */
function groupByCategory(schema: PredicateSchema[]): Record<string, PredicateSchema[]> {
	const categories: Record<string, PredicateSchema[]> = {};
	for (const s of schema) {
		const list = categories[s.category] ?? [];
		list.push(s);
		categories[s.category] = list;
	}
	return categories;
}

/** A single predicate row. Compact mode: single inline row for compound groups. */
function PredicateRow({
	rule,
	schema,
	onChange,
	onDelete,
	compact,
}: {
	rule: Rule;
	schema: PredicateSchema[];
	onChange: (rule: Rule) => void;
	onDelete?: () => void;
	compact?: boolean;
}) {
	const predType = isPredRule(rule) ? rule.type : null;
	const predSchema = predType ? schema.find((s) => s.typeName === predType) : null;
	const categories = groupByCategory(schema);

	const handleTypeChange = (typeName: string) => {
		const newSchema = schema.find((s) => s.typeName === typeName);
		if (!newSchema) return;
		onChange(defaultRule(newSchema));
	};

	return (
		<div
			class={`compound-pred-row${compact ? " compact" : ""}`}
			{...(compact && predSchema ? { title: predSchema.description } : {})}
		>
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
					{!compact && (
						<div class="setting-description" style={{ margin: "4px 0" }}>
							{predSchema.description}
						</div>
					)}
					<PredicateEditor
						rule={rule}
						schema={predSchema}
						onChange={onChange}
						{...(compact ? { compact: true } : {})}
					/>
				</>
			)}
			{onDelete && (
				<button
					type="button"
					class="compound-pred-delete"
					onClick={onDelete}
					title="Remove condition"
				>
					&times;
				</button>
			)}
		</div>
	);
}

// ── Compound Group Editor (recursive) ────────────────────────────────

const DEPTH_COLORS = ["#af6025", "#866040", "#6b4d38"];

/** Renders a compound rule (All/Any) with collapsible groups, depth-colored
 *  borders, clickable AND/OR pills, and natural-language header. */
function CompoundGroupEditor({
	rule,
	schema,
	onChange,
	onDelete,
	depth,
}: {
	rule: { rule_type: "All" | "Any"; rules: Rule[] };
	schema: PredicateSchema[];
	onChange: (updated: Rule) => void;
	onDelete?: () => void;
	depth: number;
}) {
	const [collapsed, setCollapsed] = useState(false);
	const mode = rule.rule_type;
	const color = DEPTH_COLORS[depth % DEPTH_COLORS.length] ?? "#af6025";

	const toggleMode = () => {
		onChange({ ...rule, rule_type: mode === "All" ? "Any" : "All" });
	};

	const updateChild = (index: number, updated: Rule) => {
		const next = [...rule.rules];
		next[index] = updated;
		onChange({ ...rule, rules: next });
	};

	const deleteChild = (index: number) => {
		const next = rule.rules.filter((_: Rule, i: number) => i !== index);
		if (next.length === 0) {
			const firstSchema = schema[0];
			if (firstSchema) onChange({ ...rule, rules: [defaultRule(firstSchema)] });
		} else {
			onChange({ ...rule, rules: next });
		}
	};

	const addCondition = () => {
		const firstSchema = schema[0];
		if (!firstSchema) return;
		onChange({ ...rule, rules: [...rule.rules, defaultRule(firstSchema)] });
	};

	const addSubGroup = () => {
		const firstSchema = schema[0];
		if (!firstSchema) return;
		const subGroup: Rule = {
			rule_type: mode === "All" ? "Any" : "All",
			rules: [defaultRule(firstSchema)],
		};
		onChange({ ...rule, rules: [...rule.rules, subGroup] });
	};

	return (
		<div
			class="compound-group"
			style={{
				borderLeft: `2px solid ${color}`,
				background: `rgba(255, 255, 255, ${0.015 + depth * 0.015})`,
			}}
		>
			{/* Natural language header: "▼ Match [all ▾] of:" */}
			{/* biome-ignore lint/a11y/useKeyWithClickEvents: header is mouse-only toggle */}
			<div class="compound-group-header" onClick={() => setCollapsed(!collapsed)}>
				<span class={`cg-chevron${collapsed ? "" : " expanded"}`}>{"\u25B6"}</span>
				<span>Match</span>
				<button
					type="button"
					class="cg-mode-dropdown"
					onClick={(e) => {
						e.stopPropagation();
						toggleMode();
					}}
				>
					{mode === "All" ? "all" : "any"}
				</button>
				<span>of:</span>
				<span class="cg-count">{rule.rules.length}</span>
				{onDelete && (
					<button
						type="button"
						class="compound-group-delete"
						onClick={(e) => {
							e.stopPropagation();
							onDelete();
						}}
						title="Remove group"
					>
						&times;
					</button>
				)}
			</div>

			{/* Collapsed: one-line summary */}
			{collapsed && <div class="compound-group-summary">{summarizeRule(rule, schema)}</div>}

			{/* Expanded: conditions with AND/OR pills between them */}
			{!collapsed && (
				<div class="compound-group-body">
					{rule.rules.map((child: Rule, i: number) => (
						// biome-ignore lint/suspicious/noArrayIndexKey: sub-rules have no stable ID
						<div key={i}>
							{i > 0 && (
								<div class="compound-pill-row">
									<button
										type="button"
										class="compound-pill"
										onClick={toggleMode}
										title={`Click to switch to ${mode === "All" ? "OR" : "AND"}`}
									>
										{mode === "All" ? "AND" : "OR"}
									</button>
								</div>
							)}
							{isCompoundRule(child) ? (
								<CompoundGroupEditor
									rule={child}
									schema={schema}
									onChange={(updated) => updateChild(i, updated)}
									{...(rule.rules.length > 1 ? { onDelete: () => deleteChild(i) } : {})}
									depth={depth + 1}
								/>
							) : (
								<PredicateRow
									rule={child}
									schema={schema}
									onChange={(updated) => updateChild(i, updated)}
									{...(rule.rules.length > 1 ? { onDelete: () => deleteChild(i) } : {})}
									compact
								/>
							)}
						</div>
					))}
					<div class="compound-actions">
						<button type="button" class="compound-add-btn btn btn-small" onClick={addCondition}>
							+ Condition
						</button>
						<button type="button" class="compound-add-btn btn btn-small" onClick={addSubGroup}>
							+ Sub-Group
						</button>
					</div>
				</div>
			)}
		</div>
	);
}

// ── Scoring Rule Editor ──────────────────────────────────────────────

/** Editor for a single scoring rule: label + weight + predicate(s). */
function ScoringRuleEditor({
	rule,
	schema,
	modified,
	onChange,
	onDelete,
}: {
	rule: ScoringRule;
	schema: PredicateSchema[];
	modified?: boolean;
	onChange: (updated: ScoringRule) => void;
	onDelete: () => void;
}) {
	const [expanded, setExpanded] = useState(false);

	const toggleCompound = () => {
		if (isCompoundRule(rule.rule)) {
			const firstPred = findFirstPred(rule.rule);
			const fallbackSchema = schema[0];
			if (!firstPred && !fallbackSchema) return;
			onChange({
				...rule,
				rule: firstPred ?? defaultRule(fallbackSchema as PredicateSchema),
			});
		} else {
			onChange({ ...rule, rule: { rule_type: "All", rules: [rule.rule] } });
		}
	};

	return (
		<div class={`scoring-rule${modified ? " modified" : ""}`}>
			<div class="scoring-rule-header">
				<span class="drag-handle" title="Drag to reorder">
					{"\u2801\u2801"}
				</span>
				<div class="scoring-rule-label">
					<input
						type="text"
						value={rule.label}
						onInput={(e) => onChange({ ...rule, label: (e.target as HTMLInputElement).value })}
						placeholder="Rule label..."
					/>
				</div>
				<WeightBar weight={rule.weight} onChange={(w) => onChange({ ...rule, weight: w })} />
				<button
					type="button"
					class="btn btn-small"
					onClick={() => setExpanded(!expanded)}
					title={expanded ? "Collapse" : "Edit"}
				>
					{expanded ? "\u25B2" : "\u25BC"}
				</button>
				<button
					type="button"
					class="btn btn-small"
					onClick={onDelete}
					title="Delete rule"
					style={{ color: "var(--quality-low)" }}
				>
					&times;
				</button>
			</div>

			{/* Collapsed summary for compound rules */}
			{!expanded && isCompoundRule(rule.rule) && (
				<div class="scoring-rule-summary">{summarizeRule(rule.rule, schema)}</div>
			)}

			{expanded && (
				<div class="scoring-rule-body">
					{/* Single / Group toggle */}
					<div class="compound-mode-selector">
						<button
							type="button"
							class={`btn btn-small ${!isCompoundRule(rule.rule) ? "btn-primary" : ""}`}
							onClick={() => {
								if (isCompoundRule(rule.rule)) toggleCompound();
							}}
						>
							Single
						</button>
						<button
							type="button"
							class={`btn btn-small ${isCompoundRule(rule.rule) ? "btn-primary" : ""}`}
							onClick={() => {
								if (!isCompoundRule(rule.rule)) toggleCompound();
							}}
						>
							Group
						</button>
					</div>

					{/* Single mode: one predicate with description */}
					{!isCompoundRule(rule.rule) && (
						<PredicateRow
							rule={rule.rule}
							schema={schema}
							onChange={(newRule) => onChange({ ...rule, rule: newRule })}
						/>
					)}

					{/* Group mode: compound editor with depth coloring */}
					{isCompoundRule(rule.rule) && (
						<CompoundGroupEditor
							rule={rule.rule}
							schema={schema}
							onChange={(updated) => onChange({ ...rule, rule: updated })}
							depth={0}
						/>
					)}
				</div>
			)}
		</div>
	);
}

/** Walk a rule tree and return the first Pred node found. */
function findFirstPred(r: Rule): Rule | null {
	if (isPredRule(r)) return r;
	if (isCompoundRule(r)) {
		for (const child of r.rules) {
			const found = findFirstPred(child);
			if (found) return found;
		}
	}
	return null;
}

/** Generate a plain-English summary of a rule tree for the collapsed header. */
function summarizeRule(r: Rule, schema: PredicateSchema[]): string {
	if (isPredRule(r)) {
		const s = schema.find((sc) => sc.typeName === r.type);
		return s?.label ?? r.type;
	}
	if (isCompoundRule(r)) {
		const joiner = r.rule_type === "All" ? " AND " : " OR ";
		const parts = r.rules.map((child) => {
			const text = summarizeRule(child, schema);
			// Wrap nested compound in parens for clarity
			return isCompoundRule(child) ? `(${text})` : text;
		});
		return parts.join(joiner);
	}
	return "?";
}

// ── Weight Bar (unified bar + click-to-edit numeric) ─────────────────────

/** Bar widget with 4 discrete levels. Click label to toggle numeric input. */
function WeightBar({
	weight,
	onChange,
}: {
	weight: number;
	onChange: (weight: number) => void;
}) {
	const [editingNumeric, setEditingNumeric] = useState(false);
	const inputRef = useRef<HTMLInputElement>(null);

	const matchedLevel = weightMatchesLevel(weight);
	const displayLevel = matchedLevel ?? weightToLevel(weight);
	const activeIndex = WEIGHT_LEVELS.indexOf(displayLevel);

	// Focus input when switching to numeric mode
	useEffect(() => {
		if (editingNumeric && inputRef.current) inputRef.current.focus();
	}, [editingNumeric]);

	// User toggled → numeric mode (shows actual value)
	if (editingNumeric) {
		return (
			<div class="weight-bar">
				<input
					ref={inputRef}
					type="number"
					class="weight-bar-numeric"
					value={weight}
					min={0}
					max={999}
					onInput={(e) => onChange(Number((e.target as HTMLInputElement).value))}
					onBlur={() => setEditingNumeric(false)}
					onKeyDown={(e) => {
						if (e.key === "Enter") {
							(e.target as HTMLInputElement).blur();
						}
					}}
				/>
				<button
					type="button"
					class="weight-bar-label"
					onMouseDown={(e) => {
						e.preventDefault(); // prevent input blur race
						setEditingNumeric(false);
					}}
					title="Switch to bar"
				>
					pts
				</button>
			</div>
		);
	}

	return (
		<div class="weight-bar">
			<div class="weight-bar-blocks">
				{WEIGHT_LEVELS.map((l, i) => (
					<button
						key={l}
						type="button"
						class={`weight-bar-block ${i <= activeIndex ? "filled" : ""}`}
						onClick={() => onChange(WEIGHT_VALUES[l])}
						title={`${LEVEL_LABELS[l]} (${WEIGHT_VALUES[l]} pts)`}
					/>
				))}
			</div>
			<button
				type="button"
				class="weight-bar-label"
				onMouseDown={() => setEditingNumeric(true)}
				title="Click for custom value"
			>
				{LEVEL_LABELS[displayLevel]}
			</button>
		</div>
	);
}

// ── Compact Stat Row (single-line for HasStatId rules) ───────────────────

/** Single-line row for simple stat-weight rules. */
function CompactStatRow({
	rule,
	modified,
	onChange,
	onDelete,
}: {
	rule: ScoringRule;
	modified?: boolean;
	onChange: (updated: ScoringRule) => void;
	onDelete: () => void;
}) {
	return (
		<div class={`compact-stat-row${modified ? " modified" : ""}`}>
			<span class="drag-handle" title="Drag to reorder">
				{"\u2801\u2801"}
			</span>
			<span class="compact-stat-label" title={rule.label}>
				{rule.label}
			</span>
			<WeightBar weight={rule.weight} onChange={(w) => onChange({ ...rule, weight: w })} />
			<button type="button" class="compact-stat-delete" onClick={onDelete} title="Remove">
				&times;
			</button>
		</div>
	);
}

// ── Display Tab ──────────────────────────────────────────────────────────

/** Display sub-tab — tier colors and preview */
function DisplayTab({
	display,
	onUpdate,
}: {
	display: DisplayPrefs;
	onUpdate: (display: DisplayPrefs) => void;
}) {
	const { qualityColors, highlightWeights, dimIgnored } = display;

	const updateColor = (key: keyof QualityColors, value: string) => {
		onUpdate({
			...display,
			qualityColors: { ...qualityColors, [key]: value },
		});
	};

	return (
		<>
			<div class="setting-group">
				<h3>Mod Quality Colors</h3>

				<ColorRow
					label="Best"
					color={qualityColors.best}
					onChange={(v) => updateColor("best", v)}
				/>
				<ColorRow
					label="Great / Good"
					color={qualityColors.good}
					onChange={(v) => updateColor("good", v)}
				/>
				<ColorRow label="Mid" color={qualityColors.mid} onChange={(v) => updateColor("mid", v)} />
				<ColorRow label="Low" color={qualityColors.low} onChange={(v) => updateColor("low", v)} />
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
						quality="Best"
						color={qualityColors.best}
					/>
					<PreviewLine
						tier="T3"
						label="S"
						text="+31% Cold Resistance"
						pct={50}
						quality="Great"
						color={qualityColors.good}
					/>
					<PreviewLine
						tier="T5"
						label="P"
						text="+12% Spell Damage"
						pct={20}
						quality="Mid"
						color={qualityColors.mid}
					/>
					<PreviewLine
						tier="T8"
						label="S"
						text="+14 to Dexterity"
						pct={25}
						quality="Low"
						color={qualityColors.low}
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
	quality,
	color,
}: {
	tier: string;
	label: string;
	text: string;
	pct: number;
	quality: string;
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
			<span style={{ fontSize: "10px", color: "var(--poe-text-dim)", marginRight: "4px" }}>
				{quality}
			</span>
			<span style={{ fontSize: "10px", color: "var(--poe-text-dim)" }}>{pct}%</span>
		</div>
	);
}

// ── Profile Warnings ─────────────────────────────────────────────────────

function ProfileWarnings({ profiles }: { profiles: StoredProfile[] }) {
	const hasPrimary = profiles.some((p) => p.role === "primary");
	const allOff = profiles.every((p) => p.role === "off");
	const hasWatching = profiles.some((p) => p.role === "watching");

	if (allOff) {
		return (
			<div class="profile-warning">
				All profiles are off. The overlay will show item data without any scoring.
			</div>
		);
	}

	if (!hasPrimary) {
		return (
			<div class="profile-warning">
				No primary profile. Overlay will show item data without scoring.
				{hasWatching && " Watching profiles will still evaluate in the background."}
			</div>
		);
	}

	return null;
}

// ── Watch Color Picker ───────────────────────────────────────────────────

function WatchColorPicker({
	color,
	onChange,
}: {
	color: string;
	onChange: (color: string) => void;
}) {
	const [open, setOpen] = useState(false);
	const ref = useRef<HTMLDivElement>(null);

	useEffect(() => {
		if (!open) return;
		const handler = (e: MouseEvent) => {
			if (ref.current && !ref.current.contains(e.target as Node)) {
				setOpen(false);
			}
		};
		document.addEventListener("mousedown", handler);
		return () => document.removeEventListener("mousedown", handler);
	}, [open]);

	return (
		<div class="watch-color-picker" ref={ref}>
			<button
				type="button"
				class="watch-color-dot"
				style={{ background: color }}
				onClick={() => setOpen(!open)}
				title="Change watch color"
			/>
			{open && (
				<div class="watch-color-palette">
					{WATCH_COLORS.map((c) => (
						<button
							key={c}
							type="button"
							class={`watch-color-swatch ${c === color ? "selected" : ""}`}
							style={{ background: c }}
							onClick={() => {
								onChange(c);
								setOpen(false);
							}}
						/>
					))}
				</div>
			)}
		</div>
	);
}
