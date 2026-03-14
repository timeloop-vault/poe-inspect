/**
 * Settings persistence via tauri-plugin-store.
 *
 * Two store files in the app data directory:
 * - settings.json: General + Hotkey settings
 * - profiles.json: Array of profile objects (eval + display)
 */
import { invoke } from "@tauri-apps/api/core";
import { load } from "@tauri-apps/plugin-store";
import type { EvalProfile, ScoringRule } from "./types";

// ── Types ─────────────────────────────────────────────────────────────────

export type OverlayPosition = "cursor" | "panel";

/** Discrete weight levels for the mod weight editor. */
export type WeightLevel = "low" | "medium" | "high" | "critical";

/** A stat weight with resolved stat IDs for matching. */
export interface ModWeight {
	/** Human-readable template text (e.g. "+# to maximum Life"). */
	template: string;
	/** Internal stat IDs resolved from the reverse index (e.g. ["base_maximum_life"]). */
	statIds: string[];
	level: WeightLevel;
}

/** Numeric scoring weight for each level. */
export const WEIGHT_VALUES: Record<WeightLevel, number> = {
	low: 5,
	medium: 15,
	high: 50,
	critical: 100,
};

export interface GeneralSettings {
	overlayScale: number;
	uiScale: number;
	overlayPosition: OverlayPosition;
	poeVersion: "poe1" | "poe2";
	startMinimized: boolean;
	launchOnBoot: boolean;
	requirePoeFocus: boolean;
	showRollBars: boolean;
	showTierBadges: boolean;
	showTypeBadges: boolean;
	showOpenAffixes: boolean;
	showStatIds: boolean;
}

export interface HotkeySettings {
	inspectItem: string;
	dismissOverlay: string;
	openSettings: string;
	cycleProfile: string;
}

export interface QualityColors {
	best: string;
	good: string;
	mid: string;
	low: string;
}

/** @deprecated Use QualityColors. Kept for migration from old profiles. */
export type TierColors = QualityColors;

/** App-owned display preferences (per profile). */
export interface DisplayPrefs {
	qualityColors: QualityColors;
	highlightWeights: boolean;
	dimIgnored: boolean;
}

/** Profile role: primary (one at a time), watching (background), or off. */
export type ProfileRole = "primary" | "watching" | "off";

// ── Map danger assessment ────────────────────────────────────────────────

/** Danger classification for a map mod. */
export type DangerLevel = "deadly" | "warning" | "good";

/**
 * Per-profile map danger classifications.
 *
 * Keys are stat templates (e.g. "Players have #% less Area of Effect").
 * Unclassified mods are absent from the record.
 */
export type MapDangerConfig = Record<string, DangerLevel>;

/** Preset colors for watching profiles (PoE-themed palette). */
export const WATCH_COLORS = [
	"#3498db", // blue
	"#2ecc71", // green
	"#e74c3c", // red
	"#9b59b6", // purple
	"#f1c40f", // gold
	"#1abc9c", // teal
] as const;

/** A stored profile — links an eval profile with display prefs. */
export interface StoredProfile {
	id: string;
	name: string;
	role: ProfileRole;
	/** Color used for watching indicator in the overlay. */
	watchColor: string;
	/** poe-eval evaluation profile. null = use built-in default. */
	evalProfile: EvalProfile | null;
	/** Stat weights from the mod weight editor. Merged into scoring at sync time. */
	modWeights: ModWeight[];
	/** App-owned display settings. */
	display: DisplayPrefs;
	/** Per-stat danger classifications for map mods. */
	mapDanger: MapDangerConfig;
}

export interface TradeSettings {
	league: string;
	valueRelaxation: number;
	onlineOnly: boolean;
	poesessid: string;
}

// ── Defaults ──────────────────────────────────────────────────────────────

export const defaultGeneral: GeneralSettings = {
	overlayScale: 100,
	uiScale: 100,
	overlayPosition: "cursor",
	poeVersion: "poe1",
	startMinimized: true,
	launchOnBoot: false,
	requirePoeFocus: true,
	showRollBars: true,
	showTierBadges: true,
	showTypeBadges: true,
	showOpenAffixes: true,
	showStatIds: false,
};

export const defaultTrade: TradeSettings = {
	league: "",
	valueRelaxation: 0.85,
	onlineOnly: true,
	poesessid: "",
};

