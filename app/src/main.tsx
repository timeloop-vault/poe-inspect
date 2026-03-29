import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { ComponentType } from "preact";
import { render } from "preact";
import "./styles/overlay.css";
import "./styles/settings.css";
import "./styles/browser.css";
if (import.meta.env.DEV) {
	import("tauri-plugin-mcp").then((m) => m.setupPluginListeners());
}

// Dynamic imports so each window only loads its own module tree.
// App.tsx has a top-level await (RQE) that blocks all other windows
// if imported statically.
const windowLabel = getCurrentWebviewWindow().label;
console.log("[main] window label:", windowLabel);
let Root: ComponentType;
try {
	switch (windowLabel) {
		case "settings":
			console.log("[main] loading SettingsApp...");
			Root = (await import("./SettingsApp")).SettingsApp;
			break;
		case "toast":
			console.log("[main] loading ToastApp...");
			Root = (await import("./ToastApp")).ToastApp;
			break;
		case "browser":
			console.log("[main] loading BrowserApp...");
			Root = (await import("./BrowserApp")).BrowserApp;
			break;
		default:
			console.log("[main] loading App...");
			Root = (await import("./App")).App;
			break;
	}
	console.log("[main] module loaded successfully");
} catch (e) {
	console.error("[main] failed to load module:", e);
	document.body.textContent = `Failed to load: ${e}`;
	throw e;
}

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
