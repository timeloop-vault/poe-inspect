import type { DangerLevel, MapDangerConfig } from "../store";
import type { ItemPayload } from "../types";
import type { ProfileSummary } from "./ItemOverlay";

interface CompactPillProps {
	item: ItemPayload;
	profiles: ProfileSummary[];
	mapDanger: MapDangerConfig;
}

function scoreClass(pct: number): string {
	if (pct >= 70) return "score-high";
	if (pct >= 40) return "score-mid";
	return "score-low";
}

/** Replace numbers with # to match map danger config keys. */
function toTemplateKey(text: string): string {
	return text.replace(/[+-]?\d+(?:\.\d+)?/g, "#");
}

/** Compute map danger verdict from explicit mods + user config. */
function computeMapVerdict(
	item: ItemPayload,
	mapDanger: MapDangerConfig,
): { label: string; cls: string } | null {
	if (item.item.header.itemClass !== "Maps") return null;

	const levels: (DangerLevel | null)[] = (item.item.explicits ?? []).map((mod) => {
		const text = mod.statLines
			.filter((sl) => !sl.isReminder)
			.map((sl) => sl.displayText)
			.join("\n");
		const template = toTemplateKey(text);
		return mapDanger[template] ?? null;
	});

	if (levels.some((l) => l === "deadly")) {
		return { label: "DEADLY", cls: "danger-deadly" };
	}
	if (levels.some((l) => l === "warning")) {
		return { label: "CAUTION", cls: "danger-warning" };
	}
	if (levels.length > 0 && levels.every((l) => l === "good")) {
		return { label: "SAFE", cls: "danger-good" };
	}
	return { label: "UNRATED", cls: "danger-unclassified" };
}

export function CompactPill({ item, profiles, mapDanger }: CompactPillProps) {
	const name = item.item.header.name ?? item.item.header.baseType;
	const truncatedName = name.length > 28 ? `${name.slice(0, 26)}...` : name;

	const score = item.eval.score;
	const applicable = score?.applicable;
	const scorePct = applicable ? Math.round(score.percent) : null;

	const watchingProfiles = profiles.filter((p) => p.role === "watching");
	const watchingResults = item.eval.watchingScores ?? [];

	const mapVerdict = computeMapVerdict(item, mapDanger);

	return (
		<div class={`compact-pill ${!applicable && !mapVerdict ? "compact-pill-gray" : ""}`}>
			<span class="compact-pill-name">{truncatedName}</span>
			{mapVerdict ? (
				<span class={`compact-pill-verdict ${mapVerdict.cls}`}>{mapVerdict.label}</span>
			) : scorePct !== null ? (
				<span class={`compact-pill-score ${scoreClass(scorePct)}`}>{scorePct}%</span>
			) : (
				<span class="compact-pill-score compact-pill-na">N/A</span>
			)}
			{watchingResults.length > 0 && (
				<span class="compact-pill-dots">
					{watchingResults.map((ws, i) => {
						const profile = watchingProfiles[i];
						const color = ws.color || profile?.watchColor || "#888";
						const passed = ws.score.applicable;
						return (
							<span
								key={profile?.id ?? i}
								class={`compact-dot ${passed ? "" : "compact-dot-dim"}`}
								style={{ background: color }}
								title={`${ws.profileName}: ${Math.round(ws.score.percent)}%`}
							/>
						);
					})}
				</span>
			)}
		</div>
	);
}

export function CompactPillNA() {
	return (
		<div class="compact-pill compact-pill-gray">
			<span class="compact-pill-score compact-pill-na">N/A</span>
		</div>
	);
}
