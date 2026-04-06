import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "preact/hooks";
import type { EditFilter } from "../generated/EditFilter";
import type { TradeEditSchema } from "../generated/TradeEditSchema";
import type { UniqueCandidate } from "../generated/UniqueCandidate";
import type {
	MappedStat,
	QueryBuildResult,
	SocketInfo,
	StatFilterOverride,
	TradeFilterConfig,
	TradeQueryConfig,
	TypeSearchScope,
} from "../types";

/** User's override for a single schema filter. */
export interface FilterOverride {
	enabled: boolean;
	/** For range filters: the min value. */
	rangeMin?: number | null;
	/** For option filters: the selected option ID. */
	selectedId?: string | null;
	/** For socket filters: per-color counts + total min/max. */
	socketRed?: number | null;
	socketGreen?: number | null;
	socketBlue?: number | null;
	socketWhite?: number | null;
	socketMin?: number | null;
	socketMax?: number | null;
}

export interface TradeFilters {
	editMode: boolean;
	toggleEditMode: () => void;
	mappedStats: MappedStat[];
	typeScope: TypeSearchScope;
	setTypeScope: (scope: TypeSearchScope) => void;
	toggleStat: (statIndex: number) => void;
	setStatMin: (statIndex: number, min: number | null) => void;
	isStatEnabled: (statIndex: number) => boolean;
	getStatMin: (statIndex: number) => number | null;
	getStatMax: (statIndex: number) => number | null;
	setStatMax: (statIndex: number, max: number | null) => void;
	socketInfo: SocketInfo | null;
	quality: number | null;
	filterConfig: TradeFilterConfig | null;
	/** Schema for structural filters (ilvl, links, corrupted, rarity, etc.) */
	editSchema: TradeEditSchema | null;
	/** User overrides for schema filters, keyed by filter ID. */
	filterOverrides: Map<string, FilterOverride>;
	/** Callback to update a schema filter override. */
	onFilterOverride: (filterId: string, override: FilterOverride) => void;
	/** Flat map of all schema filters by ID (for inline lookups). */
	filterMap: Map<string, EditFilter>;
	/** Rarity filter from the schema (if applicable). */
	rarityFilter: EditFilter | null;
	/** Whether an unidentified unique needs the user to pick which unique it is. */
	needsDisambiguation: boolean;
	/** Possible uniques for this base type (empty when not applicable). */
	uniqueCandidates: UniqueCandidate[];
	/** The user's selected unique name (null = not yet selected). */
	selectedUniqueName: string | null;
	/** Set the selected unique name for disambiguation. */
	setSelectedUniqueName: (name: string | null) => void;
}

/**
 * Manages trade search filter state for the "Edit Search" mode.
 *
 * Calls `preview_trade_query` (no HTTP) to discover which stats are mappable
 * and their default min values, then lets the user toggle/adjust them.
 * Also fetches the TradeEditSchema for structural filters (ilvl, links, etc.).
 */
