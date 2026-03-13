#!/usr/bin/env node

const { execSync } = require("child_process");
const fs = require("fs");
const path = require("path");
const https = require("https");
const http = require("http");

const VERSION = "0.1.2";
const REPO = "vradko/sf-compact";

const PLATFORMS = {
  "darwin-arm64": `sf-compact-v${VERSION}-aarch64-apple-darwin.tar.gz`,
  "darwin-x64": `sf-compact-v${VERSION}-x86_64-apple-darwin.tar.gz`,
  "linux-x64": `sf-compact-v${VERSION}-x86_64-unknown-linux-musl.tar.gz`,
  "win32-x64": `sf-compact-v${VERSION}-x86_64-pc-windows-msvc.zip`,
};

function getPlatformKey() {
  return `${process.platform}-${process.arch}`;
}

function download(url) {
  return new Promise((resolve, reject) => {
    const get = url.startsWith("https") ? https.get : http.get;
    get(url, (res) => {
      if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
        return download(res.headers.location).then(resolve, reject);
      }
      if (res.statusCode !== 200) {
        return reject(new Error(`HTTP ${res.statusCode} for ${url}`));
      }
      const chunks = [];
      res.on("data", (chunk) => chunks.push(chunk));
      res.on("end", () => resolve(Buffer.concat(chunks)));
      res.on("error", reject);
    }).on("error", reject);
  });
}

async function main() {
  const key = getPlatformKey();
  const filename = PLATFORMS[key];

  if (!filename) {
    console.error(`sf-compact: unsupported platform ${key}`);
    console.error(`Supported: ${Object.keys(PLATFORMS).join(", ")}`);
    console.error("Install from source: cargo install sf-compact");
    process.exit(1);
  }

  const url = `https://github.com/${REPO}/releases/download/v${VERSION}/${filename}`;
  const isWindows = process.platform === "win32";
  const binDir = path.join(__dirname, "bin");
  const binName = isWindows ? "sf-compact-bin.exe" : "sf-compact-bin";
  const binPath = path.join(binDir, binName);

  // Skip if already installed
  if (fs.existsSync(binPath)) {
    try {
      const ver = execSync(`"${binPath}" --version`, { encoding: "utf8" }).trim();
      if (ver.includes(VERSION)) {
        return;
      }
    } catch {}
  }

  console.log(`Downloading sf-compact v${VERSION} for ${key}...`);

  const tarball = await download(url);
  const tmpFile = path.join(__dirname, filename);
  fs.writeFileSync(tmpFile, tarball);

  fs.mkdirSync(binDir, { recursive: true });

  const tmpDir = path.join(__dirname, "tmp-extract");
  fs.mkdirSync(tmpDir, { recursive: true });

  if (filename.endsWith(".tar.gz")) {
    execSync(`tar xzf "${tmpFile}" -C "${tmpDir}"`, { stdio: "inherit" });
  } else {
    execSync(`unzip -o "${tmpFile}" -d "${tmpDir}"`, { stdio: "inherit" });
  }

  // Rename to sf-compact-bin (or .exe) to avoid collision with the JS wrapper
  const extractedName = isWindows ? "sf-compact.exe" : "sf-compact";
  const extracted = path.join(tmpDir, extractedName);
  fs.renameSync(extracted, binPath);
  fs.unlinkSync(tmpFile);
  fs.rmSync(tmpDir, { recursive: true, force: true });
  fs.chmodSync(binPath, 0o755);

  console.log(`sf-compact v${VERSION} installed successfully.`);
}

main().catch((err) => {
  console.error("sf-compact install failed:", err.message);
  console.error("Install from source: cargo install sf-compact");
  process.exit(1);
});
