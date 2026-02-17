# Fix UE5 5.3 - Correct patch
$filePath = "C:\Program Files\Epic Games\UE_5.3\Engine\Source\Runtime\Core\Public\Experimental\ConcurrentLinearAllocator.h"

Write-Host "Applying correct patch..." -ForegroundColor Cyan

# Restore from backup
$backupPath = "$filePath.backup"
if (Test-Path $backupPath) {
    Copy-Item $backupPath $filePath -Force
    Write-Host "Restored from backup" -ForegroundColor Gray
}

# Remove read-only
Set-ItemProperty -Path $filePath -Name IsReadOnly -Value $false

# Read all lines
$lines = [System.IO.File]::ReadAllLines($filePath)

# Find and remove lines 31-32 (the problematic #elif and its #define)
$newLines = @()
$skipNext = $false

for ($i = 0; $i -lt $lines.Count; $i++) {
    $line = $lines[$i]
    
    if ($skipNext) {
        $skipNext = $false
        Write-Host "Removed line $($i+1): $line" -ForegroundColor Yellow
        continue
    }
    
    if ($line -match '#elif __has_feature\(address_sanitizer\)') {
        Write-Host "Removed line $($i+1): $line" -ForegroundColor Yellow
        $skipNext = $true  # Also skip next line (#define IS_ASAN_ENABLED 1)
        continue
    }
    
    $newLines += $line
}

# Write back with proper line endings
[System.IO.File]::WriteAllLines($filePath, $newLines)

Write-Host "Patch applied successfully!" -ForegroundColor Green
Write-Host "Removed problematic #elif directive" -ForegroundColor Gray
Write-Host ""
Write-Host "Press any key..."
$null = $Host.UI.RawUI.ReadKey('NoEcho,IncludeKeyDown')
