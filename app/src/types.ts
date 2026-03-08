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

/** A modifier on an item with tier and roll info */
export interface Modifier {
	/** Display name from the mod header, e.g., "Merciless" */
	modName?: string;
	/** prefix, suffix, implicit, etc. */
	type: ModType;
	/** Tier number (1 = best). Undefined for implicits/uniques. */
	tier?: number;
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
	/** Number of open prefix slots (max 3 for rares) */
	openPrefixes: number;
	/** Number of open suffix slots (max 3 for rares) */
	openSuffixes: number;
}
