# LSP immutable release preparation — 14 October 2026

This continues the approved extension publication design. It hardens the existing narrow LSP pipeline and prepares reusable publication guards. It does not publish a candidate, merge private PR #3484, or establish a public CI deployment path.

## Observed state

- Private source: `https://git.terraphim.cloud/terraphim/terraphim-ai/pulls/3484`, head `34b890841184fd96208fb11f4e2e59aceee90c7a`. Readback on 14 October: open, not merged, zero submitted reviews, `mergeable=false`; existing native build/Clippy statuses are successful.
- Public `terraphim_lsp-v1.22.0-rc.2` is a prerelease. Its annotated tag peels to `cdfb79e8b2b421d62277410fcb0975c1a1f8dac3`; the old workflow exists at that tag.
- Public main has no `release-lsp.yml`. The new private source SHA returns GitHub HTTP 422 (not present). Public rc.3 release returns HTTP 404.
- Public-repository self-hosted macOS and Linux/bigbox runners are online. Runner availability does not prove access to the current private sources/registry or authorize copying private history.
- The LSP requires private registry crates and a pinned private `terraphim_alternatives` Git dependency. Its existing six raw macOS/Linux binary names and `checksums.txt` remain the provisioning contract. No Windows build is claimed.

## Prepared changes

The source guard validates the actual checkout's repository, exact commit, crate version, existing local tag and ancestry in fetched `origin/main`. Publishing requires the workflow itself to run at the resolved source. A dry run can validate a missing prospective tag but cannot publish it. The guard accepts only the existing Terraphim public or private source repository; it records which one was actually checked out.

A mandatory qualification job runs pinned-toolchain formatting, locked core/LSP tests (including real save/export and stdio tests), and warnings-denied Clippy. It preserves command logs and a source-bound receipt even when a command fails. Both build jobs depend on that gate. These source tests do not substitute for native stock-Zed interaction receipts.

Assembly retains the six raw assets and checksum format, adding `lsp-release-manifest.json` with exact source identity, qualification/log hashes, toolchain, signing state and all binary/checksum hashes. Publication revalidates and snapshots those bytes. It can recover an owned draft by uploading only missing assets; it first downloads and compares every existing asset. Changed bytes, an incomplete published release, duplicate or unexpected assets, an unowned release, or mismatched source/tag are refused. It never moves a tag or replaces an asset. Eventual draft-list consistency is handled by bounded reads, not repeated creation. Every public asset is then independently downloaded without authentication and compared. Failure receipts are retained.

The script uses the established VS Code draft/resume/frozen-artifact policy, with an exact manifest ownership marker. Direct tests use real temporary Git histories/tags and real files. Metadata fixtures validate the pure planning policy only; no fake `gh`, subprocess patch, HTTP mock or fixture is counted as remote publication evidence.

## Deployment integration remains a gate

The current workflow preserves its existing same-source GitHub execution contract. Its checkout and tag resolver cannot retrieve the new private Gitea commit from public GitHub, and the public publisher deliberately requires the public immutable tag to resolve to the exact qualified source. Consequently, installing this workflow alone would **not** release the new candidate.

The next integration must retain private builds and hand only frozen binary assets, checksums and reviewed source provenance to a scoped public publisher. Before changing that topology, define and independently review how its immutable public tag proves the exact private source repository/commit/tag and qualification. A tag on an arbitrary public main commit is insufficient. Do not push private history to satisfy the present guard, weaken the tag comparison, or assume new credentials. A narrowly scoped release-control/provenance mechanism is a separate integration step; it is not implemented or claimed here.

Before actual publication: resolve PR #3484's merge gate without bypassing protection; retain the missing native stock-Zed Trim Done/clean-export evidence; qualify all six builds from the merged immutable source; establish the reviewed public handoff; then execute and inspect real CI publication and anonymous bytes. The current local tests certify guards and package metadata, not successful public publishing or all 41 editor requirements. Existing released tags and assets remain untouched.

## Verification of this preparation

The two new workflow regressions first failed on the old mutable publisher and absent source gate; a separate regression demonstrated the old global `/tmp` keychain cleanup. The repaired workflow cleans only its unique signing directory. All **75** focused LSP/helper and adjacent release-recovery tests pass on macOS, and actionlint passes both changed workflows. The checksum test now uses portable `chmod +x ./*`; this fixes the local GNU/BSD command mismatch without claiming a Linux run.

The private branch initially inherited the old incomplete Continue/VS Code writer inventory. That exact failure was reproduced from a clean archive of its original HEAD, then repaired by porting the already reviewed public PR #973 classification/guard tests and CI paths. The exhaustive inventory now includes the Python LSP publisher and passes; no writer was suppressed.

A real read-only network check through the new helper verified the existing rc.2 tag and anonymously downloaded its `checksums.txt` (SHA-256 `0524b1b7edc3de0e16db1eb8fc50f4c4e641bb26cc15ba31dc51f9c53affdf97`). Supplying the new private candidate SHA for that old tag was refused. This proves the read-only tag/download path for rc.2; the new candidate's publication remains **INTEGRATION_PENDING**. No remote mutation transport was mocked or executed.
