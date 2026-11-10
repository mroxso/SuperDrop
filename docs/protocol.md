# Protocol decision and implementation status

Decision: preserve MIT licensing through an independent implementation. No NearDrop or rquickshare source is copied or linked. rquickshare is GPL-3.0 and must not be introduced as an MIT dependency. Descriptions below are protocol facts, not copied implementation code. Dependency versions are pinned by Cargo.lock; review their licenses again before distribution. No upstream implementation notices are required for code that has not been reused.

References inspected on 2026-11-10:

- NearDrop `add655c6c5a40b9ba03c0329b598f2817185f5a9`, PROTOCOL.md: https://github.com/grishka/NearDrop/blob/add655c6c5a40b9ba03c0329b598f2817185f5a9/PROTOCOL.md
- rquickshare `378d8ae969941bee4bf60ad34ac9cf8bb7005eb7`, core_lib/Cargo.toml: https://github.com/Martichou/rquickshare/blob/378d8ae969941bee4bf60ad34ac9cf8bb7005eb7/core_lib/Cargo.toml

rquickshare separates a Rust core library from its Tauri application. Its platform dependencies and GPL licensing mean it is a reference, not the selected backend. Windows interoperability remains a separate unverified port. No OS version, CPU, or phone combination is certified by this implementation.

## Activation implemented

Generate an in-memory P-256 key, encode two zero version bytes followed by the 33-byte compressed public key, and show the Quick Share URL as a terminal QR. HKDF-SHA256 derives advertisingContext and encryptionKey keys. Browse `_FC9F5ED42C8A._tcp.local.`. Match the endpoint's QR TLV by advertising token or authenticate a hidden device name with AES-128-GCM and the advertising token as additional data. Expire sessions after 120 seconds. No QR private key is logged or persisted.

A matching advertisement is only a candidate recipient. It does not establish a secure transfer or prove a user-visible identity. Require recipient confirmation and the protocol authentication code before any future payload transfer.

## Required next protocol implementation

1. Connect to a QR-bound receiver's advertised LAN address, with a bounded connection timeout.
2. Exchange Nearby Connections connection request and UKEY2 ClientInit/ServerInit/ClientFinish. Validate commitments and preserve raw transcript bytes. Use Google's UKEY2 specification; do not invent a substitute handshake.
3. Derive authentication and next-protocol secrets. Derive directional D2D keys with SHA256("D2D") salt and client/server info, then encryption/HMAC keys with SHA256("SecureMessage") salt and ENC:2/SIG:1 info.
4. Exchange connection responses. Authenticate secure message headers before decrypting AES-256-CBC bodies. Enforce independent monotonic sequence numbers.
5. Exchange paired-key encryption/result frames. Preserve explicit phone acceptance; do not use QR proof to bypass prompts.
6. Send introduction with bounded file metadata and payload IDs. Await ACCEPT before streaming file chunks. Send keepalives every ten seconds. Reject malformed offsets, lengths and sequence numbers. Cancellation must close the connection and discard pending work.
7. Stream bounded chunks from an explicitly selected open file, detect changes/truncation, and report acknowledged completion accurately. A QR match alone must never be reported as transfer success.

The four-byte big-endian framing helpers enforce a one-MiB allocation limit. Socket deadlines, protobuf schemas, UKEY2, secure envelopes, acceptance and payload transport are still missing.

## Receiver design, pending implementation

Advertise a fresh four-character endpoint ID with the documented service ID and endpoint-info TXT record. Display untrusted sender and file metadata safely and require explicit approval. Bound concurrent offers, aggregate bytes, file count and metadata lengths. Open the chosen destination directory securely; create exclusive temporary files without following symlinks. Reject absolute paths, separators, dot components and platform-reserved names. Resolve collisions using exclusive creation, never overwrite. Finalize only after exact byte-count validation and completion; remove temporary files on rejection, error, shutdown or cancellation. Report disk-space and permission errors before acceptance when detectable.

## Gates

No Android device has been exercised. Milestone 2 has not passed, so PLAN.md explicitly blocks desktop UI expansion. All send/receive combinations remain untested, including macOS. Hardware validation must record phone model, Android/Quick Share versions, desktop version/architecture, network and independent SHA-256 checks. Empty, Unicode and >1GB files, cancellation, rejection and network loss remain mandatory. Desktop packaging, background receiving, Linux/Windows, signing and iOS feasibility are pending. iOS system interoperability is not promised.
