// ── Generated types (from Rust via ts-rs) ───────────────────────────────────
// Regenerate: TS_RS_EXPORT_DIR="app/src/generated" cargo test -p poe-eval --features ts export_bindings
//             (also poe-item and poe-data with same pattern)

// Display types (overlay rendering)
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

// Profile/rule types (profile editor)
export type { Profile as EvalProfile } from "./generated/Profile";
export type { ScoringRule } from "./generated/ScoringRule";
export type { Rule } from "./generated/Rule";
export type { Predicate } from "./generated/Predicate";

// Schema types (dynamic profile editor UI)
export type { PredicateSchema } from "./generated/PredicateSchema";
export type { PredicateField } from "./generated/PredicateField";
export type { FieldKind } from "./generated/FieldKind";
export type { EnumOption } from "./generated/EnumOption";
export type { Cmp } from "./generated/Cmp";

// ── Type guards ─────────────────────────────────────────────────────────────
// These belong to poe-eval conceptually (they narrow poe-eval's Rule union),
// but ts-rs can only generate type declarations, not runtime functions.
// They're the TypeScript equivalent of `if let Rule::Pred(p) = rule`.

import type { Predicate } from "./generated/Predicate";
import type { Rule } from "./generated/Rule";

export function isCompoundRule(rule: Rule): rule is { rule_type: "All" | "Any"; rules: Rule[] } {
	return rule.rule_type === "All" || rule.rule_type === "Any";
}

export function isPredRule(rule: Rule): rule is { rule_type: "Pred" } & Predicate {
	return rule.rule_type === "Pred";
}
