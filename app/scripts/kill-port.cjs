// Kill any process listening on the given port (Windows).
// Usage: node scripts/kill-port.js 1420
const { execSync } = require("node:child_process");

const port = process.argv[2];
if (!port) process.exit(0);

try {
	const out = execSync("netstat -ano", { encoding: "utf8" });
	const re = new RegExp(`[:.]${port}\\s+\\S+\\s+LISTENING\\s+(\\d+)`);
	const match = out.match(re);
	if (match) {
		const pid = match[1];
		console.log(`Killing stale process on port ${port} (PID ${pid})`);
		execSync(`taskkill /PID ${pid} /F`, { stdio: "ignore" });
	}
} catch {
	// Ignore errors — best effort cleanup
}
