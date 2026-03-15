import { useCallback, useState } from "preact/hooks";
import type { EditFilter } from "../generated/EditFilter";
import type { FilterOverride } from "../hooks/useTradeFilters";
import type { DangerLevel, MapDangerConfig } from "../store";
import type {
	ItemEvaluation,
	MappedStat,
	ModTierResult,
	ModType,
	Rarity,
	ResolvedItem,
	ResolvedMod,
	ScoreInfo,
	TierQuality,
	WatchingScore,
} from "../types";

// Tooltip header sprites (left, middle, right) per rarity/item type
import headerCurrencyLeft from "../assets/tooltip/header-currency-left.webp";
import headerCurrencyMiddle from "../assets/tooltip/header-currency-middle.webp";
import headerCurrencyRight from "../assets/tooltip/header-currency-right.webp";
import headerGemLeft from "../assets/tooltip/header-gem-left.webp";
import headerGemMiddle from "../assets/tooltip/header-gem-middle.webp";
import headerGemRight from "../assets/tooltip/header-gem-right.webp";
import headerMagicLeft from "../assets/tooltip/header-magic-left.webp";
import headerMagicMiddle from "../assets/tooltip/header-magic-middle.webp";
import headerMagicRight from "../assets/tooltip/header-magic-right.webp";
import headerNormalLeft from "../assets/tooltip/header-normal-left.webp";
import headerNormalMiddle from "../assets/tooltip/header-normal-middle.webp";
import headerNormalRight from "../assets/tooltip/header-normal-right.webp";
import headerRareLeft from "../assets/tooltip/header-rare-left.webp";
import headerRareMiddle from "../assets/tooltip/header-rare-middle.webp";
import headerRareRight from "../assets/tooltip/header-rare-right.webp";
import headerUniqueLeft from "../assets/tooltip/header-unique-left.webp";
import headerUniqueMiddle from "../assets/tooltip/header-unique-middle.webp";
import headerUniqueRight from "../assets/tooltip/header-unique-right.webp";

// Tooltip separator sprites per rarity/item type
import separatorCurrency from "../assets/tooltip/separator-currency.webp";
import separatorGem from "../assets/tooltip/separator-gem.webp";
import separatorMagic from "../assets/tooltip/separator-magic.webp";
import separatorNormal from "../assets/tooltip/separator-normal.webp";
import separatorRare from "../assets/tooltip/separator-rare.webp";
import separatorUnique from "../assets/tooltip/separator-unique.webp";

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
		default:
			return "var(--rarity-normal)";
	}
}

// ── Mod display helpers ─────────────────────────────────────────────────────

