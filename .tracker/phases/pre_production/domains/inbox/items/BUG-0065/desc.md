## Problem Description
In .github/workflows/release.yml:13-92, all three parallel matrix jobs (ubuntu-latest, macos-latest, windows-latest) invoke softprops/action-gh-release directly on v* tag pushes without a pre-created release or a single downstream publish job. Runners finishing close together can race on POST /releases and fail with HTTP 422 already_exists. Split into a matrix build job (actions/upload-artifact) followed by a single publish job (actions/download-artifact + softprops/action-gh-release).

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
