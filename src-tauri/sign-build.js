// sign-build.js
// Runs `tauri build`, injecting the Windows code-signing certificate
// thumbprint from the CERT_THUMBPRINT environment variable via a
// `--config` override.
import { spawnSync } from "child_process";
import { writeFileSync, unlinkSync } from "fs";
import { tmpdir } from "os";
import { join } from "path";

const thumbprint = process.env.CERT_THUMBPRINT;

const tauriArgs = ["tauri", "build"];
let overridePath;

if (thumbprint) {
    // Written to a temp file (rather than passed inline) so the JSON survives
    // shell quoting on both Windows (cmd.exe) and POSIX shells.
    overridePath = join(tmpdir(), `tauri-cert-override-${process.pid}.json`);
    writeFileSync(overridePath, JSON.stringify({
        bundle: {
            windows: {
                certificateThumbprint: thumbprint
            }
        }
    }));
    tauriArgs.push("--config", overridePath);
} else if (process.platform === "win32") {
    console.warn("Warning: CERT_THUMBPRINT environment variable is not set. Building without a code-signing certificate.");
}

let result;
try {
    result = spawnSync("npx", tauriArgs, {
        stdio: "inherit",
        shell: true
    });
} finally {
    if (overridePath) {
        unlinkSync(overridePath);
    }
}

process.exit(result.status ?? 1);
