import { useEffect, useRef, useState } from "preact/hooks";
import { type ProfileRole, type StoredProfile, WATCH_COLORS } from "../../store";

export function ProfileWarnings({ profiles }: { profiles: StoredProfile[] }) {
	const hasPrimary = profiles.some((p) => p.role === "primary");
	const allOff = profiles.every((p) => p.role === "off");
	const hasWatching = profiles.some((p) => p.role === "watching");

	if (allOff) {
		return (
			<div class="profile-warning">
				All profiles are off. The overlay will show item data without any scoring.
			</div>
		);
	}

	if (!hasPrimary) {
		return (
			<div class="profile-warning">
				No primary profile. Overlay will show item data without scoring.
				{hasWatching && " Watching profiles will still evaluate in the background."}
			</div>
		);
	}

	return null;
}

export function WatchColorPicker({
	color,
	onChange,
}: {
	color: string;
	onChange: (color: string) => void;
}) {
	const [open, setOpen] = useState(false);
	const ref = useRef<HTMLDivElement>(null);

	useEffect(() => {
		if (!open) return;
		const handler = (e: MouseEvent) => {
			if (ref.current && !ref.current.contains(e.target as Node)) {
				setOpen(false);
			}
		};
		document.addEventListener("mousedown", handler);
		return () => document.removeEventListener("mousedown", handler);
	}, [open]);

	return (
		<div class="watch-color-picker" ref={ref}>
			<button
				type="button"
				class="watch-color-dot"
				style={{ background: color }}
				onClick={() => setOpen(!open)}
				title="Change watch color"
			/>
			{open && (
				<div class="watch-color-palette">
					{WATCH_COLORS.map((c) => (
						<button
							key={c}
							type="button"
							class={`watch-color-swatch ${c === color ? "selected" : ""}`}
							style={{ background: c }}
							onClick={() => {
								onChange(c);
								setOpen(false);
							}}
						/>
					))}
				</div>
			)}
		</div>
	);
}

export interface ProfileListProps {
	profiles: StoredProfile[];
	onSetRole: (id: string, role: ProfileRole) => void;
	onSetWatchColor: (id: string, color: string) => void;
	onEdit: (id: string) => void;
	onDuplicate: (id: string) => void;
	onExport: (id: string) => void;
	onDelete: (id: string) => void;
	onAdd: () => void;
	onImport: () => void;
}

export function ProfileList({
	profiles,
	onSetRole,
	onSetWatchColor,
	onEdit,
	onDuplicate,
	onExport,
	onDelete,
	onAdd,
	onImport,
}: ProfileListProps) {
	return (
		<>
			<h2>Profiles</h2>

			<div class="profile-actions">
				<button type="button" class="btn btn-primary" onClick={onAdd}>
					+ New
				</button>
				<button type="button" class="btn" onClick={onImport}>
					Import
				</button>
			</div>

			<div class="setting-description" style={{ marginTop: "6px", marginBottom: "6px" }}>
				{"\u2605"} Primary = scored in overlay &nbsp; {"\u25CF"} Watching = background indicator
			</div>

			<ProfileWarnings profiles={profiles} />

			<div class="profile-list">
				{profiles.map((profile) => (
					<div
						key={profile.id}
						class={`profile-item ${profile.role === "primary" ? "active" : ""}`}
					>
						<div class="profile-role-area">
							<select
								class="profile-role-select"
								value={profile.role}
								onChange={(e) =>
									onSetRole(profile.id, (e.target as HTMLSelectElement).value as ProfileRole)
								}
								title="Profile role"
							>
								<option value="primary">{"\u2605"} Primary</option>
								<option value="watching">{"\u25CF"} Watching</option>
								<option value="off">Off</option>
							</select>
							{profile.role === "watching" && (
								<WatchColorPicker
									color={profile.watchColor}
									onChange={(c) => onSetWatchColor(profile.id, c)}
								/>
							)}
						</div>
						<span class="profile-name">{profile.name}</span>

						<div class="profile-item-actions">
							<button type="button" class="btn btn-small" onClick={() => onEdit(profile.id)}>
								Edit
							</button>
							<button
								type="button"
								class="btn btn-small"
								onClick={() => onDuplicate(profile.id)}
								title="Duplicate"
							>
								Copy
							</button>
							<button
								type="button"
								class="btn btn-small"
								onClick={() => onExport(profile.id)}
								title="Export"
							>
								Export
							</button>
							<button
								type="button"
								class="btn btn-small"
								onClick={() => onDelete(profile.id)}
								title="Delete"
							>
								Del
							</button>
						</div>
					</div>
				))}
			</div>
		</>
	);
}
