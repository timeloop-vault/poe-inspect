import { useEffect, useState } from "preact/hooks";
import { ChatMacroSettings } from "./components/settings/ChatMacroSettings";
import { GeneralSettings } from "./components/settings/GeneralSettings";
import { HotkeySettings } from "./components/settings/HotkeySettings";
import { MapDangerSettings } from "./components/settings/MapDangerSettings";
import { MarketplaceSettings } from "./components/settings/MarketplaceSettings";
import { ProfileSettings } from "./components/settings/ProfileSettings";
import { TradeSettings } from "./components/settings/TradeSettings";
import { loadGeneral } from "./store";

type Section =
	| "general"
	| "hotkeys"
	| "chat-macros"
	| "profiles"
	| "map-danger"
	| "trade"
	| "marketplace";

const sections: { id: Section; label: string }[] = [
	{ id: "general", label: "General" },
	{ id: "hotkeys", label: "Hotkeys" },
	{ id: "chat-macros", label: "Chat Macros" },
	{ id: "profiles", label: "Profiles" },
	{ id: "map-danger", label: "Map Danger" },
	{ id: "trade", label: "Trade" },
	{ id: "marketplace", label: "Marketplace" },
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

	// Apply zoom on the document element for global scaling.
	// Layout uses height:100% (not vh) so the grid respects zoomed dimensions.
	useEffect(() => {
		document.documentElement.style.zoom = uiScale !== 100 ? `${uiScale / 100}` : "";
	}, [uiScale]);

	// Listen for uiScale changes from GeneralSettings via a custom event
	useEffect(() => {
		const handler = (e: Event) => {
			setUiScale((e as CustomEvent<number>).detail);
		};
		window.addEventListener("ui-scale-changed", handler);
		return () => window.removeEventListener("ui-scale-changed", handler);
	}, []);

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
				{active === "chat-macros" && <ChatMacroSettings />}
				{active === "profiles" && <ProfileSettings />}
				{active === "map-danger" && <MapDangerSettings />}
				{active === "trade" && <TradeSettings />}
				{active === "marketplace" && <MarketplaceSettings />}
			</main>
		</div>
	);
}
