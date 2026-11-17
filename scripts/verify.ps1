<#
.SYNOPSIS
  End-to-end bootstrap verification. Safe to run in a fresh PowerShell session:
    pwsh -NoProfile -File .\scripts\verify.ps1 [-BuildDemo] [-Connect]

  1 Rust MSVC target          6 interop bridge compiles + links (real SDK)
  2 MSVC toolchain (cl+link)  7 SDK DLLs load
  3 official SDK present      8 unit tests + selftest
  4 PSDKTest built            9 (this script itself = fresh-session scripts)
  5 Rust workspace builds    10 repo hygiene: no SDK binaries / credentials tracked
  -Connect additionally runs `zoom-companion connect` (needs sign-in + Zoom Workplace).
#>
[CmdletBinding()]
param([switch]$BuildDemo, [switch]$Connect)
. "$PSScriptRoot\common.ps1"
$results = [ordered]@{}
function Step($name, [scriptblock]$body) {
    try { $d = & $body; $results[$name] = "PASS  $d" }
    catch { $results[$name] = "FAIL  $($_.Exception.Message)" }
}
Push-Location $script:RepoRoot
try {
    Step '1 rust msvc target' {
        $t = rustc -vV | Select-String '^host: (.*)$' | ForEach-Object { $_.Matches[0].Groups[1].Value }
        if ($t -ne 'x86_64-pc-windows-msvc') { throw "host is $t" }
        "$t ($(rustc --version))"
    }
    Step '2 msvc toolchain' {
        Import-VsDevEnv
        $tmp = Join-Path ([IO.Path]::GetTempPath()) "zc-msvc-$PID"
        New-Item -ItemType Directory -Force $tmp | Out-Null
        Set-Content "$tmp\h.cpp" '#include <afxwin.h>
int main(){ CString s(L"ok"); return s.GetLength() == 2 ? 0 : 1; }'
        Push-Location $tmp
        try {
            cl /nologo /EHsc /MT /D_UNICODE /DUNICODE h.cpp /link /SUBSYSTEM:CONSOLE *> build.log
            if ($LASTEXITCODE -ne 0) { throw "cl failed: $(Get-Content build.log -Raw)" }
            & .\h.exe; if ($LASTEXITCODE -ne 0) { throw 'test exe failed' }
        } finally { Pop-Location; Remove-Item $tmp -Recurse -Force }
        "cl $env:VCToolsVersion + MFC, WinSDK $env:WindowsSDKVersion"
    }
    Step '3 official SDK' {
        $sdk = Get-ActiveSdkRoot
        if (-not $sdk) { throw 'not extracted' }
        $dll = Join-Path $sdk 'bin\zToolSuiteIPCProxy.dll'
        $sig = Get-AuthenticodeSignature $dll
        if ($sig.Status -ne 'Valid' -or $sig.SignerCertificate.Subject -notmatch 'Zoom') { throw "signature $($sig.Status)" }
        "$((Get-Item $dll).VersionInfo.FileVersion) signed by Zoom @ $sdk"
    }
    Step '4 PSDKTest sample' {
        if ($BuildDemo) { & "$PSScriptRoot\build-demo.ps1" | Out-Null }
        $exe = Join-Path (Get-ActiveSdkRoot) 'bin\PSDKTest.exe'
        if (-not (Test-Path $exe)) { throw 'not built (run with -BuildDemo)' }
        $p = Start-Process $exe -WorkingDirectory (Split-Path $exe) -PassThru
        Start-Sleep 3; $p.Refresh()
        if ($p.HasExited) { throw "exited early ($($p.ExitCode))" }
        $loaded = $p.Modules | Where-Object ModuleName -eq 'zToolSuiteIPCProxy.dll'
        $p.CloseMainWindow() | Out-Null; Start-Sleep 1; if (-not $p.HasExited) { Stop-Process $p -Force }
        if (-not $loaded) { throw 'SDK DLL not loaded by sample' }
        'launched; zToolSuiteIPCProxy.dll loaded; closed'
    }
    Step '5+6 rust workspace + bridge' {
        & "$PSScriptRoot\build.ps1" *> $null
        if ($LASTEXITCODE) { throw 'build failed' }
        'built (real-SDK adapter, delay-loaded zToolSuiteIPCProxy.dll)'
    }
    Step '7 SDK DLL load' {
        $o = & .\target\debug\zoom-companion.exe doctor 2>&1 | Select-String 'SDK DLL load|SDK linked|SDK runtime beside'
        if (($o | Where-Object { $_ -match '\[ ok \]' }).Count -lt 3) { throw ($o -join '; ') }
        'doctor: SDK linked, runtime beside exe, DLL loaded'
    }
    Step '8 tests + selftest' {
        $o = cargo test --workspace 2>&1
        if ($LASTEXITCODE -ne 0) { throw 'cargo test failed' }
        $n = ($o | Select-String 'test result: ok. (\d+) passed' | ForEach-Object { [int]$_.Matches[0].Groups[1].Value } | Measure-Object -Sum).Sum
        cargo run -q -p zoom-companion -- selftest | Out-Null
        if ($LASTEXITCODE -ne 0) { throw 'selftest failed' }
        "$n tests passed; selftest passed"
    }
    Step '10 repo hygiene' {
        $files = git ls-files --cached --others --exclude-standard
        $bad = $files | Where-Object { $_ -match '\.(dll|lib|zip|7z|exe|pdb|dpapi)$' -or $_ -match '^vendor/zoom-plugin-sdk/' }
        if ($bad) { throw "binaries would be committed: $($bad -join ', ')" }
        $hits = $files | Where-Object { Test-Path $_ -PathType Leaf } | ForEach-Object {
            Select-String -Path $_ -Pattern 'eyJ[A-Za-z0-9_-]{20,}\.', 'client_secret\s*[=:]\s*["''][A-Za-z0-9]{16,}', 'Bearer [A-Za-z0-9._-]{20,}' -List
        }
        if ($hits) { throw "possible credentials: $($hits.Path -join ', ')" }
        "$(@($files).Count) files checked; no SDK binaries or credentials"
    }
    if ($Connect) {
        Step 'IPC connect' {
            $o = & .\target\debug\zoom-companion.exe connect --seconds 5 2>&1
            if ($LASTEXITCODE -ne 0) { throw (($o | Select-Object -Last 5) -join '; ') }
            ($o | Select-String 'CONNECTED|GetMeetingStatus') -join '; '
        }
    }
} finally { Pop-Location }

$results.GetEnumerator() | ForEach-Object { '{0,-26} {1}' -f $_.Key, $_.Value }
if ($results.Values | Where-Object { $_ -like 'FAIL*' }) { exit 1 }
