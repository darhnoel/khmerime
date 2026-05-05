param(
    [string]$Text = "jea{ENTER}",
    [int]$SwitchDelaySeconds = 5,
    [int]$PostTypeSeconds = 3,
    [switch]$Manual
)

$ErrorActionPreference = "Stop"
$startTime = Get-Date
$logPath = "C:\Temp\khmerime-tsf.log"
if (Test-Path $logPath) {
    Clear-Content -Path $logPath -ErrorAction SilentlyContinue
}

Write-Host "[khmerime] launching Notepad..."
$process = Start-Process -FilePath "notepad.exe" -PassThru
Start-Sleep -Seconds 2

$shell = New-Object -ComObject WScript.Shell
$activated = $shell.AppActivate($process.Id)
if (-not $activated) {
    $activated = $shell.AppActivate("Notepad")
}
if (-not $activated) {
    Write-Warning "[khmerime] Could not activate Notepad automatically."
    Write-Host "[khmerime] Click inside Notepad and switch it to KhmerIME now."
    Start-Sleep -Seconds $SwitchDelaySeconds
} else {
    Start-Sleep -Milliseconds 500
}

if ($Manual) {
    Write-Host "[khmerime] Switch Notepad to KhmerIME, type a smoke query, then press Enter here."
    Read-Host | Out-Null
} else {
    if ($activated) {
        Write-Host "[khmerime] Switch Notepad to KhmerIME now. Typing starts in $SwitchDelaySeconds seconds..."
        Start-Sleep -Seconds $SwitchDelaySeconds
    }
    $shell.SendKeys($Text)
    Start-Sleep -Seconds $PostTypeSeconds
}

$process.Refresh()
$crashEvents = Get-WinEvent -FilterHashtable @{
    LogName = "Application"
    Level = 2
    StartTime = $startTime
} -ErrorAction SilentlyContinue | Where-Object {
    $_.ProviderName -eq "Application Error" -and
    ($_.Message -like "*Notepad.exe*" -or $_.Message -like "*textinputframework.dll*" -or $_.Message -like "*msctf.dll*")
}

if ($process.HasExited) {
    Write-Error "[khmerime] Notepad exited during smoke test. ExitCode=$($process.ExitCode)"
    if (Test-Path $logPath) {
        Get-Content -Path $logPath -Tail 80
    }
    exit 1
}

if ($crashEvents) {
    Write-Error "[khmerime] Windows logged Notepad/TSF crash events during smoke test."
    $crashEvents | Select-Object TimeCreated, ProviderName, Id, Message | Format-List
    if (Test-Path $logPath) {
        Get-Content -Path $logPath -Tail 80
    }
    exit 1
}

Write-Host "[khmerime] Notepad smoke test did not detect a crash."
if (Test-Path $logPath) {
    Write-Host "[khmerime] TSF log tail:"
    Get-Content -Path $logPath -Tail 80
}
Write-Host "[khmerime] Close Notepad manually after inspecting the result."