export function useTradeFilters(
	itemText: string,
	config: TradeQueryConfig,
	autoEdit?: boolean,
	itemUniqueCandidates?: UniqueCandidate[],
): TradeFilters {
	const [editMode, setEditMode] = useState(false);
	const [mappedStats, setMappedStats] = useState<MappedStat[]>([]);
	const [typeScope, setTypeScope] = useState<TypeSearchScope>("baseType");
	const [statOverrides, setStatOverrides] = useState<Map<number, StatFilterOverride>>(new Map());
	const [socketInfo, setSocketInfo] = useState<SocketInfo | null>(null);
	const [quality, setQuality] = useState<number | null>(null);
	const [pendingAutoEdit, setPendingAutoEdit] = useState(false);
	const [uniqueCandidates, setUniqueCandidates] = useState<UniqueCandidate[]>([]);
	const [selectedUniqueName, setSelectedUniqueName] = useState<string | null>(null);

	// Schema-driven filter state (moved from TradePanel)
	const [editSchema, setEditSchema] = useState<TradeEditSchema | null>(null);
	const [filterOverrides, setFilterOverrides] = useState<Map<string, FilterOverride>>(new Map());

	// Reset when item changes; queue auto-edit if requested
	// biome-ignore lint/correctness/useExhaustiveDependencies: itemText change triggers reset intentionally
	useEffect(() => {
		setEditMode(false);
		setMappedStats([]);
		setStatOverrides(new Map());
		setTypeScope("baseType");
		setSocketInfo(null);
		setQuality(null);
		setEditSchema(null);
		setFilterOverrides(new Map());
		setUniqueCandidates(itemUniqueCandidates ?? []);
		setSelectedUniqueName(null);
		if (autoEdit && itemText) {
			setPendingAutoEdit(true);
		}
	}, [itemText]);

	// Fetch schema when entering edit mode
	// biome-ignore lint/correctness/useExhaustiveDependencies: intentional — fetch on edit mode change
	useEffect(() => {
		if (!editMode || !itemText) {
			setEditSchema(null);
			setFilterOverrides(new Map());
			return;
		}
		(async () => {
			try {
				const schema = await invoke<TradeEditSchema>("get_trade_edit_schema", {
					itemText,
					config,
				});
				setEditSchema(schema);
				// Initialize overrides from schema defaults
				const overrides = new Map<string, FilterOverride>();
				for (const group of schema.filterGroups) {
					for (const filter of group.filters) {
						if (filter.defaultValue) {
							const ov: FilterOverride = { enabled: filter.enabled };
							if (filter.defaultValue.type === "range") {
								ov.rangeMin = filter.defaultValue.min;
							} else if (filter.defaultValue.type === "selected") {
								ov.selectedId = filter.defaultValue.id;
							} else if (filter.defaultValue.type === "sockets") {
								ov.socketRed = filter.defaultValue.red;
								ov.socketGreen = filter.defaultValue.green;
								ov.socketBlue = filter.defaultValue.blue;
								ov.socketWhite = filter.defaultValue.white;
								ov.socketMin = filter.defaultValue.min;
								ov.socketMax = filter.defaultValue.max;
							}
							overrides.set(filter.id, ov);
						}
					}
				}
				setFilterOverrides(overrides);
			} catch (e) {
				console.error("Failed to fetch trade edit schema:", e);
			}
		})();
	}, [editMode, itemText]);

	/** Initialize stat + socket + quality state from a preview result. */
	const initFromPreview = useCallback((result: QueryBuildResult) => {
		setMappedStats(result.mappedStats);
		setStatOverrides(new Map());
		setTypeScope("baseType");
		setSocketInfo(result.socketInfo);
		setQuality(result.quality);
	}, []);

	// Auto-enter edit mode when triggered by trade-inspect hotkey
	useEffect(() => {
		if (!pendingAutoEdit || !itemText) return;
		setPendingAutoEdit(false);

		(async () => {
			try {
				const result = await invoke<QueryBuildResult>("preview_trade_query", {
					itemText,
					config,
				});
				initFromPreview(result);
				setEditMode(true);
			} catch (e) {
				console.error("Failed to auto-enter trade edit:", e);
			}
		})();
	}, [pendingAutoEdit, itemText, config, initFromPreview]);

	const toggleEditMode = useCallback(async () => {
		if (!editMode) {
			try {
				const result = await invoke<QueryBuildResult>("preview_trade_query", {
					itemText,
					config,
				});
				initFromPreview(result);
			} catch (e) {
				console.error("Failed to preview trade query:", e);
				return;
			}
		}
		setEditMode((prev) => !prev);
	}, [editMode, itemText, config, initFromPreview]);

	const toggleStat = useCallback(
		(statIndex: number) => {
			setStatOverrides((prev) => {
				const next = new Map(prev);
				const existing = next.get(statIndex);
				if (existing) {
					next.set(statIndex, { ...existing, enabled: !existing.enabled });
				} else {
					// No override yet — toggle from the effective default (mappedStats.included)
					const currentlyEnabled =
						mappedStats.find((s) => s.statIndex === statIndex)?.included ?? false;
					next.set(statIndex, {
						statIndex,
						enabled: !currentlyEnabled,
						minOverride: null,
						maxOverride: null,
					});
				}
				return next;
			});
		},
		[mappedStats],
	);

	const setStatMin = useCallback((statIndex: number, min: number | null) => {
		setStatOverrides((prev) => {
			const next = new Map(prev);
			const existing = next.get(statIndex);
			if (existing) {
				next.set(statIndex, { ...existing, minOverride: min });
			} else {
				next.set(statIndex, { statIndex, enabled: true, minOverride: min, maxOverride: null });
			}
			return next;
		});
	}, []);

	const setStatMax = useCallback((statIndex: number, max: number | null) => {
		setStatOverrides((prev) => {
			const next = new Map(prev);
			const existing = next.get(statIndex);
			if (existing) {
				next.set(statIndex, { ...existing, maxOverride: max });
			} else {
				next.set(statIndex, { statIndex, enabled: true, minOverride: null, maxOverride: max });
			}
			return next;
		});
	}, []);

	const getStatMax = useCallback(
		(statIndex: number): number | null => {
			return statOverrides.get(statIndex)?.maxOverride ?? null;
		},
		[statOverrides],
	);

	const isStatEnabled = useCallback(
		(statIndex: number): boolean => {
			const override = statOverrides.get(statIndex);
			if (override) return override.enabled;
			const mapped = mappedStats.find((s) => s.statIndex === statIndex);
			return mapped?.included ?? false;
		},
		[statOverrides, mappedStats],
	);

	const getStatMin = useCallback(
		(statIndex: number): number | null => {
			const override = statOverrides.get(statIndex);
			if (override?.minOverride != null) return override.minOverride;
			const mapped = mappedStats.find((s) => s.statIndex === statIndex);
			return mapped?.computedMin ?? null;
		},
		[statOverrides, mappedStats],
	);

	const onFilterOverride = useCallback((filterId: string, override: FilterOverride) => {
		setFilterOverrides((prev) => {
			const next = new Map(prev);
			next.set(filterId, override);
			return next;
		});
	}, []);

	// Build filter map from schema (flat lookup by filter ID)
	const filterMap = new Map<string, EditFilter>();
	if (editSchema) {
		for (const group of editSchema.filterGroups) {
			for (const filter of group.filters) {
				filterMap.set(filter.id, filter);
			}
		}
	}

	// Find the rarity filter
	const rarityFilter = filterMap.get("rarity") ?? null;

	// Translate state into TradeFilterConfig for Rust.
	// Build a config when in edit mode OR when a unique name is selected
	// (disambiguation override must reach the query builder even without edit mode).
	const hasUniqueOverride = selectedUniqueName != null;
	const filterConfig: TradeFilterConfig | null =
		editMode || hasUniqueOverride
			? buildFilterConfig(typeScope, statOverrides, filterOverrides, filterMap, selectedUniqueName)
			: null;

	const needsDisambiguation = uniqueCandidates.length > 0 && selectedUniqueName === null;

	return {
		editMode,
		toggleEditMode,
		mappedStats,
		typeScope,
		setTypeScope,
		toggleStat,
		setStatMin,
		isStatEnabled,
		getStatMin,
		getStatMax,
		setStatMax,
		socketInfo,
		quality,
		filterConfig,
		editSchema,
		filterOverrides,
		onFilterOverride,
		filterMap,
		rarityFilter,
		needsDisambiguation,
		uniqueCandidates,
		selectedUniqueName,
		setSelectedUniqueName,
	};
}

