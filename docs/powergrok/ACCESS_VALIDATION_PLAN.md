# Power Grok — Access Validation Plan (No Auth Thrash)

Purpose: prove what you can and cannot reach on this machine **without** destroying credentials.

Hard bans (still in force — failing a check is NOT permission to break these):
- Do NOT run `gh auth logout`
- Do NOT run interactive `gh auth login --web` loops as a “fix”
- Do NOT `git remote set-url` to `git@github.com:...` / SSH
- Do NOT `gh auth setup-git` while `gh auth status` is red
- Do NOT change `git config --global user.name` / `user.email`
- Do NOT print tokens, passwords, or full `gho_` / `ghp_` / `github_pat_` values
- Do NOT invent “Hermes has secret access” if a check fails — record the failing path instead

Success standard: you can independently reproduce HTTPS git + REST PR/read access using the **same** keychain-backed token path Hermes used. `gh` may still be unhealthy; that is a client-state issue, not proof the repo is unreachable.

Operator facts (ground truth):
- GitHub user: `zfifteen`
- Repo: `https://github.com/zfifteen/powergrok.git`
- Product PR base: `powergrok`
- Branch of interest: `feat/branding-powergrok`
- PR: https://github.com/zfifteen/powergrok/pull/5
- Review: https://github.com/zfifteen/powergrok/pull/5#pullrequestreview-4718327377
- Preferred protocol: HTTPS + macOS osxkeychain
- SSH: not set up on this machine (expect publickey failure; do not “fix” by enabling SSH)

Workdir for all git checks:
```sh
cd /Users/velocityworks/IdeaProjects/powergrok
```

Redaction helper (use whenever secrets might appear):
```sh
redact() { sed -E 's/(password|oauth_token|token|authorization: bearer)[=: ].*/\1=***REDACTED***/I'; }
```

Produce a report file as you go:
```sh
REPORT="/Users/velocityworks/IdeaProjects/powergrok/docs/powergrok/ACCESS_VALIDATION_REPORT.md"
mkdir -p "$(dirname "$REPORT")"
{
  echo "# Access Validation Report"
  echo
  echo "- When: $(date -Iseconds)"
  echo "- Host: $(hostname)"
  echo "- User: $(whoami)"
  echo "- Cwd: $(pwd)"
  echo
} > "$REPORT"
pass() { echo "PASS — $1" | tee -a "$REPORT"; }
fail() { echo "FAIL — $1" | tee -a "$REPORT"; }
info() { echo "INFO — $1" | tee -a "$REPORT"; }
warn() { echo "WARN — $1" | tee -a "$REPORT"; }
section() { echo; echo "## $1"; echo; echo "## $1" >> "$REPORT"; echo >> "$REPORT"; }
```

---

## Phase 0 — Preconditions (no network)

### 0.1 Repo exists and is the powergrok checkout
```sh
section "0.1 repo identity"
test -d /Users/velocityworks/IdeaProjects/powergrok/.git && pass "powergrok .git exists" || fail "powergrok checkout missing"
git -C /Users/velocityworks/IdeaProjects/powergrok rev-parse --show-toplevel
git -C /Users/velocityworks/IdeaProjects/powergrok remote -v | tee -a "$REPORT"
```
Expect:
- `origin` = `https://github.com/zfifteen/powergrok.git` (fetch and push)
- `upstream` may be `https://github.com/xai-org/grok-build.git`
Fail if origin is `git@github.com:...`.

### 0.2 Protocol policy check
```sh
section "0.2 protocol policy"
git -C /Users/velocityworks/IdeaProjects/powergrok remote get-url origin | tee -a "$REPORT"
case "$(git -C /Users/velocityworks/IdeaProjects/powergrok remote get-url origin)" in
  https://github.com/zfifteen/powergrok.git|https://github.com/zfifteen/powergrok)
    pass "origin is HTTPS powergrok" ;;
  git@github.com:*)
    fail "origin is SSH — DO NOT rewrite without operator order; report only" ;;
  *)
    fail "origin unexpected URL" ;;
esac
git config --global --get-regexp 'url\.' 2>/dev/null | tee -a "$REPORT" || info "no global url.*.insteadOf rewrites"
```

