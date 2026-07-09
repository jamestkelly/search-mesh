fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

lint:
    cargo clippy --workspace --all-targets -- -D warnings

test:
    cargo test --workspace

check: fmt-check lint test

update-local:
    git fetch -p
    git branch -vv | awk '/: gone]/{print ($1 == "*" ? $2 : $1)}' | while read branch; do git branch -D "$branch"; done
    git pull
