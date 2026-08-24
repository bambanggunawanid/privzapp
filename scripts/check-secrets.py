#!/usr/bin/env python3
"""Secret scanner for a public repo: refuses credentials in committed code.

Modes:
  --staged   scan files staged for commit (used by .githooks/pre-commit)
  --all      scan every tracked file (used by scripts/verify.sh and CI)

Secrets belong in `.env` (gitignored; see .env.example) — never in code.
A true false positive can be waived by putting `pz:allow-secret` on the
same line. Stdlib only; no network, ever.
"""

import re
import subprocess
import sys

# Paths that are vendored, binary-heavy, or are this scanner itself.
EXCLUDED = (
    "app/assets/pdfjs/",
    "crates/pz-img/fonts/",
    "app/brand/",
    "app/pwa/",
    "Cargo.lock",
    "scripts/check-secrets.py",
)

BINARY_EXT = (
    ".png", ".ico", ".jpg", ".jpeg", ".webp", ".gif", ".ttf", ".otf",
    ".woff", ".woff2", ".wasm", ".pdf", ".zip", ".apk",
)

# Files that must never be committed at all (secrets live in .env, which
# is gitignored; .env.example is the documented template and is allowed).
FORBIDDEN_FILES = re.compile(
    r"(^|/)\.env(\.[^/]+)?$"  # .env, .env.local, ...
    r"|\.(pem|p12|pfx|keystore|jks)$"
    r"|(^|/)id_(rsa|dsa|ecdsa|ed25519)(\.pub)?$"
)
ALLOWED_FILES = re.compile(r"(^|/)\.env\.example$")

PATTERNS = [
    ("private key material", re.compile(r"-----BEGIN [A-Z ]*PRIVATE KEY")),
    ("AWS access key ID", re.compile(r"\bAKIA[0-9A-Z]{16}\b")),
    ("GitHub token", re.compile(r"\b(?:gh[pousr]|github_pat)_[A-Za-z0-9_]{20,}\b")),
    ("Slack token", re.compile(r"\bxox[baprs]-[A-Za-z0-9-]{10,}\b")),
    ("Google API key", re.compile(r"\bAIza[0-9A-Za-z_-]{35}\b")),
    ("Stripe live key", re.compile(r"\b[rs]k_live_[A-Za-z0-9]{16,}\b")),
    ("OpenAI/Anthropic-style key", re.compile(r"\bsk-(?:ant-)?[A-Za-z0-9_-]{24,}\b")),
    ("JWT", re.compile(r"\beyJ[A-Za-z0-9_-]{10,}\.eyJ[A-Za-z0-9_-]{10,}\.")),
    ("URL with embedded credentials", re.compile(r"://[^/\s:@]+:[^@\s/]{4,}@")),
    (
        "hardcoded credential assignment",
        re.compile(
            r"""(?i)\b(?:password|passwd|pwd|secret|token|api_?key|access_?key|private_?key|credential)s?\s*[:=]\s*["'][^"'\s]{6,}["']"""
        ),
    ),
]

# Values that are clearly placeholders, not live credentials.
PLACEHOLDER = re.compile(r"(?i)example|changeme|placeholder|your[-_]|\$\{|^\{|x{6,}|•")
WAIVER = "pz:allow-secret"


def git_lines(*args):
    out = subprocess.run(
        ["git", *args], capture_output=True, text=True, check=True
    ).stdout
    return [line for line in out.splitlines() if line]


def staged_content(path):
    # The staged blob, not the worktree — what would actually be committed.
    res = subprocess.run(
        ["git", "show", f":{path}"], capture_output=True, check=True
    )
    return res.stdout


def main():
    mode = sys.argv[1] if len(sys.argv) > 1 else "--staged"
    if mode == "--staged":
        paths = git_lines("diff", "--cached", "--name-only", "--diff-filter=ACMR")
        read = staged_content
    elif mode == "--all":
        paths = git_lines("ls-files")
        read = lambda p: open(p, "rb").read()  # noqa: E731
    else:
        sys.exit(f"usage: {sys.argv[0]} [--staged|--all]")

    findings = []
    for path in paths:
        if FORBIDDEN_FILES.search(path) and not ALLOWED_FILES.search(path):
            findings.append((path, 0, "forbidden file type (belongs in .env / outside the repo)", path))
            continue
        if path.startswith(EXCLUDED) or path.endswith(BINARY_EXT):
            continue
        try:
            text = read(path).decode("utf-8")
        except (UnicodeDecodeError, FileNotFoundError, subprocess.CalledProcessError):
            continue  # binary or vanished — nothing to grep
        for lineno, line in enumerate(text.splitlines(), start=1):
            if WAIVER in line:
                continue
            for label, pattern in PATTERNS:
                m = pattern.search(line)
                if m and not PLACEHOLDER.search(m.group(0)):
                    findings.append((path, lineno, label, line.strip()[:120]))

    if findings:
        print("✖ possible secrets — commit blocked (this repo is public):\n", file=sys.stderr)
        for path, lineno, label, snippet in findings:
            print(f"  {path}:{lineno}  [{label}]\n      {snippet}", file=sys.stderr)
        print(
            "\nMove real credentials to .env (gitignored; template: .env.example).\n"
            f"False positive? Append `{WAIVER}` to that line.",
            file=sys.stderr,
        )
        sys.exit(1)

    print(f"secret scan: clean ({len(paths)} files, {mode})")


if __name__ == "__main__":
    main()
