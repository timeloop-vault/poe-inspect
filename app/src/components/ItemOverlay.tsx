import { useState } from "preact/hooks";
import type {
	ItemEvaluation,
	ModTierResult,
	ModType,
	Rarity,
	ResolvedItem,
	ResolvedMod,
	ScoreInfo,
	TierQuality,
	WatchingScore,
} from "../types";

// Tooltip header sprites (left, middle, right) per rarity/item type
import headerCurrencyLeft from "../assets/tooltip/header-currency-left.webp";
import headerCurrencyMiddle from "../assets/tooltip/header-currency-middle.webp";
import headerCurrencyRight from "../assets/tooltip/header-currency-right.webp";
import headerGemLeft from "../assets/tooltip/header-gem-left.webp";
import headerGemMiddle from "../assets/tooltip/header-gem-middle.webp";
import headerGemRight from "../assets/tooltip/header-gem-right.webp";
import headerMagicLeft from "../assets/tooltip/header-magic-left.webp";
import headerMagicMiddle from "../assets/tooltip/header-magic-middle.webp";
import headerMagicRight from "../assets/tooltip/header-magic-right.webp";
import headerNormalLeft from "../assets/tooltip/header-normal-left.webp";
import headerNormalMiddle from "../assets/tooltip/header-normal-middle.webp";
import headerNormalRight from "../assets/tooltip/header-normal-right.webp";
import headerRareLeft from "../assets/tooltip/header-rare-left.webp";
import headerRareMiddle from "../assets/tooltip/header-rare-middle.webp";
import headerRareRight from "../assets/tooltip/header-rare-right.webp";
import headerUniqueLeft from "../assets/tooltip/header-unique-left.webp";
import headerUniqueMiddle from "../assets/tooltip/header-unique-middle.webp";
import headerUniqueRight from "../assets/tooltip/header-unique-right.webp";

// Tooltip separator sprites per rarity/item type
import separatorCurrency from "../assets/tooltip/separator-currency.webp";
import separatorGem from "../assets/tooltip/separator-gem.webp";
import separatorMagic from "../assets/tooltip/separator-magic.webp";
import separatorNormal from "../assets/tooltip/separator-normal.webp";
import separatorRare from "../assets/tooltip/separator-rare.webp";
import separatorUnique from "../assets/tooltip/separator-unique.webp";

type HeaderSprites = { left: string; middle: string; right: string };

const headerSprites: Partial<Record<Rarity, HeaderSprites>> = {
	Normal: { left: headerNormalLeft, middle: headerNormalMiddle, right: headerNormalRight },
	Magic: { left: headerMagicLeft, middle: headerMagicMiddle, right: headerMagicRight },
	Rare: { left: headerRareLeft, middle: headerRareMiddle, right: headerRareRight },
	Unique: { left: headerUniqueLeft, middle: headerUniqueMiddle, right: headerUniqueRight },
	Currency: { left: headerCurrencyLeft, middle: headerCurrencyMiddle, right: headerCurrencyRight },
	Gem: { left: headerGemLeft, middle: headerGemMiddle, right: headerGemRight },
};
// biome-ignore lint/style/noNonNullAssertion: Normal is always present in the literal above
const defaultHeader: HeaderSprites = headerSprites.Normal!;

const separatorSprites: Partial<Record<Rarity, string>> = {
	Normal: separatorNormal,
	Magic: separatorMagic,
	Rare: separatorRare,
	Unique: separatorUnique,
	Currency: separatorCurrency,
	Gem: separatorGem,
};
const defaultSeparator = separatorNormal;

/** Color for item name based on rarity */
function rarityColor(rarity: Rarity): string {
	switch (rarity) {
		case "Normal":
			return "var(--rarity-normal)";
		case "Magic":
			return "var(--rarity-magic)";
		case "Rare":
			return "var(--rarity-rare)";
		case "Unique":
			return "var(--rarity-unique)";
		case "Gem":
			return "var(--rarity-gem, #1ba29b)";
		case "Currency":
			return "var(--rarity-currency, #aa9e82)";
		default:
			return "var(--rarity-normal)";
	}
}

// ── Mod display helpers ─────────────────────────────────────────────────────

/** Build display text from stat lines (filter out reminder text). */
function modText(mod: ResolvedMod): string {
	return mod.statLines
		.filter((sl) => !sl.isReminder)
		.map((sl) => sl.displayText)
		.join("\n");
}

/** Collect all stat_ids from a mod's non-reminder stat lines. */
function modStatIds(mod: ResolvedMod): string[] {
	return mod.statLines
		.filter((sl) => !sl.isReminder && sl.statIds)
		.flatMap((sl) => sl.statIds ?? []);
}

