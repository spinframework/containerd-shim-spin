# Changelog

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- 

## [v0.23.0] - 2026-02-25

### Added
- Add integration test for directly running the shim with docker ([#377](https://github.com/spinframework/containerd-shim-spin/pull/377))

### Changed
- Update to use Spin v3.6.1 dependencies and support packages pushed with `wkg` ([#392](https://github.com/spinframework/containerd-shim-spin/pull/392))
- Dependency updates ([#372](https://github.com/spinframework/containerd-shim-spin/pull/372), [#373](https://github.com/spinframework/containerd-shim-spin/pull/373), [#375](https://github.com/spinframework/containerd-shim-spin/pull/375), [#393](https://github.com/spinframework/containerd-shim-spin/pull/393))

### Fixed
- Fix node-installer logic for k3s and rke2 ([#389](https://github.com/spinframework/containerd-shim-spin/pull/389))
- Fix broken spin dotnet container build ([#391](https://github.com/spinframework/containerd-shim-spin/pull/391))

## [v0.22.0] - 2025-10-31

### Changed
- Update to use Spin v3.5.0 dependencies ([#369](https://github.com/spinframework/containerd-shim-spin/pull/369))
- Dependencies updates ([#364](https://github.com/spinframework/containerd-shim-spin/pull/364), [#365](https://github.com/spinframework/containerd-shim-spin/pull/365), [#366](https://github.com/spinframework/containerd-shim-spin/pull/366), [#367](https://github.com/spinframework/containerd-shim-spin/pull/367))

## [v0.21.0] - 2025-08-27

### Changed
- Update to use Spin v3.4.0 dependencies ([#355](https://github.com/spinframework/containerd-shim-spin/pull/355))
- Add script to easily update project version before releases ([#200](https://github.com/spinframework/containerd-shim-spin/pull/200))

## [v0.20.0] - 2025-06-25

### Changed

- Configure trigger builder args to set SPIN_MAX_INSTANCE_MEMORY ([#333](https://github.com/spinframework/containerd-shim-spin/pull/333))
- Update to use Spin v3.3.1 dependencies ([#331](https://github.com/spinframework/containerd-shim-spin/pull/331))
- Bump containerd-shim-wasm to v1.0.0 ([#314](https://github.com/spinframework/containerd-shim-spin/pull/314))
- Add project security policy ([#310](https://github.com/spinframework/containerd-shim-spin/pull/310))


## [v0.19.0] - 2025-03-28

### Changed

- Bump Spin dependencies to v3.2.0 ([#302](https://github.com/spinkube/containerd-shim-spin/pull/302))
- Bump containerd-shim-wasm to v1.0.0-rc.1 ([#299](https://github.com/spinkube/containerd-shim-spin/pull/299))
- Update `node-installer` to work with K3s which already configures the Spin shim ([#301](https://github.com/spinframework/containerd-shim-spin/pull/301))
- In `node-installer`, add SystemCgroup option to containerd runtime options for most K8s distributions ([#289](https://github.com/spinframework/containerd-shim-spin/pull/289))

## [v0.18.0] - 2025-01-14

### Changed

- Use component filtering utils from Spin crate, removing redundant `retain` module ([#213](https://github.com/spinkube/containerd-shim-spin/pull/213))
- Bump Spin dependencies to v3.1.2 ([#263](https://github.com/spinkube/containerd-shim-spin/pull/263))
- Updated the minimum required Rust version to 1.81

## [v0.17.0] - 2024-11-08

### Added

- Added component filtering based on env var `SPIN_COMPONENTS_TO_RETAIN` ([#197](https://github.com/spinkube/containerd-shim-spin/pull/197))
- Improved error hanlding in selective deployment ([#229](https://github.com/spinkube/containerd-shim-spin/pull/229))

### Changed

- Turn off native unwinding from Wasmtime Config to avoid faulty libunwind detection errors ([#215](https://github.com/spinkube/containerd-shim-spin/pull/215))
- Updated the spin version to v3.0.0 ([#230](https://github.com/spinkube/containerd-shim-spin/pull/230))

### Fixed

- FIxed CI errors due to old versions of Go and TinyGo and disk pressure ([#217](https://github.com/spinkube/containerd-shim-spin/pull/217))


## [v0.16.0] - 2024-10-04

### Added

- Added MQTT trigger and tests ([#175](https://github.com/spinkube/containerd-shim-spin/pull/175))
- Make container environment variables accessible as application variables ([#149](https://github.com/spinkube/containerd-shim-spin/pull/149))
- Added feature to conditionally restart the k0s controller service when present during node installation. ([#167](https://github.com/spinkube/containerd-shim-spin/pull/167))

### Changed

- Updated the minimum required Rust version to 1.79 ([#191](https://github.com/spinkube/containerd-shim-spin/pull/191))
- Refactored the shim code by splitting it into different modules ([#185](https://github.com/spinkube/containerd-shim-spin/pull/185))
- Refactored the Makefile to improve its structure and comments([#171](https://github.com/spinkube/containerd-shim-spin/pull/171))
- Merged two Redis trigger test apps into one ([#176](https://github.com/spinkube/containerd-shim-spin/pull/176))
- Simplified the run command in the documentation ([#184](https://github.com/spinkube/containerd-shim-spin/pull/184))
-  Modified Dependabot settings to group patch-level dependency updates ([#162](https://github.com/spinkube/containerd-shim-spin/pull/162))

### Fixed

- Correct currently supported triggers ([#182](https://github.com/spinkube/containerd-shim-spin/pull/182))
- Fixed an error in `setup-linux.sh` script ([#184](https://github.com/spinkube/containerd-shim-spin/pull/184))
- Updated outdated links to `spinkube.dev` ([#170](https://github.com/spinkube/containerd-shim-spin/pull/170))

---

[Unreleased]: <https://github.com/spinframework/containerd-shim-spin/compare/v0.23.0..HEAD>
[v0.23.0]: <https://github.com/spinframework/containerd-shim-spin/compare/v0.22.0...v0.23.0>
[v0.22.0]: <https://github.com/spinkube/containerd-shim-spin/compare/v0.21.0...v0.22.0>
[v0.21.0]: <https://github.com/spinkube/containerd-shim-spin/compare/v0.20.0...v0.21.0>
[v0.20.0]: <https://github.com/spinkube/containerd-shim-spin/compare/v0.19.0...v0.20.0>
[v0.19.0]: <https://github.com/spinkube/containerd-shim-spin/compare/v0.18.0...v0.19.0>
[v0.18.0]: <https://github.com/spinkube/containerd-shim-spin/compare/v0.17.0...v0.18.0>
[v0.17.0]: https://github.com/spinkube/containerd-shim-spin/compare/v0.16.0...v0.17.0
[v0.16.0]: https://github.com/spinkube/containerd-shim-spin/compare/v0.15.1...v0.16.0
