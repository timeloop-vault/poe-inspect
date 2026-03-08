/**
 * Settings persistence via tauri-plugin-store.
 *
 * Two store files in the app data directory:
 * - settings.json: General + Hotkey settings
 * - profiles.json: Array of profile objects
 */
import { load } from "@tauri-apps/plugin-store";

// ── Types ─────────────────────────────────────────────────────────────────

export interface GeneralSettings {
	overlayScale: number;
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

export type Weight = "Ignore" | "Low" | "Medium" | "High" | "Critical";

export interface TierColors {
	t1: string;
	t2_3: string;
	t4_5: string;
	low: string;
}

export interface Profile {
	id: string;
	name: string;
	active: boolean;
	modWeights: Record<string, Weight>;
	tierColors: TierColors;
	highlightWeights: boolean;
	dimIgnored: boolean;
}

// ── Defaults ──────────────────────────────────────────────────────────────

export const defaultGeneral: GeneralSettings = {
	overlayScale: 100,
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

const defaultProfiles: Profile[] = [
	{
		id: "default",
		name: "Default",
		active: true,
		modWeights: {},
		tierColors: { ...defaultTierColors },
		highlightWeights: true,
		dimIgnored: true,
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

// ── Profiles ──────────────────────────────────────────────────────────────

export async function loadProfiles(): Promise<Profile[]> {
	const store = await getProfilesStore();
	const val = await store.get<Profile[]>("profiles");
	return val ?? defaultProfiles.map((p) => ({ ...p }));
}

export async function saveProfiles(profiles: Profile[]): Promise<void> {
	const store = await getProfilesStore();
	await store.set("profiles", profiles);
}
