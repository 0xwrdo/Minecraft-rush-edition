Add-Type -AssemblyName System.Drawing

$atlasSize = 1024
$outPng = "c:\Users\0xwrdo\Documents\minecraft\assets\terrain.png"
$outBin = "c:\Users\0xwrdo\Documents\minecraft\assets\terrain_rgba.bin"
$guiDir = "c:\Users\0xwrdo\Documents\minecraft\assets\modern_gui"
$itemDir = "c:\Users\0xwrdo\Documents\minecraft\assets\modern_items"
$modernDir = "c:\Users\0xwrdo\Documents\minecraft\assets\modern_textures"

Write-Host "Creating 1024x1024 master atlas..."
$atlasBmp = New-Object System.Drawing.Bitmap $atlasSize, $atlasSize, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
$g = [System.Drawing.Graphics]::FromImage($atlasBmp)
$g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::NearestNeighbor
$g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::Half

# Helper to place a tile from modern_textures directly at (tx*16, ty*16)
function Place-Tile([int]$tx, [int]$ty, [string]$filename, [int[]]$tint = $null) {
    $path = Join-Path $modernDir $filename
    if (-not (Test-Path $path)) {
        return
    }

    $srcBmp = [System.Drawing.Bitmap]::FromFile($path)
    $srcW = [Math]::Min(16, $srcBmp.Width)
    $srcH = [Math]::Min(16, $srcBmp.Height)

    $startX = $tx * 16
    $startY = $ty * 16

    for ($y = 0; $y -lt $srcH; $y++) {
        for ($x = 0; $x -lt $srcW; $x++) {
            $px = $srcBmp.GetPixel($x, $y)
            if ($px.A -gt 0) {
                if ($tint) {
                    $r = [int](($px.R * $tint[0]) / 255)
                    $g = [int](($px.G * $tint[1]) / 255)
                    $b = [int](($px.B * $tint[2]) / 255)
                    $a = if ($tint.Length -gt 3) { [int](($px.A * $tint[3]) / 255) } else { $px.A }
                    $color = [System.Drawing.Color]::FromArgb($a, $r, $g, $b)
                } else {
                    $color = $px
                }
                $atlasBmp.SetPixel($startX + $x, $startY + $y, $color)
            }
        }
    }
    $srcBmp.Dispose()
}

# Helper to composite grass side with grass side overlay
function Place-GrassSide([int]$tx, [int]$ty) {
    $sidePath = Join-Path $modernDir "grass_block_side.png"
    $overlayPath = Join-Path $modernDir "grass_block_side_overlay.png"

    $sideBmp = [System.Drawing.Bitmap]::FromFile($sidePath)
    $overlayBmp = [System.Drawing.Bitmap]::FromFile($overlayPath)

    $startX = $tx * 16
    $startY = $ty * 16

    for ($y = 0; $y -lt 16; $y++) {
        for ($x = 0; $x -lt 16; $x++) {
            $basePx = $sideBmp.GetPixel($x, $y)
            $overPx = $overlayBmp.GetPixel($x, $y)

            if ($overPx.A -gt 10) {
                $r = [int](($overPx.R * 121) / 255)
                $g = [int](($overPx.G * 192) / 255)
                $b = [int](($overPx.B * 90) / 255)
                $color = [System.Drawing.Color]::FromArgb(255, $r, $g, $b)
            } else {
                $color = $basePx
            }
            $atlasBmp.SetPixel($startX + $x, $startY + $y, $color)
        }
    }

    $sideBmp.Dispose()
    $overlayBmp.Dispose()
}

# Helper to draw an image file at exact coords
function Draw-Sprite([string]$filePath, [int]$x, [int]$y, [int]$w = -1, [int]$h = -1, [bool]$smooth = $false, [bool]$crop = $false) {
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

    if ($crop) {
        $srcRect = New-Object System.Drawing.Rectangle 0, 0, $targetW, $targetH
        $destRect = New-Object System.Drawing.Rectangle $x, $y, $targetW, $targetH
        $g.DrawImage($bmp, $destRect, $srcRect, [System.Drawing.GraphicsUnit]::Pixel)
    } else {
        $g.DrawImage($bmp, $x, $y, $targetW, $targetH)
    }
    $bmp.Dispose()
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::NearestNeighbor
}

Write-Host "Placing exact modern Minecraft block textures into (0..256, 0..256)..."

# Row 0
Place-Tile 0 0 "grass_block_top.png" @(121, 192, 90)
Place-Tile 1 0 "stone.png"
Place-Tile 2 0 "dirt.png"
Place-GrassSide 3 0
Place-Tile 4 0 "oak_planks.png"
Place-Tile 5 0 "smooth_stone.png"
Place-Tile 6 0 "smooth_stone_slab_side.png"
Place-Tile 7 0 "bricks.png"
Place-Tile 8 0 "tnt_side.png"
Place-Tile 9 0 "tnt_top.png"
Place-Tile 10 0 "tnt_bottom.png"
Place-Tile 11 0 "cobweb.png"
Place-Tile 12 0 "dandelion.png"
Place-Tile 13 0 "poppy.png"
Place-Tile 14 0 "oak_sapling.png"
Place-Tile 15 0 "mossy_cobblestone.png"

