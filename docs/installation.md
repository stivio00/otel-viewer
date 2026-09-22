# Installation

The desktop app ships as prebuilt binaries attached to every tagged release
on the [releases page](https://github.com/stivio00/otel-viewer/releases):
a `.dmg` for macOS (Apple Silicon) and a setup `.exe` installer (NSIS) for
Windows x64. Package-manager installs are available via Homebrew and winget.

## macOS — Homebrew

Once the cask is published to homebrew-cask:

```sh
brew install --cask otel-viewer
```

Upgrade with `brew upgrade --cask otel-viewer` whenever a new version is
tagged.

> Not in homebrew-cask yet? Until the cask PR lands, download the latest
> `otel-viewer_<version>_aarch64.dmg` from the
> [releases page](https://github.com/stivio00/otel-viewer/releases),
> open it and drag **otel-viewer.app** into `/Applications`.

## Windows — winget

Once the package is published to the winget community repository
([microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs)):

```powershell
winget install Stivio00.otel-viewer
```

Upgrades follow the usual `winget upgrade` flow.

> Not published yet? Until then, download `otel-viewer_<version>_x64-setup.exe`
> from the [releases page](https://github.com/stivio00/otel-viewer/releases)
> and run it.

## First launch notes

The binaries are unsigned, so both operating systems will grumble once on
first launch:

- **macOS Gatekeeper**: right-click the app and choose **Open**, then confirm
  in the dialog (needed only the first time).
- **Windows SmartScreen**: click **More info → Run anyway** in the blue
  dialog.

The app is self-contained: it runs a local OTLP/gRPC receiver on port 4317
and an HTTP API + UI on 127.0.0.1:6666, storing telemetry in a DuckDB file
under your user application-data directory. To start from scratch, delete
that file or use the reset button in the app header.
