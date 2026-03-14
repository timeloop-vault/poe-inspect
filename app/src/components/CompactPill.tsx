import type { ItemPayload } from "../types";
import type { ProfileSummary } from "./ItemOverlay";

interface CompactPillProps {
	item: ItemPayload;
	profiles: ProfileSummary[];
}

function scoreClass(pct: number): string {
	if (pct >= 70) return "score-high";
	if (pct >= 40) return "score-mid";
	return "score-low";
}

export function CompactPill({ item, profiles }: CompactPillProps) {
	const name = item.item.header.name ?? item.item.header.baseType;
	const truncatedName = name.length > 28 ? `${name.slice(0, 26)}...` : name;

	const score = item.eval.score;
	const applicable = score !== null && score.applicable;
	const scorePct = applicable ? Math.round(score.percent) : null;

	const watchingProfiles = profiles.filter((p) => p.role === "watching");
	const watchingResults = item.eval.watchingScores ?? [];

	return (
		<div class={`compact-pill ${!applicable ? "compact-pill-gray" : ""}`}>
			<span class="compact-pill-name">{truncatedName}</span>
			{scorePct !== null ? (
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
