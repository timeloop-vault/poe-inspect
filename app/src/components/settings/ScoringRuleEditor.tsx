import { useEffect, useRef, useState } from "preact/hooks";
import { WEIGHT_VALUES, type WeightLevel } from "../../store";
import {
	type PredicateSchema,
	type Rule,
	type ScoringRule,
	isCompoundRule,
	isPredRule,
} from "../../types";
import { PredicateEditor, defaultRule } from "./PredicateEditor";

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
export function weightMatchesLevel(weight: number): WeightLevel | null {
	for (const l of WEIGHT_LEVELS) {
		if (WEIGHT_VALUES[l] === weight) return l;
	}
	return null;
}

/** Check if a ScoringRule is a simple stat weight (HasStatId predicate). */
export function isStatRule(rule: ScoringRule): boolean {
	return isPredRule(rule.rule) && (rule.rule as Record<string, unknown>).type === "HasStatId";
}

export { LEVEL_LABELS };

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
export function CompactStatRow({
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

// ── Predicate Row (reusable: single Pred rule with type selector) ─────

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
export function ScoringRuleEditor({
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
