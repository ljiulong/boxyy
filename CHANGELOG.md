# Changelog

## [1.2.1](https://github.com/ljiulong/boxyy/compare/v1.2.0...v1.2.1) (2026-01-17)


### Bug Fixes

* **gui:** 任务完成后始终强制刷新包列表 ([b081ee3](https://github.com/ljiulong/boxyy/commit/b081ee324c04ae858b680b105c893b61737e6719))
* **gui:** 修复任务删除/清空失败时的状态恢复竞态条件 ([5f94463](https://github.com/ljiulong/boxyy/commit/5f9446310d5c0d50dc4b46c767c047148cd46121))
* **gui:** 修复删除任务无即时反馈的问题 ([fe11a08](https://github.com/ljiulong/boxyy/commit/fe11a08773a4ae952a34bbc1cafce18852ac1ab8))
* **gui:** 修复单个管理器刷新后统计数据不更新的问题 ([5edfc99](https://github.com/ljiulong/boxyy/commit/5edfc99d2a90e6becd45aca61a89a9dc39fe54f7))
* **gui:** 导出 loadManagers 函数以修复运行时错误 ([1d056ed](https://github.com/ljiulong/boxyy/commit/1d056eda526a134f7a439e6138b82d76079bc397))
* **gui:** 添加任务操作失败时的状态回滚机制 ([5da5096](https://github.com/ljiulong/boxyy/commit/5da50960892c45863ee205b1190a340faf13a769))


### Performance Improvements

* **gui:** 优化 GUI 扫描性能 - 避免不必要的缓存清除 ([e9e6d46](https://github.com/ljiulong/boxyy/commit/e9e6d4681fe2a14c68d393e5fba975a4933ce633))
* 优化扫描性能 - 提升缓存利用率和并发度 ([646ab3c](https://github.com/ljiulong/boxyy/commit/646ab3c79f618460388d7301564c661f3033fb40))

## [1.2.0](https://github.com/ljiulong/boxyy/compare/v1.1.0...v1.2.0) (2026-01-15)


### Features

* force fresh package lists in CLI and TUI ([16b506f](https://github.com/ljiulong/boxyy/commit/16b506f9dd402b6d6410a3012c19d4a8efa39c06))


### Bug Fixes

* add local scope guard in task-complete package refresh handler ([d88d007](https://github.com/ljiulong/boxyy/commit/d88d00788068909d24c3ccd1d89e9829421c4e46))
* add local scope guard in task-complete package refresh handler ([a9a2f60](https://github.com/ljiulong/boxyy/commit/a9a2f603cbe2e12660792a69ee0a95ac8c3b6166))

## [1.1.0](https://github.com/ljiulong/boxyy/compare/v1.0.0...v1.1.0) (2026-01-13)


### Features

* add one-line installation script for Linux ([6534d6c](https://github.com/ljiulong/boxyy/commit/6534d6cba5ab8b525eddecb396974a86f2c29d17))
* add one-line installation script for Linux ([4e52296](https://github.com/ljiulong/boxyy/commit/4e52296c0b319e1d817d3dcf9edbea0f95485424))


### Bug Fixes

* improve binary verification to detect corrupted files ([0876ebc](https://github.com/ljiulong/boxyy/commit/0876ebc6841b6ae4bcef8321bc10fa4a3ad4a4bb))
* reject unsupported architectures explicitly ([9468609](https://github.com/ljiulong/boxyy/commit/94686097e216262a6786e0e86a28546ff5fa9248))

## [1.0.0](https://github.com/ljiulong/boxyy/compare/v0.6.4...v1.0.0) (2026-01-09)


### ⚠ BREAKING CHANGES

* 卸载行为变更，现在默认自动清理缓存

### Features

* add --clean-cache option to uninstall command ([6e192e9](https://github.com/ljiulong/boxyy/commit/6e192e9edcc9c13b936af3bbd8e489f36cc0c828))
* enable cache cleaning by default for all interfaces ([eac34bf](https://github.com/ljiulong/boxyy/commit/eac34bf87c796cda3d9ad9726edc951db48caf04))

## [0.6.4](https://github.com/ljiulong/boxyy/compare/v0.6.3...v0.6.4) (2026-01-09)


### Bug Fixes

* add window drag permission for Tauri 2.0 ([69ab9e9](https://github.com/ljiulong/boxyy/commit/69ab9e98670f691f59d249d7220203a998c0d829))

## [0.6.3](https://github.com/ljiulong/boxyy/compare/v0.6.2...v0.6.3) (2026-01-07)


### Bug Fixes

* release 0.6.2 and update versioning across multiple files ([c754c81](https://github.com/ljiulong/boxyy/commit/c754c815bd1340131f22b7244b60c1c72e71826d))
* update jsonpath structure in release-please-config for version extraction ([3835c9a](https://github.com/ljiulong/boxyy/commit/3835c9af5e8ebf720ef476854565be06204d2717))
* update version in Cargo.toml to 0.6.2 and simplify release-please-config structure ([61d594c](https://github.com/ljiulong/boxyy/commit/61d594c743070c0c5f4d7dcf45bf6b5f9770f45c))

## [0.6.2](https://github.com/ljiulong/boxyy/compare/v0.6.1...v0.6.2) (2026-01-07)


### Bug Fixes

* update Cargo.lock and add tauri-plugin-process; enhance permission schemas for process management ([ee27360](https://github.com/ljiulong/boxyy/commit/ee2736069aabb61785837815ce22b21a7697602c))

## [0.6.1](https://github.com/ljiulong/boxyy/compare/v0.6.0...v0.6.1) (2026-01-07)


### Bug Fixes

* **macos:** implement PATH management for macOS to enhance environment setup ([7e042f4](https://github.com/ljiulong/boxyy/commit/7e042f456c6ae110b6978cbb624543dd793fae73))

## [0.6.0](https://github.com/ljiulong/boxyy/compare/v0.5.6...v0.6.0) (2026-01-07)


### Features

* enhance Boxy package manager with cask support and improve README documentation ([e552f19](https://github.com/ljiulong/boxyy/commit/e552f19caf1983814491e6613748818158cda0c2))

## [0.5.6](https://github.com/ljiulong/boxyy/compare/v0.5.5...v0.5.6) (2026-01-07)


### Bug Fixes

* update release workflow to include tag name from release-please and adjust GUI title bar style ([89588c3](https://github.com/ljiulong/boxyy/commit/89588c33f4924e143c79d854d4b003990ae005fd))

## [0.5.5](https://github.com/ljiulong/boxyy/compare/v0.5.4...v0.5.5) (2026-01-07)


### Bug Fixes

* enhance JSON generation in release workflow for better readability and maintainability ([b03aa18](https://github.com/ljiulong/boxyy/commit/b03aa185adbdccc796002d574d705e08ad37cbb3))
* improve error handling in release workflow for updater artifacts ([66c4f79](https://github.com/ljiulong/boxyy/commit/66c4f797e802ca3f29e89c8f5bc5e08e8fd362b3))
* streamline JSON handling in release workflow for improved clarity ([4e6bf7c](https://github.com/ljiulong/boxyy/commit/4e6bf7c17cb565a11b9a623e983ff69f5556ed4a))

## [0.5.4](https://github.com/ljiulong/boxyy/compare/v0.5.3...v0.5.4) (2026-01-07)


### Bug Fixes

* update README to include open source license section ([3d18b29](https://github.com/ljiulong/boxyy/commit/3d18b297833a3030a9b2e734e5fdf2e1a3f15614))

## [0.5.3](https://github.com/ljiulong/boxyy/compare/v0.5.2...v0.5.3) (2026-01-07)


### Bug Fixes

* update createUpdaterArtifacts option in tauri config for version compatibility ([3ed5695](https://github.com/ljiulong/boxyy/commit/3ed56957478e9046e8a98f9fa9bfd0facc84be76))

## [0.5.2](https://github.com/ljiulong/boxyy/compare/v0.5.1...v0.5.2) (2026-01-07)


### Bug Fixes

* rename environment variables for Tauri signing in CI workflow ([ab752cc](https://github.com/ljiulong/boxyy/commit/ab752cc94fdf8dd202f31d2eae3c55cb94d4bd8c))

## [0.5.1](https://github.com/ljiulong/boxyy/compare/v0.5.0...v0.5.1) (2026-01-07)


### Bug Fixes

* enable updater artifacts in tauri config ([8d95928](https://github.com/ljiulong/boxyy/commit/8d959287bf9f1f1dc47784eec9e90e13bc7ab393))

## [0.5.0](https://github.com/ljiulong/boxyy/compare/v0.4.4...v0.5.0) (2026-01-07)


### Features

* add updater option to Tauri build command for enhanced application updates ([8439f07](https://github.com/ljiulong/boxyy/commit/8439f07eadff08b4b6d0006b03360fb82dd894fe))

## [0.4.4](https://github.com/ljiulong/boxyy/compare/v0.4.3...v0.4.4) (2026-01-07)


### Bug Fixes

* update dependencies in Cargo.lock and add tracing workspace in boxy-gui ([8ba31d2](https://github.com/ljiulong/boxyy/commit/8ba31d2988e190d14350329bd5b38a990e367114))

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
