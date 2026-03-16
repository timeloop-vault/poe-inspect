/**
 * Query editor for RQE want lists.
 *
 * Reuses PredicateRow / CompoundGroupEditor from the profile editor
 * for the UI, but owns its own state and conversion logic.
 * The boundary: this component imports UI components, but the
 * poe-eval Rule → poe-rqe Condition conversion is entirely local.
 */

import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "preact/hooks";
import type { Cmp } from "../../generated/Cmp";
import type { PredicateSchema } from "../../generated/PredicateSchema";
import type { MarketplaceSettings as MarketplaceConfig } from "../../store";
import type { Rule, ScoringRule } from "../../types";
import { PredicateEditor, defaultRule, getSchema } from "./PredicateEditor";

// --- RQE Condition format (matches rqe-server wire format) ---

interface RqeCondition {
	key: string;
	value: unknown;
	type: "string" | "integer" | "boolean" | "list";
	typeOptions: { operator?: string; count?: number } | null;
}

// --- Comparison operator mapping (poe-eval Cmp → RQE operator) ---
// RQE uses Erlang semantics: rq_value <op> entry_value
// poe-eval uses natural semantics: entry_value <op> threshold
// So Ge (entry >= threshold) → "<=" (threshold <= entry)

const CMP_TO_RQE: Record<string, string | null> = {
	Eq: null, // null typeOptions = exact equality
	Ge: "<=", // threshold <= entry → entry >= threshold
	Gt: "<", // threshold < entry → entry > threshold
	Le: ">=", // threshold >= entry → entry <= threshold
	Lt: ">", // threshold > entry → entry < threshold
	Ne: null, // RQE doesn't have != directly
};

// --- Convert RQE Conditions → poe-eval Rules (for editing) ---

const RQE_OP_TO_CMP: Record<string, Cmp> = {
	"<=": "Ge", // threshold <= entry → entry >= threshold
	"<": "Gt",
	">=": "Le",
	">": "Lt",
};

function conditionsToRules(conditions: RqeCondition[]): { label: string; rule: Rule }[] {
	return conditions
		.map((c) => conditionToRule(c))
		.filter((r): r is { label: string; rule: Rule } => r !== null);
}

function conditionToRule(c: RqeCondition): { label: string; rule: Rule } | null {
	if (c.type === "string") {
		if (c.key === "item_class") {
			return {
				label: `Item Class: ${c.value}`,
				rule: { rule_type: "Pred", type: "ItemClass", op: "Eq", value: c.value } as Rule,
			};
		}
		if (c.key === "rarity_class") {
			const rarity = c.value === "Unique" ? "Unique" : "Rare";
			return {
				label: `Rarity: ${c.value}`,
				rule: { rule_type: "Pred", type: "Rarity", op: "Ge", value: rarity } as Rule,
			};
		}
		if (c.key === "base_type") {
			return {
				label: `Base Type: ${c.value}`,
				rule: { rule_type: "Pred", type: "BaseType", op: "Eq", value: c.value } as Rule,
			};
		}
		return null;
	}

	if (c.type === "boolean") {
		if (c.key === "corrupted") {
			return {
				label: "Corrupted",
				rule: { rule_type: "Pred", type: "HasStatus", status: "Corrupted" } as Rule,
			};
		}
		if (c.key === "fractured") {
			return {
				label: "Fractured",
				rule: { rule_type: "Pred", type: "HasInfluence", influence: "Fractured" } as Rule,
			};
		}
		return null;
	}

	if (c.type === "integer") {
		const op: Cmp = c.typeOptions?.operator
			? (RQE_OP_TO_CMP[c.typeOptions.operator] ?? "Eq")
			: "Eq";
		const value = c.value as number;

		if (c.key === "item_level") {
			return {
				label: `Item Level ${op} ${value}`,
				rule: { rule_type: "Pred", type: "ItemLevel", op, value } as Rule,
			};
		}
		if (c.key === "socket_count") {
			return {
				label: `Sockets ${op} ${value}`,
				rule: { rule_type: "Pred", type: "SocketCount", op, value } as Rule,
			};
		}
		if (c.key === "max_link") {
			return {
				label: `Links ${op} ${value}`,
				rule: { rule_type: "Pred", type: "LinkCount", op, value } as Rule,
			};
		}
		// Stat value: "explicit.stat_id" → StatValue predicate
		const statMatch = c.key.match(/^(explicit|implicit|crafted|enchant)\.(.+)$/);
		if (statMatch?.[2]) {
			const statId = statMatch[2];
			return {
				label: statId,
				rule: {
					rule_type: "Pred",
					type: "StatValue",
					conditions: [{ stat_ids: [statId], value_index: 0, op, value }],
				} as Rule,
			};
		}
		return null;
	}

	// List conditions — skip for now (compound groups are complex to reverse)
	return null;
}

