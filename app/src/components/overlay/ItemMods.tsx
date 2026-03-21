import { useCallback } from "preact/hooks";
import type { DangerLevel, MapDangerConfig } from "../../store";
import type {
	MappedStat,
	ModTierResult,
	ModType,
	Rarity,
	ResolvedMod,
	TierQuality,
} from "../../types";
import { Separator } from "./ItemHeader";
import type { DisplaySettings, TradeEditOverlay } from "./ItemOverlay";

// ── Mod display helpers ─────────────────────────────────────────────────────

/** Build display text from stat lines (filter out reminder text). */
export function modText(mod: ResolvedMod): string {
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
export function influenceDisplay(kind: string): string {
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

/** Map influence display name to trade API filter ID. */
export function influenceFilterId(display: string): string | null {
	switch (display) {
		case "Shaper":
			return "shaper_item";
		case "Elder":
			return "elder_item";
		case "Crusader":
			return "crusader_item";
		case "Redeemer":
			return "redeemer_item";
		case "Hunter":
			return "hunter_item";
		case "Warlord":
			return "warlord_item";
		case "Searing Exarch":
			return "searing_exarch_item";
		case "Eater of Worlds":
			return "eater_of_worlds_item";
		default:
			return null;
	}
}

// ── Mod trade edit support ──────────────────────────────────────────────────

/** Trade edit mode props for a single mod line. */
export interface ModTradeEdit {
	/** MappedStats for this mod's non-reminder stat lines. */
	stats: MappedStat[];
	/** Whether each stat is enabled in the current filter. */
	isStatEnabled: (statIndex: number) => boolean;
	/** Current min value for each stat. */
	getStatMin: (statIndex: number) => number | null;
	/** Current max value for each stat. */
	getStatMax: (statIndex: number) => number | null;
	/** Toggle a stat on/off. */
	toggleStat: (statIndex: number) => void;
	/** Set the min value for a stat. */
	setStatMin: (statIndex: number, min: number | null) => void;
	/** Set the max value for a stat. */
	setStatMax: (statIndex: number, max: number | null) => void;
}

/** Side-by-side min/max inputs for stat range. */
function StatRangeInputs({
	min,
	max,
	onMinChange,
	onMaxChange,
}: {
	min: number | null;
	max: number | null;
	onMinChange: (v: number | null) => void;
	onMaxChange: (v: number | null) => void;
}) {
	return (
		<div class="stat-range-inputs">
			<input
				type="number"
				class="min-value-input"
				value={min != null ? Math.round(min) : ""}
				placeholder="min"
				onInput={(e) => {
					const raw = (e.target as HTMLInputElement).value;
					onMinChange(raw === "" ? null : Number(raw));
				}}
				onClick={(e) => (e.target as HTMLInputElement).select()}
			/>
			<input
				type="number"
				class="min-value-input"
				value={max != null ? Math.round(max) : ""}
				placeholder="max"
				onInput={(e) => {
					const raw = (e.target as HTMLInputElement).value;
					onMaxChange(raw === "" ? null : Number(raw));
				}}
				onClick={(e) => (e.target as HTMLInputElement).select()}
			/>
		</div>
	);
}

export function ModLine({
	mod,
	tier,
	display,
	tradeEdit,
}: {
	mod: ResolvedMod;
	tier: ModTierResult;
	display: DisplaySettings;
	tradeEdit?: ModTradeEdit | undefined;
}) {
	const quality = rollQuality(mod);
	const typeLabel = modTypeLabel(mod.displayType);
	const qualityCls = mod.displayType === "unique" ? "quality-unique" : qualityClass(tier.quality);
	const isCrafted = mod.header.source === "masterCrafted";
	const statIds = modStatIds(mod);
	const statIdTitle = statIds.length > 0 ? statIds.join(", ") : undefined;

	// In edit mode, show checkbox per mod (uses first stat's enabled state).
	// A mod is "checked" if any of its stats are enabled.
	const editStat = tradeEdit?.stats[0];
	const isEditable = tradeEdit && editStat;
	const isChecked = tradeEdit?.stats.some((s) => tradeEdit.isStatEnabled(s.statIndex)) ?? false;
	const isMappable = tradeEdit?.stats.some((s) => s.tradeId != null) ?? false;

	const handleToggle = useCallback(() => {
		if (!tradeEdit) return;
		for (const s of tradeEdit.stats) {
			tradeEdit.toggleStat(s.statIndex);
		}
	}, [tradeEdit]);

	// Min/max values from first stat that has a trade mapping
	const firstMappedStat = tradeEdit?.stats.find((s) => s.tradeId != null);
	const minValue = firstMappedStat
		? (tradeEdit?.getStatMin(firstMappedStat.statIndex) ?? null)
		: null;
	const maxValue = firstMappedStat
		? (tradeEdit?.getStatMax(firstMappedStat.statIndex) ?? null)
		: null;

	return (
		<div
			class={`mod-line ${qualityCls} ${isEditable && !isChecked ? "mod-line-disabled" : ""}`}
			title={statIdTitle}
		>
			{isEditable && (
				<label class={`mod-checkbox ${!isMappable ? "mod-unmappable" : ""}`}>
					<input
						type="checkbox"
						checked={isChecked}
						disabled={!isMappable}
						onChange={handleToggle}
					/>
				</label>
			)}
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
			{isEditable && isMappable && isChecked ? (
				<StatRangeInputs
					min={minValue}
					max={maxValue}
					onMinChange={(v) => {
						if (firstMappedStat) tradeEdit.setStatMin(firstMappedStat.statIndex, v);
					}}
					onMaxChange={(v) => {
						if (firstMappedStat) tradeEdit.setStatMax(firstMappedStat.statIndex, v);
					}}
				/>
			) : (
				display.showRollBars &&
				quality !== null && (
					<div class="roll-quality" title={`Roll: ${quality}%`}>
						<div class="roll-bar">
							<div
								class={`roll-fill ${quality >= 80 ? "roll-high" : quality >= 50 ? "roll-mid" : "roll-low"}`}
								style={{ width: `${quality}%` }}
							/>
						</div>
						<span class="roll-pct">{quality}%</span>
					</div>
				)
			)}
		</div>
	);
}

/**
 * Count non-reminder stat lines in a list of mods.
 * This matches the flat_index counting in build_query().
 */
export function countNonReminderStats(mods: ResolvedMod[]): number {
	let count = 0;
	for (const mod of mods) {
		for (const sl of mod.statLines) {
			if (!sl.isReminder) count++;
		}
	}
	return count;
}

/**
 * Build ModTradeEdit for a single mod given its starting flat index.
 * Returns the edit props and the number of stat indices consumed.
 */
export function buildModTradeEdit(
	mod: ResolvedMod,
	startIndex: number,
	tradeEdit: TradeEditOverlay,
): { edit: ModTradeEdit; consumed: number } {
	const stats: MappedStat[] = [];
	let idx = startIndex;
	for (const sl of mod.statLines) {
		if (!sl.isReminder) {
			const mapped = tradeEdit.mappedStats.find((s) => s.statIndex === idx);
			if (mapped) stats.push(mapped);
			idx++;
		}
	}
	return {
		edit: {
			stats,
			isStatEnabled: tradeEdit.isStatEnabled,
			getStatMin: tradeEdit.getStatMin,
			getStatMax: tradeEdit.getStatMax,
			toggleStat: tradeEdit.toggleStat,
			setStatMin: tradeEdit.setStatMin,
			setStatMax: tradeEdit.setStatMax,
		},
		consumed: idx - startIndex,
	};
}

export const emptyTier: ModTierResult = { tier: null, tierKind: null, quality: null };

/**
 * Render a list of mods with trade edit support.
 * Tracks the running flat stat index across mods.
 */
export function ModSection({
	mods,
	tiers,
	display,
	tradeEdit,
	statOffset,
}: {
	mods: ResolvedMod[];
	tiers: ModTierResult[];
	display: DisplaySettings;
	tradeEdit?: TradeEditOverlay | undefined;
	statOffset: number;
}) {
	let runningIndex = statOffset;
	return (
		<>
			{mods.map((mod, i) => {
				let modEdit: ModTradeEdit | undefined;
				if (tradeEdit) {
					const result = buildModTradeEdit(mod, runningIndex, tradeEdit);
					modEdit = result.edit;
					runningIndex += result.consumed;
				} else {
					// Still advance the index even without trade edit
					for (const sl of mod.statLines) {
						if (!sl.isReminder) runningIndex++;
					}
				}
				return (
					<ModLine
						key={modText(mod)}
						mod={mod}
						tier={tiers[i] ?? emptyTier}
						display={display}
						tradeEdit={modEdit}
					/>
				);
			})}
		</>
	);
}

// ── Map danger assessment ───────────────────────────────────────────────────

/** Convert display text to template key by replacing numbers with `#`. */
function toTemplateKey(text: string): string {
	return text.replace(/[+-]?\d+(?:\.\d+)?/g, "#");
}

/** Cycle danger level: unclassified → deadly → warning → good → unclassified */
const DANGER_CYCLE: (DangerLevel | null)[] = [null, "deadly", "warning", "good"];

function nextDangerLevel(current: DangerLevel | null): DangerLevel | null {
	const idx = DANGER_CYCLE.indexOf(current);
	return DANGER_CYCLE[(idx + 1) % DANGER_CYCLE.length] ?? null;
}

/** CSS class for danger level coloring. */
function dangerClass(level: DangerLevel | null): string {
	switch (level) {
		case "deadly":
			return "danger-deadly";
		case "warning":
			return "danger-warning";
		case "good":
			return "danger-good";
		default:
			return "danger-unclassified";
	}
}

/** Verdict for the map based on all mod danger levels. */
function mapVerdict(levels: (DangerLevel | null)[]): {
	label: string;
	cls: string;
} {
	if (levels.some((l) => l === "deadly")) {
		return { label: "DEADLY", cls: "danger-deadly" };
	}
	if (levels.some((l) => l === "warning")) {
		return { label: "CAUTION", cls: "danger-warning" };
	}
	if (levels.length > 0 && levels.every((l) => l === "good")) {
		return { label: "SAFE", cls: "danger-good" };
	}
	return { label: "UNRATED", cls: "danger-unclassified" };
}

export function MapDangerSection({
	mods,
	mapDanger,
	onDangerChange,
	rarity,
}: {
	mods: ResolvedMod[];
	mapDanger: MapDangerConfig;
	onDangerChange: (template: string, level: DangerLevel | null) => void;
	rarity: Rarity;
}) {
	const modEntries = mods.map((mod) => {
		const text = modText(mod);
		const template = toTemplateKey(text);
		const level = mapDanger[template] ?? null;
		return { mod, text, template, level };
	});

	const verdict = mapVerdict(modEntries.map((e) => e.level));

	return (
		<div class="map-danger-section">
			<div class={`map-verdict ${verdict.cls}`}>{verdict.label}</div>
			<Separator rarity={rarity} />
			<div class="mod-section">
				{modEntries.map((entry) => (
					// biome-ignore lint/a11y/useKeyWithClickEvents: click-to-cycle is the primary interaction for danger classification
					<div
						key={entry.template}
						class={`mod-line danger-mod-line ${dangerClass(entry.level)}`}
						onClick={() => onDangerChange(entry.template, nextDangerLevel(entry.level))}
						title="Click to cycle danger level"
					>
						<span class={`danger-indicator ${dangerClass(entry.level)}`} />
						<div class="mod-content">
							{entry.text.split("\n").map((line) => (
								<div key={line}>{line}</div>
							))}
						</div>
					</div>
				))}
			</div>
		</div>
	);
}
