# A -r binding reached through `switch-client -T prefix` (prefix None, as
# .psmux.conf does for the IME helper) must repeat without the prefix for
# repeat-time, and a non-repeatable key must end the repeat.
# Set PSMUX_TEST_BIN to test a non-installed binary.
$Bin = if ($env:PSMUX_TEST_BIN) { $env:PSMUX_TEST_BIN } else { (Get-Command psmux -EA Stop).Source }

$ErrorActionPreference = "Continue"
$src = Join-Path $PSScriptRoot "injector.cs"
$dataDir = "$env:USERPROFILE\.psmux"
$TMP = Join-Path $env:TEMP "psmux_repeatlatch"
New-Item -ItemType Directory -Force -Path $TMP | Out-Null
$SOCK = "replatch$PID"
$S = "rl"
$pass = 0; $fail = 0
function Check($ok, $m) { if ($ok) { "  [PASS] $m"; $script:pass++ } else { "  [FAIL] $m"; $script:fail++ } }
function Q([string]$f) { (& $Bin -L $SOCK display-message -p -t "$S" $f 2>&1 | Out-String).Trim() }

$inj = Join-Path $TMP "inj.exe"
if (-not (Test-Path $inj)) { & "$env:WINDIR\Microsoft.NET\Framework64\v4.0.30319\csc.exe" /nologo /optimize "/out:$inj" $src | Out-Null }

$conf = Join-Path $TMP "rl.conf"
[IO.File]::WriteAllText($conf, @"
set -g prefix None
set -g repeat-time 1500
bind-key -T root C-a switch-client -T prefix
bind -r H resize-pane -L 1
"@, (New-Object System.Text.ASCIIEncoding))

$launch = Join-Path $TMP "launch.cmd"
@"
@echo off
set PSMUX_SESSION=
set TMUX=
set TMUX_PANE=
set PSMUX_NO_WARM=1
set PATH=%SystemRoot%\System32;%PATH%
"$Bin" -L $SOCK -f "$conf" new-session -s $S -x 140 -y 34 %SystemRoot%\System32\cmd.exe
"@ | Set-Content -Path $launch -Encoding ASCII

$before = & $Bin ls 2>&1 | Out-String
$null = Start-Process -FilePath $launch -PassThru -WindowStyle Minimized
for ($i = 0; $i -lt 100 -and -not (Test-Path "$dataDir\${SOCK}__$S.port"); $i++) { Start-Sleep -Milliseconds 250 }
Start-Sleep -Seconds 4
$cli = Get-CimInstance Win32_Process -Filter "Name='psmux.exe'" | Where-Object { $_.CommandLine -match "-L $SOCK .*new-session -s $S\b" } | Select-Object -First 1
if (-not $cli) { "client did not start"; & $Bin -L $SOCK kill-server; exit 1 }
$cpid = [int]$cli.ProcessId
"binary: $(& $Bin -V)"

& $Bin -L $SOCK split-window -h -t $S | Out-Null
Start-Sleep -Seconds 2
$w0 = [int](Q '#{pane_width}')

# Prefix, then H three times without the prefix.
& $inj $cpid "^a" | Out-Null; Start-Sleep -Milliseconds 400
foreach ($n in 1..3) { & $inj $cpid "H" | Out-Null; Start-Sleep -Milliseconds 400 }
Start-Sleep -Milliseconds 300
$w1 = [int](Q '#{pane_width}')
Check ($w1 -eq $w0 + 3) "prefix H H H grows the pane by 3 ($w0 -> $w1)"

# After repeat-time, H is a plain key again.
Start-Sleep -Milliseconds 2000
Check ((Q '#{client_key_table}') -eq 'root') "the table is back to root after repeat-time"
& $inj $cpid "H" | Out-Null; Start-Sleep -Milliseconds 600
$w2 = [int](Q '#{pane_width}')
Check ($w2 -eq $w1) "H after repeat-time does not resize ($w1 -> $w2)"

# A non-repeatable key inside the window ends the repeat and reaches the pane.
& $inj $cpid "^a" | Out-Null; Start-Sleep -Milliseconds 400
& $inj $cpid "H" | Out-Null; Start-Sleep -Milliseconds 300
& $inj $cpid "x" | Out-Null; Start-Sleep -Milliseconds 300
& $inj $cpid "H" | Out-Null; Start-Sleep -Milliseconds 600
$w3 = [int](Q '#{pane_width}')
Check ($w3 -eq $w2 + 1) "x ends the repeat: only the first H resized ($w2 -> $w3)"
Check ((Q '#{client_key_table}') -eq 'root') "the table is root after x"

& $Bin -L $SOCK kill-server | Out-Null
Start-Sleep -Milliseconds 500
$after = & $Bin ls 2>&1 | Out-String
Check ($before -eq $after) "default namespace unchanged"
"pass=$pass fail=$fail"
if ($fail -gt 0) { exit 1 }