// --- Convert poe-eval Rules → RQE Conditions ---

function ruleToConditions(rule: Rule): RqeCondition[] {
	if (rule.rule_type === "Pred") {
		return predToConditions(rule as Rule & { rule_type: "Pred" });
	}
	if (rule.rule_type === "All") {
		const inner = (rule as { rules: Rule[] }).rules.flatMap(ruleToConditions);
		if (inner.length <= 1) return inner;
		return [
			{
				key: "list",
				value: inner,
				type: "list",
				typeOptions: { operator: "and" },
			},
		];
	}
	if (rule.rule_type === "Any") {
		const inner = (rule as { rules: Rule[] }).rules.flatMap(ruleToConditions);
		return [
			{
				key: "list",
				value: inner,
				type: "list",
				typeOptions: { operator: "or" },
			},
		];
	}
	if (rule.rule_type === "Not") {
		const inner = ruleToConditions((rule as { rule: Rule }).rule);
		return [
			{
				key: "list",
				value: inner,
				type: "list",
				typeOptions: { operator: "not" },
			},
		];
	}
	return [];
}

function predToConditions(pred: Record<string, unknown>): RqeCondition[] {
	const type = pred.type as string;

	switch (type) {
		case "ItemClass":
			return [
				{
					key: "item_class",
					value: pred.value as string,
					type: "string",
					typeOptions: null,
				},
			];

		case "Rarity": {
			// Map rarity to rarity_class (Non-Unique / Unique)
			const rarity = pred.value as string;
			const isUnique = rarity === "Unique";
			return [
				{
					key: "rarity_class",
					value: isUnique ? "Unique" : "Non-Unique",
					type: "string",
					typeOptions: null,
				},
			];
		}

		case "BaseType":
			return [
				{
					key: "base_type",
					value: pred.value as string,
					type: "string",
					typeOptions: null,
				},
			];

		case "ItemLevel":
			return [intCondition("item_level", pred.op as Cmp, pred.value as number)];

		case "HasStatus": {
			const status = pred.status as string;
			const key =
				status === "Corrupted"
					? "corrupted"
					: status === "Fractured"
						? "fractured"
						: status.toLowerCase();
			return [{ key, value: true, type: "boolean", typeOptions: null }];
		}

		case "SocketCount":
			return [intCondition("socket_count", pred.op as Cmp, pred.value as number)];

		case "LinkCount":
			return [intCondition("max_link", pred.op as Cmp, pred.value as number)];

		case "StatValue": {
			const conditions = pred.conditions as Array<{
				stat_ids?: string[];
				text?: string;
				value_index: number;
				op: Cmp;
				value: number;
			}>;
			return conditions.flatMap((c) => {
				const statId = c.stat_ids?.[0];
				if (!statId) return [];
				// Use "explicit." prefix for stat_ids (matching item_to_entry convention)
				return [intCondition(`explicit.${statId}`, c.op, c.value)];
			});
		}

		case "HasStatId": {
			const statId = pred.stat_id as string;
			// Presence check: use wildcard
			return [
				{
					key: `explicit.${statId}`,
					value: "_",
					type: "string",
					typeOptions: null,
				},
			];
		}

		case "InfluenceCount":
			return [intCondition("influence_count", pred.op as Cmp, pred.value as number)];

		case "Quality":
			return [intCondition("quality", pred.op as Cmp, pred.value as number)];

		default:
			// Unsupported predicate type — skip
			return [];
	}
}

function intCondition(key: string, op: Cmp, value: number): RqeCondition {
	const rqeOp = CMP_TO_RQE[op];
	return {
		key,
		value,
		type: "integer",
		typeOptions: rqeOp ? { operator: rqeOp } : null,
	};
}

