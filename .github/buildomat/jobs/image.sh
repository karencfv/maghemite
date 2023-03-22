#!/bin/bash
#:
#: name = "image"
#: variety = "basic"
#: target = "helios-latest"
#: rust_toolchain = "stable"
#: output_rules = [
#:   "/out/*",
#: ]
#: access_repos = [
#:   "oxidecomputer/dendrite",
#: ]
#:
#: [[publish]]
#: series = "image"
#: name = "maghemite.tar"
#: from_output = "/out/maghemite.tar"
#:
#: [[publish]]
#: series = "image"
#: name = "maghemite.sha256.txt"
#: from_output = "/out/maghemite.sha256.txt"
#:
#: [[publish]]
#: series = "image"
#: name = "mg-ddm.tar.gz"
#: from_output = "/out/mg-ddm.tar.gz"
#:
#: [[publish]]
#: series = "image"
#: name = "mg-ddm.sha256.txt"
#: from_output = "/out/mg-ddm.sha256.txt"
#:

set -o errexit
set -o pipefail
set -o xtrace

cargo --version
rustc --version

banner build
ptime -m cargo build --release --verbose -p ddmd -p ddmadm

banner image
ptime -m cargo run -p mg-package

banner contents
tar tvfz out/maghemite.tar

banner copy
pfexec mkdir -p /out
pfexec chown "$UID" /out
mv out/maghemite.tar /out/maghemite.tar
mv out/mg-ddm.tar.gz /out/mg-ddm.tar.gz

banner checksum
cd /out
digest -a sha256 maghemite.tar > maghemite.sha256.txt
digest -a sha256 mg-ddm.tar.gz > mg-ddm.sha256.txt