# Row 1
Place-Tile 0 1 "cobblestone.png"
Place-Tile 1 1 "bedrock.png"
Place-Tile 2 1 "sand.png"
Place-Tile 3 1 "gravel.png"
Place-Tile 4 1 "oak_log.png"
Place-Tile 5 1 "oak_log_top.png"
Place-Tile 6 1 "iron_block.png"
Place-Tile 7 1 "gold_block.png"
Place-Tile 8 1 "diamond_block.png"
Place-Tile 9 1 "emerald_block.png"
Place-Tile 10 1 "redstone_block.png"
Place-Tile 11 1 "red_mushroom.png"
Place-Tile 12 1 "brown_mushroom.png"
Place-Tile 13 1 "jungle_sapling.png"
Place-Tile 14 1 "birch_sapling.png"
Place-Tile 15 1 "spruce_sapling.png"

# Row 2
Place-Tile 0 2 "gold_ore.png"
Place-Tile 1 2 "iron_ore.png"
Place-Tile 2 2 "coal_ore.png"
Place-Tile 3 2 "bookshelf.png"
Place-Tile 4 2 "mossy_stone_bricks.png"
Place-Tile 5 2 "obsidian.png"
Place-Tile 6 2 "short_grass.png" @(121, 192, 90)
Place-Tile 7 2 "fern.png" @(121, 192, 90)
Place-Tile 8 2 "crafting_table_top.png"
Place-Tile 9 2 "furnace_front.png"
Place-Tile 10 2 "furnace_side.png"
Place-Tile 11 2 "dispenser_front.png"
Place-Tile 12 2 "sponge.png"
Place-Tile 13 2 "glass.png"
Place-Tile 14 2 "diamond_ore.png"
Place-Tile 15 2 "redstone_ore.png"

# Row 3
Place-Tile 0 3 "crafting_table_side.png"
Place-Tile 1 3 "crafting_table_front.png"
Place-Tile 2 3 "furnace_front_on.png"
Place-Tile 3 3 "piston_top.png"
Place-Tile 4 3 "oak_leaves.png" @(89, 201, 60)
Place-Tile 5 3 "oak_leaves.png" @(89, 201, 60)
Place-Tile 6 3 "stone_bricks.png"
Place-Tile 7 3 "chiseled_stone_bricks.png"
Place-Tile 8 3 "cracked_stone_bricks.png"
Place-Tile 9 3 "oak_planks.png"  # Chest side fallback
Place-Tile 10 3 "oak_planks.png" # Chest front fallback
Place-Tile 11 3 "oak_planks.png" # Chest top fallback
Place-Tile 12 3 "torch.png"
Place-Tile 13 3 "oak_door_top.png"
Place-Tile 14 3 "iron_door_top.png"
Place-Tile 15 3 "ladder.png"

# Row 4
Place-Tile 0 4 "lapis_ore.png"
Place-Tile 1 4 "glass.png"
Place-Tile 2 4 "snow.png"
Place-Tile 3 4 "ice.png"
Place-Tile 4 4 "snow.png"
Place-Tile 5 4 "cactus_top.png"
Place-Tile 6 4 "cactus_side.png"
Place-Tile 7 4 "cactus_bottom.png"
Place-Tile 8 4 "clay.png"
Place-Tile 9 4 "sugar_cane.png"
Place-Tile 10 4 "jukebox_top.png"
Place-Tile 11 4 "note_block.png"
Place-Tile 12 4 "melon_side.png"
Place-Tile 13 4 "mycelium_top.png"
Place-Tile 14 4 "mycelium_side.png"
Place-Tile 15 4 "birch_log.png"

# Row 5
Place-Tile 0 5 "torch.png"
Place-Tile 1 5 "oak_door_bottom.png"
Place-Tile 2 5 "iron_door_bottom.png"
Place-Tile 3 5 "redstone_torch.png"
Place-Tile 4 5 "redstone_torch_off.png"
Place-Tile 5 5 "pumpkin_top.png"
Place-Tile 6 5 "netherrack.png"
Place-Tile 7 5 "soul_sand.png"
Place-Tile 8 5 "glowstone.png"
Place-Tile 9 5 "piston_top_sticky.png"
Place-Tile 10 5 "piston_side.png"
Place-Tile 11 5 "piston_bottom.png"
Place-Tile 12 5 "pumpkin_side.png"
Place-Tile 13 5 "carved_pumpkin.png"
Place-Tile 14 5 "jack_o_lantern.png"
Place-Tile 15 5 "cake_top.png"

