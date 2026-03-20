import { invoke } from "@tauri-apps/api/core";
/**
 * Schema-driven predicate editor.
 *
 * Renders input fields for a predicate based on the schema from poe-eval.
 * Each FieldKind maps to a specific widget. New predicates that use
 * existing field kinds get UI automatically.
 *
 * ## StatValue Editor — Design
 *
 * StatValue uses `conditions: Vec<StatCondition>`. 1 condition = "any mod
 * with this stat satisfies the check". 2+ conditions = "all must match on
 * the SAME mod" (hybrid/multi-stat mod detection).
 *
 * ### Flow
 *
 * 1. User types in the stat template input (autocomplete against GGPK
 *    stat descriptions).
 * 2. User picks a template (e.g., "# to maximum Life").
 * 3. Backend resolves the stat_id and checks for hybrid mods containing
 *    that stat. If hybrids exist, a second dropdown appears:
 *      - First option: the single stat (confirms single mode)
 *      - Separator: "Hybrid Mods"
 *      - Hybrid options: "# to Armour (Prefix)", etc.
 *    If no hybrids, goes straight to single mode.
 * 4. Single pick → single mode. Hybrid pick → multi mode.
 *
 * ### Single mode layout
 *
 *   [stat template input] [op] [value]
 *
 * Compact, inline. The input IS the stat label — no redundancy.
 *
 * ### Multi (hybrid) mode layout
 *
 *   [stat template input]           ← re-editable to change selection
 *     [# to maximum Life]  [op] [value]    ← condition row (read-only label)
 *     [# to Armour]        [op] [value]    ← condition row (read-only label)
 *
 * The input keeps the original search text and serves as "change my mind"
 * entry point — typing re-triggers autocomplete and regenerates conditions.
 * Each condition row shows the template text as a read-only label (with
 * stat_id visible on hover/toggle). Op and value are editable per condition.
 *
 * ### No mode buttons
 *
 * Single vs multi is determined entirely by what the user picks from the
 * dropdown. To switch from multi back to single: type in the input, pick
 * a single stat or a different hybrid.
 */
import { useEffect, useRef, useState } from "preact/hooks";
import { loadGeneral } from "../../store";
import type {
	FieldKind,
	PredicateField,
	PredicateSchema,
	Rule,
	StatCondition,
	StatSuggestion,
} from "../../types";

// ── Suggestion cache ──────────────────────────────────────────────────

const suggestionCache = new Map<string, string[]>();

export async function getSuggestions(source: string): Promise<string[]> {
	const cached = suggestionCache.get(source);
	if (cached) return cached;
	try {
		const result = await invoke<string[]>("get_suggestions", { source });
		suggestionCache.set(source, result);
		return result;
	} catch {
		return [];
	}
}

// ── Schema cache ──────────────────────────────────────────────────────

let schemaCache: PredicateSchema[] | null = null;

export async function getSchema(): Promise<PredicateSchema[]> {
	if (schemaCache) return schemaCache;
	try {
		schemaCache = await invoke<PredicateSchema[]>("get_predicate_schema");
		return schemaCache;
	} catch {
		return [];
	}
}

// ── Default values for fields ─────────────────────────────────────────

function defaultFieldValue(kind: FieldKind): unknown {
	switch (kind.type) {
		case "comparison":
			return kind.allowedOps[0] ?? "Eq";
		case "number":
			return kind.min ?? 0;
		case "enum":
		case "orderedEnum":
			return kind.options[0]?.value ?? "";
		case "text":
			return "";
		case "slot":
		case "source":
			// Optional filter fields default to null (= "Any" / no filter)
			return null;
	}
}

/** Default StatCondition. */
function defaultCondition(): StatCondition {
	return { value_index: 0, op: "Ge", value: 0 };
}

/** Human-readable label for a multi-value condition based on its stat_id. */
function conditionValueLabel(cond: StatCondition, index: number): string {
	// Try to derive a label from the stat_id at this value_index
	const statId = cond.stat_ids?.[cond.value_index];
	if (statId) {
		// Extract the distinguishing part: "global_minimum_added_physical_damage" → "minimum"
		// Common patterns: minimum/maximum, added_small/added_large
		if (statId.includes("minimum")) return `# ${index + 1} (min)`;
		if (statId.includes("maximum")) return `# ${index + 1} (max)`;
	}
	return `# ${index + 1}`;
}

