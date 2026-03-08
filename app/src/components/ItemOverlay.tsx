import type { Modifier, ParsedItem, Rarity } from "../types";

// Tooltip header sprites (left, middle, right) per rarity
import headerRareLeft from "../assets/tooltip/header-rare-left.webp";
import headerRareMiddle from "../assets/tooltip/header-rare-middle.webp";
import headerRareRight from "../assets/tooltip/header-rare-right.webp";
import headerUniqueLeft from "../assets/tooltip/header-unique-left.webp";
import headerUniqueMiddle from "../assets/tooltip/header-unique-middle.webp";
import headerUniqueRight from "../assets/tooltip/header-unique-right.webp";
import headerNormalLeft from "../assets/tooltip/header-normal-left.webp";
import headerNormalMiddle from "../assets/tooltip/header-normal-middle.webp";
import headerNormalRight from "../assets/tooltip/header-normal-right.webp";
import headerMagicLeft from "../assets/tooltip/header-magic-left.webp";
import headerMagicMiddle from "../assets/tooltip/header-magic-middle.webp";
import headerMagicRight from "../assets/tooltip/header-magic-right.webp";

// Tooltip separator sprites per rarity
import separatorRare from "../assets/tooltip/separator-rare.webp";
import separatorUnique from "../assets/tooltip/separator-unique.webp";
import separatorMagic from "../assets/tooltip/separator-magic.webp";
import separatorNormal from "../assets/tooltip/separator-normal.webp";

type HeaderSprites = { left: string; middle: string; right: string };

const headerSprites: Record<Rarity, HeaderSprites> = {
	Normal: { left: headerNormalLeft, middle: headerNormalMiddle, right: headerNormalRight },
	Magic: { left: headerMagicLeft, middle: headerMagicMiddle, right: headerMagicRight },
	Rare: { left: headerRareLeft, middle: headerRareMiddle, right: headerRareRight },
	Unique: { left: headerUniqueLeft, middle: headerUniqueMiddle, right: headerUniqueRight },
};

const separatorSprites: Record<Rarity, string> = {
	Normal: separatorNormal,
	Magic: separatorMagic,
	Rare: separatorRare,
	Unique: separatorUnique,
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
	}
}

/** CSS class for tier coloring */
function tierClass(tier: number | undefined): string {
	if (tier === undefined) return "tier-none";
	if (tier === 1) return "tier-1";
	if (tier <= 3) return "tier-2-3";
	if (tier <= 5) return "tier-4-5";
	return "tier-low";
}

/** Calculate roll quality as 0-100 percentage */
function rollQuality(mod: Modifier): number | null {
	if (mod.value === undefined || mod.min === undefined || mod.max === undefined)
		return null;
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
		case "fractured":
			return "Fr";
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
function ItemHeader({ rarity, name, baseType, doubleLine }: {
	rarity: Rarity;
	name: string;
	baseType: string;
	doubleLine: boolean;
}) {
	const sprites = headerSprites[rarity];
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
					<div class="item-name">{name}{baseType !== name ? ` ${baseType}` : ""}</div>
				)}
			</div>
		</div>
	);
}

function ModLine({ mod }: { mod: Modifier }) {
	const quality = rollQuality(mod);
	const typeLabel = modTypeLabel(mod);
	const tierCls = mod.type === "unique" ? "tier-unique" : tierClass(mod.tier);

	return (
		<div class={`mod-line ${tierCls}`}>
			<div class="mod-badges">
				{mod.tier !== undefined && (
					<span class={`tier-badge ${tierCls}`}>T{mod.tier}</span>
				)}
				{typeLabel !== null && (
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
			{quality !== null && (
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

export function ItemOverlay({ item }: { item: ParsedItem }) {
	const doubleLine = item.rarity === "Rare" || item.rarity === "Unique";

	return (
		<div class="item-card">
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
							<div
								key={prop.name}
								class={`property-line ${prop.augmented ? "augmented" : ""}`}
							>
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
							<ModLine key={mod.text} mod={mod} />
						))}
					</div>
					<Separator rarity={item.rarity} />
				</>
			)}

			{/* Explicit mods */}
			{item.explicits.length > 0 && (
				<div class="mod-section explicit-section">
					{item.explicits.map((mod) => (
						<ModLine key={mod.text} mod={mod} />
					))}
				</div>
			)}

			{/* Open affixes */}
			{item.rarity === "Rare" &&
				(item.openPrefixes > 0 || item.openSuffixes > 0) && (
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
			{item.rarity === "Rare" && (
				<div class="affix-summary">
					Prefixes: {3 - item.openPrefixes}/3 · Suffixes:{" "}
					{3 - item.openSuffixes}/3
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
		</div>
	);
}
