import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "preact/hooks";
import type {
	MappedStat,
	QueryBuildResult,
	StatFilterOverride,
	TradeFilterConfig,
	TradeQueryConfig,
	TypeSearchScope,
} from "../types";

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
	filterConfig: TradeFilterConfig | null;
}

/**
 * Manages trade search filter state for the "Edit Search" mode.
 *
 * Calls `preview_trade_query` (no HTTP) to discover which stats are mappable
 * and their default min values, then lets the user toggle/adjust them.
 */
export function useTradeFilters(itemText: string, config: TradeQueryConfig): TradeFilters {
	const [editMode, setEditMode] = useState(false);
	const [mappedStats, setMappedStats] = useState<MappedStat[]>([]);
	const [typeScope, setTypeScope] = useState<TypeSearchScope>("baseType");
	const [statOverrides, setStatOverrides] = useState<Map<number, StatFilterOverride>>(new Map());

	// Reset when item changes
	// biome-ignore lint/correctness/useExhaustiveDependencies: itemText change triggers reset intentionally
	useEffect(() => {
		setEditMode(false);
		setMappedStats([]);
		setStatOverrides(new Map());
		setTypeScope("baseType");
	}, [itemText]);

	const toggleEditMode = useCallback(async () => {
		if (!editMode) {
			try {
				const result = await invoke<QueryBuildResult>("preview_trade_query", {
					itemText,
					config,
				});
				setMappedStats(result.mappedStats);
				setStatOverrides(new Map());
				setTypeScope("baseType");
			} catch (e) {
				console.error("Failed to preview trade query:", e);
				return;
			}
		}
		setEditMode((prev) => !prev);
	}, [editMode, itemText, config]);

	const toggleStat = useCallback((statIndex: number) => {
		setStatOverrides((prev) => {
			const next = new Map(prev);
			const existing = next.get(statIndex);
			if (existing) {
				next.set(statIndex, { ...existing, enabled: !existing.enabled });
			} else {
				next.set(statIndex, { statIndex, enabled: false, minOverride: null });
			}
			return next;
		});
	}, []);

	const setStatMin = useCallback((statIndex: number, min: number | null) => {
		setStatOverrides((prev) => {
			const next = new Map(prev);
			const existing = next.get(statIndex);
			if (existing) {
				next.set(statIndex, { ...existing, minOverride: min });
			} else {
				next.set(statIndex, { statIndex, enabled: true, minOverride: min });
			}
			return next;
		});
	}, []);

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

	const filterConfig: TradeFilterConfig | null = editMode
		? {
				typeScope,
				statOverrides: Array.from(statOverrides.values()),
			}
		: null;

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
		filterConfig,
	};
}
