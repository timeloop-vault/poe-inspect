interface Props {
	candidates: string[];
	selected: string | null;
	onSelect: (name: string | null) => void;
}

export function UniqueDisambiguationPicker({ candidates, selected, onSelect }: Props) {
	return (
		<div class="unique-picker">
			<div class="unique-picker-header">Unidentified unique. Which item?</div>
			<div class="unique-picker-list">
				{candidates.map((name) => (
					<button
						type="button"
						key={name}
						class={`unique-picker-item ${selected === name ? "unique-picker-selected" : ""}`}
						onClick={() => onSelect(name)}
					>
						{name}
					</button>
				))}
				<button
					type="button"
					class={`unique-picker-item unique-picker-any ${selected === "" ? "unique-picker-selected" : ""}`}
					onClick={() => onSelect("")}
				>
					Search any unidentified
				</button>
			</div>
		</div>
	);
}
