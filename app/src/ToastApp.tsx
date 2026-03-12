import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "preact/hooks";
import "./styles/overlay.css";

interface ToastData {
	profileName: string;
	color: string;
	zoom: number;
}

const FADE_MS = 300;
const DISPLAY_MS = 2000;

export function ToastApp() {
	const [toast, setToast] = useState<ToastData | null>(null);
	const [fading, setFading] = useState(false);

	useEffect(() => {
		const unlisten = listen<ToastData>("show-toast", (event) => {
			setToast(event.payload);
			setFading(false);
		});
		return () => {
			unlisten.then((fn) => fn());
		};
	}, []);

	useEffect(() => {
		if (!toast) return;
		const fadeTimer = setTimeout(() => setFading(true), DISPLAY_MS);
		const doneTimer = setTimeout(() => setToast(null), DISPLAY_MS + FADE_MS);
		return () => {
			clearTimeout(fadeTimer);
			clearTimeout(doneTimer);
		};
	}, [toast]);

	if (!toast) return null;

	const zoom = toast.zoom ?? 1;
	// Match overlay scaling: transform: scale(zoom) with transformOrigin top left
	const style: Record<string, string | number> = {};
	if (zoom !== 1) {
		style.transform = `scale(${zoom})`;
		style.transformOrigin = "top left";
	}

	return (
		<div class={`toast-window ${fading ? "toast-fading" : ""}`} style={style}>
			<span class="toast-dot" style={toast.color ? { background: toast.color } : undefined} />
			<span class="toast-text">
				Active: <strong>{toast.profileName}</strong>
			</span>
		</div>
	);
}
