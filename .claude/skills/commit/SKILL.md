---
name: commit
description: Run cargo fmt + clippy, then create a conventional commit. Use this skill whenever the user says "commit", "checkin", "提交", "格式化并提交", or asks to finalize changes. Also trigger when the user finishes implementing a feature or fix and wants to wrap up — even if they don't explicitly say "commit".
---

## Workflow

Execute these steps in order. Do not skip any step.

### Step 1: Format

```bash
cargo fmt --all
```

Run unconditionally. If it changes files, note which ones for the commit message body.

### Step 2: Lint

```bash
cargo clippy --workspace
```

- If warnings exist: fix them, re-run `cargo fmt --all` if needed, then re-run clippy to confirm clean.
- If errors exist: fix them before proceeding. Do not commit code that fails clippy.
- If clean: proceed.

### Step 3: Stage and review

```bash
git status
git diff --staged
git diff
```

Stage relevant files with `git add <specific files>`. Avoid `git add -A` or `git add .`.

### Step 4: Write commit message

Analyze `git diff --cached` and recent conversation context to determine the commit type and scope.

**Format:** `type(scope): imperative description`

Types: `feat`, `fix`, `refactor`, `style`, `docs`, `test`, `build`, `ci`, `chore`, `perf`

Rules:

- Scope is the crate or module most affected (e.g., `cache`, `gameplay`, `model`). Omit scope if changes span many crates.
- Description: lowercase, imperative mood, no period, under 72 chars for the subject line.
- Body (optional): blank line after subject, then concise explanation of *why* — not *what*. Wrap at 80 chars.
- If `cargo fmt` changed files, mention it in the body: `Includes cargo fmt changes.`

**Examples:**

```
fix(cache): skip version check when no version requested

Serves local file directly when the client requests without a version
parameter, preventing unnecessary re-downloads for unversioned resources.
```

```
feat(gameplay): add ship modernization logic
```

```
style: resolve workspace clippy warnings
```

### Step 5: Commit

```bash
git commit -m "$(cat <<'EOF'
<commit message here>
EOF
)"
```

Verify with `git status` after commit.

### Important

- Never push unless the user explicitly asks.
- Never use `--no-verify` or `--no-gpg-sign`.
- If a pre-commit hook fails, fix the issue and create a NEW commit (do not amend).
- Write commit messages in English.
