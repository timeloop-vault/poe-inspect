/**
 * Profile share codes — compress a profile to a compact string for sharing.
 *
 * Format: version byte (1) + deflate(minified JSON) → base64url
 *
 * The exported data matches the JSON file export: evalProfile, modWeights,
 * display, mapDanger, and name. Internal fields (id, role, watchColor) are
 * excluded.
 */
import { deflateSync, inflateSync } from "fflate";

/** Current encoding version. Bump when the format changes. */
const VERSION = 1;

interface ExportedProfile {
	name: string;
	evalProfile: unknown;
	modWeights: unknown[];
	display: unknown;
	mapDanger: Record<string, unknown>;
}

/** Encode a profile into a share code string. */
export function profileToCode(profile: {
	name: string;
	evalProfile: unknown;
	modWeights: unknown[];
	display: unknown;
	mapDanger: Record<string, unknown>;
}): string {
	const { name, evalProfile, modWeights, display, mapDanger } = profile;
	const json = JSON.stringify({ name, evalProfile, modWeights, display, mapDanger });
	const compressed = deflateSync(new TextEncoder().encode(json));

	// Prepend version byte
	const payload = new Uint8Array(1 + compressed.length);
	payload[0] = VERSION;
	payload.set(compressed, 1);

	return uint8ToBase64url(payload);
}

/** Decode a share code string back into profile data. Throws on invalid input. */
export function codeToProfile(code: string): ExportedProfile {
	const payload = base64urlToUint8(code);
	if (payload.length < 2) throw new Error("Invalid share code: too short");

	const version = payload[0];
	if (version !== VERSION) throw new Error(`Unsupported share code version: ${version}`);

	const decompressed = inflateSync(payload.slice(1));
	const json = new TextDecoder().decode(decompressed);
	const data = JSON.parse(json) as Partial<ExportedProfile>;

	if (!data.name || typeof data.name !== "string") {
		throw new Error("Invalid share code: missing profile name");
	}

	return {
		name: data.name,
		evalProfile: data.evalProfile ?? null,
		modWeights: Array.isArray(data.modWeights) ? data.modWeights : [],
		display: data.display ?? null,
		mapDanger: data.mapDanger != null && typeof data.mapDanger === "object" ? data.mapDanger : {},
	};
}

// ── Base64url helpers (no padding, URL-safe alphabet) ────────────────────

function uint8ToBase64url(bytes: Uint8Array): string {
	let binary = "";
	for (const b of bytes) binary += String.fromCharCode(b);
	return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

function base64urlToUint8(str: string): Uint8Array {
	const padded = str.replace(/-/g, "+").replace(/_/g, "/");
	const binary = atob(padded);
	const bytes = new Uint8Array(binary.length);
	for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
	return bytes;
}
