# Compact v7 lock records

Tiny genuine Conda tar.bz2 archives (USTAR, bzip2) containing `info/index.json`,
`info/files`, and `share/compact.txt`. Index and repodata agree on each package's
build number and noarch kind. Files have mode 0644 and timestamp zero.

These public writer tests remove URL-derived fields from repodata to model v7
lock serialization. Cases cover numeric-only builds, trailing decimal digits,
explicit non-trailing build numbers, no numeric suffix (default zero), and inferred
Python/generic noarch kinds. <!-- stale-ref-allowed: Conda runtime package kind. --> Archives are local fixture data, not downloaded packages.

Semantics: rattler_lock's v7 `CondaPackageDataModel` and
`utils/derived_fields.rs` (`derive_build_number_from_build`, `derive_noarch_type`):
https://github.com/conda/rattler/tree/main/crates/rattler_lock/src
