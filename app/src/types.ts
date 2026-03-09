/** Rarity levels for PoE items */
export type Rarity = "Normal" | "Magic" | "Rare" | "Unique";

/** Modifier source type */
export type ModType =
	| "prefix"
	| "suffix"
	| "implicit"
	| "enchant"
	| "unique"
	| "crafted"
	| "fractured";

/** A single property line (e.g., "Armour: 890") */
export interface ItemProperty {
	name: string;
	value: string;
	augmented: boolean;
}

/** A single requirement (e.g., "Level: 70") */
export interface Requirement {
	name: string;
	value: string;
}

/** Quality classification from poe-data (via poe-eval tier analysis) */
export type TierQuality = "best" | "great" | "good" | "mid" | "low";

/** Whether a mod number is a "tier" (regular) or "rank" (bench craft) */
export type TierKind = "tier" | "rank";

/** A modifier on an item with tier and roll info */
export interface Modifier {
	/** Display name from the mod header, e.g., "Merciless" */
	modName?: string;
	/** prefix, suffix, implicit, etc. */
	type: ModType;
	/** Raw tier/rank number. Undefined for implicits/uniques. */
	tier?: number;
	/** Whether this is "tier" (lower=better) or "rank" (higher=better). */
	tierKind?: TierKind;
	/** Quality level from poe-data classification. Use this for coloring. */
	quality?: TierQuality;
	/** Mod group tags, e.g., ["Damage", "Physical", "Attack"] */
	tags: string[];
	/** The stat text lines (what the player sees) */
	text: string;
	/** Current rolled value (if single-value mod) */
	value?: number;
	/** Min possible roll for this tier */
	min?: number;
	/** Max possible roll for this tier */
	max?: number;
	/** Is this a fractured mod? */
	fractured?: boolean;
	/** Is this a master-crafted mod? */
	crafted?: boolean;
}

/** Fully structured item for overlay display */
export interface ParsedItem {
	itemClass: string;
	rarity: Rarity;
	name: string;
	baseType: string;
	itemLevel: number;
	properties: ItemProperty[];
	requirements: Requirement[];
	sockets?: string;
	/** URL to item art from PoE CDN */
	iconUrl?: string;
	enchants: Modifier[];
	implicits: Modifier[];
	explicits: Modifier[];
	/** Influence types on the item */
	influences: string[];
	fractured?: boolean;
	corrupted?: boolean;
	/** Unique item flavor text */
	flavorText?: string;
	/** Number of open prefix slots */
	openPrefixes: number;
	/** Number of open suffix slots */
	openSuffixes: number;
	/** Maximum prefix slots for this item's rarity (from poe-data) */
	maxPrefixes: number;
	/** Maximum suffix slots for this item's rarity (from poe-data) */
	maxSuffixes: number;
	/** Profile scoring result (if a profile was active) */
	score?: ScoreInfo;
}

/** Result of scoring an item against a profile */
export interface ScoreInfo {
	total: number;
	maxPossible: number;
	percent: number;
	applicable: boolean;
	matched: RuleMatch[];
	unmatched: RuleMatch[];
}

export interface RuleMatch {
	label: string;
	weight: number;
}

// ── Eval Profile (matches poe-eval's serde format — snake_case keys) ──

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
