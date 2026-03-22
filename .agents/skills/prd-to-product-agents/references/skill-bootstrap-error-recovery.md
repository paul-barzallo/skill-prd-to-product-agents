
# Bootstrap Error Recovery

Error recovery procedures for the skill CLI (`prd-to-product-agents-cli`)
bootstrap and validation operations.

## YAML invalid

- keep the generated file for inspection
- infrastructure will not sync invalid content into the audit ledger
- record the error in the report
- if possible, place a corrected suggestion in `.bootstrap-overlays/`

## SQLite missing (infrastructure handles recovery)

- infrastructure tries automated installation if possible
- if not possible, leaves `.state/sqlite-bootstrap.pending.md`
- continue with canonical docs only

## SQL schema failure (infrastructure)

- stop DB initialization
- report which table or statement failed
- do not leave the DB in a fake ready state

## Git init failure

- keep all generated files
- record the failure in the report
- do not retry dangerous commands blindly

## File collision

- preserve the existing file
- write the proposed version under `.bootstrap-overlays/`
- report the collision

## Git stderr warnings (CRLF)

Git writes CRLF normalization warnings to stderr. These are
**informational, not errors**. PowerShell's `$ErrorActionPreference =
"Stop"` treats any stderr output as a terminating error, which breaks
`git add` calls.

### Symptoms

```text
git : warning: in the working copy of '.github/agents/.instructions.md',
CRLF will be replaced by LF the next time Git touches it
```

The script aborts on the first `git add` even though the file was
staged successfully.

### Fix pattern

Suppress stderr from git commands. In PowerShell:

```powershell
$oldPref = $ErrorActionPreference
$ErrorActionPreference = 'Continue'
git add -- "$p" 2>&1 | Out-Null
$ErrorActionPreference = $oldPref
```

In Bash:

```bash
git add -- "${path}" 2>/dev/null
```

### Root cause

The `.gitattributes` file declares `*.md text eol=lf` but older shell-based
bootstrap paths generated files with `\r\n` line endings. Git's
auto-normalization detects the mismatch and
emits a warning. This is expected and harmless - Git will normalize
the file on the next checkout.

## Template token left in files

If `{{PROJECT_NAME}}` appears in generated files (e.g. `vision.md`),
the bootstrap script failed to replace the token.

### Detection for template tokens

```powershell
Get-ChildItem -Recurse -Filter '*.md' | Select-String '{{PROJECT_NAME}}'
```

### Fix for template tokens

The bootstrap now defaults to the target folder name when no project
name is provided, with `"Unnamed Project"` only as a final fallback.
If the token is still present:

1. Edit the file manually and replace the token.
2. Or re-run bootstrap with `-ProjectName "YourName"`.

## UTF-8 em-dash breaks PowerShell 5.1 parsing

UTF-8 encoded em-dash (bytes `E2 80 94`) in `.ps1` files
**without a BOM** causes PowerShell 5.1 to misparse the file.
This is a historical note - the CLIs are Rust binaries and
are unaffected by this issue.

### Root cause of em-dash parsing failure

PowerShell 5.1 reads BOM-less files as Windows-1252. In that encoding,
byte `0x94` maps to `"` (RIGHT DOUBLE QUOTATION MARK, U+201D).
PowerShell's parser recognizes Unicode smart quotes as string
delimiters, so an em-dash inside a string literal like:

```powershell
Write-Error ".git/hooks/ not found - is this a git repository?"
```

becomes a prematurely terminated string followed
by unexpected bare tokens. This cascades into `ParseException` errors
on later lines.

### Fix for em-dash parsing failure

Replace all em-dashes in `.ps1` files with ASCII `--`:

```powershell
# Bad:  Write-Error "Not found - skipping"
# Good: Write-Error "Not found -- skipping"
```

Em-dashes are safe in comments (they don't break parsing), but we
replace them everywhere for consistency. Bash `.sh` files are
unaffected since the shell reads UTF-8 natively.
