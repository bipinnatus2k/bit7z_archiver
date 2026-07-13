#!/usr/bin/env pwsh
# Dependency direction invariants for bit7z_archiver workspace.
# Run: pweng scripts/check-deps.ps1
# Exits non-zero on violation.

$ErrorActionPreference = "Stop"
$failures = 0

function Check($name, $condition) {
    if ($condition) {
        Write-Host "FAIL: $name" -ForegroundColor Red
        $script:failures++
    } else {
        Write-Host "PASS: $name" -ForegroundColor Green
    }
}

# 1. infra must not depend on app
Check "infra->app" (cargo tree -e normal --workspace 2>&1 | Select-String "bit7z-infra.*(bit7z-app)")

# 2. presentation must not depend on runtime
Check "presentation->runtime" (cargo tree -e normal -p bit7z-pres-views -p bit7z-pres-dialogs 2>&1 | Select-String "bit7z-rt-")

# 3. only infra/bit7z and infra/repo depend on ffi
$ffi_users = cargo tree -e normal -i -p bit7z-ffi 2>&1 | Select-String "bit7z-(?!ffi)" | ForEach-Object { $_.Line.Trim() }
$bad_ffi = $ffi_users | Where-Object { $_ -notmatch "bit7z-infra-bit7z" -and $_ -notmatch "bit7z-infra-repo" -and $_ -ne "bit7z-ffi" }
Check "unexpected ffi consumer" $bad_ffi

# 4. infra/events must not depend on app/checksum
Check "infra-events->app-checksum" (cargo tree -e normal -p bit7z-infra-events 2>&1 | Select-String "bit7z-app")

# 5. app/archive must not depend on ffi
Check "app-archive->ffi" (cargo tree -e normal -p bit7z-app-archive 2>&1 | Select-String "bit7z-ffi")

# 6. app/test must not depend on infra-progress
Check "app-test->infra-progress" (cargo tree -e normal -p bit7z-app-test 2>&1 | Select-String "bit7z-infra-progress")

# 7. domain must have zero workspace crate deps (exclude self)
$domain_deps = cargo tree -e normal -p bit7z-domain 2>&1 | Select-String "bit7z-" | Where-Object { $_.Line -notmatch "bit7z-domain v" }
Check "domain has workspace dep" $domain_deps

if ($failures -gt 0) {
    Write-Host "`n$failures dependency invariant(s) violated" -ForegroundColor Red
    exit 1
} else {
    Write-Host "`nAll dependency invariants satisfied" -ForegroundColor Green
    exit 0
}
