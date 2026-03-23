import type { ItemPayload } from "./types";

/**
 * Mock items for overlay development.
 * Sourced from real Ctrl+Alt+C fixtures in poe-inspect v1.
 */

/** Rare boots with Eater/Exarch implicits, mixed tiers, and a crafted mod */
export const rareBoots: ItemPayload = {
	rawText: "",
	item: {
		header: {
			itemClass: "Boots",
			rarity: "Rare",
			name: "Loath Spur",
			baseType: "Murder Boots",
		},
		itemLevel: 75,
		monsterLevel: null,
		talismanTier: null,
		requirements: [
			{ name: "Level", value: "70" },
			{ name: "Str", value: "155" },
			{ name: "Dex", value: "98" },
			{ name: "Int", value: "155" },
		],
		sockets: "R-G-R-B",
		experience: null,
		properties: [
			{ name: "Evasion Rating", value: "250", augmented: true, synthetic: false },
			{ name: "Energy Shield", value: "37", augmented: true, synthetic: false },
		],
		implicits: [
			{
				header: {
					source: "regular",
					slot: "searingExarchImplicit",
					influenceTier: null,
					name: null,
					tier: null,
					tags: [],
				},
				statLines: [
					{
						rawText: "19% chance to Avoid being Stunned (18-20)",
						displayText: "19% chance to Avoid being Stunned",
						values: [{ current: 19, min: 18, max: 20 }],
						statIds: null,
						statValues: null,
						isReminder: false,
						isUnscalable: false,
					},
				],
				isFractured: false,
				displayType: "implicit",
			},
			{
				header: {
					source: "regular",
					slot: "eaterOfWorldsImplicit",
					influenceTier: null,
					name: null,
					tier: null,
					tags: ["Life"],
				},
				statLines: [
					{
						rawText:
							"While a Unique Enemy is in your Presence, Regenerate 0.3% of Life per second per Endurance Charge",
						displayText:
							"While a Unique Enemy is in your Presence, Regenerate 0.3% of Life per second per Endurance Charge",
						values: [],
						statIds: null,
						statValues: null,
						isReminder: false,
						isUnscalable: false,
					},
				],
				isFractured: false,
				displayType: "implicit",
			},
		],
		explicits: [
			{
				header: {
					source: "regular",
					slot: "prefix",
					influenceTier: null,
					name: "Blue",
					tier: { Tier: 2 },
					tags: ["Mana"],
				},
				statLines: [
					{
						rawText: "+68 to maximum Mana (65-68)",
						displayText: "+68 to maximum Mana",
						values: [{ current: 68, min: 65, max: 68 }],
						statIds: null,
						statValues: null,
						isReminder: false,
						isUnscalable: false,
					},
				],
				isFractured: false,
				displayType: "prefix",
			},
			{
				header: {
					source: "regular",
					slot: "prefix",
					influenceTier: null,
					name: "Cheetah's",
					tier: { Tier: 2 },
					tags: ["Speed"],
				},
				statLines: [
					{
						rawText: "30% increased Movement Speed",
						displayText: "30% increased Movement Speed",
						values: [{ current: 30, min: 30, max: 30 }],
						statIds: null,
						statValues: null,
						isReminder: false,
						isUnscalable: false,
					},
				],
				isFractured: false,
				displayType: "prefix",
			},
			{
				header: {
					source: "regular",
					slot: "prefix",
					influenceTier: null,
					name: "Prior's",
					tier: { Tier: 1 },
					tags: ["Life", "Defences", "Energy Shield"],
				},
				statLines: [
					{
						rawText: "+11 to maximum Energy Shield",
						displayText: "+11 to maximum Energy Shield",
						values: [{ current: 11, min: 11, max: 11 }],
						statIds: null,
						statValues: null,
						isReminder: false,
						isUnscalable: false,
					},
					{
						rawText: "+25 to maximum Life (24-28)",
						displayText: "+25 to maximum Life",
						values: [{ current: 25, min: 24, max: 28 }],
						statIds: null,
						statValues: null,
						isReminder: false,
						isUnscalable: false,
					},
				],
				isFractured: false,
				displayType: "prefix",
			},
			{
				header: {
					source: "regular",
					slot: "suffix",
					influenceTier: null,
					name: "of the Lynx",
					tier: { Tier: 8 },
					tags: ["Attribute"],
				},
				statLines: [
					{
						rawText: "+14 to Dexterity (13-17)",
						displayText: "+14 to Dexterity",
						values: [{ current: 14, min: 13, max: 17 }],
						statIds: null,
						statValues: null,
						isReminder: false,
						isUnscalable: false,
					},
				],
				isFractured: false,
				displayType: "suffix",
			},
			{
				header: {
					source: "regular",
					slot: "suffix",
					influenceTier: null,
					name: "of the Yeti",
					tier: { Tier: 5 },
					tags: ["Elemental", "Cold", "Resistance"],
				},
				statLines: [
					{
						rawText: "+27% to Cold Resistance (24-29)",
						displayText: "+27% to Cold Resistance",
						values: [{ current: 27, min: 24, max: 29 }],
						statIds: null,
						statValues: null,
						isReminder: false,
						isUnscalable: false,
					},
				],
				isFractured: false,
				displayType: "suffix",
			},
			{
				header: {
					source: "masterCrafted",
					slot: "suffix",
					influenceTier: null,
					name: "of Craft",
					tier: { Rank: 1 },
					tags: ["Elemental", "Lightning", "Resistance"],
				},
				statLines: [
					{
						rawText: "+22% to Lightning Resistance (21-28)",
						displayText: "+22% to Lightning Resistance",
						values: [{ current: 22, min: 21, max: 28 }],
						statIds: null,
						statValues: null,
						isReminder: false,
						isUnscalable: false,
					},
				],
				isFractured: false,
				displayType: "crafted",
			},
		],
		enchants: [],
		influences: ["SearingExarch", "EaterOfWorlds"],
		statuses: [],
		isCorrupted: false,
		isFractured: false,
		isUnidentified: false,
		note: null,
		description: null,
		flavorText: null,
		gemData: null,
		socketInfo: null,
		quality: null,
		pseudoMods: [],
		unclassifiedSections: [],
	},
	eval: {
		modTiers: [
			// implicits (0 enchants, 2 implicits)
			{ tier: null, totalTiers: null, tierKind: null, quality: null },
			{ tier: null, totalTiers: null, tierKind: null, quality: null },
			// explicits
			{ tier: 2, totalTiers: 12, tierKind: "tier", quality: "great" },
			{ tier: 2, totalTiers: 10, tierKind: "tier", quality: "great" },
			{ tier: 1, totalTiers: 8, tierKind: "tier", quality: "best" },
			{ tier: 8, totalTiers: 10, tierKind: "tier", quality: "low" },
			{ tier: 5, totalTiers: 8, tierKind: "tier", quality: "mid" },
			{ tier: 1, totalTiers: 3, tierKind: "rank", quality: "mid" },
		],
		affixSummary: {
			openPrefixes: 0,
			openSuffixes: 0,
			maxPrefixes: 3,
			maxSuffixes: 3,
			modifiable: true,
		},
		score: null,
		watchingScores: [],
	},
};

