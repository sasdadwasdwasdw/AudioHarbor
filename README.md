# AudioHarbor

A native Windows audio downloader built with Tauri 2, Rust, React, strict TypeScript, Vite, Tailwind, shadcn-style Radix/CVA primitives, Lucide and Zustand. SQLite stores settings and history. The installed app has **no localhost server** and requires no Python, Node, Rust, yt-dlp, FFmpeg or Deno installation.

Download only media you own or are authorized to save. AudioHarbor does not bypass DRM, logins, paywalls or other access restrictions. Website support depends on yt-dlp and the source's availability; a valid URL does not guarantee download permission or extractor support.

## Development on Windows

Use Windows 10 22H2 or Windows 11 x64. Install these **developer tools only**:

1. Node.js 24 LTS and npm.
2. Rust stable MSVC through [rustup](https://rustup.rs/), with `x86_64-pc-windows-msvc`.
3. Visual Studio 2022 or newer, **Desktop development with C++**, MSVC and a Windows SDK.
4. WebView2 Runtime (normally present on Windows). The MSI configuration bundles its offline installer for end users.

In PowerShell, from the repository root:

```powershell
npm.cmd ci
npm.cmd run engines:prepare
npm.cmd run tauri -- dev
```

Use `npm.cmd` if PowerShell execution policy blocks npm.ps1. If Cargo is missing from PATH in the current shell:

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;" + $env:Path
rustup default stable-x86_64-pc-windows-msvc
```

The first engine preparation downloads several hundred MB. SHA256 values are pinned in `scripts/engines.lock.json`; mismatches stop preparation. It installs the official yt-dlp **Windows standalone with EJS**, full Deno, and a static LGPL FFmpeg/FFprobe build under `src-tauri/binaries/`. The same command generates application icons from `assets/icon.svg`. Executables, generated icons and downloaded license files are regenerated, not committed. To update engines, review upstream releases, review hashes from an independent trusted upstream source, edit the lock file and rerun preparation. A mutable `latest` URL is not used for engine pins. If an old upstream asset expires, archive the verified artifact in a distributor-controlled location and change the pin deliberately.

`npm run dev` alone is an honest browser UI preview. It cannot download, write settings or install updates. `npm run tauri dev` starts the real native backend. Vite's local development server is a development tool, never part of the installed app. Only one instance of the desktop application runs at a time.

## Using AudioHarbor

Paste an HTTP(S) URL, choose **Find audio**, review metadata, select a format and acknowledge permission. A playlist requires a second confirmation with its track list. Each batch is capped at 100 inspected entries; truncated playlists are identified explicitly. The pending queue is capped at 200 jobs.

The queue runs two jobs by default (configurable 1–4). Actual yt-dlp transfer speed, bytes, ETA and percentages are shown when available. Conversion has a separate progress phase. Missing values remain unknown; the UI does not manufacture transfer rates or ETA.

Cancel terminates the job's Windows process tree. Retry reuses the same staging directory and yt-dlp's `--continue` behavior; resuming is source/protocol dependent. There is no universal pause/resume promise. On app restart, unfinished jobs become **interrupted** and require Retry rather than silently starting network traffic. Lowering concurrency lets current jobs finish and constrains subsequent work.

MP3 offers 128/192/256/320 kbps **ceilings**. FFprobe measures the selected audio stream, and the encoder target is capped to the source bitrate. If a lossy bitrate cannot be measured, the job fails with guidance to use FLAC/WAV. M4A copies AAC/ALAC streams where possible; other sources use capped AAC. FLAC/WAV preserve decoded audio; converting a lossy source into these formats cannot restore lost information. Original sample rate is retained. No artwork/remote metadata is embedded into output files in this initial version.

Every output uses a sanitized Unicode title and a unique job UUID. Existing files are never overwritten: final creation uses exclusive access. Thai filenames are preserved. Each job stages downloads and conversion under `<chosen folder>/.audioharbor-work/<job UUID>`. Successful stages are cleaned up. Failed/cancelled stages are retained for retry and may use disk space. Clearing history removes records, not media or retained partial files. After abandoning a job, you may manually remove its matching staging folder.

Open file/folder actions work only for completed, recorded audio outputs. If a file was moved or deleted outside AudioHarbor, an error is shown. History supports title/filename/source search and status filters. Settings include dark (default), light and system themes. Typed English/Thai catalogs and Intl formatting prepare the language architecture; Thai navigation is a preview and secondary guidance remains English.

## Local data and security

- Settings and jobs: `%APPDATA%\io.audioharbor.desktop\audioharbor.sqlite3` (the actual directory comes from Tauri's `app_data_dir`). Keep the SQLite WAL/SHM companions with the database when backing it up while the app is open, or close the app first.
- Downloads: Windows Downloads folder / `AudioHarbor` by default, or your chosen local folder.
- Upgrades preserve application data and media. The standard WiX uninstaller removes installed program files/shortcuts, not user application data or downloaded media.
- SQLite is local and unencrypted. History intentionally stores source URLs, which can contain private query parameters. Clear history when appropriate. Engine errors are redacted before display; logs record job IDs and error categories rather than URLs or process arguments.
- There is no shell plugin or frontend filesystem permission. Rust builds argument arrays for fixed bundled executables. No user-provided shell command is accepted. Local/device/UNC paths, traversal and Windows reserved path names are rejected; output containment is checked using canonical paths. Remote thumbnail images use HTTPS with no referrer.
- The main-window capability allows only event listen/unlisten and clipboard text reading. Folder dialogs, file opening, SQLite and updater actions are narrow validated Rust commands. Updater plugin **JavaScript permissions are intentionally absent** because only Rust calls the updater. Add them only if deliberately moving updater calls into JavaScript.
- Application CSP blocks remote scripts, frames, objects and arbitrary network fetches from the webview. There is no telemetry or account requirement. Metadata requests and image loading contact their respective source hosts.

## Checks

```powershell
npm.cmd run lint
npm.cmd run typecheck
npm.cmd test
npm.cmd run build
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --locked
npm.cmd audit
```

An explicit, opt-in real-engine integration test uses a URL you may download:

```powershell
$env:AUDIOHARBOR_TEST_URL = 'https://upload.wikimedia.org/wikipedia/commons/c/c8/Example.ogg'
cargo test --manifest-path src-tauri/Cargo.toml --locked real_metadata_download_and_conversion -- --ignored --nocapture
```

That example is [Example.ogg by JesseW](https://commons.wikimedia.org/wiki/File:Example.ogg), licensed [CC BY-SA 3.0](https://creativecommons.org/licenses/by-sa/3.0/). The test converts it to MP3 in a temporary directory, verifies the codec, then removes its temporary files. It does not redistribute the result.

`minimatch` is overridden to patched 11.0.1 for development tooling. Lint and test compatibility are checked with this override. Both dependency lockfiles must be committed. See `docs/VALIDATION.md` for the actual verification record and external requirements.

Before every MSI build, `npm run licenses:collect` gathers installed production npm and Windows Rust dependency notices into the installer. Crates missing packaged license files use committed upstream texts pinned to source revisions and SHA256 in `scripts/extra-notices.lock.json`. Review `NOTICE_REVIEW.txt` after dependency upgrades. MPL-covered selectors sources are available unmodified from the exact crates.io version recorded in the installed notices; no additional restrictions are imposed on those sources.

## Build an MSI locally

Without a production updater key, create a local **unsigned** MSI (release-optimized app, updater artifacts disabled only for this command):

```powershell
npm.cmd run engines:prepare
npm.cmd run msi:local
```

The MSI is generated at:

```text
src-tauri/target/release/bundle/msi/AudioHarbor_0.1.0_x64_en-US.msi
```

With an explicit `--target x86_64-pc-windows-msvc` (as in CI), output is:

```text
src-tauri/target/x86_64-pc-windows-msvc/release/bundle/msi/
```

`--debug` builds instead go under `target/debug/bundle/msi/`. These are for development; use release builds for distribution. Close the running development app before rebuilding its executable on Windows. The first MSI build downloads WiX and a large WebView2 offline installer. Network access and Windows optional VBScript support may be needed by WiX; follow [Tauri's Windows prerequisites](https://v2.tauri.app/distribute/windows-installer/) if the bundler reports a missing component.

Run the MSI interactively to install. WiX supplies a Start Menu shortcut and standard Windows Installed Apps removal. The identity `io.audioharbor.desktop` and WiX upgrade code `D6DAEF2D-1AE6-4E2B-8F77-FB6C94AB5C19` must stay fixed across releases. Publisher metadata is in `tauri.conf.json`; set your real publisher before your first public release and retain it. Do not change the upgrade code when bumping versions. The installer is per-machine and may request elevation. Do not add uninstall actions that delete user folders.

## Configure real signed updates

The checked-in endpoint is deliberately a placeholder:

```text
https://github.com/YOUR_GITHUB_USERNAME/YOUR_REPOSITORY/releases/latest/download/latest.json
```

Create your public GitHub repository. Generate a Tauri updater key **outside this repository** and back it up securely:

```powershell
npm.cmd run tauri -- signer generate -w "$env:USERPROFILE\AudioHarbor-updater.key"
```

Keep the private `.key` and its password out of Git. The `.pub` file contains the entire base64 public-key value the application must trust. Configure the repository and public key:

```powershell
$env:GITHUB_OWNER = 'YOUR_GITHUB_USERNAME'
$env:GITHUB_REPO = 'YOUR_REPOSITORY'
$env:TAURI_PUBLIC_KEY_FILE = "$env:USERPROFILE\AudioHarbor-updater.key.pub"
npm.cmd run release:configure
```

Replace the placeholder owner/repository with your actual names. This command writes only public configuration to `src-tauri/tauri.conf.json`. Commit it before releasing. Never rotate the trusted key casually: existing installations trust the old public key. A rotation needs an update signed by the currently trusted key that introduces the new trust configuration.

For a signed local release build, put your private key **path** (or key content) and password in the process environment without logging them, then:

```powershell
# Set these in a secure shell/session, without committing them:
$env:TAURI_SIGNING_PRIVATE_KEY = "$env:USERPROFILE\AudioHarbor-updater.key"
# TAURI_SIGNING_PRIVATE_KEY_PASSWORD must contain the password chosen at generation.
npm.cmd run tauri -- build --target x86_64-pc-windows-msvc --bundles msi
```

`bundle.createUpdaterArtifacts` is **true** in the production config. Tauri creates the MSI and its `.msi.sig`. The MSI itself is the updater payload for Tauri v2, not a v1 ZIP or a custom unverified installer download. `tauri-action` combines the signature with the asset URL/version/release notes into `latest.json`. See `docs/latest.example.json` for its shape; do not publish that placeholder file.

The application checks after its UI initializes if automatic checks are enabled. About & Updates also has a manual check. The official plugin uses semantic versions, verifies the downloaded payload against the embedded public key, and starts the Windows MSI only after **Update now**. Release notes are displayed as plain text. Download progress and useful errors appear in the dialog. Active downloads defer installation; the Rust queue lock prevents a new download from slipping in after the installation check. New work is blocked during installation. The plugin closes the application on Windows when it launches the installer, so all queue/settings records are persisted beforehand. Queued jobs remain durable and become retryable after restart. Choosing Later never installs silently; check again in About when ready.

## GitHub release automation

The workflow `.github/workflows/release.yml` runs on stable `vX.Y.Z` tags or manual dispatch with an existing tag. It uses a Windows x64 runner, Node 24, Rust stable and `tauri-apps/tauri-action@v1`. It prepares verified engines, configures public trust, runs lint/types/tests/build/Clippy, builds MSI/signatures, creates a draft, attaches sources and notices, verifies the manifest and publishes the release. A failed source or manifest check leaves a draft rather than an incomplete public update.

In the repository's **Settings → Secrets and variables → Actions**, set:

| Kind | Name | Value |
| --- | --- | --- |
| Secret | `TAURI_SIGNING_PRIVATE_KEY` | Complete private key file contents (not a runner-local path) |
| Secret | `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Key password, or empty if deliberately generated without one |
| Variable | `TAURI_SIGNING_PUBLIC_KEY` | Complete contents of the generated `.pub` file |
| Variable | `FFMPEG_SOURCE_ARCHIVE_URL` | HTTPS archive with exact corresponding FFmpeg/dependency sources, patches and build scripts |
| Variable | `FFMPEG_SOURCE_ARCHIVE_SHA256` | SHA256 of that archive |

`GITHUB_TOKEN` is supplied by Actions with repository `contents: write`. No private key is printed. Source archive integrity alone cannot prove legal completeness: verify it against the pinned binaries first, as described in `THIRD_PARTY_NOTICES.md`. The workflow republishes that source archive on the same Release as the MSI. Public release remains blocked until that source archive, real repository and signing secrets are supplied.

Keep `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json` versions equal. Commit changes and lockfiles, then create/push `v0.1.0`. For subsequent releases increment all three (and the root package-lock version through npm), commit, tag and push. The verification script rejects mismatched versions/tags. Do not publish a prerelease as the stable `latest` endpoint. Use a separate test repository/channel to exercise update failures.

**Tauri updater signing and Windows Authenticode are different.** The updater key authenticates payloads to already installed AudioHarbor copies. It does not establish a Windows publisher identity or remove SmartScreen warnings. Authenticode signs the application/installer with a trusted Windows code-signing certificate (or an appropriate cloud signing service); configure the Tauri Windows signing fields/sign command and timestamping for your certificate provider before your public production release. No certificate is present in this project, so local MSI builds are not Authenticode signed. Never commit a certificate or password. See [Tauri updater signing](https://v2.tauri.app/plugin/updater/) and [Windows signing](https://v2.tauri.app/distribute/sign/windows/).

Follow `docs/UPDATE_TEST_CHECKLIST.md` to test 0.1.0 → 0.1.1 on a clean Windows VM. A real end-to-end update cannot be demonstrated without two published signed releases and the matching trusted public key.

## Source layout

```text
src/components/     accessible primitives, job presentation
src/views/          Downloader, Queue, History, Settings, About
src/hooks/          lifecycle, theme, event subscriptions
src/lib/            typed IPC, models, localization, formatting
src/store/          Zustand state and update UI state
src-tauri/src/      commands, queue, download, process, metadata,
                    paths, SQLite storage, updater, errors, tests
scripts/            engine pins/preparation, release configuration/verification
docs/               validation record and update test checklist
```

The architecture and implementation sequence are documented in `ARCHITECTURE.md`. No download progress or updater operation is mocked in production code. Automated tests cover parsing, state transitions, path rejection/containment, settings persistence, progress and bitrate caps. Cross-source extractor behavior, playlist availability, network conditions, Windows installation/uninstallation and real signed updates require integration testing.
