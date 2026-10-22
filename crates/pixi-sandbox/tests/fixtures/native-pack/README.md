# Local Conda installation fixture

This is a real, tiny `.tar.bz2` Conda package, not a helper executable or a mock
installer. Its archive contains `info/index.json`, `info/paths.json`, and
`share/hello.txt` (`native fixture\n`). The paths metadata and channel repodata
contain SHA256 hashes of the actual bytes. Metadata/archive timestamps are fixed.
There is no build, download, resolver, post-link script, or host executable.

Tests copy this directory into a test-owned home and replace only the format-1
metadata's platform with the test host. Nothing installs from the repository itself.
The sibling runtime-overlay fixture adds another real Conda archive and a standard
wheel with METADATA, WHEEL, RECORD, a module, and an entry point.

The runtime package deliberately contains **non-executable dummy interpreter files**.
Together with an empty PATH, this detects accidental helper/interpreter execution.
It proves file installation and entry-point generation, not execution by a working
interpreter. Installed code must not be described as runtime-smoke-tested.
