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

import separatorCurrency from "../../assets/tooltip/separator-currency.webp";
import separatorGem from "../../assets/tooltip/separator-gem.webp";
import separatorMagic from "../../assets/tooltip/separator-magic.webp";
import separatorNormal from "../../assets/tooltip/separator-normal.webp";
import separatorRare from "../../assets/tooltip/separator-rare.webp";
import separatorUnique from "../../assets/tooltip/separator-unique.webp";

type Rarity = "Normal" | "Magic" | "Rare" | "Unique" | "Currency" | "Gem";

type HeaderSprites = { left: string; middle: string; right: string };

const headerSprites: Record<string, HeaderSprites> = {
	Normal: { left: headerNormalLeft, middle: headerNormalMiddle, right: headerNormalRight },
	Magic: { left: headerMagicLeft, middle: headerMagicMiddle, right: headerMagicRight },
	Rare: { left: headerRareLeft, middle: headerRareMiddle, right: headerRareRight },
	Unique: { left: headerUniqueLeft, middle: headerUniqueMiddle, right: headerUniqueRight },
	Currency: { left: headerCurrencyLeft, middle: headerCurrencyMiddle, right: headerCurrencyRight },
	Gem: { left: headerGemLeft, middle: headerGemMiddle, right: headerGemRight },
};
const defaultHeader = headerSprites.Normal;

const separatorSprites: Record<string, string> = {
	Normal: separatorNormal,
	Magic: separatorMagic,
	Rare: separatorRare,
	Unique: separatorUnique,
	Currency: separatorCurrency,
	Gem: separatorGem,
};

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

export function CardHeader({
	rarity,
	name,
	baseType,
}: { rarity: Rarity; name: string; baseType?: string }) {
	// biome-ignore lint/style/noNonNullAssertion: defaultHeader is always defined
	const sprites = (headerSprites[rarity] ?? defaultHeader)!;
	const doubleLine = !!baseType && baseType !== name;

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
					<div class="item-name">{name}</div>
				)}
			</div>
		</div>
	);
}

export function CardSeparator({ rarity }: { rarity: Rarity }) {
	return (
		<div class="item-separator">
			<img src={separatorSprites[rarity] ?? separatorNormal} alt="" class="separator-img" />
		</div>
	);
}