export const defaultHotkeys: HotkeySettings = {
	inspectItem: "Ctrl+I",
	dismissOverlay: "Escape",
	openSettings: "Ctrl+Shift+I",
	cycleProfile: "Ctrl+Shift+P",
};

export const defaultQualityColors: QualityColors = {
	best: "#38d838",
	good: "#5c98cf",
	mid: "#c8c0b0",
	low: "#8c7060",
};

export const defaultDisplay: DisplayPrefs = {
	qualityColors: { ...defaultQualityColors },
	highlightWeights: true,
	dimIgnored: true,
};

const defaultProfiles: StoredProfile[] = [
	{
		id: "default",
		name: "Default",
		role: "primary",
		watchColor: WATCH_COLORS[0],
		evalProfile: null, // uses built-in Generic profile
		modWeights: [],
		display: { ...defaultDisplay },
		mapDanger: {},
	},
];

// ── Store access ──────────────────────────────────────────────────────────

let settingsStore: Awaited<ReturnType<typeof load>> | null = null;
let profilesStore: Awaited<ReturnType<typeof load>> | null = null;

async function getSettingsStore() {
	if (!settingsStore) {
		settingsStore = await load("settings.json", {
			defaults: { general: defaultGeneral, hotkeys: defaultHotkeys },
			autoSave: true,
		});
	}
	return settingsStore;
}

async function getProfilesStore() {
	if (!profilesStore) {
		profilesStore = await load("profiles.json", {
			defaults: { profiles: defaultProfiles },
			autoSave: true,
		});
	}
	return profilesStore;
}

// ── General settings ──────────────────────────────────────────────────────

export async function loadGeneral(): Promise<GeneralSettings> {
	const store = await getSettingsStore();
	const val = await store.get<GeneralSettings>("general");
	return val ?? { ...defaultGeneral };
}

export async function saveGeneral(settings: GeneralSettings): Promise<void> {
	const store = await getSettingsStore();
	await store.set("general", settings);
}

// ── Hotkey settings ───────────────────────────────────────────────────────

export async function loadHotkeys(): Promise<HotkeySettings> {
	const store = await getSettingsStore();
	const val = await store.get<HotkeySettings>("hotkeys");
	return val ?? { ...defaultHotkeys };
}

export async function saveHotkeys(hotkeys: HotkeySettings): Promise<void> {
	const store = await getSettingsStore();
	await store.set("hotkeys", hotkeys);
}

// ── Trade settings ────────────────────────────────────────────────────────

export async function loadTrade(): Promise<TradeSettings> {
	const store = await getSettingsStore();
	const val = await store.get<TradeSettings>("trade");
	return val ?? { ...defaultTrade };
}

export async function saveTrade(settings: TradeSettings): Promise<void> {
	const store = await getSettingsStore();
	await store.set("trade", settings);
}

// ── Profile migration ─────────────────────────────────────────────────────

/** Merge legacy modWeights into evalProfile.scoring as HasStatId rules. */
export function mergeModWeightsIntoScoring(profile: StoredProfile): StoredProfile {
	if (profile.modWeights.length === 0 || profile.evalProfile === null) return profile;
	const weightRules: ScoringRule[] = profile.modWeights.flatMap((mw) =>
		(mw.statIds ?? []).map((statId) => ({
			label: mw.template,
			weight: WEIGHT_VALUES[mw.level],
			rule: { rule_type: "Pred" as const, type: "HasStatId" as const, stat_id: statId },
		})),
	);
	return {
		...profile,
		evalProfile: {
			...profile.evalProfile,
			scoring: [...profile.evalProfile.scoring, ...weightRules],
		},
		modWeights: [],
	};
}

/** Migrate old tierColors keys (t1/t2_3/t4_5/low) → qualityColors (best/good/mid/low). */
function migrateQualityColors(display: Record<string, unknown>): QualityColors {
	// New format: display.qualityColors
	if ("qualityColors" in display && display.qualityColors) {
		return display.qualityColors as QualityColors;
	}
	// Old format: display.tierColors with t1/t2_3/t4_5/low keys
	if ("tierColors" in display && display.tierColors) {
		const old = display.tierColors as Record<string, string>;
		return {
			best: old.t1 ?? defaultQualityColors.best,
			good: old.t2_3 ?? defaultQualityColors.good,
			mid: old.t4_5 ?? defaultQualityColors.mid,
			low: old.low ?? defaultQualityColors.low,
		};
	}
	return { ...defaultQualityColors };
}

