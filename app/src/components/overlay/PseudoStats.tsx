import { useState } from "preact/hooks";
import type { MappedStat, Rarity, ResolvedMod } from "../../types";
import { Separator } from "./ItemHeader";

interface PseudoEditProps {
	mappedStats: MappedStat[];
	isStatEnabled: (statIndex: number) => boolean;
	getStatMin: (statIndex: number) => number | null;
	getStatMax: (statIndex: number) => number | null;
	toggleStat: (statIndex: number) => void;
	setStatMin: (statIndex: number, min: number | null) => void;
	setStatMax: (statIndex: number, max: number | null) => void;
}

/** Collapsible pseudo stats section. Collapsed by default, auto-expands in edit mode. */
export function PseudoSection({
	mods,
	rarity,
	forceOpen,
	tradeEdit,
}: {
	mods: ResolvedMod[];
	rarity: Rarity;
	forceOpen: boolean;
	tradeEdit?: PseudoEditProps | undefined;
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
							// Find mapped stat for this pseudo by matching display text.
							const mapped = tradeEdit?.mappedStats.find((ms) => ms.displayText === sl.displayText);
							const statIdx = mapped?.statIndex;
							const isMappable = mapped?.tradeId != null;
							const isChecked =
								statIdx != null && tradeEdit ? tradeEdit.isStatEnabled(statIdx) : false;

							return (
								<div key={sl.displayText} class="pseudo-line">
									{tradeEdit && statIdx != null ? (
										<>
											<label class="pseudo-check">
												<input
													type="checkbox"
													checked={isChecked}
													disabled={!isMappable}
													onChange={() => tradeEdit.toggleStat(statIdx)}
												/>
											</label>
											<span class="pseudo-label">{sl.displayText}</span>
											{isChecked && isMappable && (
												<span class="pseudo-edit-values">
													<input
														type="number"
														class="stat-min-input"
														placeholder="min"
														value={tradeEdit.getStatMin(statIdx) ?? ""}
														onInput={(e) => {
															const v = (e.target as HTMLInputElement).value;
															tradeEdit.setStatMin(statIdx, v === "" ? null : Number(v));
														}}
													/>
													<input
														type="number"
														class="stat-max-input"
														placeholder="max"
														value={tradeEdit.getStatMax(statIdx) ?? ""}
														onInput={(e) => {
															const v = (e.target as HTMLInputElement).value;
															tradeEdit.setStatMax(statIdx, v === "" ? null : Number(v));
														}}
													/>
												</span>
											)}
										</>
									) : (
										<>
											<span class="pseudo-label">{sl.displayText}</span>
											<span class="pseudo-value">{value}</span>
										</>
									)}
								</div>
							);
						})}
					</div>
				)}
			</div>
		</>
	);
}