### 0.3 Credential helper chain
```sh
section "0.3 credential helpers"
git config --global --get-regexp '^credential' | tee -a "$REPORT"
```
Healthy target after Hermes fix:
- `credential.helper=osxkeychain` present
- NO host-specific empty helper that blanks keychain for github.com-only-through-dead-gh

Fail/warn patterns:
- `credential.https://github.com.helper=` empty plus only `!gh auth git-credential` while `gh` is logged out → this previously broke all HTTPS github ops

### 0.4 SSH is expected broken (document, do not fix)
```sh
section "0.4 ssh expected fail"
ssh -o BatchMode=yes -o ConnectTimeout=5 -T git@github.com 2>&1 | tee -a "$REPORT" || true
ls -la ~/.ssh 2>&1 | tee -a "$REPORT" || true
```
Expect: `Permission denied (publickey)` and likely no `id_*` keys.  
PASS condition for this step: you **observed** the failure and did **not** attempt SSH setup.

---

## Phase 1 — Keychain / git credential path (secret-safe)

### 1.1 Credential fill (redact password)
```sh
section "1.1 git credential fill"
printf 'url=https://github.com/zfifteen/powergrok.git\n\n' \
  | git credential fill 2>&1 | redact | tee -a "$REPORT"
```
PASS if output includes:
- `protocol=https`
- `host=github.com`
- `username=zfifteen`
- `password=***REDACTED***` (present, length unknown in redacted form)

FAIL if:
- hangs waiting for username (Device not configured)
- empty password
- errors from `gh auth git-credential` only

### 1.2 Keychain direct probe (redact)
```sh
section "1.2 osxkeychain get"
printf 'protocol=https\nhost=github.com\n\n' \
  | git credential-osxkeychain get 2>&1 | redact | tee -a "$REPORT"
```
PASS if username `zfifteen` and a password line exist.  
Record only token **prefix** (first 4 chars) and length via python if needed — never full token:
```sh
python3 - <<'PY' | tee -a "$REPORT"
import subprocess
out=subprocess.check_output(["bash","-lc", r"printf 'protocol=https\nhost=github.com\n\n' | git credential-osxkeychain get"], text=True)
creds=dict(line.split("=",1) for line in out.splitlines() if "=" in line)
pw=creds.get("password","")
print(f"username={creds.get('username')}")
print(f"token_prefix={pw[:4]!r} token_len={len(pw)}")
print("token_kind=", "oauth_gho" if pw.startswith("gho_") else "pat_ghp" if pw.startswith("ghp_") else "fine_grained" if pw.startswith("github_pat_") else "other")
PY
```

### 1.3 One-shot env for API tools (optional; do not persist in shell profiles)
```sh
section "1.3 GH_TOKEN one-shot"
export GH_TOKEN="$(printf 'protocol=https\nhost=github.com\n\n' | git credential-osxkeychain get | awk -F= '/^password=/{print substr($0,10)}')"
if [ -n "$GH_TOKEN" ]; then pass "GH_TOKEN exported in this shell only (not printed)"; else fail "could not export GH_TOKEN"; fi
# never echo "$GH_TOKEN"
```

---

## Phase 2 — HTTPS git to GitHub (authoritative for “can I reach the repo?”)

### 2.1 ls-remote origin
```sh
section "2.1 ls-remote"
GIT_TERMINAL_PROMPT=0 git -C /Users/velocityworks/IdeaProjects/powergrok ls-remote origin HEAD 2>&1 | tee -a "$REPORT"
```
PASS: prints a SHA and `HEAD`.  
FAIL: auth errors, repo not found with bad auth, device not configured.

### 2.2 fetch PR branch
```sh
section "2.2 fetch branches"
GIT_TERMINAL_PROMPT=0 git -C /Users/velocityworks/IdeaProjects/powergrok fetch origin powergrok feat/branding-powergrok 2>&1 | tee -a "$REPORT"
git -C /Users/velocityworks/IdeaProjects/powergrok rev-parse origin/powergrok origin/feat/branding-powergrok 2>&1 | tee -a "$REPORT"
```
PASS: both refs resolve.

