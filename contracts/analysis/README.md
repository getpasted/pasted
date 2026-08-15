# Analysis JSON contracts

The `v1` directory contains canonical, privacy-safe examples of the public JSON returned by the Analyzer and each participant surface. Rust serialization tests and CLI integration tests compare real results with these files, while the frontend parity audit checks its mocks against the same shared metadata and result fields.

These fixtures are API contracts, not snapshots to refresh mechanically. Additive fields may be added to version 1 when existing consumers can safely ignore them. Renaming or removing a field, changing its meaning or type, or changing an outcome value requires a new `formatVersion` and fixture directory.

Fixture values deliberately use neutral synthetic input and stable references. Clipboard contents, transformation input, credentials, file paths, and database-generated identifiers do not belong here.