/** Rare body armour with enchant, Redeemer influence, open suffix */
export const rareBodyArmour: ItemPayload = {
	rawText: "",
	item: {
		header: {
			itemClass: "Body Armours",
			rarity: "Rare",
			name: "Agony Carapace",
			baseType: "Majestic Plate",
		},
		itemLevel: 100,
		monsterLevel: null,
		talismanTier: null,
		requirements: [
			{ name: "Level", value: "68" },
			{ name: "Str", value: "144" },
		],
		sockets: "R-R-R-R",
		experience: null,
		properties: [{ name: "Armour", value: "890", augmented: true, synthetic: false }],
		implicits: [],
		explicits: [
			{
				header: {
					source: "regular",
					slot: "prefix",
					influenceTier: null,
					name: "Mammoth's",
					tier: { Tier: 1 },
					tags: ["Defences"],
				},
				statLines: [
					{
						rawText: "39% increased Armour (39-42)",
						displayText: "39% increased Armour",
						values: [{ current: 39, min: 39, max: 42 }],
						statIds: null,
						statValues: null,
						isReminder: false,
						isUnscalable: false,
					},
					{
						rawText: "16% increased Stun and Block Recovery",
						displayText: "16% increased Stun and Block Recovery",
						values: [{ current: 16, min: 16, max: 16 }],
						statIds: null,
						statValues: null,
						isReminder: false,
						isUnscalable: false,
					},
				],
				isFractured: false,
				displayType: "prefix",
			},
			{
				header: {
					source: "regular",
					slot: "prefix",
					influenceTier: null,
					name: "Rotund",
					tier: { Tier: 7 },
					tags: ["Life"],
				},
				statLines: [
					{
						rawText: "+67 to maximum Life (60-69)",
						displayText: "+67 to maximum Life",
						values: [{ current: 67, min: 60, max: 69 }],
						statIds: null,
						statValues: null,
						isReminder: false,
						isUnscalable: false,
					},
				],
				isFractured: false,
				displayType: "prefix",
			},
			{
				header: {
					source: "regular",
					slot: "prefix",
					influenceTier: null,
					name: "Layered",
					tier: { Tier: 7 },
					tags: ["Defences"],
				},
				statLines: [
					{
						rawText: "29% increased Armour (27-42)",
						displayText: "29% increased Armour",
						values: [{ current: 29, min: 27, max: 42 }],
						statIds: null,
						statValues: null,
						isReminder: false,
						isUnscalable: false,
					},
				],
				isFractured: false,
				displayType: "prefix",
			},
			{
				header: {
					source: "regular",
					slot: "suffix",
					influenceTier: null,
					name: "of Numbing",
					tier: { Tier: 1 },
					tags: ["Physical"],
				},
				statLines: [
					{
						rawText: "4% additional Physical Damage Reduction (3-4)",
						displayText: "4% additional Physical Damage Reduction",
						values: [{ current: 4, min: 3, max: 4 }],
						statIds: null,
						statValues: null,
						isReminder: false,
						isUnscalable: false,
					},
				],
				isFractured: false,
				displayType: "suffix",
			},
			{
				header: {
					source: "regular",
					slot: "suffix",
					influenceTier: null,
					name: "of the Tempest",
					tier: { Tier: 4 },
					tags: ["Elemental", "Lightning", "Resistance"],
				},
				statLines: [
					{
						rawText: "+32% to Lightning Resistance (30-35)",
						displayText: "+32% to Lightning Resistance",
						values: [{ current: 32, min: 30, max: 35 }],
						statIds: null,
						statValues: null,
						isReminder: false,
						isUnscalable: false,
					},
				],
				isFractured: false,
				displayType: "suffix",
			},
		],
		enchants: [
			{
				header: {
					source: "regular",
					slot: "enchant",
					influenceTier: null,
					name: null,
					tier: null,
					tags: [],
				},
				statLines: [
					{
						rawText: "Quality does not increase Defences",
						displayText: "Quality does not increase Defences",
						values: [],
						statIds: null,
						statValues: null,
						isReminder: false,
						isUnscalable: false,
					},
				],
				isFractured: false,
				displayType: "enchant",
			},
			{
				header: {
					source: "regular",
					slot: "enchant",
					influenceTier: null,
					name: null,
					tier: null,
					tags: [],
				},
				statLines: [
					{
						rawText: "Grants +1 to Maximum Life per 2% Quality",
						displayText: "Grants +1 to Maximum Life per 2% Quality",
						values: [],
						statIds: null,
						statValues: null,
						isReminder: false,
						isUnscalable: false,
					},
				],
				isFractured: false,
				displayType: "enchant",
			},
		],
		influences: ["Redeemer"],
		statuses: [],
		isCorrupted: false,
		isFractured: false,
		isUnidentified: false,
		note: null,
		description: null,
		flavorText: null,
		gemData: null,
		socketInfo: null,
		quality: null,
		pseudoMods: [],
		unclassifiedSections: [],
	},
	eval: {
		modTiers: [
			// enchants
			{ tier: null, totalTiers: null, tierKind: null, quality: null },
			{ tier: null, totalTiers: null, tierKind: null, quality: null },
			// implicits (none)
			// explicits
			{ tier: 1, totalTiers: 10, tierKind: "tier", quality: "best" },
			{ tier: 7, totalTiers: 8, tierKind: "tier", quality: "low" },
			{ tier: 7, totalTiers: 8, tierKind: "tier", quality: "low" },
			{ tier: 1, totalTiers: 12, tierKind: "tier", quality: "best" },
			{ tier: 4, totalTiers: 8, tierKind: "tier", quality: "good" },
		],
		affixSummary: {
			openPrefixes: 0,
			openSuffixes: 1,
			maxPrefixes: 3,
			maxSuffixes: 3,
			modifiable: true,
		},
		score: null,
		watchingScores: [],
	},
};

