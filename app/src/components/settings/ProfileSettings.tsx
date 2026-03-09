import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { readTextFile, writeTextFile } from "@tauri-apps/plugin-fs";
import { useCallback, useEffect, useRef, useState } from "preact/hooks";
import {
	type DisplayPrefs,
	type ModWeight,
	type StoredProfile,
	type TierColors,
	WEIGHT_VALUES,
	type WeightLevel,
	defaultDisplay,
	loadProfiles,
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
import {
	PredicateEditor,
	defaultCompoundRule,
	defaultRule,
	getSchema,
	getSuggestions,
} from "./PredicateEditor";

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
	if (weight >= 37) return "high";
	if (weight >= 17) return "medium";
	return "low";
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

	const setActive = useCallback(
		(id: string) => {
			const next = profilesRef.current.map((p) => ({
				...p,
				active: p.id === id,
			}));
			persist(next);
			syncActiveProfile(next);
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
			if (first && !filtered.some((p) => p.active)) {
				filtered[0] = { ...first, active: true };
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
					active: false,
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
				modWeights: data.modWeights ?? [],
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

// ── Profile Editor ───────────────────────────────────────────────────────

/** Profile editor with Scoring Rules, Mod Weights, and Display sub-tabs */
function ProfileEditor({
	profile,
	onBack,
	onSave,
}: {
	profile: StoredProfile;
	onBack: () => void;
	onSave: (patch: Partial<StoredProfile>) => void;
}) {
	const [tab, setTab] = useState<"scoring" | "weights" | "display">("scoring");
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
	const currentModWeights = (draft.modWeights as ModWeight[] | undefined) ?? profile.modWeights;
	const currentDisplay = (draft.display as DisplayPrefs | undefined) ?? profile.display;

	const save = () => {
		if (hasChanges) onSave(draft);
		onBack();
	};

	const discard = () => {
		onBack();
	};

	/** Customize from the built-in profile: split HasStatId rules into mod weights. */
	const handleCustomize = () => {
		if (!builtinProfile) return;
		const modWeights: ModWeight[] = [];
		const otherRules: ScoringRule[] = [];
		for (const sr of builtinProfile.scoring) {
			const ruleData = sr.rule as Record<string, unknown>;
			if (sr.rule.rule_type === "Pred" && ruleData.type === "HasStatId") {
				modWeights.push({
					template: sr.label,
					statIds: [ruleData.stat_id as string],
					level: weightToLevel(sr.weight),
				});
			} else {
				otherRules.push(sr);
			}
		}
		setDraft({
			...draft,
			evalProfile: {
				...builtinProfile,
				name: "Custom",
				description: "Customized from Generic",
				scoring: otherRules,
			},
			modWeights,
		});
	};

	const updateEvalScoring = (scoring: ScoringRule[]) => {
		if (!currentEval) return;
		setDraft({ ...draft, evalProfile: { ...currentEval, scoring } });
	};

	const resetToDefault = () => {
		setDraft({ ...draft, evalProfile: null, modWeights: [] });
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

			{tab === "scoring" && currentEval === null && (
				<DefaultProfileView builtinProfile={builtinProfile} onCustomize={handleCustomize} />
			)}
			{tab === "scoring" && currentEval !== null && (
				<CustomProfileView
					scoring={currentEval.scoring}
					schema={schema}
					onUpdateScoring={updateEvalScoring}
					onReset={resetToDefault}
				/>
			)}

			{tab === "weights" && currentEval === null && (
				<div class="setting-group">
					<div class="setting-description">
						Customize the profile to add mod weights. Click the <strong>Scoring Rules</strong> tab
						and press <strong>Customize</strong>.
					</div>
				</div>
			)}
			{tab === "weights" && currentEval !== null && (
				<ModWeightsTab
					modWeights={currentModWeights}
					onUpdate={(modWeights) => setDraft({ ...draft, modWeights })}
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

	const addGroup = () => {
		const firstSchema = schema[0];
		if (!firstSchema) return;
		onUpdateScoring([
			...scoring,
			{ label: "Rule Group", weight: 10, rule: defaultCompoundRule(firstSchema) },
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
					<button type="button" class="btn btn-small btn-primary" onClick={addGroup}>
						+ Add Group
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

/** A single predicate: type selector + fields + optional delete button. */
function PredicateRow({
	rule,
	schema,
	onChange,
	onDelete,
}: {
	rule: Rule;
	schema: PredicateSchema[];
	onChange: (rule: Rule) => void;
	onDelete?: () => void;
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
		<div class="compound-pred-row">
			<div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
				<select
					class="pred-type-select"
					style={{ flex: 1, marginBottom: 0 }}
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
				{onDelete && (
					<button
						type="button"
						class="btn btn-small"
						onClick={onDelete}
						title="Remove condition"
						style={{ color: "var(--tier-low)", flexShrink: 0 }}
					>
						&times;
					</button>
				)}
			</div>
			{predSchema && (
				<>
					<div class="setting-description" style={{ marginBottom: "8px" }}>
						{predSchema.description}
					</div>
					<PredicateEditor rule={rule} schema={predSchema} onChange={onChange} />
				</>
			)}
		</div>
	);
}

// ── Compound Group Editor (recursive) ────────────────────────────────

/** Renders a compound rule (All/Any) with its children. Supports nesting. */
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
	const mode = rule.rule_type;

	const setMode = (newMode: "All" | "Any") => {
		if (newMode === mode) return;
		onChange({ ...rule, rule_type: newMode });
	};

	const updateChild = (index: number, updated: Rule) => {
		const next = [...rule.rules];
		next[index] = updated;
		onChange({ ...rule, rules: next });
	};

	const deleteChild = (index: number) => {
		const next = rule.rules.filter((_: Rule, i: number) => i !== index);
		if (next.length === 0) {
			// If nested, remove this group entirely; if top-level, add a default
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
		<div class={`compound-predicates ${depth > 0 ? "compound-nested" : ""}`}>
			<div class="compound-group-header">
				<button
					type="button"
					class={`btn btn-small ${mode === "All" ? "btn-primary" : ""}`}
					onClick={() => setMode("All")}
				>
					All (AND)
				</button>
				<button
					type="button"
					class={`btn btn-small ${mode === "Any" ? "btn-primary" : ""}`}
					onClick={() => setMode("Any")}
				>
					Any (OR)
				</button>
				{onDelete && (
					<button
						type="button"
						class="btn btn-small"
						onClick={onDelete}
						title="Remove group"
						style={{ color: "var(--tier-low)", marginLeft: "auto" }}
					>
						&times;
					</button>
				)}
			</div>
			{rule.rules.map((child: Rule, i: number) => (
				// biome-ignore lint/suspicious/noArrayIndexKey: sub-rules have no stable ID
				<div key={i}>
					{i > 0 && <div class="compound-separator">{mode === "All" ? "AND" : "OR"}</div>}
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
	);
}

// ── Scoring Rule Editor ──────────────────────────────────────────────

type RuleMode = "Simple" | "All" | "Any";

/** Get current mode from a rule. */
function getRuleMode(r: Rule): RuleMode {
	return isCompoundRule(r) ? r.rule_type : "Simple";
}

/** Editor for a single scoring rule: label + weight + predicate(s). */
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

	const mode = getRuleMode(rule.rule);

	const setMode = (newMode: RuleMode) => {
		if (newMode === mode) return;
		if (newMode === "Simple") {
			// Unwrap: use first leaf predicate or a default
			const firstPred = findFirstPred(rule.rule);
			const fallbackSchema = schema[0];
			if (!firstPred && !fallbackSchema) return;
			onChange({ ...rule, rule: firstPred ?? defaultRule(fallbackSchema as PredicateSchema) });
		} else {
			// Wrap current rule into compound
			const inner = isCompoundRule(rule.rule) ? rule.rule.rules : [rule.rule];
			onChange({ ...rule, rule: { rule_type: newMode, rules: inner } });
		}
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

			{/* Collapsed summary for compound rules */}
			{!expanded && isCompoundRule(rule.rule) && (
				<div class="scoring-rule-summary">{summarizeRule(rule.rule, schema)}</div>
			)}

			{expanded && (
				<div class="scoring-rule-body">
					{/* Mode selector */}
					<div class="compound-mode-selector">
						{(["Simple", "All", "Any"] as const).map((m) => (
							<button
								key={m}
								type="button"
								class={`btn btn-small ${mode === m ? "btn-primary" : ""}`}
								onClick={() => setMode(m)}
							>
								{m === "All" ? "All (AND)" : m === "Any" ? "Any (OR)" : m}
							</button>
						))}
					</div>

					{/* Simple mode: single predicate */}
					{!isCompoundRule(rule.rule) && (
						<PredicateRow
							rule={rule.rule}
							schema={schema}
							onChange={(newRule) => onChange({ ...rule, rule: newRule })}
						/>
					)}

					{/* Compound mode: recursive group editor */}
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

// ── Mod Weights ──────────────────────────────────────────────────────────

/** Mod weight editor — search for stats and assign weight levels. */
function ModWeightsTab({
	modWeights,
	onUpdate,
}: {
	modWeights: ModWeight[];
	onUpdate: (modWeights: ModWeight[]) => void;
}) {
	const [suggestions, setSuggestions] = useState<string[]>([]);
	const [search, setSearch] = useState("");
	const [showDropdown, setShowDropdown] = useState(false);
	const [selectedIndex, setSelectedIndex] = useState(-1);
	const searchRef = useRef<HTMLDivElement>(null);

	useEffect(() => {
		getSuggestions("stat_texts").then(setSuggestions);
	}, []);

	// Close dropdown on outside click
	useEffect(() => {
		const handler = (e: MouseEvent) => {
			if (searchRef.current && !searchRef.current.contains(e.target as Node)) {
				setShowDropdown(false);
			}
		};
		document.addEventListener("mousedown", handler);
		return () => document.removeEventListener("mousedown", handler);
	}, []);

	const existingTemplates = new Set(modWeights.map((mw) => mw.template));
	const filtered = search
		? suggestions
				.filter((s) => !existingTemplates.has(s) && s.toLowerCase().includes(search.toLowerCase()))
				.slice(0, 30)
		: [];

	const addWeight = async (template: string) => {
		const statIds = await invoke<string[]>("resolve_stat_template", { template });
		onUpdate([...modWeights, { template, statIds, level: "medium" }]);
		setSearch("");
		setShowDropdown(false);
		setSelectedIndex(-1);
	};

	const updateLevel = (index: number, level: WeightLevel) => {
		const next = [...modWeights];
		const existing = next[index];
		if (existing) next[index] = { ...existing, level };
		onUpdate(next);
	};

	const removeWeight = (index: number) => {
		onUpdate(modWeights.filter((_, i) => i !== index));
	};

	const handleKeyDown = (e: KeyboardEvent) => {
		if (!showDropdown || filtered.length === 0) return;
		if (e.key === "ArrowDown") {
			e.preventDefault();
			setSelectedIndex((i) => Math.min(i + 1, filtered.length - 1));
		} else if (e.key === "ArrowUp") {
			e.preventDefault();
			setSelectedIndex((i) => Math.max(i - 1, 0));
		} else if (e.key === "Enter" && selectedIndex >= 0) {
			e.preventDefault();
			const selected = filtered[selectedIndex];
			if (selected !== undefined) addWeight(selected);
		} else if (e.key === "Escape") {
			setShowDropdown(false);
		}
	};

	return (
		<div class="setting-group">
			<h3>Mod Weights ({modWeights.length})</h3>
			<div class="setting-description" style={{ marginBottom: "12px" }}>
				Add stats you care about. Items with these stats score higher.
			</div>

			{/* Search to add new stats */}
			<div class="mod-weight-search" ref={searchRef}>
				<input
					type="text"
					class="pred-input"
					value={search}
					onInput={(e) => {
						setSearch((e.target as HTMLInputElement).value);
						setShowDropdown(true);
						setSelectedIndex(-1);
					}}
					onFocus={() => {
						if (search) setShowDropdown(true);
					}}
					onKeyDown={handleKeyDown}
					placeholder="Search stats to add..."
					style={{ width: "100%" }}
				/>
				{showDropdown && filtered.length > 0 && (
					<div class="pred-dropdown" style={{ position: "absolute", left: 0, right: 0 }}>
						{filtered.map((item, i) => (
							<div
								key={item}
								class={`pred-dropdown-item ${i === selectedIndex ? "selected" : ""}`}
								onMouseDown={(e) => {
									e.preventDefault();
									addWeight(item);
								}}
								onMouseEnter={() => setSelectedIndex(i)}
							>
								{item}
							</div>
						))}
					</div>
				)}
			</div>

			{/* Weighted stat list */}
			<div class="mod-weight-list">
				{modWeights.map((mw, i) => (
					<div key={mw.template} class="mod-weight-row">
						<span class="mod-weight-text">{mw.template}</span>
						<WeightSelector level={mw.level} onChange={(level) => updateLevel(i, level)} />
						<button
							type="button"
							class="btn btn-small"
							onClick={() => removeWeight(i)}
							title="Remove"
							style={{ color: "var(--tier-low)" }}
						>
							&times;
						</button>
					</div>
				))}
			</div>

			{modWeights.length === 0 && (
				<div class="setting-description" style={{ marginTop: "8px" }}>
					No mod weights set. Search above to add stats you care about.
				</div>
			)}
		</div>
	);
}

/** Clickable weight level selector with filled blocks. */
function WeightSelector({
	level,
	onChange,
}: {
	level: WeightLevel;
	onChange: (level: WeightLevel) => void;
}) {
	const activeIndex = WEIGHT_LEVELS.indexOf(level);

	return (
		<div class="mod-weight-selector">
			<div class="mod-weight-blocks">
				{WEIGHT_LEVELS.map((l, i) => (
					<button
						key={l}
						type="button"
						class={`mod-weight-block ${i <= activeIndex ? "filled" : ""}`}
						onClick={() => onChange(l)}
						title={`${LEVEL_LABELS[l]} (${WEIGHT_VALUES[l]} pts)`}
					/>
				))}
			</div>
			<span class="mod-weight-level-label">{LEVEL_LABELS[level]}</span>
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
