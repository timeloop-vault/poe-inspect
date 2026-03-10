// ── Generated types (from Rust via ts-rs) ───────────────────────────────────
// Regenerate: TS_RS_EXPORT_DIR="app/src/generated" cargo test -p poe-eval --features ts
//             (also poe-item and poe-data with same pattern)

// Item display types (from poe-item)
export type { Rarity } from "./generated/Rarity";
export type { ModType } from "./generated/ModType";
export type { TierKind } from "./generated/TierKind";
export type { ItemProperty } from "./generated/ItemProperty";
export type { Requirement } from "./generated/Requirement";
export type { ResolvedItem } from "./generated/ResolvedItem";
export type { ResolvedMod } from "./generated/ResolvedMod";
export type { ResolvedStatLine } from "./generated/ResolvedStatLine";
export type { ResolvedHeader } from "./generated/ResolvedHeader";
export type { ModHeader } from "./generated/ModHeader";
export type { ValueRange } from "./generated/ValueRange";
export type { InfluenceKind } from "./generated/InfluenceKind";
export type { StatusKind } from "./generated/StatusKind";

// Evaluation types (from poe-eval)
export type { TierQuality } from "./generated/TierQuality";
export type { ItemEvaluation } from "./generated/ItemEvaluation";
export type { ModTierResult } from "./generated/ModTierResult";
export type { AffixInfo } from "./generated/AffixInfo";
export type { WatchingScoreInfo as WatchingScore } from "./generated/WatchingScoreInfo";
export type { ScoreInfo } from "./generated/ScoreInfo";
export type { RuleMatch } from "./generated/RuleMatch";

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

// Stat suggestion types (from poe-data, hybrid mod picker)
export type { StatSuggestion } from "./generated/StatSuggestion";
export type { StatSuggestionKind } from "./generated/StatSuggestionKind";

// Trade types (from poe-trade)
export type { TradeQueryConfig } from "./generated/TradeQueryConfig";
export type { PriceCheckResult } from "./generated/PriceCheckResult";
export type { Price } from "./generated/Price";
export type { League } from "./generated/League";
export type { LeagueList } from "./generated/LeagueList";
export type { QueryBuildResult } from "./generated/QueryBuildResult";

// ── App-owned combined payload ──────────────────────────────────────────────
// Matches the Rust `ItemPayload` struct in app/src-tauri/src/lib.rs.

import type { ItemEvaluation } from "./generated/ItemEvaluation";
import type { ResolvedItem } from "./generated/ResolvedItem";

export interface ItemPayload {
	item: ResolvedItem;
	eval: ItemEvaluation;
	rawText: string;
}

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
