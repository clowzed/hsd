# HSD - Honest Sign Scanner

## Release Process
When asked to release or bump the version:
1. Run `./scripts/bump-version.sh <version>` to update version in all 3 files (`package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`)
2. Commit the version bump and push/merge to main
3. CI detects version change on main, auto-tags, and publishes a GitHub Release (`.github/workflows/release.yml`)

When making any code changes, always bump the version and include it in the commit so users get the update automatically.

## Architecture
- **Tauri 2** desktop app, macOS only (arm64 + x64)
- **Frontend**: React 19, TypeScript, Vite 6, Tailwind 4, Zustand 5, shadcn/ui (Radix)
- **Backend**: Rust with Tokio async runtime
- **Auto-updater**: `tauri-plugin-updater` checks GitHub Releases on startup, silent download, shows "Перезапустить" banner when ready
- **UI language**: Russian (all user-facing text)
- **Repo**: `clowzed/hsd` (public)

## Project Structure
```
src/                          # React frontend
  components/
    common/                   # ErrorBanner, UpdateBanner
    layout/                   # Header, Sidebar
    scan/                     # LastScanResult, CodeBuffer
    scanner/                  # ScannerStatus indicator
    settings/                 # ModeToggle, PrinterSelect, BarcodeSettings
    ui/                       # shadcn/ui primitives (button, card, dialog, etc.)
  hooks/
    useCommands.ts            # All Tauri invoke() wrappers (24 functions)
    useTauriEvents.ts         # Listens to Rust→frontend events
  store/
    useAppStore.ts            # Central Zustand store (scanner, codes, settings, PDF)
    useThemeStore.ts          # Dark/light mode
  types/index.ts              # All TypeScript interfaces

src-tauri/src/                # Rust backend
  lib.rs                      # App init, scanner pipeline, validation pipeline
  commands.rs                 # 17 Tauri commands (PDF, printing, settings)
  api/crpt.rs                 # CRPT API client (code verification)
  services/
    scanner_manager.rs        # USB scanner auto-connect + status broadcasting
    validator.rs              # HonestSignValidator (parse, checksum, API call)
  domain/                     # BarcodeScanner trait
  infrastructure/             # MertechScanner USB implementation
  pdf/
    generator.rs              # 58×40mm label PDF generation (DataMatrix)
    printer.rs                # CUPS printing (fixed 58×40mm or auto-size)
    barcode.rs                # Find marketplace barcode PDFs by vendor code
    merge.rs                  # Merge honest sign + barcode PDFs (lopdf)
  audio/                      # Success/error sound feedback (rodio)
  ui/
    state.rs                  # AppSettings, ScannedCode, ScannerStatus types
    persistence.rs            # JSON settings save/load (~/.config/honest-sign-scanner/)
```

## Key Patterns
- **Tauri commands** return `Result<T, String>`. Errors show as banner in UI
- **Events flow**: Rust emits → `useTauriEvents` listens → updates Zustand store → React re-renders
- **Settings auto-persist** to `~/.config/honest-sign-scanner/settings.json` on every change
- **Two modes**: Buffered (accumulate codes, print batch) and Instant (scan → print immediately)
- **Barcode merging**: In instant mode, honest sign label + marketplace barcode merged into single PDF
- **Scanner auto-reconnects** with 2-second retry loop
- **Duplicate detection** is runtime-only (resets on app restart)

## CI/CD Secrets (on clowzed/hsd)
- `TAURI_SIGNING_PRIVATE_KEY` — signs updater bundles for auto-update verification
