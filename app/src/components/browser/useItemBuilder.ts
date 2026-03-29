import { invoke } from "@tauri-apps/api/core";
import { useCallback, useState } from "preact/hooks";

// ── Types mirroring Rust browser module ────────────────────────────────────

export interface SearchResult {
	name: string;
	kind: string;
	item_class?: string;
	category?: string;
}

export interface BaseTypeDetail {
	name: string;
	itemClassId: string;
	itemClassName: string;
	category: string;
	dropLevel: number;
	width: number;
	height: number;
	implicits: string[];
	tags: string[];
	defences?: {
		armourMin: number;
		armourMax: number;
		evasionMin: number;
		evasionMax: number;
		esMin: number;
		esMax: number;
		wardMin: number;
		wardMax: number;
	};
	weapon?: {
		critical: number;
		speed: number;
		damageMin: number;
		damageMax: number;
		range: number;
	};
	block?: number;
}

export interface ModTierStat {
	statId: string;
	min: number;
	max: number;
	displayText: string;
}

export interface ModTier {
	modId: string;
	name: string;
	tier: number;
	requiredLevel: number;
	eligible: boolean;
	spawnWeight: number;
	stats: ModTierStat[];
	tags: string[];
}

export interface ModFamily {
	familyId: string;
	tiers: ModTier[];
	taken: boolean;
}

export interface ModPoolResult {
	prefixes: ModFamily[];
	suffixes: ModFamily[];
	availablePrefixCount: number;
	availableSuffixCount: number;
}

// ── Slotted mod ────────────────────────────────────────────────────────────

export interface SlottedMod {
	familyId: string;
	modId: string;
	name: string;
	tier: number;
	stats: ModTierStat[];
	isPrefix: boolean;
}

export type BrowserRarity = "Normal" | "Magic" | "Rare";

// ── Hook ───────────────────────────────────────────────────────────────────

