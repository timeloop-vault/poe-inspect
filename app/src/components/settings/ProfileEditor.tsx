import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "preact/hooks";
import type { DisplayPrefs, QualityColors, StoredProfile } from "../../store";
import type { EvalProfile, PredicateSchema, ScoringRule } from "../../types";
import { getSchema } from "./PredicateEditor";
import { defaultCompoundRule, defaultRule } from "./PredicateEditor";
import {
	CompactStatRow,
	LEVEL_LABELS,
	ScoringRuleEditor,
	isStatRule,
	weightMatchesLevel,
} from "./ScoringRuleEditor";

/** Profile editor with Scoring and Display sub-tabs */
export function ProfileEditor({
	profile,
	onBack,
	onSave,
}: {
	profile: StoredProfile;
	onBack: () => void;
	onSave: (patch: Partial<StoredProfile>) => void;
}) {
	const [tab, setTab] = useState<"scoring" | "display">("scoring");
	const [draft, setDraft] = useState<Partial<StoredProfile>>({});
	const [builtinProfile, setBuiltinProfile] = useState<EvalProfile | null>(null);
	const [schema, setSchema] = useState<PredicateSchema[]>([]);

	const hasChanges = Object.keys(draft).length > 0;

	useEffect(() => {
		invoke<string | null>("get_default_profile").then((json) => {
			if (json) {
				try {
					setBuiltinProfile(JSON.parse(json) as EvalProfile);
				} catch {
					/* ignore */
				}
			}
		});
		getSchema().then(setSchema);
	}, []);

	const currentName = (draft.name as string | undefined) ?? profile.name;
	const currentEval =
		"evalProfile" in draft ? (draft.evalProfile as EvalProfile | null) : profile.evalProfile;
	const currentDisplay = (draft.display as DisplayPrefs | undefined) ?? profile.display;

	const save = () => {
		if (hasChanges) onSave(draft);
		setDraft({});
	};

	const back = () => {
		onBack();
	};

	/** Customize from the built-in profile — clone all rules into evalProfile. */
	const handleCustomize = () => {
		if (!builtinProfile) return;
		setDraft({
			...draft,
			evalProfile: {
				...structuredClone(builtinProfile),
				name: "Custom",
				description: "Customized from Generic",
			},
		});
	};

	const updateEvalScoring = (scoring: ScoringRule[]) => {
		if (!currentEval) return;
		setDraft({ ...draft, evalProfile: { ...currentEval, scoring } });
	};

	const resetToDefault = () => {
		setDraft({ ...draft, evalProfile: null });
	};

	return (
		<>
			<div
				style={{
					display: "flex",
					alignItems: "center",
					gap: "8px",
					marginBottom: "16px",
				}}
			>
				<button type="button" class="btn btn-small" onClick={back}>
					{hasChanges ? "Discard" : "\u2190 Back"}
				</button>
				<input
					type="text"
					value={currentName}
					class="hotkey-display"
					style={{
						flex: 1,
						textAlign: "left",
						fontFamily: "inherit",
						fontSize: "16px",
						color: "var(--poe-header)",
					}}
					onInput={(e) => setDraft({ ...draft, name: (e.target as HTMLInputElement).value })}
				/>
				<button
					type="button"
					class={`btn btn-small ${hasChanges ? "btn-save-pulse" : ""}`}
					onClick={save}
					disabled={!hasChanges}
				>
					Save
				</button>
			</div>

			<div style={{ display: "flex", gap: "4px", marginBottom: "20px" }}>
				<button
					type="button"
					class={`btn ${tab === "scoring" ? "btn-primary" : ""}`}
					onClick={() => setTab("scoring")}
				>
					Scoring
				</button>
				<button
					type="button"
					class={`btn ${tab === "display" ? "btn-primary" : ""}`}
					onClick={() => setTab("display")}
				>
					Display
				</button>
			</div>

			{tab === "scoring" && currentEval === null && (
				<DefaultProfileView builtinProfile={builtinProfile} onCustomize={handleCustomize} />
			)}
			{tab === "scoring" && currentEval !== null && (
				<CustomProfileView
					scoring={currentEval.scoring}
					originalScoring={profile.evalProfile?.scoring ?? null}
					schema={schema}
					onUpdateScoring={updateEvalScoring}
					onReset={resetToDefault}
				/>
			)}

			{tab === "display" && (
				<DisplayTab
					display={currentDisplay}
					onUpdate={(display) => setDraft({ ...draft, display })}
				/>
			)}

			{hasChanges && (
				<div class="unsaved-bar">
					<button type="button" class="btn btn-small" onClick={back}>
						Discard
					</button>
					<span>Unsaved changes</span>
					<button type="button" class="btn btn-small btn-save-pulse" onClick={save}>
						Save
					</button>
				</div>
			)}
		</>
	);
}

