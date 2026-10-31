# AudioHarbor architecture

AudioHarbor is a Windows x64 Tauri 2 application. The installed UI loads packaged assets; it has no HTTP backend. Vite is used only during development.

- `src/`: strict React/TypeScript views, accessible shadcn-style primitives, Zustand store, typed IPC adapter, event subscription hook, English/Thai message catalogs, updater UI.
- `src-tauri/src/`: narrow commands delegate to metadata, queue, process, storage, path validation, and signed updater services. Only Rust starts executables.
- `src-tauri/binaries/`: verified yt-dlp standalone (with EJS), FFmpeg/FFprobe and Deno; target-suffixed files become Tauri sidecars. Runtime resolution uses the installation directory or this development directory, never PATH.
- SQLite in the application data directory stores settings and jobs. Interrupted jobs become retryable on restart. A bounded async scheduler runs two jobs by default. Windows Job Objects terminate descendant FFmpeg/Deno processes on cancellation or exit.
- Every job uses a stable private staging directory under the selected output folder. Retry can reuse yt-dlp partial data. Final publication uses exclusive file creation and canonical path checks. MP3 bitrate is capped to the measured/selected source bitrate; lossless outputs do not imply recovery of lost source quality.
- Official Tauri updater plugin owns signature verification and installer execution. Rust gates installation against active jobs and blocks new work during installation. Configuration requires the distributor's public key and GitHub repository. A fixed WiX upgrade code preserves upgrade identity.
- Windows GitHub Actions verifies pinned engine hashes, runs checks, builds MSI plus Tauri signatures, and publishes release metadata. Private keys exist only in local environment variables or GitHub Secrets.

Implementation sequence: scaffold → UI → engine/metadata → queue/conversion → persistence → MSI/updater → release automation → tests/build and explicit external setup.
