import { useState } from "preact/hooks";
import type { Modifier, ParsedItem, Rarity, ScoreInfo, WatchingScore } from "../types";

import headerMagicLeft from "../assets/tooltip/header-magic-left.webp";
import headerMagicMiddle from "../assets/tooltip/header-magic-middle.webp";
import headerMagicRight from "../assets/tooltip/header-magic-right.webp";
import headerNormalLeft from "../assets/tooltip/header-normal-left.webp";
import headerNormalMiddle from "../assets/tooltip/header-normal-middle.webp";
import headerNormalRight from "../assets/tooltip/header-normal-right.webp";
// Tooltip header sprites (left, middle, right) per rarity
import headerRareLeft from "../assets/tooltip/header-rare-left.webp";
import headerRareMiddle from "../assets/tooltip/header-rare-middle.webp";
import headerRareRight from "../assets/tooltip/header-rare-right.webp";
import headerUniqueLeft from "../assets/tooltip/header-unique-left.webp";
import headerUniqueMiddle from "../assets/tooltip/header-unique-middle.webp";
import headerUniqueRight from "../assets/tooltip/header-unique-right.webp";

import separatorMagic from "../assets/tooltip/separator-magic.webp";
import separatorNormal from "../assets/tooltip/separator-normal.webp";
// Tooltip separator sprites per rarity
import separatorRare from "../assets/tooltip/separator-rare.webp";
import separatorUnique from "../assets/tooltip/separator-unique.webp";

type HeaderSprites = { left: string; middle: string; right: string };

const headerSprites: Record<Rarity, HeaderSprites> = {
	Normal: { left: headerNormalLeft, middle: headerNormalMiddle, right: headerNormalRight },
	Magic: { left: headerMagicLeft, middle: headerMagicMiddle, right: headerMagicRight },
	Rare: { left: headerRareLeft, middle: headerRareMiddle, right: headerRareRight },
	Unique: { left: headerUniqueLeft, middle: headerUniqueMiddle, right: headerUniqueRight },
	Gem: { left: headerNormalLeft, middle: headerNormalMiddle, right: headerNormalRight },
	Currency: { left: headerNormalLeft, middle: headerNormalMiddle, right: headerNormalRight },
	Unknown: { left: headerNormalLeft, middle: headerNormalMiddle, right: headerNormalRight },
};

const separatorSprites: Record<Rarity, string> = {
	Normal: separatorNormal,
	Magic: separatorMagic,
	Rare: separatorRare,
	Unique: separatorUnique,
	Gem: separatorNormal,
	Currency: separatorNormal,
	Unknown: separatorNormal,
};

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
		case "Unknown":
			return "var(--rarity-normal)";
	}
}

/** CSS class for tier coloring — driven by quality from poe-data, not raw tier number. */
function tierClass(mod: Modifier): string {
	switch (mod.quality) {
		case "best":
			return "tier-1";
		case "great":
			return "tier-2-3";
		case "good":
			return "tier-2-3";
		case "mid":
			return "tier-4-5";
		case "low":
			return "tier-low";
		default:
			return "tier-none";
	}
}

/** Badge label: "T1" for tiers, "R1" for ranks */
function tierBadgeLabel(mod: Modifier): string {
	if (mod.tier == null) return "";
	const prefix = mod.tierKind === "rank" ? "R" : "T";
	return `${prefix}${mod.tier}`;
}

/** Calculate roll quality as 0-100 percentage */
function rollQuality(mod: Modifier): number | null {
	if (mod.value == null || mod.min == null || mod.max == null) return null;
	const range = mod.max - mod.min;
	if (range === 0) return 100;
	return Math.round(((mod.value - mod.min) / range) * 100);
}