// ── Scoring Rules ────────────────────────────────────────────────────────

/** Read-only view of the built-in Generic profile */
function DefaultProfileView({
	builtinProfile,
	onCustomize,
}: {
	builtinProfile: EvalProfile | null;
	onCustomize: () => void;
}) {
	return (
		<>
			<div class="setting-group">
				<div class="setting-description" style={{ marginBottom: "12px" }}>
					Using built-in <strong>Generic</strong> profile. Scores universally desirable stats (life,
					resistances, movement speed).
				</div>
				{builtinProfile && (
					<button type="button" class="btn" onClick={onCustomize}>
						Customize...
					</button>
				)}
			</div>

			{builtinProfile && (
				<div class="setting-group">
					<h3>{builtinProfile.scoring.length} Scoring Rules</h3>
					{builtinProfile.scoring.map((rule) => {
						const level = weightMatchesLevel(rule.weight);
						return (
							<div key={rule.label} class="setting-row" style={{ padding: "4px 0" }}>
								<div class="setting-label" style={{ fontSize: "12px" }}>
									{rule.label}
								</div>
								<span
									style={{
										fontSize: "11px",
										color: "var(--poe-text-dim)",
										minWidth: "48px",
										textAlign: "right",
									}}
								>
									{level ? LEVEL_LABELS[level] : `${rule.weight} pts`}
								</span>
							</div>
						);
					})}
				</div>
			)}
		</>
	);
}

