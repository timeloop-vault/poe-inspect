import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState } from "preact/hooks";

// ── Types mirroring Rust browser module ────────────────────────────────────

interface SearchResult {
	name: string;
	kind: string;
	item_class?: string;
	category?: string;
}

interface BaseTypeDetail {
	name: string;
	itemClassId: string;
	itemClassName: string;
	category: string;
	dropLevel: number;
	width: number;
	height: number;
	implicits: string[];
	tags: string[];
	defences?: {
		armourMin: number;
		armourMax: number;
		evasionMin: number;
		evasionMax: number;
		esMin: number;
		esMax: number;
		wardMin: number;
		wardMax: number;
	};
	weapon?: {
		critical: number;
		speed: number;
		damageMin: number;
		damageMax: number;
		range: number;
	};
	block?: number;
}

interface ModTierStat {
	statId: string;
	min: number;
	max: number;
}

interface ModTier {
	modId: string;
	name: string;
	tier: number;
	requiredLevel: number;
	eligible: boolean;
	spawnWeight: number;
	stats: ModTierStat[];
	tags: string[];
}

interface ModFamily {
	familyId: string;
	tiers: ModTier[];
	taken: boolean;
}

interface ModPoolResult {
	prefixes: ModFamily[];
	suffixes: ModFamily[];
	availablePrefixCount: number;
	availableSuffixCount: number;
}

// ── Components ─────────────────────────────────────────────────────────────

function SearchBar({ onSelect }: { onSelect: (name: string) => void }) {
	const [query, setQuery] = useState("");
	const [results, setResults] = useState<SearchResult[]>([]);
	const [open, setOpen] = useState(false);
	const [selectedIdx, setSelectedIdx] = useState(0);
	const inputRef = useRef<HTMLInputElement>(null);
	const debounceRef = useRef<ReturnType<typeof setTimeout>>();

	const search = useCallback((q: string) => {
		if (q.length < 2) {
			setResults([]);
			setOpen(false);
			return;
		}
		invoke<SearchResult[]>("browser_search", { query: q, limit: 20 }).then((r) => {
			setResults(r);
			setOpen(r.length > 0);
			setSelectedIdx(0);
		});
	}, []);

	const onInput = useCallback(
		(e: Event) => {
			const val = (e.target as HTMLInputElement).value;
			setQuery(val);
			clearTimeout(debounceRef.current);
			debounceRef.current = setTimeout(() => search(val), 120);
		},
		[search],
	);

	const onKeyDown = useCallback(
		(e: KeyboardEvent) => {
			if (!open) return;
			if (e.key === "ArrowDown") {
				e.preventDefault();
				setSelectedIdx((i) => Math.min(i + 1, results.length - 1));
			} else if (e.key === "ArrowUp") {
				e.preventDefault();
				setSelectedIdx((i) => Math.max(i - 1, 0));
			} else if (e.key === "Enter" && results[selectedIdx]) {
				e.preventDefault();
				onSelect(results[selectedIdx].name);
				setOpen(false);
				setQuery(results[selectedIdx].name);
			} else if (e.key === "Escape") {
				setOpen(false);
			}
		},
		[open, results, selectedIdx, onSelect],
	);

	// Focus on mount.
	useEffect(() => {
		inputRef.current?.focus();
	}, []);

	const kindIcon = (kind: string) => {
		switch (kind) {
			case "equipment":
				return "E";
			case "jewel":
				return "J";
			case "flask":
				return "F";
			case "gem":
				return "G";
			case "currency":
				return "C";
			case "divination_card":
				return "D";
			case "map":
				return "M";
			default:
				return "?";
		}
	};

	return (
		<div class="browser-search">
			<input
				ref={inputRef}
				type="text"
				class="browser-search-input"
				placeholder="Search items, mods, gems..."
				value={query}
				onInput={onInput}
				onKeyDown={onKeyDown}
				onFocus={() => results.length > 0 && setOpen(true)}
				onBlur={() => setTimeout(() => setOpen(false), 200)}
			/>
			{open && (
				<div class="browser-search-dropdown">
					{results.map((r, i) => (
						<div
							key={r.name}
							class={`browser-search-item ${i === selectedIdx ? "selected" : ""}`}
							onMouseDown={() => {
								onSelect(r.name);
								setOpen(false);
								setQuery(r.name);
							}}
							onMouseEnter={() => setSelectedIdx(i)}
						>
							<span class="browser-search-kind">{kindIcon(r.kind)}</span>
							<span class="browser-search-name">{r.name}</span>
							{r.item_class && <span class="browser-search-class">{r.item_class}</span>}
						</div>
					))}
				</div>
			)}
		</div>
	);
}

