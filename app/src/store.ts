/**
 * Settings persistence via tauri-plugin-store.
 *
 * Two store files in the app data directory:
 * - settings.json: General + Hotkey settings
 * - profiles.json: Array of profile objects (eval + display)
 */
import { invoke } from "@tauri-apps/api/core";
import { load } from "@tauri-apps/plugin-store";
import type { EvalProfile } from "./types";

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
	showRollBars: boolean;
	showTierBadges: boolean;
	showTypeBadges: boolean;
	showOpenAffixes: boolean;
}

export interface HotkeySettings {
	inspectItem: string;
	dismissOverlay: string;
	openSettings: string;
}

export interface TierColors {
	t1: string;
	t2_3: string;
	t4_5: string;
	low: string;
}

/** App-owned display preferences (per profile). */
export interface DisplayPrefs {
	tierColors: TierColors;
	highlightWeights: boolean;
	dimIgnored: boolean;
}

/** Profile role: primary (one at a time), watching (background), or off. */
export type ProfileRole = "primary" | "watching" | "off";

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
}

// ── Defaults ──────────────────────────────────────────────────────────────

export const defaultGeneral: GeneralSettings = {
	overlayScale: 100,
	uiScale: 100,
	overlayPosition: "cursor",
	poeVersion: "poe1",
	startMinimized: true,
	launchOnBoot: false,
	showRollBars: true,
	showTierBadges: true,
	showTypeBadges: true,
	showOpenAffixes: true,
};

export const defaultHotkeys: HotkeySettings = {
	inspectItem: "Ctrl+I",
	dismissOverlay: "Escape",
	openSettings: "Ctrl+Shift+I",
};

export const defaultTierColors: TierColors = {
	t1: "#38d838",
	t2_3: "#5c98cf",
	t4_5: "#c8c0b0",
	low: "#8c7060",
};

export const defaultDisplay: DisplayPrefs = {
	tierColors: { ...defaultTierColors },
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

// ── Profile migration ─────────────────────────────────────────────────────

/** Merge legacy modWeights into evalProfile.scoring as HasStatId rules. */
export function mergeModWeightsIntoScoring(profile: StoredProfile): StoredProfile {
	if (profile.modWeights.length === 0 || profile.evalProfile === null) return profile;
	const weightRules = profile.modWeights.flatMap((mw) =>
		(mw.statIds ?? []).map((statId) => ({
			label: mw.template,
			weight: WEIGHT_VALUES[mw.level],
			rule: { rule_type: "Pred" as const, type: "HasStatId", stat_id: statId },
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

/** Migrate old profile format to current format.
 *  Handles: modWeights-only (pre-8D), active→role (pre-8e). */
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
				tierColors: (raw.tierColors as TierColors) ?? { ...defaultTierColors },
				highlightWeights: (raw.highlightWeights as boolean) ?? true,
				dimIgnored: (raw.dimIgnored as boolean) ?? true,
			},
		};
	} else {
		// New format — pass through with defaults for missing fields
		result = {
			id: (raw.id as string) ?? String(Date.now()),
			name: (raw.name as string) ?? "Profile",
			role,
			watchColor,
			evalProfile: (raw.evalProfile as EvalProfile | null) ?? null,
			modWeights: (raw.modWeights as ModWeight[] | undefined) ?? [],
			display: (raw.display as DisplayPrefs) ?? { ...defaultDisplay },
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
}

/** Load tier colors from the primary profile (or defaults if none). */
export async function loadActiveTierColors(): Promise<TierColors> {
	const profiles = await loadProfiles();
	const primary = profiles.find((p) => p.role === "primary");
	return primary?.display.tierColors ?? { ...defaultTierColors };
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
