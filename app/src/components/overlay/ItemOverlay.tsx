import { useState } from "preact/hooks";
import type { EditFilter } from "../../generated/EditFilter";
import type { FilterOverride } from "../../hooks/useTradeFilters";
import type { DangerLevel, MapDangerConfig } from "../../store";
import type {
	ItemEvaluation,
	MappedStat,
	ResolvedItem,
	ScoreInfo,
	TypeSearchScope,
	WatchingScore,
} from "../../types";
import { ItemHeader, Separator } from "./ItemHeader";
import {
	MapDangerSection,
	ModSection,
	countNonReminderStats,
	influenceDisplay,
	influenceFilterId,
	modText,
} from "./ItemMods";
import {
	InlineFilterCheckbox,
	InlineFilterInput,
	InlineToggleControl,
	findPropertyFilter,
} from "./ItemProperties";
import { SocketFilterRow } from "./ItemSockets";
import { PseudoSection } from "./PseudoStats";

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
	setTypeScope: (scope: TypeSearchScope) => void;
}

/** Lightweight profile summary for the switch pills. */
export interface ProfileSummary {
	id: string;
	name: string;
	role: string;
	watchColor: string;
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

			{/* Pseudo stats — collapsible, auto-expands in trade edit mode */}
			{item.pseudoMods && item.pseudoMods.length > 0 && (
				<PseudoSection mods={item.pseudoMods} rarity={rarity} forceOpen={!!tradeEdit} />
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
