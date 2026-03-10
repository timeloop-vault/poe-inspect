import { invoke } from "@tauri-apps/api/core";
/**
 * Schema-driven predicate editor.
 *
 * Renders input fields for a predicate based on the schema from poe-eval.
 * Each FieldKind maps to a specific widget. New predicates that use
 * existing field kinds get UI automatically.
 */
import { useEffect, useRef, useState } from "preact/hooks";
import type { FieldKind, PredicateField, PredicateSchema, Rule } from "../../types";

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
			return kind.options[0]?.value ?? "Prefix";
	}
}

/** Build a default Rule from a predicate schema. */
export function defaultRule(schema: PredicateSchema): Rule {
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

	const updateField = (name: string, value: unknown) => {
		const updated = { ...rule, [name]: value } as Rule;
		// Auto-resolve stat_id when user picks a stat template
		if (
			name === "text" &&
			typeof value === "string" &&
			(schema.typeName === "StatValue" || schema.typeName === "RollPercent")
		) {
			invoke<string[]>("resolve_stat_template", { template: value }).then((ids) => {
				if (ids.length > 0) {
					onChange({ ...updated, stat_id: ids[0] } as Rule);
				} else {
					onChange(updated);
				}
			});
			return;
		}
		onChange(updated);
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
				<EnumField
					label={field.label}
					options={field.kind.options}
					value={String(value ?? "Prefix")}
					onChange={onChange}
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
			<span class="pred-field-label">{label}</span>
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
			<span class="pred-field-label">{label}</span>
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
			<span class="pred-field-label">{label}</span>
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
