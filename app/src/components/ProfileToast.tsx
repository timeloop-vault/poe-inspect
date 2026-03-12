import { useEffect, useState } from "preact/hooks";

interface ToastMessage {
	profileName: string;
	color?: string | undefined;
}

const DISPLAY_MS = 2000;
const FADE_MS = 300;

export function ProfileToast({
	message,
	onDone,
}: { message: ToastMessage | null; onDone: () => void }) {
	const [visible, setVisible] = useState(false);
	const [fading, setFading] = useState(false);

	useEffect(() => {
		if (!message) return;
		setVisible(true);
		setFading(false);

		const fadeTimer = setTimeout(() => setFading(true), DISPLAY_MS);
		const doneTimer = setTimeout(() => {
			setVisible(false);
			onDone();
		}, DISPLAY_MS + FADE_MS);

		return () => {
			clearTimeout(fadeTimer);
			clearTimeout(doneTimer);
		};
	}, [message, onDone]);

	if (!visible || !message) return null;

	return (
		<div class={`profile-toast ${fading ? "toast-fading" : ""}`}>
			<span class="toast-dot" style={message.color ? { background: message.color } : undefined} />
			<span class="toast-text">
				Active: <strong>{message.profileName}</strong>
			</span>
		</div>
	);
}