/**
 * Translate generic filter overrides back to TradeFilterConfig
 * (adapter — keeps Rust unchanged for now).
 */
function buildFilterConfig(
	typeScope: TypeSearchScope,
	statOverrides: Map<number, StatFilterOverride>,
	schemaOverrides: Map<string, FilterOverride>,
	filterMap: Map<string, EditFilter>,
	selectedUniqueName: string | null,
): TradeFilterConfig {
	// Links (socket-type filter — use socketMin)
	const linksOv = schemaOverrides.get("links");
	const linksFilter = filterMap.get("links");
	const linksEnabled = linksOv ? linksOv.enabled : (linksFilter?.enabled ?? false);
	const linksDefault =
		linksFilter?.defaultValue?.type === "sockets"
			? linksFilter.defaultValue.min
			: linksFilter?.defaultValue?.type === "range"
				? linksFilter.defaultValue.min
				: null;
	const linksMin = linksOv?.socketMin ?? linksOv?.rangeMin ?? linksDefault;

	// Quality
	const qualityOv = schemaOverrides.get("quality");
	const qualityFilter = filterMap.get("quality");
	const qualityEnabled = qualityOv ? qualityOv.enabled : (qualityFilter?.enabled ?? false);
	const qualityDefault =
		qualityFilter?.defaultValue?.type === "range" ? qualityFilter.defaultValue.min : null;
	const qualityMin = qualityOv?.rangeMin ?? qualityDefault;

	// Rarity
	const rarityOv = schemaOverrides.get("rarity");
	const rarityFilter = filterMap.get("rarity");
	let rarityOverride: string | null = null;
	if (rarityOv) {
		rarityOverride = rarityOv.selectedId ?? null;
	} else if (rarityFilter?.defaultValue?.type === "selected") {
		rarityOverride = rarityFilter.defaultValue.id;
	}

	// Item level
	const ilvlOv = schemaOverrides.get("ilvl");
	const ilvlFilter = filterMap.get("ilvl");
	const ilvlEnabled = ilvlOv ? ilvlOv.enabled : (ilvlFilter?.enabled ?? false);
	const ilvlDefault =
		ilvlFilter?.defaultValue?.type === "range" ? ilvlFilter.defaultValue.min : null;
	const ilvlMin = ilvlOv?.rangeMin ?? ilvlDefault;

	// Corrupted
	const corruptedOv = schemaOverrides.get("corrupted");
	const corruptedFilter = filterMap.get("corrupted");
	let corruptedOverride: boolean | null = null;
	if (corruptedOv) {
		corruptedOverride =
			corruptedOv.enabled && corruptedOv.selectedId === "true"
				? true
				: corruptedOv.enabled
					? null
					: false;
	} else if (corruptedFilter?.enabled) {
		corruptedOverride = true;
	}

	// Fractured
	const fracturedOv = schemaOverrides.get("fractured_item");
	const fracturedFilter = filterMap.get("fractured_item");
	let fracturedOverride: boolean | null = null;
	if (fracturedOv) {
		fracturedOverride =
			fracturedOv.enabled && fracturedOv.selectedId === "true"
				? true
				: fracturedOv.enabled
					? null
					: false;
	} else if (fracturedFilter?.enabled) {
		fracturedOverride = true;
	}

	return {
		typeScope,
		statOverrides: Array.from(statOverrides.values()),
		minLinksEnabled: linksEnabled,
		minLinks: linksMin != null ? Math.round(linksMin) : null,
		qualityEnabled,
		qualityMin: qualityMin != null ? Math.round(qualityMin) : null,
		rarityOverride,
		ilvlEnabled,
		ilvlMin: ilvlMin != null ? Math.round(ilvlMin) : null,
		corruptedOverride,
		fracturedOverride,
		uniqueNameOverride: selectedUniqueName,
	};
}
