# Changelog

All notable changes to SnapPea are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [1.2.0] - 2026-10-23

### Added

- Audio recording for system audio and the microphone.
- A mute button with a live level meter in the recording toolbar.
- The editor shows the audio track in the scrubber.
- Default save folders are now Screenshots and Screencasts.

### Fixed

- Exports from the editor kept dropping the audio track.
- The microphone fades in at the start of a recording.
- When the Wayland connection is lost, the portal exits too.

## [1.1.0] - 2026-10-19

### Added

- A toggle to disable the recording region border.

### Changed

- Enter is the capture key.
- The recording border is not drawn over the screen edges.

### Fixed

- Enter started a delayed capture even when a delay was set.
- The capture delay no longer defaults to 3 seconds.

## [1.0.0] - 2026-10-13

### Changed

- Redesigned interface (#23).
- Follows the screenshot portal and returns the screenshot as a response.
- Fewer redraws when nothing is being drawn.

## [0.9.1] - 2026-09-03

- Initial release.
