import type { EditFilter } from "../../generated/EditFilter";
import type { FilterOverride } from "../../hooks/useTradeFilters";

/** Alias table for matching property names to schema filter IDs. */
export const PROPERTY_ALIASES: Record<string, string> = {
	"Evasion Rating": "ev",
	"Chance to Block": "block",
	"Energy Shield": "es",
	Armour: "ar",
	Level: "gem_level",
};

/** Look up a schema filter by property name. */
export function findPropertyFilter(
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

/** Compact numeric input for a single value (used by inline property/meta controls). */
export function InlineInput({
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

/** Inline checkbox for a schema filter (left side of line). */
export function InlineFilterCheckbox({
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
export function InlineFilterInput({
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
export function InlineToggleControl({
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