function BaseTypeView({
	detail,
	itemLevel,
	onItemLevelChange,
}: {
	detail: BaseTypeDetail;
	itemLevel: number;
	onItemLevelChange: (lvl: number) => void;
}) {
	const defVal = (min: number, max: number) => (min === max ? `${min}` : `${min}-${max}`);

	return (
		<div class="browser-base-type">
			<div class="browser-base-header">
				<h2 class="browser-base-name">{detail.name}</h2>
				<span class="browser-base-class">{detail.itemClassName}</span>
			</div>
			<div class="browser-base-props">
				<div class="browser-prop">
					<span class="browser-prop-label">Drop Level:</span>
					<span class="browser-prop-value">{detail.dropLevel}</span>
				</div>
				{detail.defences && (
					<>
						{detail.defences.armourMax > 0 && (
							<div class="browser-prop">
								<span class="browser-prop-label">Armour:</span>
								<span class="browser-prop-value">
									{defVal(detail.defences.armourMin, detail.defences.armourMax)}
								</span>
							</div>
						)}
						{detail.defences.evasionMax > 0 && (
							<div class="browser-prop">
								<span class="browser-prop-label">Evasion:</span>
								<span class="browser-prop-value">
									{defVal(detail.defences.evasionMin, detail.defences.evasionMax)}
								</span>
							</div>
						)}
						{detail.defences.esMax > 0 && (
							<div class="browser-prop">
								<span class="browser-prop-label">Energy Shield:</span>
								<span class="browser-prop-value">
									{defVal(detail.defences.esMin, detail.defences.esMax)}
								</span>
							</div>
						)}
						{detail.defences.wardMax > 0 && (
							<div class="browser-prop">
								<span class="browser-prop-label">Ward:</span>
								<span class="browser-prop-value">
									{defVal(detail.defences.wardMin, detail.defences.wardMax)}
								</span>
							</div>
						)}
					</>
				)}
				{detail.weapon && (
					<>
						<div class="browser-prop">
							<span class="browser-prop-label">Physical Damage:</span>
							<span class="browser-prop-value">
								{detail.weapon.damageMin}-{detail.weapon.damageMax}
							</span>
						</div>
						<div class="browser-prop">
							<span class="browser-prop-label">Critical Strike Chance:</span>
							<span class="browser-prop-value">{(detail.weapon.critical / 100).toFixed(2)}%</span>
						</div>
						<div class="browser-prop">
							<span class="browser-prop-label">Attacks per Second:</span>
							<span class="browser-prop-value">{(1000 / detail.weapon.speed).toFixed(2)}</span>
						</div>
					</>
				)}
				{detail.block !== undefined && detail.block !== null && (
					<div class="browser-prop">
						<span class="browser-prop-label">Block Chance:</span>
						<span class="browser-prop-value">{detail.block}%</span>
					</div>
				)}
				{detail.implicits.length > 0 && (
					<div class="browser-implicits">
						{detail.implicits.map((imp) => (
							<div key={imp} class="browser-implicit-line">
								{imp}
							</div>
						))}
					</div>
				)}
			</div>
			<div class="browser-ilvl-control">
				<label>
					Item Level:
					<input
						type="range"
						min="1"
						max="100"
						value={itemLevel}
						onInput={(e) =>
							onItemLevelChange(Number.parseInt((e.target as HTMLInputElement).value))
						}
					/>
					<span class="browser-ilvl-value">{itemLevel}</span>
				</label>
			</div>
		</div>
	);
}

