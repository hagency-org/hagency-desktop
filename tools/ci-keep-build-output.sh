#!/usr/bin/env bash
# Prepare a checkout on a machine that keeps its work folder between jobs (the
# project's own Mac), so that cargo can reuse what the job before it built.
#
#   bash tools/ci-keep-build-output.sh
#
# actions/checkout removes every untracked file, cargo's `target` folders
# among them, so each job compiled every crate again. With `clean: false` on
# the checkout and this script after it, the checkout is cleaned the same way
# except for:
#
#   target     cargo's build output, at any depth. Cargo rebuilds a crate when
#              one of its files, its dependencies, its features or the
#              compiler changed, and git leaves the time of a file it did not
#              change alone, so a job compiles what differs from the job before
#              it on this machine. The workflow also sets CARGO_INCREMENTAL=1,
#              so that of a crate that did change the compiler redoes only the
#              parts that changed; that data is kept in `target` too.
#   .sources   the pinned source checkouts of .jit/sources.lock. A checkout
#              whose commit is not the pin, or that has changed files, is
#              removed here and fetched again by .jit/sources.sh.
#
# Build output grows: every set of features and every compiler version keeps
# its own files, and incremental data about doubles the size. The `target`
# folders of this checkout are removed, and this job builds everything again,
# when together they pass the limit or when the disk has less free space than
# the reserve. The reserve is what two jobs side by side may add.
#
# Release builds do not use this: release.yml runs on machines that start
# empty.
set -euo pipefail

limit_gb="${CI_BUILD_OUTPUT_LIMIT_GB:-120}"
reserve_gb="${CI_BUILD_OUTPUT_FREE_GB:-80}"
root="$(git rev-parse --show-toplevel)"
cd "$root"

git clean -ffdx -e target -e .sources >/dev/null

if [ -f .jit/sources.lock ] && [ -d .sources ]; then
    while read -r name _url rev _rest; do
        case "$name" in '' | '#'*) continue ;; esac
        dir=".sources/$name"
        [ -e "$dir" ] || continue
        if [ -L "$dir" ]; then
            # A link leads out of this folder; a job never made one here.
            rm -f "$dir"
            echo "removed  $dir (a link)"
        elif [ "$(git -C "$dir" rev-parse HEAD 2>/dev/null)" != "$rev" ] ||
            [ -n "$(git -C "$dir" status --porcelain 2>/dev/null | head -1)" ]; then
            rm -rf "$dir"
            echo "removed  $dir (not the pinned commit, or changed files)"
        else
            echo "kept     $dir at $rev"
        fi
    done <.jit/sources.lock
fi

# -prune: a `target` folder is counted whole and never searched for more.
folders=()
while IFS= read -r folder; do
    folders+=("$folder")
done < <(find . -path ./.git -prune -o -path ./.sources -prune -o -type d -name target -prune -print)

if [ "${#folders[@]}" = 0 ]; then
    echo "no build output from an earlier job: this job builds everything"
    exit 0
fi
used_kb="$(du -sk "${folders[@]}" | awk '{ sum += $1 } END { print sum + 0 }')"
used_gb=$((used_kb / 1024 / 1024))
free_gb="$(df -Pk . | awk 'NR == 2 { print int($4 / 1024 / 1024) }')"
if [ "$used_gb" -ge "$limit_gb" ] || [ "$free_gb" -lt "$reserve_gb" ]; then
    rm -rf "${folders[@]}"
    echo "removed  ${#folders[@]} build folders: ${used_gb} GB (limit ${limit_gb} GB), ${free_gb} GB free on the disk (reserve ${reserve_gb} GB). This job builds everything."
else
    echo "kept     ${#folders[@]} build folders, ${used_gb} GB (limit ${limit_gb} GB), ${free_gb} GB free on the disk (reserve ${reserve_gb} GB)"
fi
