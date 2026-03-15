/**
 * Schema-driven trade filter controls.
 *
 * Renders filter groups from TradeEditSchema generically:
 * - Range filters → checkbox + number input
 * - Option filters → checkbox + dropdown
 *
 * Zero trade domain knowledge — all labels, types, options, and defaults
 * come from the schema (which poe-trade computes from filters.json + item data).
 */

import { useState } from "preact/hooks";
import type { EditFilter } from "../generated/EditFilter";
import type { EditFilterGroup } from "../generated/EditFilterGroup";
import type { TradeEditSchema } from "../generated/TradeEditSchema";

/** User's override for a single filter. */
export interface FilterOverride {
	enabled: boolean;
	/** For range filters: the min value. */
	rangeMin?: number | null;
	/** For option filters: the selected option ID. */
	selectedId?: string | null;
}

interface SchemaFiltersProps {
	schema: TradeEditSchema;
	overrides: Map<string, FilterOverride>;
	onOverride: (filterId: string, override: FilterOverride) => void;
}

export function SchemaFilters({ schema, overrides, onOverride }: SchemaFiltersProps) {
	return (
		<div class="schema-filters">
			{schema.filterGroups.map((group) => (
				<FilterGroup key={group.id} group={group} overrides={overrides} onOverride={onOverride} />
			))}
		</div>
	);
}

function FilterGroup({
	group,
	overrides,
	onOverride,
}: {
	group: EditFilterGroup;
	overrides: Map<string, FilterOverride>;
	onOverride: (filterId: string, override: FilterOverride) => void;
}) {
	const [collapsed, setCollapsed] = useState(true);

	// Only show filters that have a default value (applicable to this item)
	const applicable = group.filters.filter((f) => f.defaultValue !== null || f.enabled);
	if (applicable.length === 0) return null;

	const activeCount = applicable.filter((f) => {
		const ov = overrides.get(f.id);
		return ov ? ov.enabled : f.enabled;
	}).length;

	return (
		<div class="filter-group">
			<button type="button" class="filter-group-header" onClick={() => setCollapsed(!collapsed)}>
				<span class="filter-group-chevron">{collapsed ? "\u25b8" : "\u25be"}</span>
				<span class="filter-group-title">{group.title}</span>
				{activeCount > 0 && <span class="filter-group-badge">{activeCount}</span>}
			</button>
			{!collapsed && (
				<div class="filter-group-body">
					{applicable.map((filter) => (
						<SchemaFilterRow
							key={filter.id}
							filter={filter}
							override={overrides.get(filter.id)}
							onOverride={(ov) => onOverride(filter.id, ov)}
						/>
					))}
				</div>
			)}
		</div>
	);
}

function SchemaFilterRow({
	filter,
	override,
	onOverride,
}: {
	filter: EditFilter;
	override: FilterOverride | undefined;
	onOverride: (ov: FilterOverride) => void;
}) {
	const enabled = override ? override.enabled : filter.enabled;

	if (filter.kind.type === "range") {
		const defaultMin = filter.defaultValue?.type === "range" ? filter.defaultValue.min : null;
		const currentMin = override?.rangeMin ?? defaultMin;

		return (
			<label class="socket-filter-row" title={filter.tip ?? undefined}>
				<input
					type="checkbox"
					checked={enabled}
					onChange={() => onOverride({ ...override, enabled: !enabled, rangeMin: currentMin })}
				/>
				<span class="socket-filter-label">{filter.text}</span>
				<input
					type="number"
					class="socket-filter-input"
					value={currentMin ?? ""}
					disabled={!enabled}
					onInput={(e) => {
						const v = Number.parseFloat((e.target as HTMLInputElement).value);
						onOverride({
							enabled,
							rangeMin: Number.isNaN(v) ? null : v,
						});
					}}
				/>
				{defaultMin !== null && <span class="socket-filter-hint">{formatDefault(defaultMin)}</span>}
			</label>
		);
	}

	if (filter.kind.type === "option") {
		const defaultId = filter.defaultValue?.type === "selected" ? filter.defaultValue.id : null;
		const currentId = override?.selectedId !== undefined ? override.selectedId : defaultId;

		// For simple yes/no/any filters (3 options), render as checkbox
		if (filter.kind.options.length <= 3) {
			const isYes = currentId === "true";
			return (
				<label class="socket-filter-row" title={filter.tip ?? undefined}>
					<input
						type="checkbox"
						checked={enabled && isYes}
						onChange={() => {
							if (enabled && isYes) {
								// Uncheck: disable the filter
								onOverride({ enabled: false, selectedId: null });
							} else {
								// Check: enable with "true"
								onOverride({ enabled: true, selectedId: "true" });
							}
						}}
					/>
					<span class="socket-filter-label">{filter.text}</span>
				</label>
			);
		}

		// For multi-option filters, render as dropdown
		return (
			<div class="socket-filter-row" title={filter.tip ?? undefined}>
				<span class="socket-filter-label">{filter.text}</span>
				<select
					class="socket-filter-select"
					value={currentId ?? ""}
					onChange={(e) => {
						const val = (e.target as HTMLSelectElement).value;
						onOverride({
							enabled: val !== "",
							selectedId: val || null,
						});
					}}
				>
					{filter.kind.options.map((opt) => (
						<option key={opt.id ?? "__any__"} value={opt.id ?? ""}>
							{opt.text}
						</option>
					))}
				</select>
			</div>
		);
	}

	return null;
}

function formatDefault(value: number): string {
	if (Number.isInteger(value)) return String(value);
	return value.toFixed(1);
}
