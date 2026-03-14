import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "preact/hooks";
import {
	type GeneralSettings as GeneralSettingsType,
	defaultGeneral,
	loadGeneral,
	saveGeneral,
} from "../../store";

export function GeneralSettings() {
	const [settings, setSettings] = useState<GeneralSettingsType>(defaultGeneral);
	const [loaded, setLoaded] = useState(false);
	const [uiScalePreview, setUiScalePreview] = useState<number | null>(null);

	useEffect(() => {
		Promise.all([loadGeneral(), invoke<boolean>("get_autostart")]).then(([s, autostart]) => {
			// Sync stored launchOnBoot with actual OS autostart state
			setSettings({ ...s, launchOnBoot: autostart });
			setLoaded(true);
		});
	}, []);

	const update = useCallback((patch: Partial<GeneralSettingsType>) => {
		// If toggling launchOnBoot, sync with OS autostart
		if (patch.launchOnBoot !== undefined) {
			invoke("set_autostart", { enabled: patch.launchOnBoot });
		}
		// If toggling requirePoeFocus, sync with backend gate
		if (patch.requirePoeFocus !== undefined) {
			invoke("set_require_poe_focus", { enabled: patch.requirePoeFocus });
		}
		// If toggling stashScroll, sync with backend hook
		if (patch.stashScroll !== undefined) {
			invoke("set_stash_scroll", { enabled: patch.stashScroll });
		}
		// If changing stashScrollModifier, sync with backend hook
		if (patch.stashScrollModifier !== undefined) {
			invoke("set_stash_scroll_modifier", { modifier: patch.stashScrollModifier });
		}
		setSettings((prev) => {
			const next = { ...prev, ...patch };
			saveGeneral(next);
			// Notify parent to apply UI scale immediately
			if (patch.uiScale !== undefined) {
				window.dispatchEvent(new CustomEvent("ui-scale-changed", { detail: patch.uiScale }));
			}
			return next;
		});
	}, []);

	if (!loaded) return null;

	return (
		<>
			<h2>General</h2>

			<div class="setting-group">
				<h3>UI Scale</h3>

				<div class="setting-row">
					<div class="setting-label">
						Settings window
						<div class="setting-description">
							Zoom factor for this settings window (applies on release)
						</div>
					</div>
					<div class="setting-slider">
						<input
							type="range"
							min={75}
							max={200}
							step={5}
							value={uiScalePreview ?? settings.uiScale}
							onInput={(e) => setUiScalePreview(Number((e.target as HTMLInputElement).value))}
							onChange={(e) => {
								const val = Number((e.target as HTMLInputElement).value);
								setUiScalePreview(null);
								update({ uiScale: val });
							}}
						/>
						<span class="slider-value">{uiScalePreview ?? settings.uiScale}%</span>
					</div>
				</div>

				<div class="setting-row">
					<div class="setting-label">
						Overlay panel
						<div class="setting-description">
							Zoom factor for the item overlay (applies on next inspect)
						</div>
					</div>
					<div class="setting-slider">
						<input
							type="range"
							min={50}
							max={200}
							step={5}
							value={settings.overlayScale}
							onInput={(e) =>
								update({ overlayScale: Number((e.target as HTMLInputElement).value) })
							}
						/>
						<span class="slider-value">{settings.overlayScale}%</span>
					</div>
				</div>
			</div>

			<div class="setting-group">
				<h3>Overlay Position</h3>

				<div class="setting-row">
					<div class="setting-label">
						Where to show the overlay
						<div class="setting-description">
							"At cursor" follows your mouse. "Next to panel" places it beside the inventory or
							stash panel depending on which side your cursor is on (like Awakened Trade).
						</div>
					</div>
					<div class="setting-radio-group">
						<label class="setting-radio">
							<input
								type="radio"
								name="overlay-position"
								checked={settings.overlayPosition === "cursor"}
								onChange={() => update({ overlayPosition: "cursor" })}
							/>
							At cursor
						</label>
						<label class="setting-radio">
							<input
								type="radio"
								name="overlay-position"
								checked={settings.overlayPosition === "panel"}
								onChange={() => update({ overlayPosition: "panel" })}
							/>
							Next to panel
						</label>
					</div>
				</div>
			</div>

			<div class="setting-group">
				<h3>Game Version</h3>

				<div class="setting-row">
					<div class="setting-label">Path of Exile version</div>
					<div class="setting-radio-group">
						<label class="setting-radio">
							<input
								type="radio"
								name="poe-version"
								checked={settings.poeVersion === "poe1"}
								onChange={() => update({ poeVersion: "poe1" })}
							/>
							PoE 1
						</label>
						<label class="setting-radio">
							<input
								type="radio"
								name="poe-version"
								checked={settings.poeVersion === "poe2"}
								onChange={() => update({ poeVersion: "poe2" })}
							/>
							PoE 2
						</label>
					</div>
				</div>
			</div>

			<div class="setting-group">
				<h3>Startup</h3>

				<div class="setting-row">
					<div class="setting-label">Start minimized to tray</div>
					<Toggle
						checked={settings.startMinimized}
						onChange={(v) => update({ startMinimized: v })}
					/>
				</div>

				<div class="setting-row">
					<div class="setting-label">Launch on system startup</div>
					<Toggle checked={settings.launchOnBoot} onChange={(v) => update({ launchOnBoot: v })} />
				</div>
			</div>

			<div class="setting-group">
				<h3>Behavior</h3>

				<div class="setting-row">
					<div class="setting-label">
						Only respond when PoE is focused
						<div class="setting-description">
							Hotkeys like inspect and cycle profile will only fire when Path of Exile is
							the active window. Disable this if the check doesn't work on your platform.
						</div>
					</div>
					<Toggle
						checked={settings.requirePoeFocus}
						onChange={(v) => update({ requirePoeFocus: v })}
					/>
				</div>

				<div class="setting-row">
					<div class="setting-label">
						Stash tab scrolling
						<div class="setting-description">
							Scroll wheel navigates between stash tabs when Path of Exile is focused.
							Scroll up goes left, scroll down goes right.
						</div>
					</div>
					<Toggle
						checked={settings.stashScroll}
						onChange={(v) => update({ stashScroll: v })}
					/>
				</div>

				{settings.stashScroll && (
					<div class="setting-row">
						<div class="setting-label">Scroll modifier key</div>
						<select
							class="setting-select"
							value={settings.stashScrollModifier}
							onChange={(e) =>
								update({ stashScrollModifier: (e.target as HTMLSelectElement).value })
							}
						>
							<option value="Ctrl">Ctrl + Scroll</option>
							<option value="Shift">Shift + Scroll</option>
							<option value="Alt">Alt + Scroll</option>
							<option value="None">Scroll (no modifier)</option>
						</select>
					</div>
				)}
			</div>

			<div class="setting-group">
				<h3>Display</h3>

				<div class="setting-row">
					<div class="setting-label">Show roll quality bars</div>
					<Toggle checked={settings.showRollBars} onChange={(v) => update({ showRollBars: v })} />
				</div>

				<div class="setting-row">
					<div class="setting-label">Show tier badges</div>
					<Toggle
						checked={settings.showTierBadges}
						onChange={(v) => update({ showTierBadges: v })}
					/>
				</div>

				<div class="setting-row">
					<div class="setting-label">Show prefix/suffix labels</div>
					<Toggle
						checked={settings.showTypeBadges}
						onChange={(v) => update({ showTypeBadges: v })}
					/>
				</div>

				<div class="setting-row">
					<div class="setting-label">Show open affix count</div>
					<Toggle
						checked={settings.showOpenAffixes}
						onChange={(v) => update({ showOpenAffixes: v })}
					/>
				</div>

				<div class="setting-row">
					<div class="setting-label">Show stat IDs (power user)</div>
					<Toggle checked={settings.showStatIds} onChange={(v) => update({ showStatIds: v })} />
				</div>
				<div class="setting-description">
					Show internal stat IDs on mod lines in the overlay and rule builder.
				</div>
			</div>
		</>
	);
}

function Toggle({ checked, onChange }: { checked: boolean; onChange: (v: boolean) => void }) {
	return (
		<label class="setting-toggle">
			<input
				type="checkbox"
				checked={checked}
				onChange={(e) => onChange((e.target as HTMLInputElement).checked)}
			/>
			<span class="toggle-track" />
		</label>
	);
}
