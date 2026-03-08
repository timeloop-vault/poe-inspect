import { render } from "preact";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { App } from "./App";
import { SettingsApp } from "./SettingsApp";
import "./styles/overlay.css";
import "./styles/settings.css";

const windowLabel = getCurrentWebviewWindow().label;
const Root = windowLabel === "settings" ? SettingsApp : App;

render(<Root />, document.getElementById("root")!);
