# Verification record

Executed on 2026-10-31 on Windows 11 Home Single Language x64, build 26300. Node 24.19.0, npm 11.17.0, Rust 1.95.0 MSVC, Visual Studio 18 Community. Dependency versions are fixed by package-lock.json and Cargo.lock.

| Check | Observed result |
| --- | --- |
| Frontend lint and strict TypeScript | Passed |
| Vitest | 4 tests passed |
| Vite production build | Passed; latest asset paths verified inside release executable |
| npm audit | Zero reported vulnerabilities, including development dependencies |
| cargo fmt --check | Passed |
| cargo clippy --locked --all-targets -- -D warnings | Passed |
| Rust unit tests | 12 passed; optional network integration test excluded by default |
| Explicit real-engine integration test | Passed: actual metadata → download → MP3 conversion → FFprobe codec verification, including Thai folder and filename |
| Native development UI | Launched; metadata inspection, authorization acknowledgment and queue submission exercised with the licensed Example.ogg sample |
| Native queue + SQLite | Sample completed; output exists and completed record persisted in the actual application database |
| Release executable | Launched packaged frontend assets through Tauri's internal `tauri.localhost` asset origin; this is not a localhost HTTP backend |
| Browser UI checks | All five views navigable; no horizontal overflow at 780 and 1180 px in light and dark themes; zero browser console errors in final run |
| Bundled engines | Four pinned archives/executables hash-verified; version launches and required FFmpeg encoders checked |
| Windows runtime dependencies | PE import inspection found Windows system/UCRT libraries rather than separately installed Python/Node or VC redistributable imports in the inspected app/engine executables |
| Application dependency notices | Collected license texts from production npm/Windows Rust packages; seven missing crate-package license files supplied from pinned upstream sources; zero unresolved application notice entries |
| MSI build | Release-mode x64 WiX MSI produced; final artifact details are in MSI-ARTIFACT.txt |
| MSI database inspection | ProductName AudioHarbor; ProductVersion 0.1.0; Manufacturer AudioHarbor; fixed UpgradeCode verified; all four engines and component licenses present |
| Authenticode | Not signed; no code-signing certificate supplied |

The local MSI uses `tauri.local.conf.json` to disable updater artifact creation because no updater private key was provided. Production `tauri.conf.json` retains `createUpdaterArtifacts: true`. No signed updater payload, valid public latest.json, published GitHub Release or successful installed update is claimed.

The UI integration test created one small MP3 under the default `Downloads/AudioHarbor` folder and one history record. Source: [Example.ogg by JesseW](https://commons.wikimedia.org/wiki/File:Example.ogg), [CC BY-SA 3.0](https://creativecommons.org/licenses/by-sa/3.0/), converted from Ogg Vorbis to MP3 for local testing. The separate automated integration test used temporary storage and removed its own outputs. No sample media is embedded into AudioHarbor or redistributed with the MSI.

Resolved validation failures:

- Rust process capture initially borrowed temporary limited readers across `tokio::join!`; named/bounded readers fixed the compiler error. Final capture drains both streams concurrently, enforces limits and times out.
- Vite watched Cargo's generated executables and failed with Windows `EBUSY`; its watcher now excludes Rust outputs and caches.
- Building a debug executable while that same executable was running caused Windows file access denial. Closing the app resolves this; release packaging succeeded.
- Initial test/lint dependencies contained reported advisories. Current Vitest/ESLint and the patched minimatch override produce a clean npm audit.
- Clippy and ESLint findings were corrected, then their checks passed.

External release prerequisites and tests still outstanding:

1. Supply actual GitHub owner/repository and public updater key. Create and securely retain the matching private key/password outside the repository; configure GitHub Secrets/Variables.
2. Supply and verify the complete corresponding FFmpeg + linked-dependency source archive matching the pinned LGPL build, including patches/build recipes. CI refuses public release without this input. License text collection does not replace corresponding-source obligations.
3. Configure Authenticode signing if distributing with a trusted Windows publisher. Tauri updater signing alone does not provide that trust.
4. Run the GitHub Actions workflow on the real repository. It is implemented but was not executed here.
5. Install/uninstall on clean Windows 10 22H2 and Windows 11 VMs, including the no-WebView2 case. MSI contents and WiX configuration were inspected; a clean-machine installation was not performed on this workstation.
6. Publish signed 0.1.0 and 0.1.1 releases and execute UPDATE_TEST_CHECKLIST.md, including tampered signatures, active-job deferral and data preservation.
7. Perform authorized YouTube/playlist and additional-extractor acceptance tests on the target network. The successful live test used Wikimedia; no live YouTube challenge test or claim of universal extractor/resume support is made.
