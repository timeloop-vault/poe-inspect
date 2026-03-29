import { CardHeader, CardSeparator } from "./ItemCardHeader";
import type { BaseTypeDetail, BrowserRarity, ModTierStat, SlottedMod } from "./useItemBuilder";

function formatStat(s: ModTierStat): string {
	if (s.min === s.max) return s.displayText.replace("#", `${s.min}`);
	return s.displayText.replace("#", `(${s.min}-${s.max})`);
}

function defVal(min: number, max: number): string {
	return min === max ? `${min}` : `${min}-${max}`;
}

function ModSlot({
	mod,
	isPrefix,
	index,
	onUnslot,
}: {
	mod: SlottedMod | null;
	isPrefix: boolean;
	index: number;
	onUnslot: (isPrefix: boolean, index: number) => void;
}) {
	if (!mod) {
		return (
			<div class={`card-mod-slot empty ${isPrefix ? "prefix" : "suffix"}`}>
				{isPrefix ? "\u2014 Empty Prefix \u2014" : "\u2014 Empty Suffix \u2014"}
			</div>
		);
	}

	return (
		<button
			type="button"
			class={`card-mod-slot filled ${isPrefix ? "prefix" : "suffix"}`}
			onClick={() => onUnslot(isPrefix, index)}
			title="Click to remove"
		>
			<span class="card-mod-badge">
				<span class={`card-mod-type ${isPrefix ? "prefix" : "suffix"}`}>
					{isPrefix ? "P" : "S"}
				</span>
				<span class="card-mod-tier">T{mod.tier}</span>
				<span class="card-mod-name">{mod.name}</span>
			</span>
			<span class="card-mod-stats">
				{mod.stats.map((s) => (
					<span key={s.statId} class="card-stat-line">
						{formatStat(s)}
					</span>
				))}
			</span>
		</button>
	);
}

export function ItemCard({
	detail,
	rarity,
	slottedPrefixes,
	slottedSuffixes,
	onUnslot,
}: {
	detail: BaseTypeDetail;
	rarity: BrowserRarity;
	slottedPrefixes: (SlottedMod | null)[];
	slottedSuffixes: (SlottedMod | null)[];
	onUnslot: (isPrefix: boolean, index: number) => void;
}) {
	const hasProperties =
		detail.defences || detail.weapon || (detail.block !== undefined && detail.block !== null);
	const isClusterJewel = detail.itemClassName === "Cluster Jewels";

	return (
		<div class="browser-item-card">
			<CardHeader rarity={rarity} name={detail.name} baseType={detail.itemClassName} />

			{/* Properties */}
			{hasProperties && (
				<div class="card-properties">
					{detail.defences && (
						<>
							{detail.defences.armourMax > 0 && (
								<div class="card-prop">
									<span class="card-prop-label">Armour: </span>
									<span class="card-prop-value">
										{defVal(detail.defences.armourMin, detail.defences.armourMax)}
									</span>
								</div>
							)}
							{detail.defences.evasionMax > 0 && (
								<div class="card-prop">
									<span class="card-prop-label">Evasion Rating: </span>
									<span class="card-prop-value">
										{defVal(detail.defences.evasionMin, detail.defences.evasionMax)}
									</span>
								</div>
							)}
							{detail.defences.esMax > 0 && (
								<div class="card-prop">
									<span class="card-prop-label">Energy Shield: </span>
									<span class="card-prop-value">
										{defVal(detail.defences.esMin, detail.defences.esMax)}
									</span>
								</div>
							)}
							{detail.defences.wardMax > 0 && (
								<div class="card-prop">
									<span class="card-prop-label">Ward: </span>
									<span class="card-prop-value">
										{defVal(detail.defences.wardMin, detail.defences.wardMax)}
									</span>
								</div>
							)}
						</>
					)}
					{detail.weapon && (
						<>
							<div class="card-prop">
								<span class="card-prop-label">Physical Damage: </span>
								<span class="card-prop-value">
									{detail.weapon.damageMin}-{detail.weapon.damageMax}
								</span>
							</div>
							<div class="card-prop">
								<span class="card-prop-label">Critical Strike Chance: </span>
								<span class="card-prop-value">{(detail.weapon.critical / 100).toFixed(2)}%</span>
							</div>
							<div class="card-prop">
								<span class="card-prop-label">Attacks per Second: </span>
								<span class="card-prop-value">{(1000 / detail.weapon.speed).toFixed(2)}</span>
							</div>
						</>
					)}
					{detail.block !== undefined && detail.block !== null && (
						<div class="card-prop">
							<span class="card-prop-label">Chance to Block: </span>
							<span class="card-prop-value">{detail.block}%</span>
						</div>
					)}
				</div>
			)}

			{/* Implicits */}
			{detail.implicits.length > 0 && (
				<>
					<CardSeparator rarity={rarity} />
					<div class="card-implicits">
						{detail.implicits.map((imp) => (
							<div key={imp} class="card-implicit-line">
								{imp}
							</div>
						))}
					</div>
				</>
			)}

			<CardSeparator rarity={rarity} />

			{/* Cluster jewel warning */}
			{isClusterJewel && (
				<div class="card-cluster-warning">
					Cluster Jewels require an enchantment to determine their mod pool. Enchantment selection
					coming in Phase 2.
				</div>
			)}

			{/* Mod slots */}
			{rarity !== "Normal" && (
				<div class="card-mod-slots">
					{slottedPrefixes.map((mod, i) => (
						<ModSlot
							key={mod?.modId ?? `empty-p${i}`}
							mod={mod}
							isPrefix={true}
							index={i}
							onUnslot={onUnslot}
						/>
					))}
					{slottedSuffixes.map((mod, i) => (
						<ModSlot
							key={mod?.modId ?? `empty-s${i}`}
							mod={mod}
							isPrefix={false}
							index={i}
							onUnslot={onUnslot}
						/>
					))}
				</div>
			)}

			{/* Open mod count */}
			{rarity !== "Normal" && (
				<div class="card-open-mods">
					{slottedPrefixes.filter((m) => !m).length} open prefix
					{slottedPrefixes.filter((m) => !m).length !== 1 ? "es" : ""},{" "}
					{slottedSuffixes.filter((m) => !m).length} open suffix
					{slottedSuffixes.filter((m) => !m).length !== 1 ? "es" : ""}
				</div>
			)}
		</div>
	);
}
