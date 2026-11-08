# PowerShell script to compile and run GenerateReference.java
$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$outputFile = Join-Path $scriptDir "expected_random.csv"
$javaFile = Join-Path $scriptDir "GenerateReference.java"

$javaBin = "C:\Program Files\Eclipse Adoptium\jre-25.0.5.8-hotspot\bin\java.exe"
$ecjJar = "$env:TEMP\ecj.jar"

if (-not (Test-Path $ecjJar)) {
    Write-Host "Downloading ECJ standalone compiler..."
    Invoke-WebRequest -Uri "https://repo1.maven.org/maven2/org/eclipse/jdt/ecj/3.38.0/ecj-3.38.0.jar" -OutFile $ecjJar
}

Write-Host "Compiling GenerateReference.java..."
& $javaBin -jar $ecjJar -7 -d $scriptDir $javaFile

Write-Host "Running GenerateReference..."
& $javaBin -cp $scriptDir GenerateReference $outputFile

# Clean up class file
Remove-Item -Path (Join-Path $scriptDir "GenerateReference.class") -Force -ErrorAction SilentlyContinue

Write-Host "Done! Generated $outputFile"

