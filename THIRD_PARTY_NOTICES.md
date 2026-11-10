# Third-party notices

The protobuf schemas in `crates/core/proto/*.proto` are Copyright 2020 Google LLC, licensed under Apache License 2.0. They were obtained from NearDrop revision `add655c6c5a40b9ba03c0329b598f2817185f5a9`, `NearbyShare/ProtobufSource`, unchanged. The license is included as `crates/core/proto/LICENSE-APACHE-2.0`. Their original copyright and license headers are retained. Generated Rust schema bindings derive from these files.

SuperDrop's original Rust implementation remains MIT. No GPL rquickshare implementation is incorporated. Cargo dependencies retain their respective licenses; release packaging must include the applicable notices.

Protocol reference: Google's UKEY2 specification and reference implementation at https://github.com/google/ukey2 (consulted 2026-11-10). In particular the deployed implementation uses SHA-256 of the ECDH result, SHA-256 HKDF, and a NUL-terminated authentication salt and an un-terminated next-secret salt. SHA-512 is used for the client-finish commitment. TCP length prefixes are not included in the UKEY2 transcript.