### 2.3 push dry-run (write auth without changing remote)
```sh
section "2.3 push dry-run"
GIT_TERMINAL_PROMPT=0 git -C /Users/velocityworks/IdeaProjects/powergrok push --dry-run origin HEAD 2>&1 | tee -a "$REPORT"
```
PASS: dry-run completes (up-to-date or would push).  
If this PASS but `gh` FAIL later → your conclusion must be: **git HTTPS works; gh client path is broken**, not “no GitHub access.”

### 2.4 Negative control (bogus password must fail)
```sh
section "2.4 negative control"
GIT_TERMINAL_PROMPT=0 git -c credential.helper= \
  -c credential.helper='!f(){ echo username=zfifteen; echo password=definitely-invalid; }; f' \
  ls-remote https://github.com/zfifteen/powergrok.git HEAD 2>&1 | tee -a "$REPORT" || true
```
PASS for this step: authentication **fails** with invalid password. Proves you are not accidentally hitting a weird anonymous-success path for write-sensitive ops.

---

## Phase 3 — GitHub REST API (what Hermes used)

Use `GH_TOKEN` from Phase 1.3. Retry up to 5 times on HTTP 503 (GitHub partial degradation is real).

### 3.1 GET /user
```sh
section "3.1 api /user"
python3 - <<'PY' | tee -a "$REPORT"
import os, json, urllib.request, time
token=os.environ.get("GH_TOKEN","")
assert token, "GH_TOKEN missing"
for i in range(1,6):
    req=urllib.request.Request("https://api.github.com/user", headers={
        "Authorization": f"Bearer {token}",
        "Accept":"application/vnd.github+json",
        "User-Agent":"powergrok-access-validation",
        "X-GitHub-Api-Version":"2022-11-28",
    })
    try:
        with urllib.request.urlopen(req, timeout=30) as r:
            d=json.load(r)
            scopes=r.headers.get("X-OAuth-Scopes") or r.headers.get("x-oauth-scopes")
            print(f"PASS attempt={i} login={d.get('login')} id={d.get('id')} scopes={scopes}")
            break
    except Exception as e:
        code=getattr(e,"code",None)
        body=b""
        try: body=e.read()
        except Exception: pass
        print(f"attempt {i} FAIL code={code} body={body[:120]!r}")
        time.sleep(2*i)
else:
    print("FAIL /user after retries")
PY
```
PASS: `login=zfifteen` (or expected account).  
If 503 only: WARN “API degraded”, continue other checks; do not logout.

### 3.2 GET repo
```sh
section "3.2 api repo"
python3 - <<'PY' | tee -a "$REPORT"
import os, json, urllib.request, time
token=os.environ["GH_TOKEN"]
url="https://api.github.com/repos/zfifteen/powergrok"
for i in range(1,6):
    req=urllib.request.Request(url, headers={
        "Authorization": f"Bearer {token}",
        "Accept":"application/vnd.github+json",
        "User-Agent":"powergrok-access-validation",
        "X-GitHub-Api-Version":"2022-11-28",
    })
    try:
        with urllib.request.urlopen(req, timeout=30) as r:
            d=json.load(r)
            print("PASS full_name=", d.get("full_name"), "private=", d.get("private"),
                  "default_branch=", d.get("default_branch"), "permissions=", d.get("permissions"))
            break
    except Exception as e:
        code=getattr(e,"code",None)
        print("attempt", i, "FAIL", code)
        time.sleep(2*i)
else:
    print("FAIL repo GET")
PY
```
PASS: `full_name=zfifteen/powergrok` and permissions include pull/push as expected for owner.

