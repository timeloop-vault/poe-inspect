import { useCallback, useRef, useState } from "preact/hooks";
import type { ModFamily, ModPoolResult, ModTierStat } from "./useItemBuilder";

function formatStat(s: ModTierStat): string {
	if (s.min === s.max) return s.displayText.replace("#", `${s.min}`);
	return s.displayText.replace("#", `(${s.min}-${s.max})`);
}

function ModPickerRow({
	family,
	isPrefix,
	canSlot,
	onSlot,
}: {
	family: ModFamily;
	isPrefix: boolean;
	canSlot: boolean;
	onSlot: (family: ModFamily, isPrefix: boolean) => void;
}) {
	const bestTier = family.tiers.find((t) => t.eligible) ?? family.tiers[0];
	if (!bestTier) return null;

	const disabled = family.taken || !canSlot;

	return (
		<button
			type="button"
			class={`picker-row ${disabled ? "disabled" : ""} ${family.taken ? "taken" : ""}`}
			onClick={() => {
				if (!disabled) onSlot(family, isPrefix);
			}}
			disabled={disabled}
		>
			<span class={`picker-slot-badge ${isPrefix ? "prefix" : "suffix"}`}>
				{isPrefix ? "P" : "S"}
			</span>
			<span class="picker-mod-info">
				<span class="picker-mod-header">
					<span class="picker-mod-name">{bestTier.name}</span>
					<span class="picker-mod-tier">T{bestTier.tier}</span>
					<span class="picker-mod-meta">
						ilvl {bestTier.requiredLevel} &middot; w:{bestTier.spawnWeight}
					</span>
				</span>
				<span class="picker-mod-stats">
					{bestTier.stats.map((s) => (
						<span key={s.statId} class="picker-stat-line">
							{formatStat(s)}
						</span>
					))}
				</span>
			</span>
		</button>
	);
}

export function ModPicker({
	pool,
	slottedPrefixCount,
	slottedSuffixCount,
	maxPrefixes,
	maxSuffixes,
	onSlotMod,
	isClusterJewel,
}: {
	pool: ModPoolResult;
	slottedPrefixCount: number;
	slottedSuffixCount: number;
	maxPrefixes: number;
	maxSuffixes: number;
	onSlotMod: (family: ModFamily, isPrefix: boolean) => void;
	isClusterJewel: boolean;
}) {
	const [filter, setFilter] = useState<"all" | "prefix" | "suffix">("all");
	const [search, setSearch] = useState("");
	const searchRef = useRef<HTMLInputElement>(null);

	const canSlotPrefix = slottedPrefixCount < maxPrefixes;
	const canSlotSuffix = slottedSuffixCount < maxSuffixes;

	const matchesSearch = useCallback(
		(family: ModFamily): boolean => {
			if (!search) return true;
			const q = search.toLowerCase();
			const bestTier = family.tiers.find((t) => t.eligible) ?? family.tiers[0];
			if (!bestTier) return false;
			if (bestTier.name.toLowerCase().includes(q)) return true;
			return bestTier.stats.some(
				(s) => s.displayText.toLowerCase().includes(q) || s.statId.toLowerCase().includes(q),
			);
		},
		[search],
	);

	// Build combined list with prefix/suffix annotation.
	const families: { family: ModFamily; isPrefix: boolean }[] = [];
	if (filter !== "suffix") {
		for (const f of pool.prefixes) {
			if (matchesSearch(f)) families.push({ family: f, isPrefix: true });
		}
	}
	if (filter !== "prefix") {
		for (const f of pool.suffixes) {
			if (matchesSearch(f)) families.push({ family: f, isPrefix: false });
		}
	}

	// Sort: available first, then taken. Within each group, alphabetical.
	families.sort((a, b) => {
		if (a.family.taken !== b.family.taken) return a.family.taken ? 1 : -1;
		const aName = a.family.tiers[0]?.name ?? "";
		const bName = b.family.tiers[0]?.name ?? "";
		return aName.localeCompare(bName);
	});

	return (
		<div class="browser-mod-picker">
			<div class="picker-header">
				<div class="picker-filters">
					<button
						type="button"
						class={filter === "all" ? "active" : ""}
						onClick={() => setFilter("all")}
					>
						All
					</button>
					<button
						type="button"
						class={filter === "prefix" ? "active" : ""}
						onClick={() => setFilter("prefix")}
					>
						Prefix ({pool.availablePrefixCount})
					</button>
					<button
						type="button"
						class={filter === "suffix" ? "active" : ""}
						onClick={() => setFilter("suffix")}
					>
						Suffix ({pool.availableSuffixCount})
					</button>
				</div>
				<input
					ref={searchRef}
					type="text"
					class="picker-search"
					placeholder="Filter mods..."
					value={search}
					onInput={(e) => setSearch((e.target as HTMLInputElement).value)}
				/>
			</div>

			{isClusterJewel && (
				<div class="picker-cluster-note">
					Showing base mod pool. Enchantment-specific mods not yet available.
				</div>
			)}

			<div class="picker-list">
				{families.map(({ family, isPrefix }) => (
					<ModPickerRow
						key={family.familyId}
						family={family}
						isPrefix={isPrefix}
						canSlot={isPrefix ? canSlotPrefix : canSlotSuffix}
						onSlot={onSlotMod}
					/>
				))}
				{families.length === 0 && (
					<div class="picker-empty">
						{search ? "No mods match your filter." : "No mods available."}
					</div>
				)}
			</div>
		</div>
	);
}
