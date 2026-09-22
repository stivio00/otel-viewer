# Installation

The desktop app ships as prebuilt binaries attached to every tagged release
on the [releases page](https://github.com/stivio00/otel-viewer/releases):
a `.dmg` for macOS (Apple Silicon) and a setup `.exe` installer (NSIS) for
Windows x64. Package-manager installs are available via Homebrew and winget.

## macOS — Homebrew (custom tap)

The desktop app is distributed through the project's own Homebrew tap:

```sh
brew trust stivio00/otel-viewer        # once — brew 7+ distrusts new taps by default
brew tap stivio00/otel-viewer
brew install --cask otel-viewer
```

Upgrades follow the usual `brew upgrade --cask otel-viewer` whenever a new
version is released. The tap lives at
[stivio00/homebrew-otel-viewer](https://github.com/stivio00/homebrew-otel-viewer).

> A submission to the central `homebrew-cask` repository additionally
> requires the project to meet Homebrew's notability bar (75+ stars or
> 30+ forks/watchers) and ideally a signed + notarized binary — until then
> the custom tap is the official channel.

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