/** Editable scoring rules list with stat search, drag-and-drop reordering. */
function CustomProfileView({
	scoring,
	originalScoring,
	schema,
	onUpdateScoring,
	onReset,
}: {
	scoring: ScoringRule[];
	originalScoring: ScoringRule[] | null;
	schema: PredicateSchema[];
	onUpdateScoring: (scoring: ScoringRule[]) => void;
	onReset: () => void;
}) {
	const [dragFrom, setDragFrom] = useState<number | null>(null);
	const [dragOver, setDragOver] = useState<number | null>(null);
	const [draggableIdx, setDraggableIdx] = useState<number | null>(null);

	const addRule = () => {
		const firstSchema = schema[0];
		if (!firstSchema) return;
		onUpdateScoring([
			...scoring,
			{ label: firstSchema.label, weight: 15, rule: defaultRule(firstSchema) },
		]);
	};

	const addGroup = () => {
		const firstSchema = schema[0];
		if (!firstSchema) return;
		onUpdateScoring([
			...scoring,
			{ label: "Rule Group", weight: 15, rule: defaultCompoundRule(firstSchema) },
		]);
	};

	const handleDrop = (targetIndex: number) => {
		if (dragFrom === null || dragFrom === targetIndex) return;
		const next = [...scoring];
		const moved = next[dragFrom];
		if (!moved) return;
		next.splice(dragFrom, 1);
		next.splice(targetIndex, 0, moved);
		onUpdateScoring(next);
	};

	const handleChange = (i: number, updated: ScoringRule) => {
		const next = [...scoring];
		next[i] = updated;
		onUpdateScoring(next);
	};

	return (
		<div class="setting-group">
			<div
				style={{
					display: "flex",
					justifyContent: "space-between",
					alignItems: "center",
					marginBottom: "12px",
				}}
			>
				<h3 style={{ margin: 0 }}>Scoring ({scoring.length})</h3>
				<div style={{ display: "flex", gap: "6px" }}>
					<button type="button" class="btn btn-small btn-primary" onClick={addRule}>
						+ Rule
					</button>
					<button type="button" class="btn btn-small btn-primary" onClick={addGroup}>
						+ Group
					</button>
					<button
						type="button"
						class="btn btn-small"
						onClick={onReset}
						title="Reset to built-in Generic profile"
					>
						Reset
					</button>
				</div>
			</div>

			{/* Scoring rules with drag-and-drop reordering */}
			{scoring.map((rule, i) => {
				const orig = originalScoring ? originalScoring[i] : undefined;
				const modified = !orig || JSON.stringify(rule) !== JSON.stringify(orig);
				const isDragging = dragFrom === i;
				const isDropTarget = dragOver === i && dragFrom !== null && dragFrom !== i;
				return (
					<div
						// biome-ignore lint/suspicious/noArrayIndexKey: rules have no stable ID
						key={i}
						class={`scoring-rule-slot${isDragging ? " dragging" : ""}${isDropTarget ? " drop-target" : ""}`}
						draggable={draggableIdx === i}
						onPointerDown={(e) => {
							if ((e.target as HTMLElement).closest(".drag-handle")) {
								setDraggableIdx(i);
							}
						}}
						onPointerUp={() => setDraggableIdx(null)}
						onDragStart={(e) => {
							setDragFrom(i);
							const dt = (e as DragEvent).dataTransfer;
							if (dt) {
								dt.effectAllowed = "move";
								dt.setData("text/plain", String(i));
							}
						}}
						onDragOver={(e) => {
							e.preventDefault();
							const dt = (e as DragEvent).dataTransfer;
							if (dt) dt.dropEffect = "move";
							if (dragFrom !== null && dragFrom !== i) setDragOver(i);
						}}
						onDragEnter={(e) => {
							e.preventDefault();
						}}
						onDrop={(e) => {
							e.preventDefault();
							handleDrop(i);
						}}
						onDragEnd={() => {
							setDragFrom(null);
							setDragOver(null);
							setDraggableIdx(null);
						}}
					>
						{isStatRule(rule) ? (
							<CompactStatRow
								rule={rule}
								modified={modified}
								onChange={(updated) => handleChange(i, updated)}
								onDelete={() => onUpdateScoring(scoring.filter((_, j) => j !== i))}
							/>
						) : (
							<ScoringRuleEditor
								rule={rule}
								schema={schema}
								modified={modified}
								onChange={(updated) => handleChange(i, updated)}
								onDelete={() => onUpdateScoring(scoring.filter((_, j) => j !== i))}
							/>
						)}
					</div>
				);
			})}

			{scoring.length === 0 && (
				<div class="setting-description">
					No scoring rules. Search for stats above or click "+ Rule" to add one.
				</div>
			)}
		</div>
	);
}

// ── Display Tab ──────────────────────────────────────────────────────────

