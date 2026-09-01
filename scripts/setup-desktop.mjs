#!/usr/bin/env node
// Helper to prepare sidecar binaries for Tauri bundling
// Usage: node scripts/setup-desktop.mjs
import { execSync } from "node:child_process";
import { existsSync, mkdirSync, copyFileSync } from "node:fs";
import { join } from "node:path";
import os from "node:os";

const targetTriple = () => {
  const platform = os.platform();
  const arch = os.arch();
  if (platform === "win32") return arch === "arm64" ? "aarch64-pc-windows-msvc" : "x86_64-pc-windows-msvc";
  if (platform === "darwin") return arch === "arm64" ? "aarch64-apple-darwin" : "x86_64-apple-darwin";
  return arch === "arm64" ? "aarch64-unknown-linux-gnu" : "x86_64-unknown-linux-gnu";
};

const triple = targetTriple();
console.log(`[setup-desktop] Target triple: ${triple}`);

// Build backend sidecar binary
console.log("[setup-desktop] Building novaclip-api sidecar (release)...");
try {
  execSync("cargo build --release -p novaclip-api", { stdio: "inherit", cwd: join(process.cwd(), "backend") });
} catch (e) {
  console.error("[setup-desktop] cargo build failed", e);
  process.exit(1);
}

const srcBin = process.platform === "win32"
  ? join(process.cwd(), "backend", "target", "release", "novaclip-api.exe")
  : join(process.cwd(), "backend", "target", "release", "novaclip-api");

const destDir = join(process.cwd(), "src-tauri", "binaries");
mkdirSync(destDir, { recursive: true });

const destBin = join(destDir, `novaclip-api-${triple}${process.platform === "win32" ? ".exe" : ""}`);

if (!existsSync(srcBin)) {
  console.error(`[setup-desktop] Expected binary not found: ${srcBin}`);
  process.exit(1);
}

copyFileSync(srcBin, destBin);
console.log(`[setup-desktop] Sidecar copied: ${destBin}`);

// Also copy without triple for dev sidecar exec fallback (Tauri dev expects externalBin with triple suffix, but dev may use direct binary)
const devCopy = join(destDir, `novaclip-api${process.platform === "win32" ? ".exe" : ""}`);
try { copyFileSync(srcBin, devCopy); console.log(`[setup-desktop] Dev copy: ${devCopy}`); } catch {}

// Ensure frontend dist exists for quick check
if (!existsSync(join(process.cwd(), "frontend", "dist"))) {
  console.log("[setup-desktop] Frontend dist missing - run `npm run build --prefix frontend`");
}

console.log("[setup-desktop] Done. Now run `npm run tauri:dev` or `npm run tauri:build`.");