/** Build a default Rule from a predicate schema. */
export function defaultRule(schema: PredicateSchema): Rule {
	// StatValue uses conditions array, not flat fields
	if (schema.typeName === "StatValue") {
		return {
			rule_type: "Pred",
			type: "StatValue",
			conditions: [defaultCondition()],
		} as Rule;
	}
	const rule: Record<string, unknown> = {
		rule_type: "Pred",
		type: schema.typeName,
	};
	for (const field of schema.fields) {
		rule[field.name] = defaultFieldValue(field.kind);
	}
	return rule as Rule;
}

/** Build a default compound Rule (All or Any) containing one default predicate. */
export function defaultCompoundRule(schema: PredicateSchema, mode: "All" | "Any" = "All"): Rule {
	return { rule_type: mode, rules: [defaultRule(schema)] };
}

// ── Comparison operator display ───────────────────────────────────────

const CMP_LABELS: Record<string, string> = {
	Eq: "=",
	Ne: "!=",
	Gt: ">",
	Ge: ">=",
	Lt: "<",
	Le: "<=",
};

// ── Main component ────────────────────────────────────────────────────

/** Render fields for a Pred rule based on its schema. */
export function PredicateEditor({
	rule,
	schema,
	onChange,
	compact,
}: {
	rule: Rule;
	schema: PredicateSchema;
	onChange: (rule: Rule) => void;
	compact?: boolean;
}) {
	if (rule.rule_type !== "Pred") return null;

	// StatValue has a custom editor with conditions + hybrid detection
	if (schema.typeName === "StatValue") {
		return (
			<StatValueEditor rule={rule} onChange={onChange} {...(compact ? { compact: true } : {})} />
		);
	}

	// StatTier: same stat autocomplete as StatValue, plus kind/op/tier fields
	if (schema.typeName === "StatTier") {
		return (
			<StatTierEditor rule={rule} onChange={onChange} {...(compact ? { compact: true } : {})} />
		);
	}

	// RollPercent: same stat autocomplete as StatValue, plus op/percent fields
	if (schema.typeName === "RollPercent") {
		return (
			<RollPercentEditor rule={rule} onChange={onChange} {...(compact ? { compact: true } : {})} />
		);
	}

	const updateField = (name: string, value: unknown) => {
		onChange({ ...rule, [name]: value } as Rule);
	};

	return (
		<div class={`predicate-fields${compact ? " compact" : ""}`}>
			{schema.fields.map((field) => (
				<FieldWidget
					key={field.name}
					field={field}
					value={(rule as Record<string, unknown>)[field.name]}
					onChange={(v) => updateField(field.name, v)}
				/>
			))}
		</div>
	);
}

// ── StatValue custom editor ─────────────────────────────────────────

/** Dropdown item for the stat search / hybrid choice dropdown. */
type StatDropdownItem =
	| { kind: "stat"; text: string }
	| { kind: "separator"; text: string }
	| { kind: "hybrid"; suggestion: StatSuggestion; label: string };

