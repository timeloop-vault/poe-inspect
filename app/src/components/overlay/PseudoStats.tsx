import { useState } from "preact/hooks";
import type { Rarity, ResolvedMod } from "../../types";
import { Separator } from "./ItemHeader";

/** Collapsible pseudo stats section. Collapsed by default, auto-expands in edit mode. */
export function PseudoSection({
	mods,
	rarity,
	forceOpen,
}: {
	mods: ResolvedMod[];
	rarity: Rarity;
	forceOpen: boolean;
}) {
	const [userOpen, setUserOpen] = useState(false);
	const isOpen = forceOpen || userOpen;

	return (
		<>
			<Separator rarity={rarity} />
			<div class="pseudo-section">
				<button type="button" class="pseudo-header" onClick={() => setUserOpen((v) => !v)}>
					<span class="pseudo-chevron">{isOpen ? "\u25be" : "\u25b8"}</span>
					<span class="pseudo-title">Pseudo</span>
					<span class="pseudo-count">{mods.length}</span>
				</button>
				{isOpen && (
					<div class="pseudo-body">
						{mods.map((mod) => {
							const sl = mod.statLines[0];
							if (!sl) return null;
							const value = sl.values[0]?.current ?? 0;
							return (
								<div key={sl.displayText} class="pseudo-line">
									<span class="pseudo-label">{sl.displayText}</span>
									<span class="pseudo-value">{value}</span>
								</div>
							);
						})}
					</div>
				)}
			</div>
		</>
	);
}
