import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { render } from "preact";
import { App } from "./App";
import { SettingsApp } from "./SettingsApp";
import { ToastApp } from "./ToastApp";
import "./styles/overlay.css";
import "./styles/settings.css";
if (import.meta.env.DEV) {
	import("tauri-plugin-mcp").then((m) => m.setupPluginListeners());
}

const windowLabel = getCurrentWebviewWindow().label;
const Root = windowLabel === "settings" ? SettingsApp : windowLabel === "toast" ? ToastApp : App;

// Properly unmount previous Preact tree before mounting a new one.
// When Vite HMR re-executes this module, the old tree's useEffect cleanup
// functions must run to unsubscribe Tauri event listeners. Without this,
// orphaned listeners accumulate and create duplicate DOM nodes on each event.
declare global {
	interface Window {
		__unmountApp?: () => void;
	}
}
if (window.__unmountApp) window.__unmountApp();

const root = document.getElementById("root");
if (root) {
	render(<Root />, root);
	window.__unmountApp = () => render(null, root);
}
