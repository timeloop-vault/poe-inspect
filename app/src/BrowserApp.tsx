import { useEffect, useState } from "preact/hooks";
import { BrowserTopBar } from "./components/browser/BrowserTopBar";
import { ItemCard } from "./components/browser/ItemCard";
import { ModPicker } from "./components/browser/ModPicker";
import { useItemBuilder } from "./components/browser/useItemBuilder";
import { loadGeneral } from "./store";

export function BrowserApp() {
	const builder = useItemBuilder();
	const [uiScale, setUiScale] = useState(100);

	useEffect(() => {
		document.documentElement.style.background = "rgba(12, 10, 8, 1)";
		document.body.style.background = "rgba(12, 10, 8, 1)";
		loadGeneral().then((s) => setUiScale(s.uiScale));
	}, []);

	useEffect(() => {
		document.documentElement.style.zoom = uiScale !== 100 ? `${uiScale / 100}` : "";
	}, [uiScale]);

	const isClusterJewel = builder.detail?.itemClassName === "Cluster Jewels";

	return (
		<div class="browser-layout">
			<BrowserTopBar
				onSelect={builder.selectBaseType}
				rarity={builder.rarity}
				onRarityChange={builder.setRarity}
				itemLevel={builder.itemLevel}
				onItemLevelChange={builder.setItemLevel}
				onClear={builder.clearAllMods}
				hasDetail={!!builder.detail}
			/>
			<div class="browser-content">
				{builder.detail ? (
					<>
						<ItemCard
							detail={builder.detail}
							rarity={builder.rarity}
							slottedPrefixes={builder.slottedPrefixes}
							slottedSuffixes={builder.slottedSuffixes}
							onUnslot={builder.unslotMod}
						/>
						{builder.pool && (
							<ModPicker
								pool={builder.pool}
								slottedPrefixCount={builder.slottedPrefixes.filter(Boolean).length}
								slottedSuffixCount={builder.slottedSuffixes.filter(Boolean).length}
								maxPrefixes={builder.maxPrefixes}
								maxSuffixes={builder.maxSuffixes}
								onSlotMod={builder.slotMod}
								isClusterJewel={isClusterJewel}
							/>
						)}
					</>
				) : (
					<div class="browser-empty">
						<p>Search for an item to explore its mod pool.</p>
						<p class="browser-empty-hint">Try "Vaal Regalia", "Cobalt Jewel", or "Spine Bow"</p>
					</div>
				)}
			</div>
		</div>
	);
}