### 3.3 GET PR #5 + reviews + review comments
```sh
section "3.3 api PR #5 + reviews"
python3 - <<'PY' | tee -a "$REPORT"
import os, json, urllib.request, time

token=os.environ["GH_TOKEN"]
H={
  "Authorization": f"Bearer {token}",
  "Accept":"application/vnd.github+json",
  "User-Agent":"powergrok-access-validation",
  "X-GitHub-Api-Version":"2022-11-28",
}

def get(url):
    last=None
    for i in range(1,6):
        req=urllib.request.Request(url, headers=H)
        try:
            with urllib.request.urlopen(req, timeout=45) as r:
                return r.status, json.load(r)
        except Exception as e:
            last=e
            code=getattr(e,"code",None)
            print(f"retry {url} attempt={i} code={code}")
            time.sleep(2*i)
    raise SystemExit(f"FAIL {url}: {last}")

status, pr = get("https://api.github.com/repos/zfifteen/powergrok/pulls/5")
print("PASS PR status", status, "title=", pr.get("title"), "state=", pr.get("state"),
      "base=", pr["base"]["ref"], "head=", pr["head"]["ref"], "sha=", pr["head"]["sha"]) 

status, reviews = get("https://api.github.com/repos/zfifteen/powergrok/pulls/5/reviews")
print("PASS reviews count", len(reviews))
for rev in reviews[-5:]:
    print(" -", rev.get("id"), rev.get("user",{}).get("login"), rev.get("state"), rev.get("html_url"))
    body=(rev.get("body") or "").strip().splitlines()
    print("   body_head:", (body[0] if body else "(empty)")[:120])

status, comments = get("https://api.github.com/repos/zfifteen/powergrok/pulls/5/comments?per_page=100")
print("PASS inline comments count", len(comments))
for c in sorted(comments, key=lambda x: (x.get("path") or "", x.get("line") or 0)):
    print(f" - {c.get('path')}:{c.get('line')} id={c.get('id')} by={c.get('user',{}).get('login')}")
    print("   ", (c.get("body") or "").splitlines()[0][:140])
PY
```
PASS criteria:
- PR #5 readable
- Hermes review present (look for review id `4718327377` or body containing `Hermes Agent Code Review`)
- Inline comments count ≥ 7 (or list all if fewer due to API lag)

### 3.4 Optional write probe (only if operator asked to validate write)
Do **not** create junk PRs. Safe write probe: post nothing by default.  
If operator explicitly says “validate write,” create then delete a test issue comment only with permission. Default = skip.

---

## Phase 4 — `gh` CLI (secondary; may fail while REST works)

### 4.1 gh auth status
```sh
section "4.1 gh auth status"
gh auth status 2>&1 | tee -a "$REPORT" || true
```
Record PASS/FAIL honestly.  
If FAIL but Phase 2+3 PASS → conclude: **repo access works via keychain/REST; gh session is unhealthy.**

### 4.2 hosts.yml shape (redact tokens)
```sh
section "4.2 hosts.yml"
sed -E 's/(oauth_token:).*/\1 ***REDACTED***/' ~/.config/gh/hosts.yml 2>&1 | tee -a "$REPORT" || warn "no hosts.yml"
```

### 4.3 gh with one-shot token (no login/logout)
```sh
section "4.3 gh with GH_TOKEN"
GH_TOKEN="$GH_TOKEN" gh api user --jq .login 2>&1 | tee -a "$REPORT" || true
GH_TOKEN="$GH_TOKEN" gh api repos/zfifteen/powergrok/pulls/5 --jq '{title,.state,base:.base.ref,head:.head.ref}' 2>&1 | tee -a "$REPORT" || true
```
PASS if these work even when `gh auth status` is red. That is the supported workaround Hermes used (token from keychain, not logout/login).

### 4.4 Forbidden actions check (self-audit)
```sh
section "4.4 forbidden actions self-audit"
echo "Confirm you did NOT run: gh auth logout | gh auth login | remote set-url ssh | setup-git | global user rewrite" | tee -a "$REPORT"
```
PASS only if none of those were executed during this validation.

---

## Phase 5 — Local review artifacts (offline fallback)

Even if API is 503, you must still be able to work from operator-provided files:

```sh
section "5 offline artifacts"
for f in \
  /Users/velocityworks/IdeaProjects/powergrok/docs/powergrok/HERMES_PR5_REVIEW_FOR_GROK.md \
  /Users/velocityworks/IdeaProjects/powergrok/docs/powergrok/NO_SECRET_GITHUB_ACCESS.md \
  /Users/velocityworks/IdeaProjects/powergrok/docs/powergrok/PR_DESCRIPTION.md
do
  if [ -f "$f" ]; then pass "exists $f ($(wc -c <"$f") bytes)"; else fail "missing $f"; fi
done
```