// --- Component ---

export function QueryEditor({
	settings,
	editingQuery,
	onSave,
	onCancel,
}: {
	settings: MarketplaceConfig;
	editingQuery?: { id: number; labels: string[]; conditions: unknown[] } | null;
	onSave: () => void;
	onCancel: () => void;
}) {
	const [schema, setSchema] = useState<PredicateSchema[]>([]);
	const [name, setName] = useState("");
	const [label, setLabel] = useState("");
	const [rules, setRules] = useState<(ScoringRule & { _key: number })[]>([]);
	const [nextKey, setNextKey] = useState(0);
	const [saving, setSaving] = useState(false);
	const [error, setError] = useState<string | null>(null);

	useEffect(() => {
		getSchema().then(setSchema);
	}, []);

	// Initialize from editing query if provided — reverse-convert conditions to rules,
	// then resolve stat_ids to human-readable template text.
	useEffect(() => {
		if (!editingQuery || schema.length === 0) return;

		setName(editingQuery.labels[0] ?? "");
		setLabel(editingQuery.labels[1] ?? "");

		const restored = conditionsToRules(editingQuery.conditions as RqeCondition[]);

		// Collect all stat_ids that need template resolution
		const statIds: string[] = [];
		for (const r of restored) {
			const pred = r.rule as Record<string, unknown>;
			if (pred.type === "StatValue") {
				const conditions = pred.conditions as Array<{ stat_ids?: string[] }>;
				for (const c of conditions) {
					if (c.stat_ids) statIds.push(...c.stat_ids);
				}
			}
		}

		// Resolve stat_ids to templates, then update the rules
		const applyRules = (templateMap: Record<string, string>) => {
			let keyCounter = 0;
			const withText = restored.map((r) => {
				const pred = r.rule as Record<string, unknown>;
				if (pred.type === "StatValue") {
					const conditions = (pred.conditions as Array<Record<string, unknown>>).map((c) => {
						const ids = c.stat_ids as string[] | undefined;
						const template = ids?.[0] ? templateMap[ids[0]] : undefined;
						return { ...c, text: template ?? ids?.[0] ?? null };
					});
					return {
						_key: keyCounter++,
						label: r.label,
						weight: 0,
						rule: { ...pred, conditions } as Rule,
					};
				}
				return { _key: keyCounter++, label: r.label, weight: 0, rule: r.rule };
			});
			setRules(withText);
			setNextKey(keyCounter);
		};

		if (statIds.length > 0) {
			invoke<Record<string, string>>("resolve_stat_templates", {
				statIds,
			})
				.then(applyRules)
				.catch(() => applyRules({}));
		} else {
			applyRules({});
		}
	}, [editingQuery, schema]);

	const addCondition = useCallback(() => {
		const firstSchema = schema[0];
		if (!firstSchema) return;
		const key = nextKey;
		setNextKey((k) => k + 1);
		setRules((prev) => [
			...prev,
			{ _key: key, label: firstSchema.label, weight: 0, rule: defaultRule(firstSchema) },
		]);
	}, [schema, nextKey]);

	const handleSave = useCallback(async () => {
		if (rules.length === 0) {
			setError("Add at least one condition");
			return;
		}

		setError(null);
		setSaving(true);

		try {
			// Convert all rules to RQE conditions
			const conditions: RqeCondition[] = rules.flatMap((r) => ruleToConditions(r.rule));

			if (conditions.length === 0) {
				setError("No valid conditions generated. Check your rules.");
				setSaving(false);
				return;
			}

			const labels = [name || "Unnamed Want List", ...(label ? [label] : [])];

			const headers: Record<string, string> = {
				"Content-Type": "application/json",
			};
			if (settings.apiKey) {
				headers["X-API-Key"] = settings.apiKey;
			}

			// Edit = delete old + create new
			if (editingQuery) {
				await fetch(`${settings.serverUrl}/queries/${editingQuery.id}`, {
					method: "DELETE",
					headers,
				});
			}

			const resp = await fetch(`${settings.serverUrl}/queries`, {
				method: "POST",
				headers,
				body: JSON.stringify({
					conditions,
					labels,
					owner: settings.accountName,
				}),
			});

			if (!resp.ok) {
				const text = await resp.text();
				setError(`Server error: ${text}`);
				return;
			}

			onSave();
		} catch {
			setError("Failed to save. Is the server running?");
		} finally {
			setSaving(false);
		}
	}, [rules, name, label, settings, editingQuery, onSave]);

	if (schema.length === 0) {
		return <p class="setting-description">Loading predicate schema...</p>;
	}

	return (
		<div class="setting-group">
			<h3>{editingQuery ? "Edit Want List" : "New Want List"}</h3>

			<label class="setting-row">
				<span class="setting-label">Name</span>
				<input
					type="text"
					class="setting-select"
					placeholder="e.g., Fast Cold Res Boots"
					value={name}
					onInput={(e) => setName((e.target as HTMLInputElement).value)}
					style={{ width: 280 }}
				/>
			</label>

			<label class="setting-row">
				<span class="setting-label">Label (optional)</span>
				<input
					type="text"
					class="setting-select"
					placeholder="e.g., build:cyclone"
					value={label}
					onInput={(e) => setLabel((e.target as HTMLInputElement).value)}
					style={{ width: 280 }}
				/>
			</label>

			<div style={{ marginTop: 12 }}>
				<div
					style={{
						display: "flex",
						justifyContent: "space-between",
						alignItems: "center",
						marginBottom: 8,
					}}
				>
					<h4 style={{ margin: 0, color: "var(--poe-header)" }}>Conditions ({rules.length})</h4>
					<button type="button" class="btn btn-small btn-primary" onClick={addCondition}>
						+ Condition
					</button>
				</div>

				{rules.length === 0 && (
					<p class="setting-description">
						No conditions yet. Click "+ Condition" to start building your want list.
					</p>
				)}

				{rules.map((rule, i) => (
					<QueryConditionRow
						key={rule._key}
						rule={rule}
						schema={schema}
						onChange={(updated) => {
							const next = [...rules];
							next[i] = { ...updated, _key: rule._key };
							setRules(next);
						}}
						onDelete={() => setRules(rules.filter((r) => r._key !== rule._key))}
					/>
				))}
			</div>

			{error && <p style={{ color: "#e04040", marginTop: 8 }}>{error}</p>}

			<div style={{ display: "flex", gap: 8, marginTop: 16 }}>
				<button type="button" class="btn" onClick={onCancel}>
					Cancel
				</button>
				<button type="button" class="btn btn-primary" onClick={handleSave} disabled={saving}>
					{saving ? "Saving..." : "Save Want List"}
				</button>
			</div>
		</div>
	);
}