/** Migrate old profile format to current format.
 *  Handles: modWeights-only (pre-8D), active→role (pre-8e), tierColors→qualityColors. */
function migrateProfile(raw: Record<string, unknown>): StoredProfile {
	let result: StoredProfile;

	// Phase 8e: migrate active (boolean) → role (string)
	let role: ProfileRole = "off";
	if ("role" in raw && typeof raw.role === "string") {
		role = raw.role as ProfileRole;
	} else if ("active" in raw && raw.active === true) {
		role = "primary";
	}

	const watchColor =
		(typeof raw.watchColor === "string" ? raw.watchColor : null) ?? WATCH_COLORS[0];

	const mapDanger = (raw.mapDanger as MapDangerConfig | undefined) ?? {};

	// Old format detection: has modWeights but no evalProfile
	if ("modWeights" in raw && !("evalProfile" in raw)) {
		result = {
			id: (raw.id as string) ?? String(Date.now()),
			name: (raw.name as string) ?? "Migrated",
			role,
			watchColor,
			evalProfile: null,
			modWeights: [],
			display: {
				qualityColors: migrateQualityColors(raw),
				highlightWeights: (raw.highlightWeights as boolean) ?? true,
				dimIgnored: (raw.dimIgnored as boolean) ?? true,
			},
			mapDanger,
		};
	} else {
		// New format — pass through with defaults for missing fields
		const rawDisplay = (raw.display as Record<string, unknown>) ?? {};
		result = {
			id: (raw.id as string) ?? String(Date.now()),
			name: (raw.name as string) ?? "Profile",
			role,
			watchColor,
			evalProfile: (raw.evalProfile as EvalProfile | null) ?? null,
			modWeights: (raw.modWeights as ModWeight[] | undefined) ?? [],
			display: {
				qualityColors: migrateQualityColors(rawDisplay),
				highlightWeights: (rawDisplay.highlightWeights as boolean) ?? true,
				dimIgnored: (rawDisplay.dimIgnored as boolean) ?? true,
			},
			mapDanger,
		};
	}

	// Phase 8D: merge any remaining modWeights into evalProfile.scoring
	return mergeModWeightsIntoScoring(result);
}

// ── Profiles ──────────────────────────────────────────────────────────────

export async function loadProfiles(): Promise<StoredProfile[]> {
	const store = await getProfilesStore();
	const val = await store.get<Record<string, unknown>[]>("profiles");
	if (!val) return defaultProfiles.map((p) => ({ ...p }));
	return val.map(migrateProfile);
}

export async function saveProfiles(profiles: StoredProfile[]): Promise<void> {
	const store = await getProfilesStore();
	await store.set("profiles", profiles);
	// Update tray menu whenever profiles change (name, add, delete, role, etc.)
	const trayProfiles = profiles.map((p) => ({ id: p.id, name: p.name, role: p.role }));
	await invoke("update_tray_profiles", { profilesJson: JSON.stringify(trayProfiles) });
}

/** Load quality colors from the primary profile (or defaults if none). */
export async function loadActiveQualityColors(): Promise<QualityColors> {
	const profiles = await loadProfiles();
	const primary = profiles.find((p) => p.role === "primary");
	return primary?.display.qualityColors ?? { ...defaultQualityColors };
}

// ── Backend profile sync ──────────────────────────────────────────────────

/** Send primary + watching profiles to the backend for scoring.
 *  If the primary profile has no custom evalProfile, sends empty string
 *  to tell the backend to use its built-in default.
 *  Pass `known` to avoid re-reading from the store (prevents race with save). */
export async function syncActiveProfile(known?: StoredProfile[]): Promise<void> {
	const profiles = known ?? (await loadProfiles());
	const primary = profiles.find((p) => p.role === "primary");
	const watching = profiles.filter((p) => p.role === "watching" && p.evalProfile !== null);

	// "none" = no primary profile (show overlay without scoring)
	// "" = use built-in default profile
	// JSON = custom profile
	const primaryJson =
		primary === undefined ? "none" : primary.evalProfile ? JSON.stringify(primary.evalProfile) : "";
	const watchingJson = JSON.stringify(
		watching.map((w) => ({
			name: w.name,
			color: w.watchColor,
			profile: w.evalProfile,
		})),
	);

	await invoke("set_active_profile", { primaryJson, watchingJson });
}
