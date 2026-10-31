# Signed update acceptance: 0.1.0 → 0.1.1

Use a clean Windows 10 22H2 x64 VM and Windows 11 x64 VM, a real public test GitHub repository, and the same protected updater key for both versions. Never exercise corrupt update scenarios against a public production channel.

- [ ] Set actual owner/repository, publisher and public key before building 0.1.0. Preserve identifier and WiX upgrade code.
- [ ] Supply the exact FFmpeg corresponding-source archive. Configure GitHub signing Secrets and public Variables.
- [ ] Set 0.1.0 in all three version manifests, update lockfiles and publish signed tag `v0.1.0` through Actions.
- [ ] Verify release contains x64 MSI, `.msi.sig`, `latest.json`, engine pins, notices and corresponding sources. Inspect `windows-x86_64` (or MSI-specific target), HTTPS URL and full Tauri signature.
- [ ] Install 0.1.0 on a machine with no Python/Node/Rust/yt-dlp/FFmpeg/Deno. Test WebView2 offline provisioning if the machine has no runtime. Verify Start Menu shortcut and engine diagnostics.
- [ ] Download authorized audio, including a Thai title to a Thai folder. Change theme, language, output directory and concurrency. Record file hashes and SQLite/settings values.
- [ ] Test a supported playlist, cancellation, retry/partial support, unavailable video, invalid URL and offline errors. Check no child yt-dlp/Deno/FFmpeg processes remain after cancellation/exit.
- [ ] Set 0.1.1 in package.json, Cargo.toml and tauri.conf.json, run npm install to update lock metadata, update release notes, commit and publish `v0.1.1` using the **same updater key**.
- [ ] In 0.1.0, manual Check for Updates shows 0.1.1 and release notes. Startup check also finds it with no UI freeze.
- [ ] Select Later. Confirm nothing installs. Check again and confirm the update dialog can be reopened.
- [ ] With an active download/conversion, Update now is disabled. Attempting the Rust command directly must return `downloads_active`. Wait for completion, then install. New jobs must not start during update installation.
- [ ] Confirm progress displays actual update bytes. Confirm the app closes on Windows when MSI starts and installation presents basic UI/elevation as appropriate.
- [ ] Restart. About reports 0.1.1. Exactly one installed AudioHarbor entry remains. Existing settings, history, files and recorded hashes survive. Queued/interrupted work is retryable.
- [ ] Check again. Semantic comparison finds no update for equal/older versions; test 0.1.9 → 0.1.10 ordering in the test channel if needed.
- [ ] On the isolated test channel, change one MSI byte while leaving the signature unchanged. Installation must fail signature verification and 0.1.0 must remain usable. Restore correct assets.
- [ ] Test wrong public key/signature, missing latest.json, 404 installer, invalid JSON, TLS/network failure and interrupted update transfer. Errors must be useful; no unsigned install is permitted.
- [ ] Disable automatic checks and restart. Confirm no startup update request, while manual check still works.
- [ ] Uninstall via Installed Apps. Program files and shortcut disappear; user settings, history and downloaded media remain. Reinstall and verify they are readable.
- [ ] Verify Authenticode separately if a certificate was configured; a valid Tauri updater signature alone does not imply a trusted Windows publisher.

Record OS build, application tags/commit hashes, engine versions, hashes, test channel and results. These are manual acceptance tasks, not claims of tests already completed.