/** CSS class for mod quality coloring. */
function qualityClass(quality: TierQuality | null | undefined): string {
	switch (quality) {
		case "best":
			return "quality-best";
		case "great":
		case "good":
			return "quality-good";
		case "mid":
			return "quality-mid";
		case "low":
			return "quality-low";
		default:
			return "quality-none";
	}
}

/** Badge label: "T1" for tiers, "R1" for ranks */
function tierBadgeLabel(tier: ModTierResult): string {
	if (tier.tier == null) return "";
	const prefix = tier.tierKind === "rank" ? "R" : "T";
	return `${prefix}${tier.tier}`;
}

/** Calculate roll quality as 0-100 percentage from the first stat line's value range. */
function rollQuality(mod: ResolvedMod): number | null {
	const vr = mod.statLines.find((sl) => !sl.isReminder && sl.values.length > 0)?.values[0];
	if (!vr) return null;
	const range = vr.max - vr.min;
	if (range === 0) return 100;
	return Math.round(((vr.current - vr.min) / range) * 100);
}

/** Short label for mod type */
function modTypeLabel(displayType: ModType): string | null {
	switch (displayType) {
		case "prefix":
			return "P";
		case "suffix":
			return "S";
		case "crafted":
			return "C";
		default:
			return null;
	}
}

/** Display name for influence kind */
function influenceDisplay(kind: string): string {
	switch (kind) {
		case "SearingExarch":
			return "Searing Exarch";
		case "EaterOfWorlds":
			return "Eater of Worlds";
		case "RelicUnique":
			return "Relic";
		default:
			return kind;
	}
}

function Separator({ rarity }: { rarity: Rarity }) {
	return (
		<div class="item-separator">
			<img src={separatorSprites[rarity] ?? defaultSeparator} alt="" class="separator-img" />
		</div>
	);
}

/** PoE-style item header with left cap, tiling middle, right cap */
function ItemHeader({
	rarity,
	name,
	baseType,
	doubleLine,
}: {
	rarity: Rarity;
	name: string;
	baseType: string;
	doubleLine: boolean;
}) {
	const sprites = headerSprites[rarity] ?? defaultHeader;
	return (
		<div class={`item-header ${doubleLine ? "header-double" : "header-single"}`}>
			<div class="header-bg">
				<img src={sprites.left} alt="" class="header-left" />
				<div class="header-middle" style={{ backgroundImage: `url(${sprites.middle})` }} />
				<img src={sprites.right} alt="" class="header-right" />
			</div>
			<div class="header-text" style={{ color: rarityColor(rarity) }}>
				{doubleLine ? (
					<>
						<div class="item-name">{name}</div>
						<div class="item-base">{baseType}</div>
					</>
				) : (
					<div class="item-name">
						{name}
						{baseType !== name ? ` ${baseType}` : ""}
					</div>
				)}
			</div>
		</div>
	);
}

export interface DisplaySettings {
	showRollBars: boolean;
	showTierBadges: boolean;
	showTypeBadges: boolean;
	showOpenAffixes: boolean;
	showStatIds: boolean;
}

export const defaultDisplay: DisplaySettings = {
	showRollBars: true,
	showTierBadges: true,
	showTypeBadges: true,
	showOpenAffixes: true,
	showStatIds: false,
};

function ModLine({
	mod,
	tier,
	display,
}: { mod: ResolvedMod; tier: ModTierResult; display: DisplaySettings }) {
	const quality = rollQuality(mod);
	const typeLabel = modTypeLabel(mod.displayType);
	const qualityCls = mod.displayType === "unique" ? "quality-unique" : qualityClass(tier.quality);
	const isCrafted = mod.header.source === "masterCrafted";
	const statIds = modStatIds(mod);
	const statIdTitle = statIds.length > 0 ? statIds.join(", ") : undefined;

	return (
		<div class={`mod-line ${qualityCls}`} title={statIdTitle}>
			<div class="mod-badges">
				{display.showTierBadges && tier.tier != null && (
					<span class={`tier-badge ${qualityCls}`}>{tierBadgeLabel(tier)}</span>
				)}
				{display.showTypeBadges && typeLabel !== null && (
					<span class={`type-badge type-${mod.displayType}`}>{typeLabel}</span>
				)}
			</div>
			<div class="mod-content">
				{modText(mod)
					.split("\n")
					.map((line) => (
						<div key={line}>{line}</div>
					))}
				{isCrafted && <span class="crafted-tag">(crafted)</span>}
				{mod.isFractured && <span class="fractured-tag">(fractured)</span>}
				{display.showStatIds && statIds.length > 0 && (
					<div class="stat-id-line">{statIds.join(", ")}</div>
				)}
			</div>
			{display.showRollBars && quality !== null && (
				<div class="roll-quality" title={`Roll: ${quality}%`}>
					<div class="roll-bar">
						<div
							class={`roll-fill ${quality >= 80 ? "roll-high" : quality >= 50 ? "roll-mid" : "roll-low"}`}
							style={{ width: `${quality}%` }}
						/>
					</div>
					<span class="roll-pct">{quality}%</span>
				</div>
			)}
		</div>
	);
}