/** Custom editor for StatValue: manages conditions[] with hybrid detection. */
function StatValueEditor({
	rule,
	onChange,
	compact,
}: {
	rule: Rule;
	onChange: (rule: Rule) => void;
	compact?: boolean;
}) {
	const r = rule as Record<string, unknown>;
	const conditions: StatCondition[] = (r.conditions as StatCondition[]) ?? [defaultCondition()];
	const isMulti = conditions.length > 1;

	// Dropdown state — shared between autocomplete and hybrid-choice phases
	const [suggestions, setSuggestions] = useState<string[]>([]);
	const [hybridOptions, setHybridOptions] = useState<StatSuggestion[]>([]);
	const [pickedTemplate, setPickedTemplate] = useState<string | null>(null);
	const [showDropdown, setShowDropdown] = useState(false);
	const [selectedIndex, setSelectedIndex] = useState(-1);
	const [filterText, setFilterText] = useState("");
	const [showStatIds, setShowStatIds] = useState(false);
	const wrapperRef = useRef<HTMLLabelElement>(null);
	const dropdownRef = useRef<HTMLDivElement>(null);
	const editorRef = useRef<HTMLDivElement>(null);

	// Load stat suggestions on mount + check power-user setting
	useEffect(() => {
		getSuggestions("stat_texts").then(setSuggestions);
		loadGeneral().then((s) => setShowStatIds(s.showStatIds));
	}, []);

	// Scroll dropdown into view when it appears
	useEffect(() => {
		if (showDropdown && dropdownRef.current) {
			dropdownRef.current.scrollIntoView({ block: "nearest", behavior: "smooth" });
		}
	}, [showDropdown]);

	// Scroll editor into view when switching to multi mode
	useEffect(() => {
		if (isMulti && editorRef.current) {
			editorRef.current.scrollIntoView({ block: "nearest", behavior: "smooth" });
		}
	}, [isMulti]);

	// Close dropdown on outside click
	useEffect(() => {
		const handler = (e: MouseEvent) => {
			if (wrapperRef.current && !wrapperRef.current.contains(e.target as Node)) {
				setShowDropdown(false);
			}
		};
		document.addEventListener("mousedown", handler);
		return () => document.removeEventListener("mousedown", handler);
	}, []);

	// Build dropdown items based on phase
	const dropdownItems: StatDropdownItem[] = (() => {
		if (pickedTemplate && hybridOptions.length > 0) {
			// Phase 2: hybrid choice — single stat first, separator, then hybrids
			const items: StatDropdownItem[] = [{ kind: "stat", text: pickedTemplate }];
			items.push({ kind: "separator", text: "Hybrid Mods" });
			for (const h of hybridOptions) {
				if (h.kind.type !== "Hybrid") continue;
				const gen = h.kind.generation_type === 1 ? "Prefix" : "Suffix";
				const otherStats = h.kind.other_templates.join(", ");
				items.push({
					kind: "hybrid",
					suggestion: h,
					label: `${otherStats} (${gen})`,
				});
			}
			// Filter by text if user is typing in phase 2
			if (filterText) {
				const words = filterText.toLowerCase().split(/\s+/).filter(Boolean);
				return items.filter((item) => {
					if (item.kind === "separator") return true;
					const text = item.kind === "stat" ? item.text : item.label;
					const lower = text.toLowerCase();
					return words.every((w) => lower.includes(w));
				});
			}
			return items;
		}
		// Phase 1: autocomplete — filter stat templates
		const text = conditions[0]?.text ?? "";
		if (!text || suggestions.length === 0) return [];
		const words = text.toLowerCase().split(/\s+/).filter(Boolean);
		if (words.length === 0) return [];
		return suggestions
			.filter((s) => {
				const lower = s.toLowerCase();
				return words.every((w) => lower.includes(w));
			})
			.slice(0, 50)
			.map((s): StatDropdownItem => ({ kind: "stat", text: s }));
	})();

	const clickableItems = dropdownItems.filter((item) => item.kind !== "separator");

	const updateCondition = (index: number, partial: Partial<StatCondition>) => {
		const newConds = conditions.map((c, i) => (i === index ? { ...c, ...partial } : c));
		onChange({ ...rule, conditions: newConds } as Rule);
	};

	const handleTextInput = (text: string) => {
		// User is typing — stay in / return to autocomplete phase
		const newConds = [...conditions];
		newConds[0] = { ...(conditions[0] ?? defaultCondition()), text };
		onChange({ ...rule, conditions: newConds } as Rule);
		setPickedTemplate(null);
		setHybridOptions([]);
		setFilterText("");
		setShowDropdown(true);
		setSelectedIndex(-1);
	};

	const handleStatPick = (template: string) => {
		// User selected a stat template — resolve stat_ids and check for
		// multi-value templates and hybrid mods.
		invoke<StatSuggestion[]>("get_stat_suggestions", { query: template }).then((suggestions) => {
			const single = suggestions.find((s) => s.kind.type === "Single" && s.template === template);
			const statIds = single?.stat_ids ?? [];

			// Count # placeholders in template to detect multi-value stats
			// (e.g., "Adds # to # Physical Damage" → 2 values)
			const valueCount = (template.match(/#/g) ?? []).length;

			let newConditions: StatCondition[];
			if (valueCount > 1 && statIds.length > 1) {
				// Multi-value: one condition per # placeholder
				newConditions = Array.from({ length: valueCount }, (_, i) => ({
					...defaultCondition(),
					text: template,
					stat_ids: statIds,
					value_index: i,
				}));
			} else {
				newConditions = [{ ...defaultCondition(), text: template, stat_ids: statIds }];
			}
			onChange({ ...rule, conditions: newConditions } as Rule);

			if (statIds.length > 0 && valueCount <= 1) {
				// Only offer hybrid choice for single-value stats
				const hybrids = suggestions.filter(
					(s) =>
						s.kind.type === "Hybrid" &&
						s.template === template &&
						s.kind.other_templates.length > 0 &&
						s.kind.other_templates.some((t) => t.length > 0),
				);
				if (hybrids.length > 0) {
					setPickedTemplate(template);
					setHybridOptions(hybrids);
					setFilterText("");
					setShowDropdown(true);
					setSelectedIndex(-1);
				} else {
					setShowDropdown(false);
				}
			} else {
				setShowDropdown(false);
			}
		});
	};

	const handleHybridPick = (suggestion: StatSuggestion) => {
		if (suggestion.kind.type !== "Hybrid") return;
		const h = suggestion.kind;
		const primaryCond: StatCondition = {
			text: suggestion.template,
			stat_ids: suggestion.stat_ids,
			value_index: 0,
			op: conditions[0]?.op ?? "Ge",
			value: conditions[0]?.value ?? 0,
		};
		const otherConds: StatCondition[] = h.other_templates.map((template, i) => ({
			text: template,
			stat_ids: h.other_stat_ids[i] ? [h.other_stat_ids[i]] : [],
			value_index: 0,
			op: "Ge" as const,
			value: 0,
		}));
		onChange({ ...rule, type: "StatValue", conditions: [primaryCond, ...otherConds] } as Rule);
		setPickedTemplate(null);
		setHybridOptions([]);
		setShowDropdown(false);
	};

	const handleDropdownSelect = (item: StatDropdownItem) => {
		if (item.kind === "separator") return;
		if (item.kind === "stat") {
			if (pickedTemplate) {
				// Phase 2: user picked the single stat option — confirm single mode, done
				setPickedTemplate(null);
				setHybridOptions([]);
				setShowDropdown(false);
			} else {
				// Phase 1: user picked from autocomplete
				handleStatPick(item.text);
			}
		} else if (item.kind === "hybrid") {
			handleHybridPick(item.suggestion);
		}
	};

	const handleKeyDown = (e: KeyboardEvent) => {
		if (!showDropdown || clickableItems.length === 0) return;
		if (e.key === "ArrowDown") {
			e.preventDefault();
			setSelectedIndex((i) => Math.min(i + 1, clickableItems.length - 1));
		} else if (e.key === "ArrowUp") {
			e.preventDefault();
			setSelectedIndex((i) => Math.max(i - 1, 0));
		} else if (e.key === "Enter" && selectedIndex >= 0) {
			e.preventDefault();
			const item = clickableItems[selectedIndex];
			if (item) handleDropdownSelect(item);
		} else if (e.key === "Escape") {
			setShowDropdown(false);
			setPickedTemplate(null);
			setHybridOptions([]);
		}
	};

	// Track clickable index for highlighting
	let clickableIndex = -1;

	// Shared stat template input with autocomplete + hybrid dropdown
	const statInput = (
		<label class="pred-field" ref={wrapperRef}>
			<div class="pred-text-wrapper">
				<input
					type="text"
					class="pred-input"
					title={conditions[0]?.stat_ids?.join(", ") || undefined}
					value={pickedTemplate ? filterText : (conditions[0]?.text ?? "")}
					onInput={(e) => {
						const v = (e.target as HTMLInputElement).value;
						if (pickedTemplate) {
							setFilterText(v);
							setSelectedIndex(-1);
						} else {
							handleTextInput(v);
						}
					}}
					onFocus={() => {
						if (dropdownItems.length > 0 || conditions[0]?.text) setShowDropdown(true);
					}}
					onKeyDown={handleKeyDown}
					placeholder={pickedTemplate ? "Filter hybrids..." : "Type to search..."}
				/>
				{pickedTemplate && <div class="stat-value-picked">{pickedTemplate}</div>}
				{showDropdown && dropdownItems.length > 0 && (
					<div class="pred-dropdown" ref={dropdownRef}>
						{dropdownItems.map((item) => {
							if (item.kind === "separator") {
								return (
									<div key={item.text} class="pred-dropdown-separator">
										{item.text}
									</div>
								);
							}
							clickableIndex++;
							const idx = clickableIndex;
							const label = item.kind === "stat" ? item.text : item.label;
							return (
								<div
									key={label}
									class={`pred-dropdown-item${item.kind === "hybrid" ? " hybrid" : ""}${idx === selectedIndex ? " selected" : ""}`}
									onMouseDown={(e) => {
										e.preventDefault();
										handleDropdownSelect(item);
									}}
									onMouseEnter={() => setSelectedIndex(idx)}
								>
									{label}
								</div>
							);
						})}
					</div>
				)}
			</div>
		</label>
	);

	// Inline stat_ids display for a condition (shown in power-user mode)
	const statIdBox = (cond: StatCondition) =>
		showStatIds && cond.stat_ids && cond.stat_ids.length > 0 ? (
			<input
				type="text"
				class="stat-id-readonly"
				value={cond.stat_ids.join(", ")}
				readOnly
				title={cond.stat_ids.join(", ")}
			/>
		) : null;

	// Single mode: template | stat_id | op | value
	if (!isMulti) {
		return (
			<div class={`predicate-fields stat-value-editor${compact ? " compact" : ""}`}>
				{statInput}
				{conditions[0] && statIdBox(conditions[0])}
				<ComparisonField
					label=""
					allowedOps={["Eq", "Ge", "Gt", "Le", "Lt"]}
					value={conditions[0]?.op ?? "Ge"}
					onChange={(v) => updateCondition(0, { op: v as StatCondition["op"] })}
				/>
				<NumberField
					label=""
					min={null}
					max={null}
					value={conditions[0]?.value ?? 0}
					onChange={(v) => updateCondition(0, { value: Number(v) })}
				/>
			</div>
		);
	}

	// Check if this is multi-value (same template, different value_index)
	// vs hybrid (different templates)
	const isMultiValue = isMulti && conditions.every((c) => c.text === conditions[0]?.text);

	// Multi (hybrid or multi-value) mode: input on top, condition rows below
	return (
		<div
			ref={editorRef}
			class={`predicate-fields stat-value-editor multi${compact ? " compact" : ""}`}
		>
			{statInput}
			<div class="stat-value-conditions">
				{conditions.map((cond, i) => (
					// biome-ignore lint/suspicious/noArrayIndexKey: conditions have no stable ID
					<div key={i} class="stat-value-condition">
						<span class="stat-value-condition-label" title={cond.stat_ids?.join(", ") ?? ""}>
							{isMultiValue
								? conditionValueLabel(cond, i)
								: cond.text || cond.stat_ids?.[0] || `Condition ${i + 1}`}
						</span>
						{statIdBox(cond)}
						<ComparisonField
							label=""
							allowedOps={["Eq", "Ge", "Gt", "Le", "Lt"]}
							value={cond.op}
							onChange={(v) => updateCondition(i, { op: v as StatCondition["op"] })}
						/>
						<NumberField
							label=""
							min={null}
							max={null}
							value={cond.value}
							onChange={(v) => updateCondition(i, { value: Number(v) })}
						/>
					</div>
				))}
			</div>
		</div>
	);
}

// ── StatTier custom editor ────────────────────────────────────────────

/** Custom editor for StatTier: same stat autocomplete as StatValue, plus kind/op/tier. */
function StatTierEditor({
	rule,
	onChange,
	compact,
}: {
	rule: Rule;
	onChange: (rule: Rule) => void;
	compact?: boolean;
}) {
	const r = rule as Record<string, unknown>;
	const text = (r.text as string) ?? "";
	const statIds = (r.stat_ids as string[]) ?? [];
	const kind = (r.kind as string) ?? "Tier";
	const op = (r.op as string) ?? "Le";
	const value = (r.value as number) ?? 1;
	const source = (r.source as string | null) ?? null;

	const [suggestions, setSuggestions] = useState<string[]>([]);
	const [showDropdown, setShowDropdown] = useState(false);
	const [selectedIndex, setSelectedIndex] = useState(-1);
	const [showStatIds, setShowStatIds] = useState(false);
	const wrapperRef = useRef<HTMLLabelElement>(null);
	const dropdownRef = useRef<HTMLDivElement>(null);

	useEffect(() => {
		getSuggestions("stat_texts").then(setSuggestions);
		loadGeneral().then((s) => setShowStatIds(s.showStatIds));
	}, []);

	useEffect(() => {
		if (showDropdown && dropdownRef.current) {
			dropdownRef.current.scrollIntoView({ block: "nearest", behavior: "smooth" });
		}
	}, [showDropdown]);

	useEffect(() => {
		const handler = (e: MouseEvent) => {
			if (wrapperRef.current && !wrapperRef.current.contains(e.target as Node)) {
				setShowDropdown(false);
			}
		};
		document.addEventListener("mousedown", handler);
		return () => document.removeEventListener("mousedown", handler);
	}, []);

	// Filter suggestions by user text
	const filtered = (() => {
		if (!text || suggestions.length === 0) return [];
		const words = text.toLowerCase().split(/\s+/).filter(Boolean);
		if (words.length === 0) return [];
		return suggestions
			.filter((s) => {
				const lower = s.toLowerCase();
				return words.every((w) => lower.includes(w));
			})
			.slice(0, 50);
	})();

	const handleTextInput = (newText: string) => {
		onChange({ ...rule, text: newText, stat_ids: [] } as Rule);
		setShowDropdown(true);
		setSelectedIndex(-1);
	};

	const handleStatPick = (template: string) => {
		invoke<StatSuggestion[]>("get_stat_suggestions", { query: template }).then((results) => {
			const single = results.find((s) => s.kind.type === "Single" && s.template === template);
			const ids = single?.stat_ids ?? [];
			onChange({ ...rule, text: template, stat_ids: ids } as Rule);
			setShowDropdown(false);
		});
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
			if (selected !== undefined) handleStatPick(selected);
		} else if (e.key === "Escape") {
			setShowDropdown(false);
		}
	};

	const updateField = (name: string, v: unknown) => {
		onChange({ ...rule, [name]: v } as Rule);
	};

	return (
		<div class={`predicate-fields stat-value-editor${compact ? " compact" : ""}`}>
			<label class="pred-field" ref={wrapperRef}>
				<div class="pred-text-wrapper">
					<input
						type="text"
						class="pred-input"
						title={statIds.join(", ") || undefined}
						value={text}
						onInput={(e) => handleTextInput((e.target as HTMLInputElement).value)}
						onFocus={() => {
							if (filtered.length > 0 || text) setShowDropdown(true);
						}}
						onKeyDown={handleKeyDown}
						placeholder="Type to search..."
					/>
					{showDropdown && filtered.length > 0 && (
						<div class="pred-dropdown" ref={dropdownRef}>
							{filtered.map((item, i) => (
								<div
									key={item}
									class={`pred-dropdown-item${i === selectedIndex ? " selected" : ""}`}
									onMouseDown={(e) => {
										e.preventDefault();
										handleStatPick(item);
									}}
									onMouseEnter={() => setSelectedIndex(i)}
								>
									{item}
								</div>
							))}
						</div>
					)}
				</div>
			</label>
			{showStatIds && statIds.length > 0 && (
				<input
					type="text"
					class="stat-id-readonly"
					value={statIds.join(", ")}
					readOnly
					title={statIds.join(", ")}
				/>
			)}
			<EnumField
				label="Kind"
				options={[
					{ value: "Tier", label: "Tier" },
					{ value: "Rank", label: "Rank" },
					{ value: "Either", label: "Either" },
				]}
				value={kind}
				onChange={(v) => updateField("kind", v)}
			/>
			<ComparisonField
				label=""
				allowedOps={["Eq", "Ge", "Gt", "Le", "Lt"]}
				value={op}
				onChange={(v) => updateField("op", v)}
			/>
			<NumberField
				label=""
				min={1}
				max={20}
				value={value}
				onChange={(v) => updateField("value", Number(v))}
			/>
			<OptionalEnumField
				label="Source"
				options={[
					{ value: "Regular", label: "Regular" },
					{ value: "Fractured", label: "Fractured" },
					{ value: "MasterCrafted", label: "Crafted" },
				]}
				value={source}
				onChange={(v) => updateField("source", v)}
				placeholder="Any"
			/>
		</div>
	);
}

// ── RollPercent custom editor ─────────────────────────────────────────

/** Custom editor for RollPercent: same stat autocomplete as StatValue, plus op/percent. */
function RollPercentEditor({
	rule,
	onChange,
	compact,
}: {
	rule: Rule;
	onChange: (rule: Rule) => void;
	compact?: boolean;
}) {
	const r = rule as Record<string, unknown>;
	const text = (r.text as string) ?? "";
	const statIds = (r.stat_ids as string[]) ?? [];
	const op = (r.op as string) ?? "Ge";
	const value = (r.value as number) ?? 0;

	const [suggestions, setSuggestions] = useState<string[]>([]);
	const [showDropdown, setShowDropdown] = useState(false);
	const [selectedIndex, setSelectedIndex] = useState(-1);
	const [showStatIds, setShowStatIds] = useState(false);
	const wrapperRef = useRef<HTMLLabelElement>(null);
	const dropdownRef = useRef<HTMLDivElement>(null);

	useEffect(() => {
		getSuggestions("stat_texts").then(setSuggestions);
		loadGeneral().then((s) => setShowStatIds(s.showStatIds));
	}, []);

	useEffect(() => {
		if (showDropdown && dropdownRef.current) {
			dropdownRef.current.scrollIntoView({ block: "nearest", behavior: "smooth" });
		}
	}, [showDropdown]);

	useEffect(() => {
		const handler = (e: MouseEvent) => {
			if (wrapperRef.current && !wrapperRef.current.contains(e.target as Node)) {
				setShowDropdown(false);
			}
		};
		document.addEventListener("mousedown", handler);
		return () => document.removeEventListener("mousedown", handler);
	}, []);

	const filtered = (() => {
		if (!text || suggestions.length === 0) return [];
		const words = text.toLowerCase().split(/\s+/).filter(Boolean);
		if (words.length === 0) return [];
		return suggestions
			.filter((s) => {
				const lower = s.toLowerCase();
				return words.every((w) => lower.includes(w));
			})
			.slice(0, 50);
	})();

	const handleTextInput = (newText: string) => {
		onChange({ ...rule, text: newText, stat_ids: [], value_index: 0 } as Rule);
		setShowDropdown(true);
		setSelectedIndex(-1);
	};

	const handleStatPick = (template: string) => {
		invoke<StatSuggestion[]>("get_stat_suggestions", { query: template }).then((results) => {
			const single = results.find((s) => s.kind.type === "Single" && s.template === template);
			const ids = single?.stat_ids ?? [];
			onChange({ ...rule, text: template, stat_ids: ids, value_index: 0 } as Rule);
			setShowDropdown(false);
		});
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
			if (selected !== undefined) handleStatPick(selected);
		} else if (e.key === "Escape") {
			setShowDropdown(false);
		}
	};

	return (
		<div class={`predicate-fields stat-value-editor${compact ? " compact" : ""}`}>
			<label class="pred-field" ref={wrapperRef}>
				<div class="pred-text-wrapper">
					<input
						type="text"
						class="pred-input"
						title={statIds.join(", ") || undefined}
						value={text}
						onInput={(e) => handleTextInput((e.target as HTMLInputElement).value)}
						onFocus={() => {
							if (filtered.length > 0 || text) setShowDropdown(true);
						}}
						onKeyDown={handleKeyDown}
						placeholder="Type to search..."
					/>
					{showDropdown && filtered.length > 0 && (
						<div class="pred-dropdown" ref={dropdownRef}>
							{filtered.map((item, i) => (
								<div
									key={item}
									class={`pred-dropdown-item${i === selectedIndex ? " selected" : ""}`}
									onMouseDown={(e) => {
										e.preventDefault();
										handleStatPick(item);
									}}
									onMouseEnter={() => setSelectedIndex(i)}
								>
									{item}
								</div>
							))}
						</div>
					)}
				</div>
			</label>
			{showStatIds && statIds.length > 0 && (
				<input
					type="text"
					class="stat-id-readonly"
					value={statIds.join(", ")}
					readOnly
					title={statIds.join(", ")}
				/>
			)}
			<ComparisonField
				label=""
				allowedOps={["Eq", "Ge", "Gt", "Le", "Lt"]}
				value={op}
				onChange={(v) => onChange({ ...rule, op: v } as Rule)}
			/>
			<NumberField
				label=""
				min={0}
				max={100}
				value={value}
				onChange={(v) => onChange({ ...rule, value: Number(v) } as Rule)}
			/>
		</div>
	);
}

