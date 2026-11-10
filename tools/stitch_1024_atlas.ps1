Add-Type -AssemblyName System.Drawing

$atlasSize = 1024
$outPng = "c:\Users\0xwrdo\Documents\minecraft\assets\terrain.png"
$outBin = "c:\Users\0xwrdo\Documents\minecraft\assets\terrain_rgba.bin"
$tempTerrain256 = "c:\Users\0xwrdo\Documents\minecraft\assets\terrain_base256.png"
$guiDir = "c:\Users\0xwrdo\Documents\minecraft\assets\modern_gui"
$itemDir = "c:\Users\0xwrdo\Documents\minecraft\assets\modern_items"
$blockDir = "c:\Users\0xwrdo\Documents\minecraft\assets\modern_textures"

Write-Host "Creating 1024x1024 master atlas..."
$atlasBmp = New-Object System.Drawing.Bitmap $atlasSize, $atlasSize, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
$g = [System.Drawing.Graphics]::FromImage($atlasBmp)
$g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::NearestNeighbor
$g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::Half

# Helper to draw an image file at exact coords
function Draw-Sprite([string]$filePath, [int]$x, [int]$y, [int]$w = -1, [int]$h = -1, [bool]$smooth = $false) {
    if (-not (Test-Path $filePath)) {
        Write-Warning "File not found: $filePath"
        return
    }
    $bmp = [System.Drawing.Bitmap]::FromFile($filePath)
    $targetW = if ($w -gt 0) { $w } else { $bmp.Width }
    $targetH = if ($h -gt 0) { $h } else { $bmp.Height }

    if ($smooth) {
        $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    } else {
        $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::NearestNeighbor
    }

    $g.DrawImage($bmp, $x, $y, $targetW, $targetH)
    $bmp.Dispose()
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::NearestNeighbor
}

# 1. Base Terrain at (0, 0, 256, 256)
Draw-Sprite $tempTerrain256 0 0 256 256
Write-Host "Drawn base terrain (256x256) at (0, 0)"

# 2. HUD Sprites (X = 256..512, Y = 0..128)
$hudDir = Join-Path $guiDir "sprites\hud"
$heartDir = Join-Path $hudDir "heart"

# Hotbar, Selection, Offhand, Crosshair
Draw-Sprite (Join-Path $hudDir "hotbar.png") 256 0 182 22
Draw-Sprite (Join-Path $hudDir "hotbar_selection.png") 440 0 24 24
Draw-Sprite (Join-Path $hudDir "hotbar_offhand_left.png") 466 0 29 24
Draw-Sprite (Join-Path $hudDir "crosshair.png") 496 0 15 15

# XP Bar
Draw-Sprite (Join-Path $hudDir "experience_bar_background.png") 256 24 182 5
Draw-Sprite (Join-Path $hudDir "experience_bar_progress.png") 256 30 182 5

# Hearts (each 9x9, step 10)
Draw-Sprite (Join-Path $heartDir "container.png") 256 40 9 9
Draw-Sprite (Join-Path $heartDir "container_blinking.png") 266 40 9 9
Draw-Sprite (Join-Path $heartDir "full.png") 276 40 9 9
Draw-Sprite (Join-Path $heartDir "half.png") 286 40 9 9
Draw-Sprite (Join-Path $heartDir "container_hardcore.png") 296 40 9 9
Draw-Sprite (Join-Path $heartDir "hardcore_full.png") 306 40 9 9
Draw-Sprite (Join-Path $heartDir "hardcore_half.png") 316 40 9 9
Draw-Sprite (Join-Path $heartDir "absorbing_full.png") 326 40 9 9
Draw-Sprite (Join-Path $heartDir "absorbing_half.png") 336 40 9 9
Draw-Sprite (Join-Path $heartDir "poisoned_full.png") 346 40 9 9
Draw-Sprite (Join-Path $heartDir "poisoned_half.png") 356 40 9 9
Draw-Sprite (Join-Path $heartDir "withered_full.png") 366 40 9 9
Draw-Sprite (Join-Path $heartDir "withered_half.png") 376 40 9 9

