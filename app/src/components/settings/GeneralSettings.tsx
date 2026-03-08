import { useState } from "preact/hooks";

export function GeneralSettings() {
	const [scale, setScale] = useState(100);
	const [poeVersion, setPoeVersion] = useState<"poe1" | "poe2">("poe1");
	const [startMinimized, setStartMinimized] = useState(true);
	const [launchOnBoot, setLaunchOnBoot] = useState(false);
	const [showRollBars, setShowRollBars] = useState(true);
	const [showTierBadges, setShowTierBadges] = useState(true);
	const [showTypeBadges, setShowTypeBadges] = useState(true);
	const [showOpenAffixes, setShowOpenAffixes] = useState(true);

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
							value={scale}
							onInput={(e) =>
								setScale(Number((e.target as HTMLInputElement).value))
							}
						/>
						<span class="slider-value">{scale}%</span>
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
								checked={poeVersion === "poe1"}
								onChange={() => setPoeVersion("poe1")}
							/>
							PoE 1
						</label>
						<label class="setting-radio">
							<input
								type="radio"
								name="poe-version"
								checked={poeVersion === "poe2"}
								onChange={() => setPoeVersion("poe2")}
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
						checked={startMinimized}
						onChange={setStartMinimized}
					/>
				</div>

				<div class="setting-row">
					<div class="setting-label">Launch on system startup</div>
					<Toggle checked={launchOnBoot} onChange={setLaunchOnBoot} />
				</div>
			</div>

			<div class="setting-group">
				<h3>Display</h3>

				<div class="setting-row">
					<div class="setting-label">Show roll quality bars</div>
					<Toggle checked={showRollBars} onChange={setShowRollBars} />
				</div>

				<div class="setting-row">
					<div class="setting-label">Show tier badges</div>
					<Toggle
						checked={showTierBadges}
						onChange={setShowTierBadges}
					/>
				</div>

				<div class="setting-row">
					<div class="setting-label">
						Show prefix/suffix labels
					</div>
					<Toggle
						checked={showTypeBadges}
						onChange={setShowTypeBadges}
					/>
				</div>

				<div class="setting-row">
					<div class="setting-label">Show open affix count</div>
					<Toggle
						checked={showOpenAffixes}
						onChange={setShowOpenAffixes}
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