/** Short label for mod type */
function modTypeLabel(mod: Modifier): string | null {
	switch (mod.type) {
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

function Separator({ rarity }: { rarity: Rarity }) {
	return (
		<div class="item-separator">
			<img src={separatorSprites[rarity]} alt="" class="separator-img" />
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
	const sprites = headerSprites[rarity] ?? headerSprites.Normal;
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
}

export const defaultDisplay: DisplaySettings = {
	showRollBars: true,
	showTierBadges: true,
	showTypeBadges: true,
	showOpenAffixes: true,
};

function ModLine({ mod, display }: { mod: Modifier; display: DisplaySettings }) {
	const quality = rollQuality(mod);
	const typeLabel = modTypeLabel(mod);
	const tierCls = mod.type === "unique" ? "tier-unique" : tierClass(mod);

	return (
		<div class={`mod-line ${tierCls}`}>
			<div class="mod-badges">
				{display.showTierBadges && mod.tier != null && (
					<span class={`tier-badge ${tierCls}`}>{tierBadgeLabel(mod)}</span>
				)}
				{display.showTypeBadges && typeLabel !== null && (
					<span class={`type-badge type-${mod.type}`}>{typeLabel}</span>
				)}
			</div>
			<div class="mod-content">
				{mod.text.split("\n").map((line) => (
					<div key={line}>{line}</div>
				))}
				{mod.crafted && <span class="crafted-tag">(crafted)</span>}
				{mod.fractured && <span class="fractured-tag">(fractured)</span>}
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

export function ItemOverlay({
	item,
	display = defaultDisplay,
}: { item: ParsedItem; display?: DisplaySettings }) {
	const doubleLine = item.rarity === "Rare" || item.rarity === "Unique";
	const [swapView, setSwapView] = useState<WatchingScore | null>(null);

	// The active score to display (swapped watching profile or primary)
	const activeScore = swapView?.score ?? item.score;
	const watchingScores = item.watchingScores ?? [];

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
			<ItemHeader
				rarity={item.rarity}
				name={item.name}
				baseType={item.baseType}
				doubleLine={doubleLine}
			/>

			<Separator rarity={item.rarity} />

			{/* Properties */}
			{item.properties.length > 0 && (
				<>
					<div class="item-properties">
						{item.properties.map((prop) => (
							<div key={prop.name} class={`property-line ${prop.augmented ? "augmented" : ""}`}>
								{prop.name}: {prop.value}
							</div>
						))}
					</div>
					<Separator rarity={item.rarity} />
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
					<Separator rarity={item.rarity} />
				</>
			)}

			{/* Sockets & Item Level */}
			<div class="item-meta">
				{item.sockets && <div class="item-sockets">Sockets: {item.sockets}</div>}
				<div class="item-level">Item Level: {item.itemLevel}</div>
			</div>

			<Separator rarity={item.rarity} />

			{/* Enchants */}
			{item.enchants.length > 0 && (
				<>
					<div class="mod-section enchant-section">
						{item.enchants.map((mod) => (
							<div key={mod.text} class="mod-line enchant-line">
								<div class="mod-content">{mod.text}</div>
							</div>
						))}
					</div>
					<Separator rarity={item.rarity} />
				</>
			)}

			{/* Implicits */}
			{item.implicits.length > 0 && (
				<>
					<div class="mod-section implicit-section">
						{item.implicits.map((mod) => (
							<ModLine key={mod.text} mod={mod} display={display} />
						))}
					</div>
					<Separator rarity={item.rarity} />
				</>
			)}

			{/* Explicit mods */}
			{item.explicits.length > 0 && (
				<div class="mod-section explicit-section">
					{item.explicits.map((mod) => (
						<ModLine key={mod.text} mod={mod} display={display} />
					))}
				</div>
			)}

			{/* Open affixes */}
			{display.showOpenAffixes && (item.openPrefixes > 0 || item.openSuffixes > 0) && (
				<>
					<Separator rarity={item.rarity} />
					<div class="open-affixes">
						{item.openPrefixes > 0 && (
							<span class="open-prefix">
								{item.openPrefixes} open prefix
								{item.openPrefixes > 1 ? "es" : ""}
							</span>
						)}
						{item.openPrefixes > 0 && item.openSuffixes > 0 && " · "}
						{item.openSuffixes > 0 && (
							<span class="open-suffix">
								{item.openSuffixes} open suffix
								{item.openSuffixes > 1 ? "es" : ""}
							</span>
						)}
					</div>
				</>
			)}

			{/* Affix count summary */}
			{item.maxPrefixes > 0 && (
				<div class="affix-summary">
					Prefixes: {item.maxPrefixes - item.openPrefixes}/{item.maxPrefixes} · Suffixes:{" "}
					{item.maxSuffixes - item.openSuffixes}/{item.maxSuffixes}
				</div>
			)}

			{/* Influence tags */}
			{item.influences.length > 0 && (
				<>
					<Separator rarity={item.rarity} />
					<div class="influence-tags">
						{item.influences.map((inf) => (
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
					<Separator rarity={item.rarity} />
					<div class="flavor-text">{item.flavorText}</div>
				</>
			)}

			{/* Profile score (primary or swapped watching) */}
			{activeScore?.applicable && (
				<>
					<Separator rarity={item.rarity} />
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