function ModPoolView({ pool }: { pool: ModPoolResult }) {
	const [filter, setFilter] = useState<"all" | "prefix" | "suffix">("all");
	const [expandedFamily, setExpandedFamily] = useState<string | null>(null);

	const families =
		filter === "prefix"
			? pool.prefixes
			: filter === "suffix"
				? pool.suffixes
				: [...pool.prefixes, ...pool.suffixes];

	const sortedFamilies = [...families].sort((a, b) => {
		// Taken families go to the bottom.
		if (a.taken !== b.taken) return a.taken ? 1 : -1;
		const aName = a.tiers[0]?.name ?? "";
		const bName = b.tiers[0]?.name ?? "";
		return aName.localeCompare(bName);
	});

	return (
		<div class="browser-mod-pool">
			<div class="browser-mod-pool-header">
				<h3>Mod Pool</h3>
				<div class="browser-mod-pool-counts">
					<span class="browser-count-prefix">{pool.availablePrefixCount} prefixes</span>
					<span class="browser-count-suffix">{pool.availableSuffixCount} suffixes</span>
				</div>
				<div class="browser-mod-pool-filters">
					<button
						type="button"
						class={filter === "all" ? "active" : ""}
						onClick={() => setFilter("all")}
					>
						All
					</button>
					<button
						type="button"
						class={filter === "prefix" ? "active" : ""}
						onClick={() => setFilter("prefix")}
					>
						Prefix
					</button>
					<button
						type="button"
						class={filter === "suffix" ? "active" : ""}
						onClick={() => setFilter("suffix")}
					>
						Suffix
					</button>
				</div>
			</div>
			<div class="browser-mod-families">
				{sortedFamilies.map((fam) => {
					const bestTier = fam.tiers.find((t) => t.eligible) ?? fam.tiers[0];
					if (!bestTier) return null;
					const isPrefix = pool.prefixes.some((p) => p.familyId === fam.familyId);
					const expanded = expandedFamily === fam.familyId;

					return (
						<div key={fam.familyId} class={`browser-mod-family ${fam.taken ? "taken" : ""}`}>
							<button
								type="button"
								class="browser-mod-family-row"
								onClick={() => setExpandedFamily(expanded ? null : fam.familyId)}
							>
								<span class={`browser-mod-slot ${isPrefix ? "prefix" : "suffix"}`}>
									{isPrefix ? "P" : "S"}
								</span>
								<span class="browser-mod-name">{bestTier.name}</span>
								<span class="browser-mod-tier">T{bestTier.tier}</span>
								<span class="browser-mod-stats">
									{bestTier.stats
										.map((s) => (s.min === s.max ? `${s.min}` : `${s.min}-${s.max}`))
										.join(", ")}
								</span>
								<span class="browser-mod-level">ilvl {bestTier.requiredLevel}</span>
								<span class="browser-mod-weight">w:{bestTier.spawnWeight}</span>
								<span class="browser-mod-expand">{expanded ? "\u25B2" : "\u25BC"}</span>
							</button>
							{expanded && (
								<div class="browser-mod-tiers">
									{fam.tiers.map((t) => (
										<div
											key={t.modId}
											class={`browser-mod-tier-row ${t.eligible ? "" : "ineligible"}`}
										>
											<span class="browser-tier-num">T{t.tier}</span>
											<span class="browser-tier-name">{t.name}</span>
											<span class="browser-tier-stats">
												{t.stats
													.map((s) => (s.min === s.max ? `${s.min}` : `${s.min}-${s.max}`))
													.join(", ")}
											</span>
											<span class="browser-tier-level">ilvl {t.requiredLevel}</span>
											<span class="browser-tier-weight">w:{t.spawnWeight}</span>
										</div>
									))}
								</div>
							)}
						</div>
					);
				})}
			</div>
		</div>
	);
}

// ── Main App ───────────────────────────────────────────────────────────────

export function BrowserApp() {
	const [detail, setDetail] = useState<BaseTypeDetail | null>(null);
	const [pool, setPool] = useState<ModPoolResult | null>(null);
	const [itemLevel, setItemLevel] = useState(84);

	const loadBaseType = useCallback(async (name: string) => {
		const d = await invoke<BaseTypeDetail | null>("browser_base_type_detail", {
			name,
		});
		setDetail(d);
		if (d) {
			const p = await invoke<ModPoolResult | null>("browser_mod_pool", {
				query: {
					base_type: name,
					item_level: 84,
					generation_types: [],
					taken_mod_ids: [],
				},
			});
			setPool(p);
		} else {
			setPool(null);
		}
	}, []);

	// Reload mod pool when item level changes.
	useEffect(() => {
		if (!detail) return;
		invoke<ModPoolResult | null>("browser_mod_pool", {
			query: {
				base_type: detail.name,
				item_level: itemLevel,
				generation_types: [],
				taken_mod_ids: [],
			},
		}).then(setPool);
	}, [detail, itemLevel]);

	// Set background for this window.
	useEffect(() => {
		document.documentElement.style.background = "rgba(12, 10, 8, 1)";
		document.body.style.background = "rgba(12, 10, 8, 1)";
	}, []);

	return (
		<div class="browser-layout">
			<div class="browser-top-bar">
				<span class="browser-title">Game Data Browser</span>
				<SearchBar onSelect={loadBaseType} />
			</div>
			<div class="browser-content">
				{detail ? (
					<>
						<BaseTypeView detail={detail} itemLevel={itemLevel} onItemLevelChange={setItemLevel} />
						{pool && <ModPoolView pool={pool} />}
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