/** Display sub-tab — tier colors and preview */
function DisplayTab({
	display,
	onUpdate,
}: {
	display: DisplayPrefs;
	onUpdate: (display: DisplayPrefs) => void;
}) {
	const { qualityColors, highlightWeights, dimIgnored } = display;

	const updateColor = (key: keyof QualityColors, value: string) => {
		onUpdate({
			...display,
			qualityColors: { ...qualityColors, [key]: value },
		});
	};

	return (
		<>
			<div class="setting-group">
				<h3>Mod Quality Colors</h3>
				<ColorRow
					label="Best"
					color={qualityColors.best}
					onChange={(v) => updateColor("best", v)}
				/>
				<ColorRow
					label="Great / Good"
					color={qualityColors.good}
					onChange={(v) => updateColor("good", v)}
				/>
				<ColorRow label="Mid" color={qualityColors.mid} onChange={(v) => updateColor("mid", v)} />
				<ColorRow label="Low" color={qualityColors.low} onChange={(v) => updateColor("low", v)} />
			</div>

			<div class="setting-group">
				<h3>Preview</h3>
				<div
					style={{
						background: "var(--poe-bg)",
						border: "1px solid var(--poe-border)",
						borderRadius: "4px",
						padding: "8px 12px",
					}}
				>
					<PreviewLine
						tier="T1"
						label="P"
						text="+88 to maximum Life"
						pct={95}
						quality="Best"
						color={qualityColors.best}
					/>
					<PreviewLine
						tier="T3"
						label="S"
						text="+31% Cold Resistance"
						pct={50}
						quality="Great"
						color={qualityColors.good}
					/>
					<PreviewLine
						tier="T5"
						label="P"
						text="+12% Spell Damage"
						pct={20}
						quality="Mid"
						color={qualityColors.mid}
					/>
					<PreviewLine
						tier="T8"
						label="S"
						text="+14 to Dexterity"
						pct={25}
						quality="Low"
						color={qualityColors.low}
					/>
				</div>
			</div>

			<div class="setting-group">
				<h3>Options</h3>
				<div class="setting-row">
					<div class="setting-label">Highlight mods matching profile weights</div>
					<label class="setting-toggle">
						<input
							type="checkbox"
							checked={highlightWeights}
							onChange={(e) =>
								onUpdate({
									...display,
									highlightWeights: (e.target as HTMLInputElement).checked,
								})
							}
						/>
						<span class="toggle-track" />
					</label>
				</div>
				<div class="setting-row">
					<div class="setting-label">Dim mods with weight = Ignore</div>
					<label class="setting-toggle">
						<input
							type="checkbox"
							checked={dimIgnored}
							onChange={(e) =>
								onUpdate({
									...display,
									dimIgnored: (e.target as HTMLInputElement).checked,
								})
							}
						/>
						<span class="toggle-track" />
					</label>
				</div>
			</div>
		</>
	);
}

function ColorRow({
	label,
	color,
	onChange,
}: {
	label: string;
	color: string;
	onChange: (v: string) => void;
}) {
	return (
		<div class="setting-row">
			<div class="setting-label">{label}</div>
			<div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
				<input
					type="color"
					value={color}
					onInput={(e) => onChange((e.target as HTMLInputElement).value)}
					style={{
						width: "28px",
						height: "28px",
						border: "none",
						background: "none",
						cursor: "pointer",
					}}
				/>
				<span
					style={{
						fontFamily: "Consolas, monospace",
						fontSize: "11px",
						color: "var(--poe-text-dim)",
					}}
				>
					{color}
				</span>
			</div>
		</div>
	);
}

function PreviewLine({
	tier,
	label,
	text,
	pct,
	quality,
	color,
}: {
	tier: string;
	label: string;
	text: string;
	pct: number;
	quality: string;
	color: string;
}) {
	return (
		<div
			style={{
				display: "flex",
				alignItems: "center",
				gap: "6px",
				padding: "3px 0",
				fontSize: "13px",
			}}
		>
			<span
				style={{
					fontSize: "10px",
					fontWeight: "bold",
					padding: "0 4px",
					borderRadius: "2px",
					border: `1px solid ${color}40`,
					background: `${color}20`,
					color,
					minWidth: "22px",
					textAlign: "center",
				}}
			>
				{tier}
			</span>
			<span
				style={{
					fontSize: "9px",
					fontWeight: "bold",
					padding: "0 3px",
					borderRadius: "2px",
					opacity: 0.7,
					color: label === "P" ? "#6cc" : "#c6c",
					border: `1px solid ${label === "P" ? "rgba(102,204,204,0.3)" : "rgba(204,102,204,0.3)"}`,
				}}
			>
				{label}
			</span>
			<span style={{ flex: 1, color }}>{text}</span>
			<span style={{ fontSize: "10px", color: "var(--poe-text-dim)", marginRight: "4px" }}>
				{quality}
			</span>
			<span style={{ fontSize: "10px", color: "var(--poe-text-dim)" }}>{pct}%</span>
		</div>
	);
}
