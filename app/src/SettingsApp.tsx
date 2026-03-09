import { useEffect, useState } from "preact/hooks";
import { GeneralSettings } from "./components/settings/GeneralSettings";
import { HotkeySettings } from "./components/settings/HotkeySettings";
import { ProfileSettings } from "./components/settings/ProfileSettings";
import { loadGeneral } from "./store";

type Section = "general" | "hotkeys" | "profiles";

const sections: { id: Section; label: string }[] = [
	{ id: "general", label: "General" },
	{ id: "hotkeys", label: "Hotkeys" },
	{ id: "profiles", label: "Profiles" },
];

export function SettingsApp() {
	const [active, setActive] = useState<Section>("general");
	const [uiScale, setUiScale] = useState(100);

	useEffect(() => {
		// Settings window needs a solid background (overlay uses transparent).
		// Set it here so it covers any gap when zoom < 100%.
		document.documentElement.style.background = "rgba(12, 10, 8, 1)";
		document.body.style.background = "rgba(12, 10, 8, 1)";
		loadGeneral().then((s) => setUiScale(s.uiScale));
	}, []);

	// Listen for uiScale changes from GeneralSettings via a custom event
	useEffect(() => {
		const handler = (e: Event) => {
			setUiScale((e as CustomEvent<number>).detail);
		};
		window.addEventListener("ui-scale-changed", handler);
		return () => window.removeEventListener("ui-scale-changed", handler);
	}, []);

	const zoomStyle = uiScale !== 100 ? { zoom: uiScale / 100 } : undefined;

	return (
		<div class="settings-layout" style={zoomStyle}>
			<nav class="settings-nav">
				<div class="settings-nav-header">PoE Inspect</div>
				{sections.map((s) => (
					<button
						key={s.id}
						type="button"
						class={active === s.id ? "active" : ""}
						onClick={() => setActive(s.id)}
					>
						{s.label}
					</button>
				))}
			</nav>

			<main class="settings-content">
				{active === "general" && <GeneralSettings />}
				{active === "hotkeys" && <HotkeySettings />}
				{active === "profiles" && <ProfileSettings />}
			</main>
		</div>
	);
}
