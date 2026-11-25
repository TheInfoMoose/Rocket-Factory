## Problem Description
In .github/workflows/release.yml:59,78, GITHUB_REF_NAME is interpolated directly into archive filenames (rktb-${GITHUB_REF_NAME}-${TARGET}.tar.gz and .zip). On workflow_dispatch from a branch containing '/' (e.g., release/v0.1), the slash is treated as a directory separator and tar / Compress-Archive fails because the parent directory does not exist. Sanitize '/' to '-' in the archive filename.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
