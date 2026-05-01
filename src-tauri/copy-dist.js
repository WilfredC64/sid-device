// copy-dist.js
// Runs after `tauri build` completes to copy the sid-device.exe and
// the NSIS installer to dist folder.
import { copyFileSync, mkdirSync, readFileSync } from "fs";

const distDir = "dist";
mkdirSync(distDir, { recursive: true });

function copyToDist(src, label) {
    const dest = `${distDir}/${src.split("/").pop()}`;
    try {
        copyFileSync(src, dest);
        console.log(`Copied ${label} -> ${dest}`);
    } catch (err) {
        console.error(`Failed to copy ${label}: ${err.message}`);
        process.exit(1);
    }
}

const tauriConf = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8"));
const version = tauriConf.version;

copyToDist("src-tauri/target/release/sid-device.exe", "sid-device.exe");
copyToDist(`src-tauri/target/release/bundle/nsis/sid-device_${version}_x64-setup.exe`, "NSIS installer");