// ── Field widgets ─────────────────────────────────────────────────────

function FieldWidget({
	field,
	value,
	onChange,
}: {
	field: PredicateField;
	value: unknown;
	onChange: (v: unknown) => void;
}) {
	switch (field.kind.type) {
		case "comparison":
			return (
				<ComparisonField
					label={field.label}
					allowedOps={field.kind.allowedOps}
					value={String(value ?? "Eq")}
					onChange={onChange}
				/>
			);
		case "number":
			return (
				<NumberField
					label={field.label}
					min={field.kind.min}
					max={field.kind.max}
					value={Number(value ?? 0)}
					onChange={onChange}
				/>
			);
		case "enum":
		case "orderedEnum":
			return (
				<EnumField
					label={field.label}
					options={field.kind.options}
					value={String(value ?? "")}
					onChange={onChange}
				/>
			);
		case "text":
			return (
				<TextField
					label={field.label}
					suggestionsFrom={field.kind.suggestionsFrom}
					value={String(value ?? "")}
					onChange={onChange}
				/>
			);
		case "slot":
			return (
				<OptionalEnumField
					label={field.label}
					options={field.kind.options}
					value={value as string | null}
					onChange={onChange}
					placeholder="Any"
				/>
			);
		case "source":
			return (
				<OptionalEnumField
					label={field.label}
					options={field.kind.options}
					value={value as string | null}
					onChange={onChange}
					placeholder="Any"
				/>
			);
	}
}

