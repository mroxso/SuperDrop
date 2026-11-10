# SuperDrop implementation plan

## Goal

Build a polished desktop application with a shared Rust core that sends files between macOS, Linux, or Windows desktops and Android devices through Android's built-in Quick Share, in both directions. Install SuperDrop only on the computer; require no companion Android app, account, cloud relay, or separately installed transfer utility.

Start with a MacBook sending one file to a real Android phone over the same local network. Cross-platform support is the destination, not a claim about the initial prototype.

## Starting point and scope

The repository currently contains a README and MIT license, with no implementation. This plan derives from the attached “Rust App for Android Quick Share” discussion, with upstream documentation checked on 2026-11-10.

The first release will support a system tray icon (menu bar on macOS), background receiving with explicit acceptance, a configurable download folder, selecting files, QR-assisted receiver activation, recipient selection/confirmation, transfer progress, cancellation, and clear success or failure feedback. Both devices must be on a LAN that permits peer communication and discovery. A phone must already have a compatible Quick Share implementation.

Defer folders, automatic resume, Google account/contact integration, internet transfers, Wi-Fi Direct, Bluetooth payload transport, and mobile apps. Explore iOS interoperability in a future milestone; it is not a first-release compatibility commitment. Automatic nearby discovery is optional enhancement work after the QR flow succeeds.

## Technical feasibility and decisions

- NearDrop documents LAN transfers and QR activation of Android's receiver, including Samsung devices. It also documents a macOS BLE advertising limitation. Treat QR activation as the primary macOS path; validate actual phone behavior before promising compatibility.
- rquickshare provides a Rust implementation targeting Linux and macOS. Treat Windows as a separate port and validation milestone.
- Audit protocol framing, discovery, handshake, encryption, metadata, acceptance, payload delivery, and QR/session binding against upstream documentation and real devices. A QR code alone does not implement a transfer.
- Review source and dependency licenses before reuse. SuperDrop currently uses MIT; rquickshare uses GPL-3.0. Do not copy or link its implementation under an assumption that the combined result stays MIT. Record a deliberate licensing-compatible dependency or independent implementation decision before implementation proceeds. Preserve required notices for any reused material.
- Proposed stack: a Rust workspace with an async transfer core, a small diagnostic CLI, and a Tauri desktop shell with a lightweight TypeScript UI. Confirm framework and dependency choices after the protocol spike; keep protocol logic independent of the UI.

## Proposed structure

- `crates/core`: protocol messages, authenticated session state, transfer orchestration, streaming file I/O, errors, progress, cancellation, and bounded resource handling.
- `crates/platform`: network interfaces, discovery, optional BLE activation, permissions, and OS integration behind platform adapters.
- `crates/cli`: diagnostic discovery/send/receive commands to exercise the core without a GUI.
- `apps/desktop`: file selection, drop area, QR presentation, incoming transfer approval, recipient confirmation, progress, tray/menu bar controls, notifications, and settings.
- `docs`: protocol/reuse decisions, troubleshooting, compatibility matrix, and release instructions.
- `.github/workflows`: formatting, linting, tests, and builds across the supported OS matrix.

Expose typed commands and events between the shell and Rust. Model transfers as explicit states: idle, preparing, discovering, connecting, awaiting acceptance, transferring, completed, cancelled, and failed. Document timeouts and allowed transitions.

## Milestones

### 1. Resolve protocol and reuse strategy

- [ ] Study NearDrop's protocol documentation and QR flow, plus rquickshare's core and platform boundaries.
- [ ] Record the dependency/license decision, upstream revisions, required notices, and missing capabilities.
- [ ] Specify the minimum handshake, discovery, QR payload/session lifecycle, and encrypted transfer sequence.
- [ ] Specify desktop receiver advertisement, incoming offer handling, acceptance, destination selection, and safe file finalization.
- [ ] Define initial desktop OS versions, CPU architectures, and Android test devices from available hardware.

