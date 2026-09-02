# graphify-nobet.ps1 — el-Fihrist kutuphanesini izler, degisince grafi yeniler.
#
# graphify-rs'nin kendi `watch` modunu tek surec olarak ayakta tutar; bu betigin
# isi surec yonetimi (cift baslatma engeli, PID, log, Baslangic kancasi).
#
# TARIHCE: 2026-09-02'de watch KULLANILAMIYORDU — `Commands::Watch` kolu
# graphify-rs.toml'u hic okumuyordu, her yeniden insa LLM anlamsal katmanini
# atlayip tam grafi kod-only bir grafla eziyordu (494 dugum -> 260). O gun
# yukari akisa duzeltme yazildi (watch artik `build` ile ayni boru hattini
# kullaniyor); bu betikteki gecici polling dongusu de kaldirildi.
# Duzeltme: graphify-rs fix/watch-honors-config, RebuildFn geri cagrimi.
#
# GEREKSINIM: duzeltilmis graphify-rs (watch'in config okudugu surum).
#   graphify-rs watch --help  ciktisinda --no-llm varsa surum dogrudur.
#
# Kullanim:
#   .\graphify-nobet.ps1           # nobetci calismiyorsa baslat
#   .\graphify-nobet.ps1 -Durum    # calisiyor mu, son log ne
#   .\graphify-nobet.ps1 -Durdur   # durdur

param(
    [switch]$Durum,
    [switch]$Durdur
)

$ErrorActionPreference = 'Continue'
$kok    = $PSScriptRoot
$pidDsy = Join-Path $kok '.graphify-nobet.pid'
$log    = Join-Path $kok 'graphify-rs-out\nobet.log'

function Get-Nobetci {
    if (-not (Test-Path $pidDsy)) { return $null }
    $p = (Get-Content $pidDsy -Raw).Trim()
    if (-not $p) { return $null }
    $s = Get-Process -Id $p -ErrorAction SilentlyContinue
    # NEDEN ad kontrolu: PID geri donusturulmus olabilir, baska surece carpmayalim.
    if ($s -and $s.ProcessName -like 'graphify-rs*') { return $s }
    return $null
}

if ($Durum) {
    $s = Get-Nobetci
    if ($s) { Write-Host "nobetci CALISIYOR (PID $($s.Id))" -ForegroundColor Green }
    else    { Write-Host "nobetci calismiyor." -ForegroundColor DarkGray }
    if (Test-Path $log) {
        Write-Host "`n--- son 20 satir ---" -ForegroundColor DarkGray
        Get-Content $log -Tail 20
    }
    exit 0
}

if ($Durdur) {
    $s = Get-Nobetci
    if ($s) { Stop-Process -Id $s.Id -Force; Write-Host "nobetci durduruldu (PID $($s.Id))." -ForegroundColor Yellow }
    else    { Write-Host "zaten calismiyor." -ForegroundColor DarkGray }
    Remove-Item $pidDsy -ErrorAction SilentlyContinue
    exit 0
}

if (Get-Nobetci) {
    Write-Host "nobetci zaten calisiyor — ikinci kopya baslatilmadi." -ForegroundColor DarkGray
    exit 0
}

$exe = (Get-Command graphify-rs -ErrorAction SilentlyContinue).Source
if (-not $exe) {
    # ponytail: PATH'te yoksa cargo bin'e bak, orada da yoksa acikca soyle.
    $aday = Join-Path $env:USERPROFILE '.cargo\bin\graphify-rs.exe'
    if (Test-Path $aday) { $exe = $aday }
    else {
        Write-Host "graphify-rs bulunamadi. Kurulum: cargo install --path <graphify-rs klasoru>" -ForegroundColor Red
        exit 1
    }
}

# Surum kapisi: duzeltilmemis bir graphify-rs sessizce kod-only graf uretir.
# `--no-llm` bayragi duzeltmeyle birlikte watch'a eklendi, imza olarak kullaniyoruz.
if (-not ((& $exe watch --help 2>&1) -match '--no-llm')) {
    Write-Host "UYARI: bu graphify-rs surumunde watch config okumuyor —" -ForegroundColor Yellow
    Write-Host "       graf her yenilemede anlamsal katmanini kaybeder." -ForegroundColor Yellow
    Write-Host "       Duzeltilmis surumu kur: cargo install --path <graphify-rs> --force" -ForegroundColor Yellow
    exit 1
}

New-Item -ItemType Directory -Force (Split-Path $log) | Out-Null
$s = Start-Process -FilePath $exe -ArgumentList @('watch','--path',$kok) `
        -WorkingDirectory $kok -WindowStyle Hidden -PassThru `
        -RedirectStandardOutput $log -RedirectStandardError "$log.err"
Set-Content -Path $pidDsy -Value $s.Id -Encoding ascii
Write-Host "nobetci basladi (PID $($s.Id)) — izlenen: $kok" -ForegroundColor Green
Write-Host "log: $log" -ForegroundColor DarkGray