// --- Simplified condition row (no weight, no drag) ---

function QueryConditionRow({
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
	const predType =
		rule.rule.rule_type === "Pred" ? (rule.rule as Record<string, unknown>).type : null;
	const predSchema = predType ? schema.find((s) => s.typeName === predType) : null;

	const categories = groupByCategory(schema);

	const handleTypeChange = (typeName: string) => {
		const newSchema = schema.find((s) => s.typeName === typeName);
		if (!newSchema) return;
		onChange({ ...rule, label: newSchema.label, rule: defaultRule(newSchema) });
	};

	// PredicateEditor imported at top of file via static import

	return (
		<div
			style={{
				padding: "8px 0",
				borderBottom: "1px solid var(--poe-border)",
				display: "flex",
				gap: 8,
				alignItems: "flex-start",
			}}
		>
			<div style={{ flex: 1 }}>
				<select
					class="pred-type-select"
					value={(predType as string) ?? ""}
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
					<div style={{ marginTop: 4 }}>
						<PredicateEditor
							rule={rule.rule}
							schema={predSchema}
							onChange={(newRule: Rule) => onChange({ ...rule, rule: newRule })}
							compact
						/>
					</div>
				)}
			</div>
			<button
				type="button"
				class="btn btn-small btn-danger"
				onClick={onDelete}
				title="Remove condition"
			>
				&times;
			</button>
		</div>
	);
}

function groupByCategory(schema: PredicateSchema[]): Record<string, PredicateSchema[]> {
	const categories: Record<string, PredicateSchema[]> = {};
	for (const s of schema) {
		const cat = s.category || "Other";
		if (!categories[cat]) categories[cat] = [];
		categories[cat].push(s);
	}
	return categories;
}
