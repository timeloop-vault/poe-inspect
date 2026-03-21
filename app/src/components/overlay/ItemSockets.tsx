import type { EditFilter } from "../../generated/EditFilter";
import type { FilterOverride } from "../../hooks/useTradeFilters";

/** Socket-type filter control: R/G/B/W color inputs + min/max. */
export function SocketFilterRow({
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
