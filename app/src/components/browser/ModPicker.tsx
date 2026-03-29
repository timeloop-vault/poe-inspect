import { useCallback, useRef, useState } from "preact/hooks";
import type { ModFamily, ModPoolResult, ModTier, ModTierStat } from "./useItemBuilder";

function formatStatRange(s: ModTierStat): string {
	if (s.min === s.max) return s.displayText.replace("#", `${s.min}`);
	return s.displayText.replace("#", `(${s.min}-${s.max})`);
}

/** Summary stat line for the family header — uses the best tier's range. */
function familyStatSummary(tier: ModTier): string {
	return tier.stats.map((s) => formatStatRange(s)).join(", ");
}

/** Compact tier range — just the numbers, no template text. */
function tierValueRange(tier: ModTier): string {
	return tier.stats.map((s) => (s.min === s.max ? `${s.min}` : `${s.min}-${s.max}`)).join(", ");
}

function ModFamilyRow({
	family,
	isPrefix,
	canSlot,
	expanded,
	onToggle,
	onSlotTier,
}: {
	family: ModFamily;
	isPrefix: boolean;
	canSlot: boolean;
	expanded: boolean;
	onToggle: () => void;
	onSlotTier: (family: ModFamily, tier: ModTier, isPrefix: boolean) => void;
}) {
	const bestTier = family.tiers.find((t) => t.eligible) ?? family.tiers[0];
	if (!bestTier) return null;

	const disabled = family.taken || !canSlot;

	return (
		<div class={`picker-family ${disabled ? "disabled" : ""} ${family.taken ? "taken" : ""}`}>
			{/* Family header — shows stat description, click to expand tiers */}
			<button type="button" class="picker-family-header" onClick={onToggle} disabled={disabled}>
				<span class={`picker-slot-badge ${isPrefix ? "prefix" : "suffix"}`}>
					{isPrefix ? "P" : "S"}
				</span>
				<span class="picker-family-stats">{familyStatSummary(bestTier)}</span>
				<span class="picker-family-meta">
					{family.tiers.length}T &middot; ilvl {bestTier.requiredLevel}
				</span>
				<span class="picker-expand">{expanded ? "\u25B2" : "\u25BC"}</span>
			</button>

			{/* Expanded tier list */}
			{expanded && (
				<div class="picker-tiers">
					{family.tiers.map((t) => (
						<button
							type="button"
							key={t.modId}
							class={`picker-tier-row ${t.eligible ? "" : "ineligible"}`}
							onClick={() => {
								if (t.eligible && !disabled) onSlotTier(family, t, isPrefix);
							}}
							disabled={!t.eligible || disabled}
						>
							<span class="picker-tier-num">T{t.tier}</span>
							<span class="picker-tier-values">{tierValueRange(t)}</span>
							<span class="picker-tier-meta">
								ilvl {t.requiredLevel} &middot; w:{t.spawnWeight}
							</span>
						</button>
					))}
				</div>
			)}
		</div>
	);
}

export function ModPicker({
	pool,
	slottedPrefixCount,
	slottedSuffixCount,
	maxPrefixes,
	maxSuffixes,
	onSlotTier,
	isClusterJewel,
}: {
	pool: ModPoolResult;
	slottedPrefixCount: number;
	slottedSuffixCount: number;
	maxPrefixes: number;
	maxSuffixes: number;
	onSlotTier: (family: ModFamily, tier: ModTier, isPrefix: boolean) => void;
	isClusterJewel: boolean;
}) {
	const [filter, setFilter] = useState<"all" | "prefix" | "suffix">("all");
	const [search, setSearch] = useState("");
	const [expandedFamily, setExpandedFamily] = useState<string | null>(null);
	const searchRef = useRef<HTMLInputElement>(null);

	const canSlotPrefix = slottedPrefixCount < maxPrefixes;
	const canSlotSuffix = slottedSuffixCount < maxSuffixes;

	const matchesSearch = useCallback(
		(family: ModFamily): boolean => {
			if (!search) return true;
			const q = search.toLowerCase();
			const bestTier = family.tiers.find((t) => t.eligible) ?? family.tiers[0];
			if (!bestTier) return false;
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

	// Sort by stat description text (not mod name).
	families.sort((a, b) => {
		if (a.family.taken !== b.family.taken) return a.family.taken ? 1 : -1;
		const aText = a.family.tiers[0]?.stats[0]?.displayText ?? "";
		const bText = b.family.tiers[0]?.stats[0]?.displayText ?? "";
		return aText.localeCompare(bText);
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
					<ModFamilyRow
						key={family.familyId}
						family={family}
						isPrefix={isPrefix}
						canSlot={isPrefix ? canSlotPrefix : canSlotSuffix}
						expanded={expandedFamily === family.familyId}
						onToggle={() =>
							setExpandedFamily((prev) => (prev === family.familyId ? null : family.familyId))
						}
						onSlotTier={onSlotTier}
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