function ComparisonField({
	label,
	allowedOps,
	value,
	onChange,
}: {
	label: string;
	allowedOps: string[];
	value: string;
	onChange: (v: string) => void;
}) {
	return (
		<label class="pred-field">
			{label && <span class="pred-field-label">{label}</span>}
			<select
				class="pred-select"
				value={value}
				onChange={(e) => onChange((e.target as HTMLSelectElement).value)}
			>
				{allowedOps.map((op) => (
					<option key={op} value={op}>
						{CMP_LABELS[op] ?? op}
					</option>
				))}
			</select>
		</label>
	);
}

function NumberField({
	label,
	min,
	max,
	value,
	onChange,
}: {
	label: string;
	min: number | null;
	max: number | null;
	value: number;
	onChange: (v: number) => void;
}) {
	return (
		<label class="pred-field">
			{label && <span class="pred-field-label">{label}</span>}
			<input
				type="number"
				class="pred-input pred-input-number"
				value={value}
				min={min ?? undefined}
				max={max ?? undefined}
				onInput={(e) => onChange(Number((e.target as HTMLInputElement).value))}
			/>
		</label>
	);
}

function EnumField({
	label,
	options,
	value,
	onChange,
}: {
	label: string;
	options: { value: string; label: string }[];
	value: string;
	onChange: (v: string) => void;
}) {
	return (
		<label class="pred-field">
			<span class="pred-field-label">{label}</span>
			<select
				class="pred-select"
				value={value}
				onChange={(e) => onChange((e.target as HTMLSelectElement).value)}
			>
				{options.map((opt) => (
					<option key={opt.value} value={opt.value}>
						{opt.label}
					</option>
				))}
			</select>
		</label>
	);
}