# Hunger (each 9x9, step 10)
Draw-Sprite (Join-Path $hudDir "food_empty.png") 256 52 9 9
Draw-Sprite (Join-Path $hudDir "food_half.png") 266 52 9 9
Draw-Sprite (Join-Path $hudDir "food_full.png") 276 52 9 9
Draw-Sprite (Join-Path $hudDir "food_empty_hunger.png") 286 52 9 9
Draw-Sprite (Join-Path $hudDir "food_half_hunger.png") 296 52 9 9
Draw-Sprite (Join-Path $hudDir "food_full_hunger.png") 306 52 9 9

# Armor (each 9x9, step 10)
Draw-Sprite (Join-Path $hudDir "armor_empty.png") 256 64 9 9
Draw-Sprite (Join-Path $hudDir "armor_half.png") 266 64 9 9
Draw-Sprite (Join-Path $hudDir "armor_full.png") 276 64 9 9

# Air Bubbles (each 9x9, step 10)
Draw-Sprite (Join-Path $hudDir "air.png") 256 76 9 9
Draw-Sprite (Join-Path $hudDir "air_bursting.png") 266 76 9 9
Draw-Sprite (Join-Path $hudDir "air_empty.png") 276 76 9 9

# 3. Modern Title Logo & Badges (X = 512..1024, Y = 0..64)
$titleDir = Join-Path $guiDir "title"
Draw-Sprite (Join-Path $titleDir "minecraft.png") 512 0 256 64 $true
Draw-Sprite (Join-Path $titleDir "edition.png") 768 0 128 16 $true

# Icons for Accessibility & Language (X = 900..940, Y = 0..20)
$iconDir = Join-Path $guiDir "sprites\icon"
Draw-Sprite (Join-Path $iconDir "accessibility.png") 900 0 15 15
Draw-Sprite (Join-Path $iconDir "language.png") 920 0 15 15

# 4. Buttons (X = 0..600, Y = 424..444)
$widgetDir = Join-Path $guiDir "sprites\widget"
Draw-Sprite (Join-Path $widgetDir "button.png") 0 424 200 20
Draw-Sprite (Join-Path $widgetDir "button_highlighted.png") 200 424 200 20
Draw-Sprite (Join-Path $widgetDir "button_disabled.png") 400 424 200 20

# 5. Container Screens (Y = 256..480)
$containerDir = Join-Path $guiDir "container"
Draw-Sprite (Join-Path $containerDir "inventory.png") 0 256 176 166
Draw-Sprite (Join-Path $containerDir "crafting_table.png") 176 256 176 166
Draw-Sprite (Join-Path $containerDir "furnace.png") 352 256 176 166
Draw-Sprite (Join-Path $containerDir "generic_54.png") 528 256 176 222

# 6. Modern Item Sprites (Y = 480..600)
# Row 0: Basic Blocks & Materials (Y = 480)
$row0 = @(
    @{ X = 0; File = "stone.png"; Block = $true },
    @{ X = 16; File = "grass_block_side.png"; Block = $true },
    @{ X = 32; File = "dirt.png"; Block = $true },
    @{ X = 48; File = "cobblestone.png"; Block = $true },
    @{ X = 64; File = "oak_planks.png"; Block = $true },
    @{ X = 80; File = "sand.png"; Block = $true },
    @{ X = 96; File = "gravel.png"; Block = $true },
    @{ X = 112; File = "oak_log.png"; Block = $true },
    @{ X = 128; File = "glass.png"; Block = $true },
    @{ X = 144; File = "torch.png"; Block = $true },
    @{ X = 160; File = "crafting_table_top.png"; Block = $true },
    @{ X = 176; File = "furnace_front.png"; Block = $true },
    @{ X = 192; File = "barrel_top.png"; Block = $true },
    @{ X = 208; File = "stick.png"; Block = $false },
    @{ X = 224; File = "coal.png"; Block = $false },
    @{ X = 240; File = "iron_ingot.png"; Block = $false },
    @{ X = 256; File = "gold_ingot.png"; Block = $false },
    @{ X = 272; File = "diamond.png"; Block = $false },
    @{ X = 288; File = "apple.png"; Block = $false },
    @{ X = 304; File = "wheat.png"; Block = $false },
    @{ X = 320; File = "bread.png"; Block = $false },
    @{ X = 336; File = "porkchop.png"; Block = $false },
    @{ X = 352; File = "cooked_porkchop.png"; Block = $false },
    @{ X = 368; File = "lava_bucket.png"; Block = $false },
    @{ X = 384; File = "blaze_rod.png"; Block = $false }
)
foreach ($it in $row0) {
    $dir = if ($it.Block) { $blockDir } else { $itemDir }
    Draw-Sprite (Join-Path $dir $it.File) $it.X 480 16 16
}