**Exit:** an actionable protocol design and compatible reuse strategy; unresolved interoperability assumptions are listed explicitly.

### 2. Prove macOS-to-Android transfer

- [ ] Create the Rust workspace and diagnostic CLI using the selected strategy.
- [ ] Implement LAN discovery/connectivity and QR-assisted activation with ephemeral session material.
- [ ] Send one file through the phone's existing Quick Share acceptance flow.
- [ ] Implement streaming, progress, timeout, rejection, cancellation, and cleanup.
- [ ] Exercise empty files, Unicode filenames, and a large file without buffering it entirely in memory.
- [ ] Verify received size and SHA-256 against the original using test-side tools.

**Exit:** repeatable transfers from a MacBook to a real Android phone with no additional phone software. Record phone model, Android/Quick Share versions, network setup, and results. Stop UI expansion and investigate if this requirement cannot be met.

### 3. Prove Android-to-macOS receiving

- [ ] Advertise the desktop receiver so Android's built-in Quick Share can discover it.
- [ ] Show sender identity and file names/sizes before accepting or rejecting an incoming offer; require explicit acceptance by default.
- [ ] Stream accepted files into temporary files in the selected destination and finalize only completed transfers.
- [ ] Sanitize untrusted filenames, prevent path traversal and symlink escapes, and resolve collisions without silently overwriting existing files.
- [ ] Handle insufficient disk space, unwritable destinations, oversized offers, cancellation, disconnects, and partial-file cleanup.
- [ ] Verify received sizes and hashes and repeat transfers from available Google/Pixel-style and Samsung phones.

**Exit:** a real Android phone sends files to the Mac using built-in Quick Share, without companion software, and rejected or failed transfers leave no completed-looking files.

### 4. Build the macOS desktop MVP

- [ ] Add a clear file picker and drag-and-drop surface, selected-file list, and removal controls.
- [ ] Show same-network guidance, a scannable QR code, recipient identity/verification information supported by the protocol, and acceptance status.
- [ ] Display bytes transferred, progress, cancellation, completion, and actionable errors.
- [ ] Handle permission denial, no receiver, expired QR sessions, blocked local traffic, disconnects, and receiver rejection.
- [ ] Support keyboard navigation, accessible labels, readable contrast, and light/dark appearance.
- [ ] Add a persistent system tray/menu bar icon with Open SuperDrop, Send files, receiving visibility/status, Pause receiving, and Quit actions.
- [ ] Keep receiving available when the window closes; expose this behavior clearly in settings. Quit must stop listeners, advertisements, and transfers.
- [ ] Surface incoming offers through notifications that open an approval UI; provide an in-app fallback when notifications are disabled.
- [ ] Make the destination folder configurable and offer Open file / Show in folder after completion.
- [ ] Distinguish paused, available, and transferring tray states with accessible labels; make launch at login optional.
- [ ] Prevent duplicate sends and cleanly stop sessions when the app exits.

**Exit:** both send and receive flows, including receiving with the window closed, work through the packaged app on a clean Mac, with no CLI steps required by the user.

### 5. Add Linux and Windows

- [ ] Implement Linux adapters and document runtime/network dependencies; assess optional BLE activation.
- [ ] Implement Windows networking/discovery and permission/firewall handling; establish QR parity before optional BLE work.
- [ ] Resolve multiple interfaces, VPNs, interface changes, stale discovery records, and port conflicts.
- [ ] Select initial Linux packaging targets and Windows installer format based on tested environments.
- [ ] Integrate tray icons, notifications, background lifecycle, and incoming approval on each platform; provide visible window controls on Linux desktops without a supported tray host.
- [ ] Run the same real-phone send and receive acceptance checks independently on each OS.

**Exit:** each advertised platform has a working installable build and recorded end-to-end results. Compilation alone does not establish compatibility.

### 6. Harden and release