/** Unique ring with variable rolls (Ventor's Gamble) */
export const uniqueRing: ItemPayload = {
	rawText: "",
	item: {
		header: {
			itemClass: "Rings",
			rarity: "Unique",
			name: "Ventor's Gamble",
			baseType: "Gold Ring",
		},
		itemLevel: 75,
		monsterLevel: null,
		talismanTier: null,
		requirements: [{ name: "Level", value: "65" }],
		sockets: null,
		experience: null,
		properties: [],
		implicits: [
			{
				header: {
					source: "regular",
					slot: "implicit",
					influenceTier: null,
					name: null,
					tier: null,
					tags: [],
				},
				statLines: [
					{
						rawText: "15% increased Rarity of Items found (6-15)",
						displayText: "15% increased Rarity of Items found",
						values: [{ current: 15, min: 6, max: 15 }],
						statIds: null,
						statValues: null,
						isReminder: false,
						isUnscalable: false,
					},
				],
				isFractured: false,
				displayType: "implicit",
			},
		],
		explicits: [
			{
				header: {
					source: "regular",
					slot: "unique",
					influenceTier: null,
					name: null,
					tier: null,
					tags: ["Life"],
				},
				statLines: [
					{
						rawText: "+44 to maximum Life (0-60)",
						displayText: "+44 to maximum Life",
						values: [{ current: 44, min: 0, max: 60 }],
						statIds: null,
						statValues: null,
						isReminder: false,
						isUnscalable: false,
					},
				],
				isFractured: false,
				displayType: "unique",
			},
			{
				header: {
					source: "regular",
					slot: "unique",
					influenceTier: null,
					name: null,
					tier: null,
					tags: ["Elemental", "Fire", "Resistance"],
				},
				statLines: [
					{
						rawText: "+4% to Fire Resistance (-25-50)",
						displayText: "+4% to Fire Resistance",
						values: [{ current: 4, min: -25, max: 50 }],
						statIds: null,
						statValues: null,
						isReminder: false,
						isUnscalable: false,
					},
				],
				isFractured: false,
				displayType: "unique",
			},
			{
				header: {
					source: "regular",
					slot: "unique",
					influenceTier: null,
					name: null,
					tier: null,
					tags: ["Elemental", "Cold", "Resistance"],
				},
				statLines: [
					{
						rawText: "-9% to Cold Resistance (-25-50)",
						displayText: "-9% to Cold Resistance",
						values: [{ current: -9, min: -25, max: 50 }],
						statIds: null,
						statValues: null,
						isReminder: false,
						isUnscalable: false,
					},
				],
				isFractured: false,
				displayType: "unique",
			},
			{
				header: {
					source: "regular",
					slot: "unique",
					influenceTier: null,
					name: null,
					tier: null,
					tags: ["Elemental", "Lightning", "Resistance"],
				},
				statLines: [
					{
						rawText: "+40% to Lightning Resistance (-25-50)",
						displayText: "+40% to Lightning Resistance",
						values: [{ current: 40, min: -25, max: 50 }],
						statIds: null,
						statValues: null,
						isReminder: false,
						isUnscalable: false,
					},
				],
				isFractured: false,
				displayType: "unique",
			},
			{
				header: {
					source: "regular",
					slot: "unique",
					influenceTier: null,
					name: null,
					tier: null,
					tags: [],
				},
				statLines: [
					{
						rawText: "1% reduced Quantity of Items found (-10-10)",
						displayText: "1% reduced Quantity of Items found",
						values: [{ current: 1, min: -10, max: 10 }],
						statIds: null,
						statValues: null,
						isReminder: false,
						isUnscalable: false,
					},
				],
				isFractured: false,
				displayType: "unique",
			},
			{
				header: {
					source: "regular",
					slot: "unique",
					influenceTier: null,
					name: null,
					tier: null,
					tags: [],
				},
				statLines: [
					{
						rawText: "40% increased Rarity of Items found (-40-40)",
						displayText: "40% increased Rarity of Items found",
						values: [{ current: 40, min: -40, max: 40 }],
						statIds: null,
						statValues: null,
						isReminder: false,
						isUnscalable: false,
					},
				],
				isFractured: false,
				displayType: "unique",
			},
		],
		enchants: [],
		influences: [],
		statuses: [],
		isCorrupted: false,
		isFractured: false,
		isUnidentified: false,
		note: null,
		description: null,
		flavorText:
			'In a blaze of glory,\nAn anomaly defying all odds\nThe "unkillable" beast met the divine\nAnd Ventor met his latest trophy.',
		gemData: null,
		socketInfo: null,
		quality: null,
		pseudoMods: [],
		unclassifiedSections: [],
	},
	eval: {
		modTiers: [
			// implicit
			{ tier: null, totalTiers: null, tierKind: null, quality: null },
			// explicits (unique mods have no tier/quality)
			{ tier: null, totalTiers: null, tierKind: null, quality: null },
			{ tier: null, totalTiers: null, tierKind: null, quality: null },
			{ tier: null, totalTiers: null, tierKind: null, quality: null },
			{ tier: null, totalTiers: null, tierKind: null, quality: null },
			{ tier: null, totalTiers: null, tierKind: null, quality: null },
			{ tier: null, totalTiers: null, tierKind: null, quality: null },
		],
		affixSummary: {
			openPrefixes: 0,
			openSuffixes: 0,
			maxPrefixes: 0,
			maxSuffixes: 0,
			modifiable: false,
		},
		score: null,
		watchingScores: [],
	},
};

/** All mock items for cycling through in the overlay */
export const mockItems: ItemPayload[] = [rareBoots, rareBodyArmour, uniqueRing];
