import { useState } from "preact/hooks";
import { GeneralSettings } from "./components/settings/GeneralSettings";
import { HotkeySettings } from "./components/settings/HotkeySettings";
import { ProfileSettings } from "./components/settings/ProfileSettings";

type Section = "general" | "hotkeys" | "profiles";

const sections: { id: Section; label: string }[] = [
	{ id: "general", label: "General" },
	{ id: "hotkeys", label: "Hotkeys" },
	{ id: "profiles", label: "Profiles" },
];

export function SettingsApp() {
	const [active, setActive] = useState<Section>("general");

	return (
		<div class="settings-layout">
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
