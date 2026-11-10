Add-Type -AssemblyName System.Drawing
$bmp = [System.Drawing.Bitmap]::FromFile("c:\Users\0xwrdo\Documents\minecraft\assets\modern_gui\container\inventory.png")
for ($y = 5; $y -lt 85; $y += 5) {
    $line = "Y=$y : "
    for ($x = 20; $x -lt 90; $x += 4) {
        $c = $bmp.GetPixel($x, $y)
        $line += "($x,$($c.R)) "
    }
    Write-Host $line
}
$bmp.Dispose()
