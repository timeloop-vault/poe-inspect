import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { render } from "preact";
import { App } from "./App";
import { SettingsApp } from "./SettingsApp";
import "./styles/overlay.css";
import "./styles/settings.css";

const windowLabel = getCurrentWebviewWindow().label;
if (windowLabel !== "settings") {
	document.body.classList.add("overlay-window");
}
const Root = windowLabel === "settings" ? SettingsApp : App;

const root = document.getElementById("root");
if (root) render(<Root />, root);