export function useItemBuilder() {
	const [detail, setDetail] = useState<BaseTypeDetail | null>(null);
	const [rarity, setRarityState] = useState<BrowserRarity>("Rare");
	const [itemLevel, setItemLevel] = useState(84);
	const [maxPrefixes, setMaxPrefixes] = useState(3);
	const [maxSuffixes, setMaxSuffixes] = useState(3);
	const [slottedPrefixes, setSlottedPrefixes] = useState<(SlottedMod | null)[]>([]);
	const [slottedSuffixes, setSlottedSuffixes] = useState<(SlottedMod | null)[]>([]);
	const [pool, setPool] = useState<ModPoolResult | null>(null);

	// Helper: collect taken mod IDs from slotted mods.
	const getTakenModIds = useCallback(
		(prefixes: (SlottedMod | null)[], suffixes: (SlottedMod | null)[]): string[] =>
			[...prefixes, ...suffixes].filter((m): m is SlottedMod => m !== null).map((m) => m.modId),
		[],
	);

	// Fetch mod pool from backend.
	const refreshPool = useCallback(
		(
			baseName: string,
			ilvl: number,
			prefixes: (SlottedMod | null)[],
			suffixes: (SlottedMod | null)[],
		) => {
			invoke<ModPoolResult | null>("browser_mod_pool", {
				query: {
					base_type: baseName,
					item_level: ilvl,
					generation_types: [],
					taken_mod_ids: getTakenModIds(prefixes, suffixes),
				},
			}).then(setPool);
		},
		[getTakenModIds],
	);

	// Fetch affix limits and resize slot arrays.
	const refreshLimits = useCallback(async (itemClassName: string, rar: BrowserRarity) => {
		const [p, s] = await invoke<[number, number]>("browser_affix_limits", {
			itemClass: itemClassName,
			rarity: rar,
		});
		setMaxPrefixes(p);
		setMaxSuffixes(s);
		return [p, s] as const;
	}, []);

	// Resize slot arrays when limits change, dropping excess from the end.
	const resizeSlots = useCallback(
		(
			curPrefixes: (SlottedMod | null)[],
			curSuffixes: (SlottedMod | null)[],
			newMaxP: number,
			newMaxS: number,
		) => {
			const newP: (SlottedMod | null)[] = Array.from({ length: newMaxP }, (_, i) =>
				i < curPrefixes.length ? (curPrefixes[i] ?? null) : null,
			);
			const newS: (SlottedMod | null)[] = Array.from({ length: newMaxS }, (_, i) =>
				i < curSuffixes.length ? (curSuffixes[i] ?? null) : null,
			);
			setSlottedPrefixes(newP);
			setSlottedSuffixes(newS);
			return [newP, newS] as const;
		},
		[],
	);

	// Select a base type — reset everything.
	const selectBaseType = useCallback(
		async (name: string) => {
			const d = await invoke<BaseTypeDetail | null>("browser_base_type_detail", { name });
			setDetail(d);
			if (!d) {
				setPool(null);
				setSlottedPrefixes([]);
				setSlottedSuffixes([]);
				return;
			}
			const [p, s] = await refreshLimits(d.itemClassName, rarity);
			const emptyP: (SlottedMod | null)[] = Array.from({ length: p }, () => null);
			const emptyS: (SlottedMod | null)[] = Array.from({ length: s }, () => null);
			setSlottedPrefixes(emptyP);
			setSlottedSuffixes(emptyS);
			refreshPool(d.name, itemLevel, emptyP, emptyS);
		},
		[rarity, itemLevel, refreshLimits, refreshPool],
	);

	// Change rarity — resize slots.
	const setRarity = useCallback(
		async (r: BrowserRarity) => {
			setRarityState(r);
			if (!detail) return;
			const [p, s] = await refreshLimits(detail.itemClassName, r);
			const [newP, newS] = resizeSlots(slottedPrefixes, slottedSuffixes, p, s);
			refreshPool(detail.name, itemLevel, newP, newS);
		},
		[detail, itemLevel, slottedPrefixes, slottedSuffixes, refreshLimits, resizeSlots, refreshPool],
	);

	// Change item level — refresh pool.
	const setItemLevelAndRefresh = useCallback(
		(lvl: number) => {
			setItemLevel(lvl);
			if (detail) refreshPool(detail.name, lvl, slottedPrefixes, slottedSuffixes);
		},
		[detail, slottedPrefixes, slottedSuffixes, refreshPool],
	);

	// Slot a mod into the next empty prefix or suffix slot.
	const slotMod = useCallback(
		(family: ModFamily, isPrefix: boolean) => {
			if (!detail) return;
			const bestTier = family.tiers.find((t) => t.eligible) ?? family.tiers[0];
			if (!bestTier) return;

			const mod: SlottedMod = {
				familyId: family.familyId,
				modId: bestTier.modId,
				name: bestTier.name,
				tier: bestTier.tier,
				stats: bestTier.stats,
				isPrefix,
			};

			if (isPrefix) {
				setSlottedPrefixes((prev) => {
					const idx = prev.indexOf(null);
					if (idx === -1) return prev;
					const next = [...prev];
					next[idx] = mod;
					refreshPool(detail.name, itemLevel, next, slottedSuffixes);
					return next;
				});
			} else {
				setSlottedSuffixes((prev) => {
					const idx = prev.indexOf(null);
					if (idx === -1) return prev;
					const next = [...prev];
					next[idx] = mod;
					refreshPool(detail.name, itemLevel, slottedPrefixes, next);
					return next;
				});
			}
		},
		[detail, itemLevel, slottedPrefixes, slottedSuffixes, refreshPool],
	);

	// Unslot a mod by index.
	const unslotMod = useCallback(
		(isPrefix: boolean, index: number) => {
			if (!detail) return;
			if (isPrefix) {
				setSlottedPrefixes((prev) => {
					const next = [...prev];
					next[index] = null;
					refreshPool(detail.name, itemLevel, next, slottedSuffixes);
					return next;
				});
			} else {
				setSlottedSuffixes((prev) => {
					const next = [...prev];
					next[index] = null;
					refreshPool(detail.name, itemLevel, slottedPrefixes, next);
					return next;
				});
			}
		},
		[detail, itemLevel, slottedPrefixes, slottedSuffixes, refreshPool],
	);

	// Clear all slotted mods.
	const clearAllMods = useCallback(() => {
		if (!detail) return;
		const emptyP: (SlottedMod | null)[] = Array.from({ length: maxPrefixes }, () => null);
		const emptyS: (SlottedMod | null)[] = Array.from({ length: maxSuffixes }, () => null);
		setSlottedPrefixes(emptyP);
		setSlottedSuffixes(emptyS);
		refreshPool(detail.name, itemLevel, emptyP, emptyS);
	}, [detail, itemLevel, maxPrefixes, maxSuffixes, refreshPool]);

	return {
		detail,
		rarity,
		itemLevel,
		maxPrefixes,
		maxSuffixes,
		slottedPrefixes,
		slottedSuffixes,
		pool,
		selectBaseType,
		setRarity,
		setItemLevel: setItemLevelAndRefresh,
		slotMod,
		unslotMod,
		clearAllMods,
	};
}
