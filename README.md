# Droplet

Droplet is a `napi.rs` Rust/Node.js package full of high-performance and low-level utils for Droplet.

## Chunker

The `chunker.rs` rapidly splits game files into 64MiB or less chunks to be used in Drop distribution. Files are chunked initially to generate a manifest, checksums (coming soon) and make it easier for the Drop server to distribute them.

## SSL

Due to Drop's TLS-based client authentication and handling, droplet has SSL utilities that generates a Root CA and then can generate leaf certificates based on client ID & names.

This still needs work because currently all certificates expire in 4095 and there is no way to revoke them.
