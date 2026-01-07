# Changelog

## [0.4.3](https://github.com/ljiulong/boxyy/compare/v0.4.2...v0.4.3) (2026-01-07)


### Bug Fixes

* move updater configuration under plugins in tauri.conf.json for better organization ([b7ce8a3](https://github.com/ljiulong/boxyy/commit/b7ce8a3d6d09bd8d912df2701d26119f99486b32))

## [0.4.2](https://github.com/ljiulong/boxyy/compare/v0.4.1...v0.4.2) (2026-01-07)


### Bug Fixes

* simplify update handling in SettingsView; replace custom update type with plugin's Update type; improve event handling for download progress and completion ([214b619](https://github.com/ljiulong/boxyy/commit/214b619921a572fcf260b1b922db32c400de272f))

## [0.4.1](https://github.com/ljiulong/boxyy/compare/v0.4.0...v0.4.1) (2026-01-07)


### Bug Fixes

* some bug ([2df3648](https://github.com/ljiulong/boxyy/commit/2df364836c25357a980e6e9a779c0626ee9af9f0))

## [0.4.0](https://github.com/ljiulong/boxyy/compare/v0.3.5...v0.4.0) (2026-01-07)


### Features

* integrate tauri-plugin-updater for automatic updates; enhance README with project structure; update CI workflow to include latest.json and signature files; improve macOS path resolution logic ([987a9e1](https://github.com/ljiulong/boxyy/commit/987a9e115f56746fdfe50f008cfae411e4a936d8))

## [0.3.5](https://github.com/ljiulong/boxyy/compare/v0.3.4...v0.3.5) (2026-01-07)


### Bug Fixes

* update README with installation instructions for different OS and add common issues section; refactor CI workflow for Homebrew updates and remove outdated update-brew workflow ([8cbdbe1](https://github.com/ljiulong/boxyy/commit/8cbdbe18b9b8a43f849b42057856039725ba16a4))

## [0.3.4](https://github.com/ljiulong/boxyy/compare/v0.3.3...v0.3.4) (2026-01-07)


### Bug Fixes

* add RELEASE_PLEASE_TOKEN to release-please workflow for enhanced security ([6b85b94](https://github.com/ljiulong/boxyy/commit/6b85b94b1a447a4e62e49ea67672ee454cfdacff))

## [0.3.3](https://github.com/ljiulong/boxyy/compare/v0.3.2...v0.3.3) (2026-01-07)


### Bug Fixes

* add section explaining Boxy as a unified package manager entry point in README ([11c7790](https://github.com/ljiulong/boxyy/commit/11c7790ccf3fe03deea648500ec350b1e2113384))

## [0.3.2](https://github.com/ljiulong/boxyy/compare/v0.3.1...v0.3.2) (2026-01-07)


### Bug Fixes

* update CI workflow to trigger on tag pushes, adjust tag name handling, and ensure proper release asset publishing ([24d96cc](https://github.com/ljiulong/boxyy/commit/24d96cc540b68215b6d0f7ed5bb6de00b586cfff))

## [0.3.1](https://github.com/ljiulong/boxyy/compare/v0.3.0...v0.3.1) (2026-01-07)


### Bug Fixes

* remove outdated command line download instructions from README ([b41019d](https://github.com/ljiulong/boxyy/commit/b41019dd828d65097db9b08d1549f5b7a64ff42d))
* update README with macOS installation instructions and add CLI/TUI usage notes; enhance CI workflow to retry SHA extraction for release assets ([87dbb46](https://github.com/ljiulong/boxyy/commit/87dbb46fc23dca56fa44e1d8498de625f41106fc))