# Row 1: Wooden & Stone Tools (Y = 496)
$row1 = @(
    "wooden_sword.png", "wooden_shovel.png", "wooden_pickaxe.png", "wooden_axe.png", "wooden_hoe.png",
    "stone_sword.png", "stone_shovel.png", "stone_pickaxe.png", "stone_axe.png", "stone_hoe.png"
)
for ($i = 0; $i -lt $row1.Length; $i++) {
    Draw-Sprite (Join-Path $itemDir $row1[$i]) ($i * 16) 496 16 16
}

# Row 2: Iron & Gold Tools (Y = 512)
$row2 = @(
    "iron_sword.png", "iron_shovel.png", "iron_pickaxe.png", "iron_axe.png", "iron_hoe.png",
    "golden_sword.png", "golden_shovel.png", "golden_pickaxe.png", "golden_axe.png", "golden_hoe.png"
)
for ($i = 0; $i -lt $row2.Length; $i++) {
    Draw-Sprite (Join-Path $itemDir $row2[$i]) ($i * 16) 512 16 16
}

# Row 3: Diamond Tools & Leather Armor (Y = 528)
$row3 = @(
    "diamond_sword.png", "diamond_shovel.png", "diamond_pickaxe.png", "diamond_axe.png", "diamond_hoe.png",
    "leather_helmet.png", "leather_chestplate.png", "leather_leggings.png", "leather_boots.png"
)
for ($i = 0; $i -lt $row3.Length; $i++) {
    Draw-Sprite (Join-Path $itemDir $row3[$i]) ($i * 16) 528 16 16
}

# Row 4: Iron, Gold, Diamond Armor (Y = 544)
$row4 = @(
    "iron_helmet.png", "iron_chestplate.png", "iron_leggings.png", "iron_boots.png",
    "golden_helmet.png", "golden_chestplate.png", "golden_leggings.png", "golden_boots.png",
    "diamond_helmet.png", "diamond_chestplate.png", "diamond_leggings.png", "diamond_boots.png"
)
for ($i = 0; $i -lt $row4.Length; $i++) {
    Draw-Sprite (Join-Path $itemDir $row4[$i]) ($i * 16) 544 16 16
}

# Copy chest front from base terrain tile (10, 3) = (160, 48) to (192, 480)
for ($cy = 0; $cy -lt 16; $cy++) {
    for ($cx = 0; $cx -lt 16; $cx++) {
        $p = $atlasBmp.GetPixel(160 + $cx, 48 + $cy)
        $atlasBmp.SetPixel(192 + $cx, 480 + $cy, $p)
    }
}

# Dedicated pure white pixel at (0, 255)
$atlasBmp.SetPixel(0, 255, [System.Drawing.Color]::FromArgb(255, 255, 255, 255))

$g.Dispose()

Write-Host "Saving $outPng..."
$atlasBmp.Save($outPng, [System.Drawing.Imaging.ImageFormat]::Png)

Write-Host "Exporting raw RGBA binary to $outBin..."
$rect = New-Object System.Drawing.Rectangle 0, 0, $atlasSize, $atlasSize
$bmpData = $atlasBmp.LockBits($rect, [System.Drawing.Imaging.ImageLockMode]::ReadOnly, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
$totalBytes = $atlasSize * $atlasSize * 4
$bytes = New-Object byte[] $totalBytes
[System.Runtime.InteropServices.Marshal]::Copy($bmpData.Scan0, $bytes, 0, $totalBytes)
$atlasBmp.UnlockBits($bmpData)
$atlasBmp.Dispose()

# Convert BGRA -> RGBA in place
for ($i = 0; $i -lt $totalBytes; $i += 4) {
    $b = $bytes[$i]
    $r = $bytes[$i + 2]
    $bytes[$i] = $r
    $bytes[$i + 2] = $b
}

[System.IO.File]::WriteAllBytes($outBin, $bytes)
Write-Host "Master atlas 1024x1024 generated with Title Logo & Buttons! Size: $totalBytes bytes."
