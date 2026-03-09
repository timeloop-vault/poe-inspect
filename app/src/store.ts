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
	low: 10,
	medium: 25,
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

/** A stored profile — links an eval profile with display prefs. */
export interface StoredProfile {
	id: string;
	name: string;
	active: boolean;
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
		active: true,
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

/** Migrate old profile format (modWeights) to new format (evalProfile + display). */
function migrateProfile(raw: Record<string, unknown>): StoredProfile {
	// Old format detection: has modWeights but no evalProfile
	if ("modWeights" in raw && !("evalProfile" in raw)) {
		return {
			id: (raw.id as string) ?? String(Date.now()),
			name: (raw.name as string) ?? "Migrated",
			active: (raw.active as boolean) ?? false,
			evalProfile: null, // old modWeights don't map to eval rules
			modWeights: [],
			display: {
				tierColors: (raw.tierColors as TierColors) ?? { ...defaultTierColors },
				highlightWeights: (raw.highlightWeights as boolean) ?? true,
				dimIgnored: (raw.dimIgnored as boolean) ?? true,
			},
		};
	}
	// New format — pass through with defaults for missing fields
	return {
		id: (raw.id as string) ?? String(Date.now()),
		name: (raw.name as string) ?? "Profile",
		active: (raw.active as boolean) ?? false,
		evalProfile: (raw.evalProfile as EvalProfile | null) ?? null,
		modWeights: (raw.modWeights as ModWeight[] | undefined) ?? [],
		display: (raw.display as DisplayPrefs) ?? { ...defaultDisplay },
	};
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

/** Load tier colors from the active profile (or defaults if none active). */
export async function loadActiveTierColors(): Promise<TierColors> {
	const profiles = await loadProfiles();
	const active = profiles.find((p) => p.active);
	return active?.display.tierColors ?? { ...defaultTierColors };
}

// ── Backend profile sync ──────────────────────────────────────────────────

/** Send the active eval profile to the backend for scoring.
 *  If the active profile has no custom evalProfile, sends empty string
 *  to tell the backend to use its built-in default.
 *  Mod weights are merged into the scoring array before sending.
 *  Pass `known` to avoid re-reading from the store (prevents race with save). */
export async function syncActiveProfile(known?: StoredProfile[]): Promise<void> {
	const profiles = known ?? (await loadProfiles());
	const active = profiles.find((p) => p.active);
	if (!active?.evalProfile) {
		await invoke("set_active_profile", { profileJson: "" });
		return;
	}
	// Merge mod weights into scoring rules (one HasStatId per stat ID)
	const weightRules = (active.modWeights ?? []).flatMap((mw) =>
		(mw.statIds ?? []).map((statId) => ({
			label: mw.template,
			weight: WEIGHT_VALUES[mw.level],
			rule: { rule_type: "Pred" as const, type: "HasStatId", stat_id: statId },
		})),
	);
	const merged = {
		...active.evalProfile,
		scoring: [...active.evalProfile.scoring, ...weightRules],
	};
	await invoke("set_active_profile", { profileJson: JSON.stringify(merged) });
}