/** Build display text from stat lines (filter out reminder text). */
function modText(mod: ResolvedMod): string {
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
function influenceDisplay(kind: string): string {
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
function influenceFilterId(display: string): string | null {
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

function Separator({ rarity }: { rarity: Rarity }) {
	return (
		<div class="item-separator">
			<img src={separatorSprites[rarity] ?? defaultSeparator} alt="" class="separator-img" />
		</div>
	);
}

/** PoE-style item header with left cap, tiling middle, right cap.
 *  In trade edit mode, integrates type scope selector and rarity dropdown. */
function ItemHeader({
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
		const options = filter.kind.options;
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
								onChange={(e) => tradeEdit.setTypeScope((e.target as HTMLSelectElement).value)}
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
							onChange={(e) => tradeEdit.setTypeScope((e.target as HTMLSelectElement).value)}
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

export interface DisplaySettings {
	showRollBars: boolean;
	showTierBadges: boolean;
	showTypeBadges: boolean;
	showOpenAffixes: boolean;
	showStatIds: boolean;
}

export const defaultDisplay: DisplaySettings = {
	showRollBars: true,
	showTierBadges: true,
	showTypeBadges: true,
	showOpenAffixes: true,
	showStatIds: false,
};

/** Trade edit mode props for a single mod line. */
interface ModTradeEdit {
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

function ModLine({
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

/** Compact numeric input for a single value (used by inline property/meta controls). */
function InlineInput({
	value,
	onChange,
	disabled,
}: { value: number | null; onChange: (v: number | null) => void; disabled?: boolean }) {
	return (
		<input
			type="number"
			class="socket-filter-input"
			value={value != null ? Math.round(value) : ""}
			disabled={disabled}
			onInput={(e) => {
				const raw = (e.target as HTMLInputElement).value;
				onChange(raw === "" ? null : Number(raw));
			}}
			onClick={(e) => (e.target as HTMLInputElement).select()}
		/>
	);
}

/** Alias table for matching property names to schema filter IDs. */
const PROPERTY_ALIASES: Record<string, string> = {
	"Evasion Rating": "ev",
	"Chance to Block": "block",
	"Energy Shield": "es",
	Armour: "ar",
};

/** Look up a schema filter by property name. */
function findPropertyFilter(
	propName: string,
	filterMap: Map<string, EditFilter>,
): EditFilter | null {
	// Try alias first, then lowercase property name
	const aliasId = PROPERTY_ALIASES[propName];
	if (aliasId) {
		const f = filterMap.get(aliasId);
		if (f) return f;
	}
	// Direct match by filter text (case-insensitive)
	for (const filter of filterMap.values()) {
		if (filter.text.toLowerCase() === propName.toLowerCase()) {
			return filter;
		}
	}
	return null;
}

/** Inline checkbox for a schema filter (left side of line). */
function InlineFilterCheckbox({
	filter,
	override,
	onOverride,
}: {
	filter: EditFilter;
	override: FilterOverride | undefined;
	onOverride: (filterId: string, ov: FilterOverride) => void;
}) {
	const enabled = override ? override.enabled : filter.enabled;
	const defaultMin = filter.defaultValue?.type === "range" ? filter.defaultValue.min : null;
	const currentMin = override?.rangeMin ?? defaultMin;

	return (
		<span class="inline-filter-checkbox">
			<input
				type="checkbox"
				checked={enabled}
				onChange={() =>
					onOverride(filter.id, {
						...override,
						enabled: !enabled,
						rangeMin: currentMin,
					})
				}
			/>
		</span>
	);
}

/** Inline value input for a schema filter (right side of line). */
function InlineFilterInput({
	filter,
	override,
	onOverride,
}: {
	filter: EditFilter;
	override: FilterOverride | undefined;
	onOverride: (filterId: string, ov: FilterOverride) => void;
}) {
	const enabled = override ? override.enabled : filter.enabled;
	const defaultMin = filter.defaultValue?.type === "range" ? filter.defaultValue.min : null;
	const currentMin = override?.rangeMin ?? defaultMin;

	return (
		<InlineInput
			value={currentMin}
			disabled={!enabled}
			onChange={(v) => onOverride(filter.id, { enabled, rangeMin: v })}
		/>
	);
}

/** Inline checkbox for a boolean/option filter (corrupted, fractured, etc.). */
function InlineToggleControl({
	filter,
	override,
	onOverride,
}: {
	filter: EditFilter;
	override: FilterOverride | undefined;
	onOverride: (filterId: string, ov: FilterOverride) => void;
}) {
	const isYes = override ? override.enabled && override.selectedId === "true" : filter.enabled;

	return (
		<span class="inline-filter-control">
			<input
				type="checkbox"
				checked={isYes}
				onChange={() => {
					if (isYes) {
						onOverride(filter.id, { enabled: false, selectedId: null });
					} else {
						onOverride(filter.id, { enabled: true, selectedId: "true" });
					}
				}}
			/>
		</span>
	);
}

/** Socket-type filter control: R/G/B/W color inputs + min/max. */
function SocketFilterRow({
	filter,
	override: ov,
	onOverride,
}: {
	filter: EditFilter;
	override: FilterOverride | undefined;
	onOverride: (filterId: string, ov: FilterOverride) => void;
}) {
	const defaults = filter.defaultValue?.type === "sockets" ? filter.defaultValue : null;
	const enabled = ov ? ov.enabled : filter.enabled;

	const red = ov?.socketRed ?? defaults?.red ?? null;
	const green = ov?.socketGreen ?? defaults?.green ?? null;
	const blue = ov?.socketBlue ?? defaults?.blue ?? null;
	const white = ov?.socketWhite ?? defaults?.white ?? null;
	const min = ov?.socketMin ?? defaults?.min ?? null;
	const max = ov?.socketMax ?? defaults?.max ?? null;

	const update = (patch: Partial<FilterOverride>) => {
		onOverride(filter.id, {
			enabled,
			socketRed: red,
			socketGreen: green,
			socketBlue: blue,
			socketWhite: white,
			socketMin: min,
			socketMax: max,
			...ov,
			...patch,
		});
	};

	const colorInput = (
		label: string,
		cls: string,
		value: number | null,
		field: keyof FilterOverride,
	) => (
		<label class={`socket-color-cell ${cls}`}>
			<span class="socket-color-label">{label}</span>
			<input
				type="number"
				class="socket-color-input"
				value={value ?? ""}
				disabled={!enabled}
				onInput={(e) => {
					const raw = (e.target as HTMLInputElement).value;
					update({ [field]: raw === "" ? null : Number(raw) });
				}}
				onClick={(e) => (e.target as HTMLInputElement).select()}
			/>
		</label>
	);

	return (
		<div class="socket-filter-row-full">
			<label class="inline-filter-checkbox">
				<input type="checkbox" checked={enabled} onChange={() => update({ enabled: !enabled })} />
			</label>
			<span class="socket-filter-label">{filter.text}</span>
			<div class="socket-color-cells">
				{colorInput("R", "socket-red", red, "socketRed")}
				{colorInput("G", "socket-green", green, "socketGreen")}
				{colorInput("B", "socket-blue", blue, "socketBlue")}
				{colorInput("W", "socket-white", white, "socketWhite")}
			</div>
			<div class="socket-minmax-cells">
				<label class="socket-minmax-cell">
					<span class="socket-minmax-label">min</span>
					<input
						type="number"
						class="socket-filter-input"
						value={min ?? ""}
						disabled={!enabled}
						onInput={(e) => {
							const raw = (e.target as HTMLInputElement).value;
							update({ socketMin: raw === "" ? null : Number(raw) });
						}}
						onClick={(e) => (e.target as HTMLInputElement).select()}
					/>
				</label>
				<label class="socket-minmax-cell">
					<span class="socket-minmax-label">max</span>
					<input
						type="number"
						class="socket-filter-input"
						value={max ?? ""}
						disabled={!enabled}
						onInput={(e) => {
							const raw = (e.target as HTMLInputElement).value;
							update({ socketMax: raw === "" ? null : Number(raw) });
						}}
						onClick={(e) => (e.target as HTMLInputElement).select()}
					/>
				</label>
			</div>
		</div>
	);
}

// ── Default empty evaluation for mock data / no-eval mode ───────────────────

const emptyEval: ItemEvaluation = {
	modTiers: [],
	affixSummary: {
		openPrefixes: 0,
		openSuffixes: 0,
		maxPrefixes: 0,
		maxSuffixes: 0,
		modifiable: false,
	},
	score: null,
	watchingScores: [],
};

const emptyTier: ModTierResult = { tier: null, tierKind: null, quality: null };

/**
 * Render a list of mods with trade edit support.
 * Tracks the running flat stat index across mods.
 */
function ModSection({
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

/** Props for trade edit mode overlay — inline on the item card. */
export interface TradeEditOverlay {
	mappedStats: MappedStat[];
	isStatEnabled: (statIndex: number) => boolean;
	getStatMin: (statIndex: number) => number | null;
	getStatMax: (statIndex: number) => number | null;
	toggleStat: (statIndex: number) => void;
	setStatMin: (statIndex: number, min: number | null) => void;
	setStatMax: (statIndex: number, max: number | null) => void;
	/** All schema filters by ID (for inline property/meta/status controls). */
	filterMap: Map<string, EditFilter>;
	/** Current user overrides for schema filters. */
	filterOverrides: Map<string, FilterOverride>;
	/** Callback to update a schema filter override. */
	onFilterOverride: (filterId: string, override: FilterOverride) => void;
	/** Rarity filter from schema (if applicable). */
	rarityFilter: EditFilter | null;
	/** Type scope options (base type / item class / any). */
	typeScopeOptions: Array<{ scope: string; label: string }>;
	/** Current type scope. */
	typeScope: string;
	/** Set the type scope. */
	setTypeScope: (scope: string) => void;
}

/**
 * Count non-reminder stat lines in a list of mods.
 * This matches the flat_index counting in build_query().
 */
function countNonReminderStats(mods: ResolvedMod[]): number {
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
function buildModTradeEdit(
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

function MapDangerSection({
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

/** Lightweight profile summary for the switch pills. */
export interface ProfileSummary {
	id: string;
	name: string;
	role: string;
	watchColor: string;
}

export function ItemOverlay({
	item,
	eval: evaluation = emptyEval,
	display = defaultDisplay,
	tradeEdit,
	mapDanger,
	onDangerChange,
	profiles,
	onSwitchProfile,
}: {
	item: ResolvedItem;
	eval?: ItemEvaluation;
	display?: DisplaySettings;
	tradeEdit?: TradeEditOverlay | undefined;
	mapDanger?: MapDangerConfig | undefined;
	onDangerChange?: ((template: string, level: DangerLevel | null) => void) | undefined;
	profiles?: ProfileSummary[] | undefined;
	onSwitchProfile?: ((profileId: string) => void) | undefined;
}) {
	const rarity = item.header.rarity;
	const name = item.header.name ?? item.header.baseType;
	const baseType = item.header.baseType;
	const doubleLine = rarity === "Rare" || rarity === "Unique";
	const isMap = item.header.itemClass === "Maps";
	const [swapView, setSwapView] = useState<WatchingScore | null>(null);

	// Split mod tiers to match enchants / implicits / explicits
	const nEnchants = item.enchants.length;
	const nImplicits = item.implicits.length;
	const implicitTiers = evaluation.modTiers.slice(nEnchants, nEnchants + nImplicits);
	const explicitTiers = evaluation.modTiers.slice(nEnchants + nImplicits);

	// Compute flat stat index offsets for each mod section (matches build_query ordering)
	const enchantStatCount = countNonReminderStats(item.enchants);
	const implicitStatOffset = enchantStatCount;
	const explicitStatOffset = implicitStatOffset + countNonReminderStats(item.implicits);

	const affix = evaluation.affixSummary;

	// The active score to display (swapped watching profile or primary)
	const activeScore = swapView?.score ?? evaluation.score;
	const watchingScores = evaluation.watchingScores ?? [];

	// Influence display names (filter out Fractured — it's a separate flag)
	const influences = item.influences
		.filter((i) => i !== "Fractured")
		.map((i) => influenceDisplay(i));

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

			{/* Header with PoE art + edit controls (type scope, rarity) */}
			<ItemHeader
				rarity={rarity}
				name={name}
				baseType={baseType}
				doubleLine={doubleLine}
				tradeEdit={tradeEdit}
			/>

			<Separator rarity={rarity} />

			{/* Properties (filter out synthetic — those are for trade matching only) */}
			{item.properties.filter((p) => !p.synthetic).length > 0 && (
				<>
					<div class="item-properties">
						{item.properties
							.filter((p) => !p.synthetic)
							.map((prop) => {
								const propFilter = tradeEdit
									? findPropertyFilter(prop.name, tradeEdit.filterMap)
									: null;
								return (
									<div
										key={prop.name}
										class={`property-line ${prop.augmented ? "augmented" : ""} ${propFilter && tradeEdit ? "property-line-editable" : ""}`}
									>
										{propFilter && tradeEdit ? (
											<InlineFilterCheckbox
												filter={propFilter}
												override={tradeEdit.filterOverrides.get(propFilter.id)}
												onOverride={tradeEdit.onFilterOverride}
											/>
										) : null}
										<span class="prop-label">{prop.name}: </span>
										<span class="prop-value">{prop.value}</span>
										{propFilter && tradeEdit ? (
											<InlineFilterInput
												filter={propFilter}
												override={tradeEdit.filterOverrides.get(propFilter.id)}
												onOverride={tradeEdit.onFilterOverride}
											/>
										) : null}
									</div>
								);
							})}
					</div>
					<Separator rarity={rarity} />
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
					<Separator rarity={rarity} />
				</>
			)}

			{/* Sockets & Item Level */}
			{(item.sockets || item.itemLevel != null) && (
				<>
					<div class="item-meta">
						{item.sockets && !tradeEdit && <div class="item-sockets">Sockets: {item.sockets}</div>}
						{item.sockets &&
							tradeEdit &&
							(() => {
								const socketsFilter = tradeEdit.filterMap.get("sockets");
								const linksFilter = tradeEdit.filterMap.get("links");
								return (
									<div class="socket-filters-section">
										<div class="item-sockets">Sockets: {item.sockets}</div>
										{socketsFilter && (
											<SocketFilterRow
												filter={socketsFilter}
												override={tradeEdit.filterOverrides.get("sockets")}
												onOverride={tradeEdit.onFilterOverride}
											/>
										)}
										{linksFilter && (
											<SocketFilterRow
												filter={linksFilter}
												override={tradeEdit.filterOverrides.get("links")}
												onOverride={tradeEdit.onFilterOverride}
											/>
										)}
									</div>
								);
							})()}
						{item.itemLevel != null &&
							(() => {
								const ilvlFilter = tradeEdit?.filterMap.get("ilvl");
								return (
									<div class="item-level meta-line-editable">
										{ilvlFilter && tradeEdit ? (
											<InlineFilterCheckbox
												filter={ilvlFilter}
												override={tradeEdit.filterOverrides.get("ilvl")}
												onOverride={tradeEdit.onFilterOverride}
											/>
										) : null}
										<span class="meta-text">Item Level: {item.itemLevel}</span>
										{ilvlFilter && tradeEdit ? (
											<InlineFilterInput
												filter={ilvlFilter}
												override={tradeEdit.filterOverrides.get("ilvl")}
												onOverride={tradeEdit.onFilterOverride}
											/>
										) : null}
									</div>
								);
							})()}
					</div>
					<Separator rarity={rarity} />
				</>
			)}

			{/* Description (currency effects, item instructions, etc.) */}
			{item.description && <div class="item-description">{item.description}</div>}

			{/* Enchants */}
			{item.enchants.length > 0 && (
				<>
					<div class="mod-section enchant-section">
						{item.enchants.map((mod) => (
							<div key={modText(mod)} class="mod-line enchant-line">
								<div class="mod-content">{modText(mod)}</div>
							</div>
						))}
					</div>
					<Separator rarity={rarity} />
				</>
			)}

			{/* Implicits */}
			{item.implicits.length > 0 && (
				<>
					<div class="mod-section implicit-section">
						<ModSection
							mods={item.implicits}
							tiers={implicitTiers}
							display={display}
							tradeEdit={tradeEdit}
							statOffset={implicitStatOffset}
						/>
					</div>
					<Separator rarity={rarity} />
				</>
			)}

			{/* Explicit mods — trade edit supersedes map danger */}
			{item.explicits.length > 0 &&
				(isMap && mapDanger && onDangerChange && !tradeEdit ? (
					<MapDangerSection
						mods={item.explicits}
						mapDanger={mapDanger}
						onDangerChange={onDangerChange}
						rarity={rarity}
					/>
				) : (
					<div class="mod-section explicit-section">
						<ModSection
							mods={item.explicits}
							tiers={explicitTiers}
							display={display}
							tradeEdit={tradeEdit}
							statOffset={explicitStatOffset}
						/>
					</div>
				))}

			{/* Open affixes */}
			{display.showOpenAffixes && (affix.openPrefixes > 0 || affix.openSuffixes > 0) && (
				<>
					<Separator rarity={rarity} />
					<div class="open-affixes">
						{affix.openPrefixes > 0 && (
							<span class="open-prefix">
								{affix.openPrefixes} open prefix
								{affix.openPrefixes > 1 ? "es" : ""}
							</span>
						)}
						{affix.openPrefixes > 0 && affix.openSuffixes > 0 && " · "}
						{affix.openSuffixes > 0 && (
							<span class="open-suffix">
								{affix.openSuffixes} open suffix
								{affix.openSuffixes > 1 ? "es" : ""}
							</span>
						)}
					</div>
				</>
			)}

			{/* Affix count summary */}
			{affix.maxPrefixes > 0 && (
				<div class="affix-summary">
					Prefixes: {affix.maxPrefixes - affix.openPrefixes}/{affix.maxPrefixes} · Suffixes:{" "}
					{affix.maxSuffixes - affix.openSuffixes}/{affix.maxSuffixes}
				</div>
			)}

			{/* Influence tags */}
			{influences.length > 0 && (
				<>
					<Separator rarity={rarity} />
					<div class="influence-tags">
						{influences.map((inf) => {
							const filterId = influenceFilterId(inf);
							const filter = filterId && tradeEdit ? tradeEdit.filterMap.get(filterId) : null;
							return (
								<span key={inf} class="influence-tag">
									{filter && tradeEdit ? (
										<InlineToggleControl
											filter={filter}
											override={tradeEdit.filterOverrides.get(filter.id)}
											onOverride={tradeEdit.onFilterOverride}
										/>
									) : null}
									{inf}
								</span>
							);
						})}
					</div>
				</>
			)}

			{/* Status lines (Corrupted, Fractured) with inline edit controls */}
			{tradeEdit && (item.isCorrupted || item.isFractured) && (
				<>
					<Separator rarity={rarity} />
					<div class="status-lines">
						{item.isCorrupted &&
							(() => {
								const filter = tradeEdit.filterMap.get("corrupted");
								return (
									<div class="status-line">
										{filter ? (
											<InlineToggleControl
												filter={filter}
												override={tradeEdit.filterOverrides.get("corrupted")}
												onOverride={tradeEdit.onFilterOverride}
											/>
										) : null}
										<span class="status-corrupted">Corrupted</span>
									</div>
								);
							})()}
						{item.isFractured &&
							(() => {
								const filter = tradeEdit.filterMap.get("fractured_item");
								return (
									<div class="status-line">
										{filter ? (
											<InlineToggleControl
												filter={filter}
												override={tradeEdit.filterOverrides.get("fractured_item")}
												onOverride={tradeEdit.onFilterOverride}
											/>
										) : null}
										<span class="status-fractured">Fractured Item</span>
									</div>
								);
							})()}
					</div>
				</>
			)}

			{/* Flavor text (uniques) */}
			{item.flavorText && (
				<>
					<Separator rarity={rarity} />
					<div class="flavor-text">{item.flavorText}</div>
				</>
			)}

			{/* Profile score (primary or swapped watching) */}
			{activeScore?.applicable && (
				<>
					<Separator rarity={rarity} />
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

			{/* Profile switch pills — only show when watching profiles have scores */}
			{profiles && profiles.length > 1 && onSwitchProfile && watchingScores.length > 0 && (
				<div class="profile-switch-pills">
					{profiles.map((p) => (
						<button
							key={p.id}
							type="button"
							class={`profile-switch-pill ${p.role === "primary" ? "active" : ""}`}
							style={{ borderColor: p.watchColor }}
							onClick={() => {
								if (p.role !== "primary") onSwitchProfile(p.id);
							}}
							title={p.role === "primary" ? `${p.name} (active)` : `Switch to ${p.name}`}
						>
							<span class="profile-switch-dot" style={{ background: p.watchColor }} />
							{p.name}
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