PASS: review package readable offline.  
If API fails but offline package exists → implement from package; do not claim blocked.

---

## Phase 6 — Scoring matrix (required end state)

Fill this table in `$REPORT`:

| Capability | Result (PASS/FAIL/WARN) | Evidence |
|---|---|---|
| origin HTTPS |  | remote -v |
| credential fill via keychain |  | redacted fill |
| git ls-remote origin |  | SHA |
| git fetch feature branch |  | origin/feat/branding-powergrok SHA |
| git push --dry-run |  | dry-run output |
| REST GET /user |  | login=zfifteen or 503 WARN |
| REST GET repo |  | permissions |
| REST GET PR #5 |  | title/state |
| REST GET reviews/comments |  | review id / comment count |
| gh auth status |  | often FAIL OK if REST PASS |
| gh api with GH_TOKEN |  | login / PR json |
| offline review package |  | file bytes |
| no forbidden auth commands |  | self-audit |

### Interpretation rules (mandatory)

1. **GitHub access is PROVEN** if Phase 2 (git HTTPS) PASSes **and** (Phase 3 REST PASSes **or** offline review package PASSes with git PASS).
2. **`gh` unhealthy alone is NOT “no access.”**
3. **Web search 401 is NOT “no access.”**
4. **SSH publickey denial is EXPECTED and not a defect to fix in this plan.**
5. **HTTP 503 on /user with successful git push/dry-run** → WARN degraded API; retry; do not logout.
6. **Only if Phase 1 credential fill fails AND git HTTPS fails** may you ask the operator for a new PAT — still without running logout first as ritual.

### Final one-liner you must print

Copy exactly one of:

- `ACCESS_OK: HTTPS git + keychain work; REST works; gh secondary.`
- `ACCESS_OK_GIT_ONLY: HTTPS git + keychain work; REST degraded/fail; offline package used; gh secondary.`
- `ACCESS_BLOCKED_CREDENTIALS: keychain/git credential path failed; operator action required (no logout by agent).`
- `ACCESS_BLOCKED_OTHER: <one sentence root cause with phase number>.`

---

## Phase 7 — What not to conclude

Forbidden conclusions from partial failure:
- “Hermes has secret GitHub access”
- “I must switch to SSH”
- “I must gh auth logout to recover”
- “Browser re-auth is the only path”
- “I cannot implement the PR review because web search 401’d”

Allowed conclusions:
- “I can read PR #5 via REST with keychain token”
- “I can push over HTTPS”
- “gh is broken but irrelevant because REST/git work”
- “API 503; using offline review package and retrying later”

---

## Minimum command set (if short on time)

Run at least these, in order, with redaction:

```sh
cd /Users/velocityworks/IdeaProjects/powergrok
git remote -v
git config --global --get-regexp '^credential'
printf 'url=https://github.com/zfifteen/powergrok.git\n\n' | git credential fill | sed -E 's/(password)=.*/\1=***REDACTED***/'
GIT_TERMINAL_PROMPT=0 git ls-remote origin HEAD
GIT_TERMINAL_PROMPT=0 git push --dry-run origin HEAD
export GH_TOKEN="$(printf 'protocol=https\nhost=github.com\n\n' | git credential-osxkeychain get | awk -F= '/^password=/{print substr($0,10)}')"
# never echo token
python3 -c 'import os,urllib.request,json;t=os.environ["GH_TOKEN"];r=urllib.request.Request("https://api.github.com/repos/zfifteen/powergrok/pulls/5",headers={"Authorization":"Bearer "+t,"User-Agent":"v","Accept":"application/vnd.github+json"});
print(json.load(urllib.request.urlopen(r,timeout=45)).get("title"))'
gh auth status || true
test -f docs/powergrok/HERMES_PR5_REVIEW_FOR_GROK.md && echo OFFLINE_REVIEW_OK
```

If the PR title prints and ls-remote works: you have access. Proceed to implement the review.

---

End of validation plan.
