// copy-dist.js
// Runs after `tauri build` completes to copy build artifacts to the dist folder:
// on Windows the NSIS installer
// on macOS a freshly re-signed .dmg
// on Linux the .deb package.
import { copyFileSync, mkdirSync, mkdtempSync, rmSync, cpSync, readFileSync, readdirSync, unlinkSync, existsSync } from "fs";
import { execFileSync } from "child_process";
import { tmpdir } from "os";
import { join } from "path";

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
const productName = tauriConf.productName;

if (process.platform === "win32") {
    copyToDist(`src-tauri/target/release/bundle/nsis/sid-device_${version}_x64-setup.exe`, "NSIS installer");
} else if (process.platform === "darwin") {
    const appPath = `src-tauri/target/release/bundle/macos/${productName}.app`;
    const bundleDmgDir = "src-tauri/target/release/bundle/dmg";
    const originalDmgName = existsSync(bundleDmgDir) ? readdirSync(bundleDmgDir).find(f => f.endsWith(".dmg")) : undefined;

    if (!originalDmgName) {
        console.error(`No .dmg found in ${bundleDmgDir}. Make sure "dmg" is included in the bundle targets.`);
        process.exit(1);
    }

    const originalDmgPath = `${bundleDmgDir}/${originalDmgName}`;
    const finalDmgDest = `${distDir}/${originalDmgName}`;
    let mountPoint;
    let stagingDir;

    try {
        execFileSync("codesign", ["--force", "--deep", "--sign", "-", appPath]);
        console.log(`Ad-hoc signed ${appPath}`);

        mountPoint = mkdtempSync(join(tmpdir(), "sid-device-dmg-"));
        stagingDir = mkdtempSync(join(tmpdir(), "sid-device-staging-"));

        execFileSync("hdiutil", ["attach", originalDmgPath, "-mountpoint", mountPoint, "-nobrowse", "-quiet"]);
        try {
            cpSync(mountPoint, stagingDir, { recursive: true });
        } finally {
            execFileSync("hdiutil", ["detach", mountPoint, "-quiet"]);
        }

        const stagedAppPath = join(stagingDir, `${productName}.app`);
        rmSync(stagedAppPath, { recursive: true, force: true });
        execFileSync("ditto", [appPath, stagedAppPath]);

        if (existsSync(finalDmgDest)) {
            unlinkSync(finalDmgDest);
        }
        execFileSync("hdiutil", ["create", "-volname", productName, "-srcfolder", stagingDir, "-ov", "-format", "UDZO", finalDmgDest]);
        console.log(`Created DMG ${finalDmgDest}`);
    } catch (err) {
        console.error(`Failed to sign/package app bundle: ${err.message}`);
        process.exit(1);
    } finally {
        if (mountPoint) rmSync(mountPoint, { recursive: true, force: true });
        if (stagingDir) rmSync(stagingDir, { recursive: true, force: true });
    }
} else if (process.platform === "linux") {
    const bundleDebDir = "src-tauri/target/release/bundle/deb";
    const debName = existsSync(bundleDebDir) ? readdirSync(bundleDebDir).find(f => f.endsWith(".deb")) : undefined;

    if (!debName) {
        console.error(`No .deb found in ${bundleDebDir}. Make sure "deb" is included in the bundle targets.`);
        process.exit(1);
    }

    copyToDist(`${bundleDebDir}/${debName}`, "Debian package");
}


