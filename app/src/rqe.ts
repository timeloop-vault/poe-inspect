/**
 * RQE integration for the overlay.
 *
 * Converts a ResolvedItem to Entry format (matching poe-rqe-client convention)
 * and posts to the rqe-server for demand matching.
 */

import type { ResolvedItem } from "./generated/ResolvedItem";
import type { ResolvedMod } from "./generated/ResolvedMod";
import type { MarketplaceSettings } from "./store";

interface MatchDetail {
	id: number;
	owner: string | null;
}

export interface RqeMatchResult {
	count: number;
	matches: MatchDetail[];
	matchUs: number;
}

/** Convert a ResolvedItem to the flat Entry JSON format for rqe-server.
 *  Mirrors the Rust poe_rqe_client::item_to_entry() logic. */
function itemToEntry(item: ResolvedItem): Record<string, string | number | boolean> {
	const entry: Record<string, string | number | boolean> = {};

	// Header
	entry.item_class = item.header.itemClass;
	entry.rarity = item.header.rarity;
	entry.rarity_class = item.header.rarity === "Unique" ? "Unique" : "Non-Unique";
	entry.base_type = item.header.baseType;
	if (item.header.name) entry.name = item.header.name;

	// Item level
	if (item.itemLevel != null) entry.item_level = item.itemLevel;

	// Booleans
	entry.corrupted = item.isCorrupted;
	entry.fractured = item.isFractured;
	entry.unidentified = item.isUnidentified;

	// Influences
	for (const inf of item.influences) {
		entry[`influence.${inf}`] = true;
	}
	entry.influence_count = item.influences.length;

	// Sockets
	if (item.sockets) {
		const socketCount = item.sockets.replace(/[^A-Za-z]/g, "").length;
		const maxLink = item.sockets
			.split(" ")
			.reduce((max, g) => Math.max(max, g.replace(/[^A-Za-z]/g, "").length), 0);
		entry.socket_count = socketCount;
		entry.max_link = maxLink;
	}

	// Requirements
	for (const req of item.requirements) {
		if (req.name === "Level") {
			const v = Number.parseInt(req.value, 10);
			if (!Number.isNaN(v)) entry.requirement_level = v;
		}
	}

	// Mods → stat entries
	insertMods(entry, item.implicits, "implicit");
	insertMods(entry, item.explicits, "explicit");
	insertMods(entry, item.enchants, "enchant");

	// Mod counts
	entry.implicit_count = item.implicits.length;
	entry.explicit_count = item.explicits.length;

	return entry;
}

function insertMods(
	entry: Record<string, string | number | boolean>,
	mods: ResolvedMod[],
	defaultSource: string,
): void {
	for (const m of mods) {
		const source = m.displayType === "crafted" ? "crafted" : defaultSource;
		for (const stat of m.statLines) {
			if (stat.isReminder) continue;

			// Prefer stat_ids (stable, language-independent)
			if (stat.statIds && stat.statValues) {
				for (let i = 0; i < stat.statIds.length && i < stat.statValues.length; i++) {
					entry[`${source}.${stat.statIds[i]}`] = stat.statValues[i];
				}
			} else if (stat.values.length > 0) {
				// Fallback: template from display text
				let template = stat.displayText;
				for (const vr of stat.values) {
					template = template.replace(String(vr.current), "#");
				}
				entry[`${source}.${template}`] = stat.values[0].current;
			}
		}
	}
}

/** Check an item against the RQE server. Returns match count, or null if unavailable. */
export async function checkDemand(
	item: ResolvedItem,
	settings: MarketplaceSettings,
): Promise<RqeMatchResult | null> {
	if (!settings.accountName || !settings.enabled) return null;

	try {
		const entry = itemToEntry(item);
		const headers: Record<string, string> = { "Content-Type": "application/json" };
		if (settings.apiKey) headers["X-API-Key"] = settings.apiKey;

		const resp = await fetch(`${settings.serverUrl}/match`, {
			method: "POST",
			headers,
			body: JSON.stringify(entry),
		});

		if (!resp.ok) return null;

		const data = await resp.json();
		const matches = (data.matches ?? []) as MatchDetail[];
		return {
			count: matches.length,
			matches,
			matchUs: data.match_us ?? 0,
		};
	} catch {
		return null;
	}
}
