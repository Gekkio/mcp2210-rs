<!--
SPDX-FileCopyrightText: 2023 Joonas Javanainen <joonas.javanainen@gmail.com>

SPDX-License-Identifier: CC0-1.0
-->

# Change Log

## [Unreleased]

## 0.2.0 - 2023-12-12

### Added

- `Debug` implementation for `UsbParameters`
- New example: `info`, which dumps all information and state from MCP2210

### Changed

- Improved loopback example
- Improved example code in README
- Switched from libudev + raw hidraw (Linux only) to `hidapi`, which works on macOS and Windows too. **Breaking change**
- Many functions in the API have been redesigned. **Breaking change**
- Bump MSRV to 1.63

## 0.1.0 - 2019-07-17

- Initial release
