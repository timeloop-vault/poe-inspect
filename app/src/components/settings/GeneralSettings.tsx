import { useState, useEffect, useCallback } from "preact/hooks";
import {
	type GeneralSettings as GeneralSettingsType,
	defaultGeneral,
	loadGeneral,
	saveGeneral,
} from "../../store";

export function GeneralSettings() {
	const [settings, setSettings] = useState<GeneralSettingsType>(defaultGeneral);
	const [loaded, setLoaded] = useState(false);

	useEffect(() => {
		loadGeneral().then((s) => {
			setSettings(s);
			setLoaded(true);
		});
	}, []);

	const update = useCallback(
		(patch: Partial<GeneralSettingsType>) => {
			setSettings((prev) => {
				const next = { ...prev, ...patch };
				saveGeneral(next);
				return next;
			});
		},
		[],
	);

	if (!loaded) return null;

	return (
		<>
			<h2>General</h2>

			<div class="setting-group">
				<h3>Overlay</h3>

				<div class="setting-row">
					<div class="setting-label">
						Scale
						<div class="setting-description">
							Zoom factor for the overlay panel
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
					<Toggle
						checked={settings.launchOnBoot}
						onChange={(v) => update({ launchOnBoot: v })}
					/>
				</div>
			</div>

			<div class="setting-group">
				<h3>Display</h3>

				<div class="setting-row">
					<div class="setting-label">Show roll quality bars</div>
					<Toggle
						checked={settings.showRollBars}
						onChange={(v) => update({ showRollBars: v })}
					/>
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
			</div>
		</>
	);
}

function Toggle({
	checked,
	onChange,
}: { checked: boolean; onChange: (v: boolean) => void }) {
	return (
		<label class="setting-toggle">
			<input
				type="checkbox"
				checked={checked}
				onChange={(e) =>
					onChange((e.target as HTMLInputElement).checked)
				}
			/>
			<span class="toggle-track" />
		</label>
	);
}
