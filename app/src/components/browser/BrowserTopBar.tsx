import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState } from "preact/hooks";
import type { BrowserRarity, SearchResult } from "./useItemBuilder";

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

export function BrowserTopBar({
	onSelect,
	rarity,
	onRarityChange,
	itemLevel,
	onItemLevelChange,
	onClear,
	hasDetail,
}: {
	onSelect: (name: string) => void;
	rarity: BrowserRarity;
	onRarityChange: (r: BrowserRarity) => void;
	itemLevel: number;
	onItemLevelChange: (lvl: number) => void;
	onClear: () => void;
	hasDetail: boolean;
}) {
	const rarities: BrowserRarity[] = ["Normal", "Magic", "Rare"];

	return (
		<div class="browser-top-bar">
			<SearchBar onSelect={onSelect} />
			{hasDetail && (
				<>
					<div class="browser-rarity-buttons">
						{rarities.map((r) => (
							<button
								key={r}
								type="button"
								class={`rarity-btn ${r.toLowerCase()} ${rarity === r ? "active" : ""}`}
								onClick={() => onRarityChange(r)}
							>
								{r}
							</button>
						))}
					</div>
					<label class="browser-ilvl-control">
						ilvl
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
					<button type="button" class="browser-clear-btn" onClick={onClear} title="Clear all mods">
						Clear
					</button>
				</>
			)}
		</div>
	);
}