# Row 6
Place-Tile 0 6 "redstone_dust_dot.png"
Place-Tile 1 6 "redstone_dust_line0.png"
Place-Tile 2 6 "repeater.png"
Place-Tile 3 6 "repeater_on.png"
Place-Tile 4 6 "white_wool.png"
Place-Tile 6 6 "melon_top.png"
Place-Tile 7 6 "cauldron_top.png"
Place-Tile 8 6 "cauldron_side.png"
Place-Tile 9 6 "glowstone.png"
Place-Tile 10 6 "end_stone.png"
Place-Tile 11 6 "end_portal_frame_top.png"
Place-Tile 12 6 "end_portal_frame_side.png"
Place-Tile 13 6 "nether_bricks.png"
Place-Tile 14 6 "nether_wart_stage2.png"
Place-Tile 15 6 "brewing_stand.png"

# Row 7 (Modern 1.21+ blocks)
Place-Tile 0 7 "deepslate.png"
Place-Tile 1 7 "copper_ore.png"
Place-Tile 2 7 "deepslate_top.png"
Place-Tile 3 7 "raw_iron_block.png"
Place-Tile 4 7 "raw_copper_block.png"
Place-Tile 5 7 "raw_gold_block.png"
Place-Tile 6 7 "amethyst_block.png"
Place-Tile 7 7 "tuff.png"
Place-Tile 8 7 "calcite.png"
Place-Tile 9 7 "dripstone_block.png"
Place-Tile 10 7 "moss_block.png"
Place-Tile 11 7 "chiseled_copper.png"
Place-Tile 12 7 "copper_grate.png"
Place-Tile 13 7 "crafter_top.png"
Place-Tile 14 7 "crafter_side.png"
Place-Tile 15 7 "trial_spawner_top.png"

# Fluids & Sandstone
Place-Tile 0 12 "sandstone.png"
Place-Tile 1 12 "sandstone_top.png"
Place-Tile 2 12 "sandstone_bottom.png"
Place-Tile 13 12 "water_still.png" @(63, 118, 228, 200)
Place-Tile 15 15 "lava_still.png"

Write-Host "Drawing HUD Sprites (X = 256..512, Y = 0..128)..."
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

# Slot Empty Background Sprites (X = 256..336, Y = 88..104)
$slotDir = Join-Path $guiDir "sprites\container\slot"
Draw-Sprite (Join-Path $slotDir "helmet.png") 256 88 16 16
Draw-Sprite (Join-Path $slotDir "chestplate.png") 272 88 16 16
Draw-Sprite (Join-Path $slotDir "leggings.png") 288 88 16 16
Draw-Sprite (Join-Path $slotDir "boots.png") 304 88 16 16
Draw-Sprite (Join-Path $slotDir "shield.png") 320 88 16 16

# 3. Modern Title Logo & Badges (X = 512..1024, Y = 0..64)
$titleDir = Join-Path $guiDir "title"
Draw-Sprite (Join-Path $titleDir "minecraft.png") 512 0 256 64 $true
Draw-Sprite (Join-Path $titleDir "edition.png") 768 0 128 16 $true

# Standard Minecraft Font (assets/font.png, 128x128 at X = 512, Y = 64)
$fontPath = "c:\Users\0xwrdo\Documents\minecraft\assets\font.png"
if (Test-Path $fontPath) {
    Draw-Sprite $fontPath 512 64 128 128 $false
}

# Icons for Accessibility & Language (X = 900..940, Y = 0..20)
$iconDir = Join-Path $guiDir "sprites\icon"
Draw-Sprite (Join-Path $iconDir "accessibility.png") 900 0 15 15
Draw-Sprite (Join-Path $iconDir "language.png") 920 0 15 15

# 4. Buttons & Sliders (X = 0..400, Y = 424..464)
$widgetDir = Join-Path $guiDir "sprites\widget"
Draw-Sprite (Join-Path $widgetDir "button.png") 0 424 200 20
Draw-Sprite (Join-Path $widgetDir "button_highlighted.png") 200 424 200 20
Draw-Sprite (Join-Path $widgetDir "button_disabled.png") 0 444 200 20
Draw-Sprite (Join-Path $widgetDir "slider_handle.png") 200 444 8 20
Draw-Sprite (Join-Path $widgetDir "slider_handle_highlighted.png") 208 444 8 20

# 5. Container Screens (Y = 256..480)
$containerDir = Join-Path $guiDir "container"
Draw-Sprite (Join-Path $containerDir "inventory.png") 0 256 176 166 $false $true
Draw-Sprite (Join-Path $containerDir "crafting_table.png") 176 256 176 166 $false $true
Draw-Sprite (Join-Path $containerDir "furnace.png") 352 256 176 166 $false $true
Draw-Sprite (Join-Path $containerDir "generic_54.png") 528 256 176 222 $false $true

# Modern Steve Player Skin (64x64 at X = 704, Y = 256)
$stevePath = "c:\Users\0xwrdo\Documents\minecraft\assets\steve.png"
if (Test-Path $stevePath) {
    Draw-Sprite $stevePath 704 256 64 64 $false
}

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
    $dir = if ($it.Block) { $modernDir } else { $itemDir }
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
Write-Host "Master atlas 1024x1024 generated successfully! Size: $totalBytes bytes."

