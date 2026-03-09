// ── Generated types (from Rust via ts-rs) ───────────────────────────────────
// Run `cargo test` in app/src-tauri to regenerate these.
export type { Rarity } from "./generated/Rarity";
export type { ModType } from "./generated/ModType";
export type { TierKind } from "./generated/TierKind";
export type { TierQuality } from "./generated/TierQuality";
export type { Modifier } from "./generated/Modifier";
export type { EvaluatedItem as ParsedItem } from "./generated/EvaluatedItem";
export type { WatchingScoreInfo as WatchingScore } from "./generated/WatchingScoreInfo";
export type { ScoreInfo } from "./generated/ScoreInfo";
export type { RuleMatch } from "./generated/RuleMatch";
export type { ItemProperty } from "./generated/ItemProperty";
export type { Requirement } from "./generated/Requirement";

// ── Manual types (poe-eval profile/rule types — not yet generated) ──────────

/** poe-eval's Profile format. Opaque to the app — built via schema UI. */
export interface EvalProfile {
	name: string;
	description: string;
	filter: Rule | null;
	scoring: ScoringRule[];
}

export interface ScoringRule {
	label: string;
	weight: number;
	rule: Rule;
}

/** Rule tree — matches poe-eval's serde format (rule_type tag). */
export type Rule =
	| ({ rule_type: "Pred"; type: string } & Record<string, unknown>)
	| { rule_type: "All"; rules: Rule[] }
	| { rule_type: "Any"; rules: Rule[] }
	| { rule_type: "Not"; rule: Rule };

export function isCompoundRule(rule: Rule): rule is { rule_type: "All" | "Any"; rules: Rule[] } {
	return rule.rule_type === "All" || rule.rule_type === "Any";
}

export function isPredRule(
	rule: Rule,
): rule is { rule_type: "Pred"; type: string } & Record<string, unknown> {
	return rule.rule_type === "Pred";
}

// ── Predicate Schema (from poe-eval, drives dynamic profile editor) ──

/** Describes one predicate type for the profile editor UI. */
export interface PredicateSchema {
	typeName: string;
	label: string;
	description: string;
	category: string;
	fields: PredicateField[];
}

/** A single input field within a predicate. */
export interface PredicateField {
	name: string;
	label: string;
	kind: FieldKind;
}

/** Widget type for a predicate field. Discriminated union on `type`. */
export type FieldKind =
	| { type: "comparison"; allowedOps: string[] }
	| { type: "number"; min: number | null; max: number | null }
	| { type: "enum"; options: EnumOption[] }
	| { type: "orderedEnum"; options: EnumOption[] }
	| { type: "text"; suggestionsFrom: string | null }
	| { type: "slot"; options: EnumOption[] };

/** A selectable option in an enum field. */
export interface EnumOption {
	value: string;
	label: string;
}