// ── Default empty evaluation for mock data / no-eval mode ───────────────────

const emptyEval: ItemEvaluation = {
	modTiers: [],
	affixSummary: {
		openPrefixes: 0,
		openSuffixes: 0,
		maxPrefixes: 0,
		maxSuffixes: 0,
		modifiable: false,
	},
	score: null,
	watchingScores: [],
};

const emptyTier: ModTierResult = { tier: null, tierKind: null, quality: null };

export function ItemOverlay({
	item,
	eval: evaluation = emptyEval,
	display = defaultDisplay,
}: { item: ResolvedItem; eval?: ItemEvaluation; display?: DisplaySettings }) {
	const rarity = item.header.rarity;
	const name = item.header.name ?? item.header.baseType;
	const baseType = item.header.baseType;
	const doubleLine = rarity === "Rare" || rarity === "Unique";
	const [swapView, setSwapView] = useState<WatchingScore | null>(null);

	// Split mod tiers to match enchants / implicits / explicits
	const nEnchants = item.enchants.length;
	const nImplicits = item.implicits.length;
	const implicitTiers = evaluation.modTiers.slice(nEnchants, nEnchants + nImplicits);
	const explicitTiers = evaluation.modTiers.slice(nEnchants + nImplicits);

	const affix = evaluation.affixSummary;

	// The active score to display (swapped watching profile or primary)
	const activeScore = swapView?.score ?? evaluation.score;
	const watchingScores = evaluation.watchingScores ?? [];

	// Influence display names (filter out Fractured — it's a separate flag)
	const influences = item.influences
		.filter((i) => i !== "Fractured")
		.map((i) => influenceDisplay(i));

	return (
		<div class="item-card">
			{/* Swap view header — shown when viewing a watching profile */}
			{swapView && (
				<div class="swap-view-header" style={{ borderColor: swapView.color }}>
					<span>
						Viewing: <strong style={{ color: swapView.color }}>{swapView.profileName}</strong>
					</span>
					<button type="button" class="swap-view-back" onClick={() => setSwapView(null)}>
						&times;
					</button>
				</div>
			)}

			{/* Header with PoE art */}
			<ItemHeader rarity={rarity} name={name} baseType={baseType} doubleLine={doubleLine} />

			<Separator rarity={rarity} />

			{/* Properties */}
			{item.properties.length > 0 && (
				<>
					<div class="item-properties">
						{item.properties.map((prop) => (
							<div key={prop.name} class={`property-line ${prop.augmented ? "augmented" : ""}`}>
								<span class="prop-label">{prop.name}: </span>
								<span class="prop-value">{prop.value}</span>
							</div>
						))}
					</div>
					<Separator rarity={rarity} />
				</>
			)}

			{/* Requirements */}
			{item.requirements.length > 0 && (
				<>
					<div class="item-requirements">
						<span class="req-label">Requires </span>
						{item.requirements.map((req, i) => (
							<span key={req.name}>
								{i > 0 && ", "}
								<span class="req-name">{req.name}</span> {req.value}
							</span>
						))}
					</div>
					<Separator rarity={rarity} />
				</>
			)}

			{/* Sockets & Item Level */}
			{(item.sockets || item.itemLevel != null) && (
				<>
					<div class="item-meta">
						{item.sockets && <div class="item-sockets">Sockets: {item.sockets}</div>}
						{item.itemLevel != null && <div class="item-level">Item Level: {item.itemLevel}</div>}
					</div>
					<Separator rarity={rarity} />
				</>
			)}

			{/* Description (currency effects, item instructions, etc.) */}
			{item.description && <div class="item-description">{item.description}</div>}

			{/* Enchants */}
			{item.enchants.length > 0 && (
				<>
					<div class="mod-section enchant-section">
						{item.enchants.map((mod) => (
							<div key={modText(mod)} class="mod-line enchant-line">
								<div class="mod-content">{modText(mod)}</div>
							</div>
						))}
					</div>
					<Separator rarity={rarity} />
				</>
			)}

			{/* Implicits */}
			{item.implicits.length > 0 && (
				<>
					<div class="mod-section implicit-section">
						{item.implicits.map((mod, i) => (
							<ModLine
								key={modText(mod)}
								mod={mod}
								tier={implicitTiers[i] ?? emptyTier}
								display={display}
							/>
						))}
					</div>
					<Separator rarity={rarity} />
				</>
			)}

			{/* Explicit mods */}
			{item.explicits.length > 0 && (
				<div class="mod-section explicit-section">
					{item.explicits.map((mod, i) => (
						<ModLine
							key={modText(mod)}
							mod={mod}
							tier={explicitTiers[i] ?? emptyTier}
							display={display}
						/>
					))}
				</div>
			)}

			{/* Open affixes */}
			{display.showOpenAffixes && (affix.openPrefixes > 0 || affix.openSuffixes > 0) && (
				<>
					<Separator rarity={rarity} />
					<div class="open-affixes">
						{affix.openPrefixes > 0 && (
							<span class="open-prefix">
								{affix.openPrefixes} open prefix
								{affix.openPrefixes > 1 ? "es" : ""}
							</span>
						)}
						{affix.openPrefixes > 0 && affix.openSuffixes > 0 && " · "}
						{affix.openSuffixes > 0 && (
							<span class="open-suffix">
								{affix.openSuffixes} open suffix
								{affix.openSuffixes > 1 ? "es" : ""}
							</span>
						)}
					</div>
				</>
			)}

			{/* Affix count summary */}
			{affix.maxPrefixes > 0 && (
				<div class="affix-summary">
					Prefixes: {affix.maxPrefixes - affix.openPrefixes}/{affix.maxPrefixes} · Suffixes:{" "}
					{affix.maxSuffixes - affix.openSuffixes}/{affix.maxSuffixes}
				</div>
			)}

			{/* Influence tags */}
			{influences.length > 0 && (
				<>
					<Separator rarity={rarity} />
					<div class="influence-tags">
						{influences.map((inf) => (
							<span key={inf} class="influence-tag">
								{inf}
							</span>
						))}
					</div>
				</>
			)}

			{/* Flavor text (uniques) */}
			{item.flavorText && (
				<>
					<Separator rarity={rarity} />
					<div class="flavor-text">{item.flavorText}</div>
				</>
			)}

			{/* Profile score (primary or swapped watching) */}
			{activeScore?.applicable && (
				<>
					<Separator rarity={rarity} />
					<ScoreDisplay
						score={activeScore}
						{...(swapView ? { label: swapView.profileName, color: swapView.color } : {})}
					/>
				</>
			)}

			{/* Watching profile indicators */}
			{!swapView && watchingScores.length > 0 && (
				<div class="watching-indicators">
					{watchingScores.map((ws) => (
						<button
							key={ws.profileName}
							type="button"
							class="watching-pill"
							style={{
								borderColor: ws.color,
								color: ws.color,
							}}
							onClick={() => setSwapView(ws)}
							title={`Click to view ${ws.profileName} scoring`}
						>
							<span class="watching-dot" style={{ background: ws.color }} />
							{ws.profileName}: {Math.round(ws.score.percent)}%
						</button>
					))}
				</div>
			)}
		</div>
	);
}

function ScoreDisplay({
	score,
	label,
	color,
}: { score: ScoreInfo; label?: string; color?: string }) {
	const pct = Math.round(score.percent);
	const barClass = pct >= 70 ? "score-high" : pct >= 40 ? "score-mid" : "score-low";

	return (
		<div class="score-section">
			<div class="score-header">
				<span class="score-label" {...(color ? { style: { color } } : {})}>
					{label ?? "Score"}
				</span>
				<span class={`score-value ${barClass}`}>{pct}%</span>
			</div>
			<div class="score-bar">
				<div class={`score-fill ${barClass}`} style={{ width: `${pct}%` }} />
			</div>
			{score.matched.length > 0 && (
				<div class="score-details">
					{score.matched.map((r) => (
						<div key={r.label} class="score-rule matched">
							<span class="rule-check">+</span>
							<span class="rule-label">{r.label}</span>
						</div>
					))}
					{score.unmatched.map((r) => (
						<div key={r.label} class="score-rule unmatched">
							<span class="rule-check">-</span>
							<span class="rule-label">{r.label}</span>
						</div>
					))}
				</div>
			)}
		</div>
	);
}
