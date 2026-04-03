import type { UniqueCandidate } from "../generated/UniqueCandidate";

interface Props {
	candidates: UniqueCandidate[];
	selected: string | null;
	onSelect: (name: string | null) => void;
}

export function UniqueDisambiguationPicker({ candidates, selected, onSelect }: Props) {
	return (
		<div class="unique-picker">
			<div class="unique-picker-header">Unidentified unique. Which item?</div>
			<div class="unique-picker-list">
				{candidates.map((c) => (
					<button
						type="button"
						key={c.name}
						class={`unique-picker-item ${selected === c.name ? "unique-picker-selected" : ""}`}
						onClick={() => onSelect(c.name)}
					>
						{c.art && <img class="unique-picker-art" src={artUrl(c.art)} alt="" />}
						<span>{c.name}</span>
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

/** Resolve art filename to an importable asset URL. */
function artUrl(filename: string): string {
	// Vite serves files from src/assets/ at build time via the asset pipeline.
	// Using new URL() with import.meta.url enables Vite's asset handling.
	return new URL(`../assets/uniques/${filename}`, import.meta.url).href;
}