- [ ] Bound frame sizes, metadata lengths, payload counts, queues, memory use, and session lifetimes; reject malformed input.
- [ ] Follow protocol authentication/encryption and recipient verification requirements; do not bypass acceptance or verification for convenience.
- [ ] Keep session secrets ephemeral and redact secrets, file contents, and unnecessary personal data from logs.
- [ ] Stream from explicit user-selected files; define behavior when files change or disappear during a send.
- [ ] Add CI, dependency/license checks, reproducible build instructions, and third-party notices.
- [ ] Prepare macOS signing/notarization, Windows signing, and Linux packages as release infrastructure permits.
- [ ] Publish tested compatibility and troubleshooting guidance for permissions, firewalls, VPNs, and isolated guest networks.

**Exit:** release artifacts pass clean-install smoke tests and the compatibility matrix; outstanding limitations are visible to users.

### 7. Research future iOS interoperability

- [ ] Define whether the goal is exchanging files with existing iPhone system sharing features or supporting a future SuperDrop iOS app.
- [ ] Investigate documented Apple interfaces and any applicable Quick Share interoperability using current primary sources and real iPhones at implementation time.
- [ ] Evaluate desktop-to-iPhone and iPhone-to-desktop separately, including discovery, permissions, sandboxing, background execution, security, and distribution constraints.
- [ ] Prefer a path requiring no phone installation; if a companion app or a different transport is necessary, document that tradeoff for a product decision.
- [ ] Keep transport-specific discovery and protocol implementations behind interfaces so a future iOS-compatible transport can reuse transfer orchestration and desktop UI where practical.

**Exit:** a written feasibility decision with tested evidence, requirements, and a separate implementation scope. Do not promise AirDrop or Quick Share compatibility on iOS before validation.

## Verification strategy

Unit-test framing and parsing, QR encoding/session expiry, state transitions, metadata validation, and cancellation. Use integration tests with controlled peers for handshake failures, rejection, disconnects, malformed input, timeout, and cleanup. Add parser fuzzing where untrusted protocol input warrants it.

Real-device interoperability is the release gate: test macOS, Linux, and Windows in both transfer directions against a Google/Pixel-style device and a Samsung device where hardware is available. Record unavailable combinations as untested. Cover single/multiple files, zero-byte files, Unicode names, a file larger than 1 GB, repeated sends, cancellation, receiver rejection, network loss, and blocked discovery. Also test tray actions, window-close versus Quit behavior, paused receiving, notification denial, incoming acceptance/rejection, filename traversal attempts, collisions, insufficient disk space, and partial-file cleanup. Verify file hashes outside the protocol where necessary. Measure memory and throughput rather than setting unsupported performance promises.

## Risks and fallback decisions

| Risk | Response |
| --- | --- |
| Unofficial protocol changes | Pin known-good references, retain regression fixtures, and maintain real-device checks. |
| QR behavior varies by phone/version | Test early on target devices and document supported combinations; never require a companion app as a silent fallback. |
| macOS discovery limitations | Keep QR activation primary and report discovery failures clearly. |
| LAN isolation, VPN, or firewall blocks peers | Detect what is possible, offer targeted troubleshooting, and fail cleanly. |
| Windows-specific gaps | Keep adapters separate and withhold Windows support claims until device tests pass. |
| Incompatible dependency licensing | Resolve reuse and distribution terms before incorporating code. |

## References

- [NearDrop README: LAN limitations and QR receiver activation](https://github.com/grishka/NearDrop#readme)
- [NearDrop protocol documentation](https://github.com/grishka/NearDrop/blob/master/PROTOCOL.md)
- [rquickshare: Rust implementation and platform limitations](https://github.com/Martichou/rquickshare)
- [rquickshare license](https://github.com/Martichou/rquickshare/blob/master/LICENSE)

## First implementation task

Complete milestone 1, then build the smallest Rust CLI that sends one file from macOS through QR-assisted Quick Share to the user's Android phone. Use that result to finalize the desktop architecture and compatibility commitments.
