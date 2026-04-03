import { listen } from "@tauri-apps/api/event";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { open, save } from "@tauri-apps/plugin-dialog";
import { readTextFile, writeTextFile } from "@tauri-apps/plugin-fs";
import { useCallback, useEffect, useRef, useState } from "preact/hooks";
import { codeToProfile, profileToCode } from "../../profileCode";
import {
	type ProfileRole,
	type StoredProfile,
	WATCH_COLORS,
	defaultDisplay,
	loadProfiles,
	mergeModWeightsIntoScoring,
	saveProfiles,
	syncActiveProfile,
} from "../../store";
import { ProfileEditor } from "./ProfileEditor";
import { ProfileList } from "./ProfileList";

export function ProfileSettings() {
	const [profiles, setProfiles] = useState<StoredProfile[]>([]);
	const [loaded, setLoaded] = useState(false);
	const [editing, setEditing] = useState<string | null>(null);

	const profilesRef = useRef(profiles);
	profilesRef.current = profiles;

	useEffect(() => {
		loadProfiles().then((p) => {
			setProfiles(p);
			setLoaded(true);
		});
		// Refresh when profiles change in another window (overlay switch, tray switch)
		const unlisten = listen("profiles-updated", () => {
			loadProfiles().then(setProfiles);
		});
		return () => {
			unlisten.then((fn) => fn());
		};
	}, []);

	const persist = useCallback((next: StoredProfile[]) => {
		setProfiles(next);
		saveProfiles(next);
	}, []);

	const setRole = useCallback(
		(id: string, role: ProfileRole) => {
			const next = profilesRef.current.map((p) => {
				if (p.id === id) return { ...p, role };
				// Only one primary allowed — demote the old primary
				if (role === "primary" && p.role === "primary") return { ...p, role: "off" as const };
				return p;
			});
			persist(next);
			syncActiveProfile(next);
		},
		[persist],
	);

	const setWatchColor = useCallback(
		(id: string, color: string) => {
			const next = profilesRef.current.map((p) => (p.id === id ? { ...p, watchColor: color } : p));
			persist(next);
			syncActiveProfile(next);
		},
		[persist],
	);

	const addProfile = useCallback(() => {
		const id = String(Date.now());
		// Auto-assign the first unused watch color
		const usedColors = new Set(profilesRef.current.map((p) => p.watchColor));
		const color = WATCH_COLORS.find((c) => !usedColors.has(c)) ?? WATCH_COLORS[0];
		const next: StoredProfile[] = [
			...profilesRef.current,
			{
				id,
				name: "New Profile",
				role: "off",
				watchColor: color,
				evalProfile: null,
				modWeights: [],
				display: { ...defaultDisplay },
				mapDanger: {},
			},
		];
		persist(next);
		syncActiveProfile(next);
		setEditing(id);
	}, [persist]);

	const deleteProfile = useCallback(
		(id: string) => {
			const filtered = profilesRef.current.filter((p) => p.id !== id);
			const first = filtered[0];
			if (first && !filtered.some((p) => p.role === "primary")) {
				filtered[0] = { ...first, role: "primary" as const };
			}
			persist(filtered);
			syncActiveProfile(filtered);
		},
		[persist],
	);

	const duplicateProfile = useCallback(
		(id: string) => {
			const source = profilesRef.current.find((p) => p.id === id);
			if (!source) return;
			const newId = String(Date.now());
			const next = [
				...profilesRef.current,
				{
					...structuredClone(source),
					id: newId,
					name: `${source.name} (copy)`,
					role: "off" as const,
				},
			];
			persist(next);
			syncActiveProfile(next);
		},
		[persist],
	);

	const importProfile = useCallback(async () => {
		const path = await open({
			filters: [{ name: "JSON", extensions: ["json"] }],
			multiple: false,
		});
		if (!path) return;
		try {
			const text = await readTextFile(path);
			const data = JSON.parse(text) as Partial<StoredProfile>;
			if (!data.name) throw new Error("Invalid profile: missing name");
			const imported = mergeModWeightsIntoScoring({
				id: String(Date.now()),
				name: data.name,
				role: "off",
				watchColor: WATCH_COLORS[0],
				evalProfile: data.evalProfile ?? null,
				modWeights: data.modWeights ?? [],
				display: data.display ?? { ...defaultDisplay },
				mapDanger: data.mapDanger ?? {},
			});
			const next = [...profilesRef.current, imported];
			persist(next);
			syncActiveProfile(next);
		} catch (e) {
			console.error("Failed to import profile:", e);
		}
	}, [persist]);

	const exportProfile = useCallback(async (id: string) => {
		const profile = profilesRef.current.find((p) => p.id === id);
		if (!profile) return;
		const path = await save({
			defaultPath: `${profile.name.replace(/[^a-zA-Z0-9_-]/g, "_")}.json`,
			filters: [{ name: "JSON", extensions: ["json"] }],
		});
		if (!path) return;
		const { id: _id, role: _role, watchColor: _wc, ...exportData } = profile;
		await writeTextFile(path, JSON.stringify(exportData, null, 2));
	}, []);

	const [shareFlash, setShareFlash] = useState<string | null>(null);

	const shareProfile = useCallback(async (id: string) => {
		const profile = profilesRef.current.find((p) => p.id === id);
		if (!profile) return;
		try {
			const code = profileToCode(profile);
			await writeText(code);
			setShareFlash(id);
			setTimeout(() => setShareFlash(null), 1500);
		} catch (e) {
			console.error("Failed to generate share code:", e);
		}
	}, []);

	const [importingCode, setImportingCode] = useState(false);
	const [codeInput, setCodeInput] = useState("");
	const [codeError, setCodeError] = useState<string | null>(null);

	const importFromCode = useCallback(() => {
		const trimmed = codeInput.trim();
		if (!trimmed) return;
		try {
			const data = codeToProfile(trimmed);
			const imported = mergeModWeightsIntoScoring({
				id: String(Date.now()),
				name: data.name,
				role: "off",
				watchColor: WATCH_COLORS[0],
				evalProfile: data.evalProfile as StoredProfile["evalProfile"],
				modWeights: data.modWeights as StoredProfile["modWeights"],
				display: (data.display as StoredProfile["display"]) ?? { ...defaultDisplay },
				mapDanger: data.mapDanger as StoredProfile["mapDanger"],
			});
			const next = [...profilesRef.current, imported];
			persist(next);
			syncActiveProfile(next);
			setImportingCode(false);
			setCodeInput("");
			setCodeError(null);
		} catch (e) {
			setCodeError(e instanceof Error ? e.message : "Invalid share code");
		}
	}, [codeInput, persist]);

	const saveProfile = useCallback(
		(id: string, patch: Partial<StoredProfile>) => {
			const next = profilesRef.current.map((p) => (p.id === id ? { ...p, ...patch } : p));
			persist(next);
			syncActiveProfile(next);
		},
		[persist],
	);

	if (!loaded) return null;

	if (editing !== null) {
		const profile = profiles.find((p) => p.id === editing);
		if (profile) {
			return (
				<ProfileEditor
					profile={profile}
					onBack={() => setEditing(null)}
					onSave={(patch) => saveProfile(editing, patch)}
				/>
			);
		}
	}

	return (
		<ProfileList
			profiles={profiles}
			onSetRole={setRole}
			onSetWatchColor={setWatchColor}
			onEdit={(id) => setEditing(id)}
			onDuplicate={duplicateProfile}
			onExport={exportProfile}
			onShare={shareProfile}
			shareFlash={shareFlash}
			onDelete={deleteProfile}
			onAdd={addProfile}
			onImport={importProfile}
			importingCode={importingCode}
			onToggleImportCode={() => {
				setImportingCode(!importingCode);
				setCodeInput("");
				setCodeError(null);
			}}
			codeInput={codeInput}
			onCodeInputChange={setCodeInput}
			codeError={codeError}
			onImportFromCode={importFromCode}
		/>
	);
}
