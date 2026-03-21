import type { Rarity, TypeSearchScope } from "../../types";
import type { TradeEditOverlay } from "./ItemOverlay";

// Tooltip header sprites (left, middle, right) per rarity/item type
import headerCurrencyLeft from "../../assets/tooltip/header-currency-left.webp";
import headerCurrencyMiddle from "../../assets/tooltip/header-currency-middle.webp";
import headerCurrencyRight from "../../assets/tooltip/header-currency-right.webp";
import headerGemLeft from "../../assets/tooltip/header-gem-left.webp";
import headerGemMiddle from "../../assets/tooltip/header-gem-middle.webp";
import headerGemRight from "../../assets/tooltip/header-gem-right.webp";
import headerMagicLeft from "../../assets/tooltip/header-magic-left.webp";
import headerMagicMiddle from "../../assets/tooltip/header-magic-middle.webp";
import headerMagicRight from "../../assets/tooltip/header-magic-right.webp";
import headerNormalLeft from "../../assets/tooltip/header-normal-left.webp";
import headerNormalMiddle from "../../assets/tooltip/header-normal-middle.webp";
import headerNormalRight from "../../assets/tooltip/header-normal-right.webp";
import headerRareLeft from "../../assets/tooltip/header-rare-left.webp";
import headerRareMiddle from "../../assets/tooltip/header-rare-middle.webp";
import headerRareRight from "../../assets/tooltip/header-rare-right.webp";
import headerUniqueLeft from "../../assets/tooltip/header-unique-left.webp";
import headerUniqueMiddle from "../../assets/tooltip/header-unique-middle.webp";
import headerUniqueRight from "../../assets/tooltip/header-unique-right.webp";

// Tooltip separator sprites per rarity/item type
import separatorCurrency from "../../assets/tooltip/separator-currency.webp";
import separatorGem from "../../assets/tooltip/separator-gem.webp";
import separatorMagic from "../../assets/tooltip/separator-magic.webp";
import separatorNormal from "../../assets/tooltip/separator-normal.webp";
import separatorRare from "../../assets/tooltip/separator-rare.webp";
import separatorUnique from "../../assets/tooltip/separator-unique.webp";

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
export function rarityColor(rarity: Rarity): string {
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

export function Separator({ rarity }: { rarity: Rarity }) {
	return (
		<div class="item-separator">
			<img src={separatorSprites[rarity] ?? defaultSeparator} alt="" class="separator-img" />
		</div>
	);
}

/** PoE-style item header with left cap, tiling middle, right cap.
 *  In trade edit mode, integrates type scope selector and rarity dropdown. */
export function ItemHeader({
	rarity,
	name,
	baseType,
	doubleLine,
	tradeEdit,
}: {
	rarity: Rarity;
	name: string;
	baseType: string;
	doubleLine: boolean;
	tradeEdit?: TradeEditOverlay | undefined;
}) {
	const sprites = headerSprites[rarity] ?? defaultHeader;
	const hasEditControls =
		tradeEdit && (tradeEdit.typeScopeOptions.length > 0 || tradeEdit.rarityFilter);

	// Rarity cycling: resolve current label and build cycle handler
	const rarityInfo = (() => {
		if (!tradeEdit?.rarityFilter || tradeEdit.rarityFilter.kind.type !== "option") return null;
		const filter = tradeEdit.rarityFilter;
		const { kind } = filter;
		if (kind.type !== "option") return null;
		const options = kind.options;
		const ov = tradeEdit.filterOverrides.get("rarity");
		const currentId =
			ov?.selectedId !== undefined
				? ov.selectedId
				: filter.defaultValue?.type === "selected"
					? filter.defaultValue.id
					: null;
		const currentLabel = options.find((o) => (o.id ?? null) === currentId)?.text ?? "Any";
		const currentIdx = options.findIndex((o) => (o.id ?? null) === currentId);
		const cycle = () => {
			const nextIdx = (currentIdx + 1) % options.length;
			const next = options[nextIdx];
			if (next) {
				tradeEdit.onFilterOverride("rarity", {
					enabled: next.id != null,
					selectedId: next.id,
				});
			}
		};
		return { label: currentLabel, cycle };
	})();

	return (
		<div
			class={`item-header ${doubleLine ? "header-double" : "header-single"} ${hasEditControls ? "header-edit" : ""}`}
		>
			<div class="header-bg">
				<img src={sprites.left} alt="" class="header-left" />
				<div class="header-middle" style={{ backgroundImage: `url(${sprites.middle})` }} />
				<img src={sprites.right} alt="" class="header-right" />
			</div>

			{/* Rarity cycling badge — left edge of header */}
			{hasEditControls && rarityInfo && (
				<button
					type="button"
					class="header-rarity-badge"
					onClick={rarityInfo.cycle}
					title="Click to cycle rarity filter"
				>
					{rarityInfo.label}
				</button>
			)}

			<div class="header-text" style={{ color: rarityColor(rarity) }}>
				{doubleLine ? (
					<>
						<div class="item-name">{name}</div>
						{hasEditControls && tradeEdit.typeScopeOptions.length > 0 ? (
							<select
								class="header-type-select"
								style={{ color: rarityColor(rarity) }}
								value={tradeEdit.typeScope}
								onChange={(e) =>
									tradeEdit.setTypeScope((e.target as HTMLSelectElement).value as TypeSearchScope)
								}
							>
								{tradeEdit.typeScopeOptions.map((opt) => (
									<option key={opt.scope} value={opt.scope}>
										{opt.label}
									</option>
								))}
							</select>
						) : (
							<div class="item-base">{baseType}</div>
						)}
					</>
				) : hasEditControls && tradeEdit.typeScopeOptions.length > 0 ? (
					<>
						<div class="item-name">{name}</div>
						<select
							class="header-type-select"
							style={{ color: rarityColor(rarity) }}
							value={tradeEdit.typeScope}
							onChange={(e) =>
								tradeEdit.setTypeScope((e.target as HTMLSelectElement).value as TypeSearchScope)
							}
						>
							{tradeEdit.typeScopeOptions.map((opt) => (
								<option key={opt.scope} value={opt.scope}>
									{opt.label}
								</option>
							))}
						</select>
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