/** Enum field with an optional "Any" (null) choice. Used for slot and source filters. */
function OptionalEnumField({
	label,
	options,
	value,
	onChange,
	placeholder,
}: {
	label: string;
	options: { value: string; label: string }[];
	value: string | null;
	onChange: (v: string | null) => void;
	placeholder: string;
}) {
	return (
		<label class="pred-field">
			<span class="pred-field-label">{label}</span>
			<select
				class="pred-select"
				value={value ?? ""}
				onChange={(e) => {
					const v = (e.target as HTMLSelectElement).value;
					onChange(v === "" ? null : v);
				}}
			>
				<option value="">{placeholder}</option>
				{options.map((opt) => (
					<option key={opt.value} value={opt.value}>
						{opt.label}
					</option>
				))}
			</select>
		</label>
	);
}

/** Text input with autocomplete suggestions loaded from game data. */
function TextField({
	label,
	suggestionsFrom,
	value,
	onChange,
}: {
	label: string;
	suggestionsFrom: string | null;
	value: string;
	onChange: (v: string) => void;
}) {
	const [suggestions, setSuggestions] = useState<string[]>([]);
	const [filtered, setFiltered] = useState<string[]>([]);
	const [showDropdown, setShowDropdown] = useState(false);
	const [selectedIndex, setSelectedIndex] = useState(-1);
	const wrapperRef = useRef<HTMLLabelElement>(null);

	// Load suggestions on mount if source is set
	useEffect(() => {
		if (suggestionsFrom) {
			getSuggestions(suggestionsFrom).then(setSuggestions);
		}
	}, [suggestionsFrom]);

	// Filter suggestions as user types (fuzzy: every word must appear as a substring)
	useEffect(() => {
		if (!value || suggestions.length === 0) {
			setFiltered([]);
			return;
		}
		const words = value.toLowerCase().split(/\s+/).filter(Boolean);
		if (words.length === 0) {
			setFiltered([]);
			return;
		}
		const matches = suggestions
			.filter((s) => {
				const lower = s.toLowerCase();
				return words.every((w) => lower.includes(w));
			})
			.slice(0, 50);
		setFiltered(matches);
	}, [value, suggestions]);

	// Close dropdown on outside click
	useEffect(() => {
		const handler = (e: MouseEvent) => {
			if (wrapperRef.current && !wrapperRef.current.contains(e.target as Node)) {
				setShowDropdown(false);
			}
		};
		document.addEventListener("mousedown", handler);
		return () => document.removeEventListener("mousedown", handler);
	}, []);

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
			if (selected !== undefined) {
				onChange(selected);
				setShowDropdown(false);
			}
		} else if (e.key === "Escape") {
			setShowDropdown(false);
		}
	};

	return (
		<label class="pred-field" ref={wrapperRef}>
			{label && <span class="pred-field-label">{label}</span>}
			<div class="pred-text-wrapper">
				<input
					type="text"
					class="pred-input"
					value={value}
					onInput={(e) => {
						onChange((e.target as HTMLInputElement).value);
						setShowDropdown(true);
						setSelectedIndex(-1);
					}}
					onFocus={() => {
						if (filtered.length > 0 || value) setShowDropdown(true);
					}}
					onKeyDown={handleKeyDown}
					placeholder={suggestionsFrom ? "Type to search..." : undefined}
				/>
				{showDropdown && filtered.length > 0 && (
					<div class="pred-dropdown">
						{filtered.map((item, i) => (
							<div
								key={item}
								class={`pred-dropdown-item ${i === selectedIndex ? "selected" : ""}`}
								onMouseDown={(e) => {
									e.preventDefault();
									onChange(item);
									setShowDropdown(false);
								}}
								onMouseEnter={() => setSelectedIndex(i)}
							>
								{item}
							</div>
						))}
					</div>
				)}
			</div>
		</label>
	);
}
