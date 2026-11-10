Add-Type -AssemblyName System.Drawing

$atlasSize = 256
$tileSize = 16
$modernDir = "c:\Users\0xwrdo\Documents\minecraft\assets\modern_textures"
$outPng = "c:\Users\0xwrdo\Documents\minecraft\assets\terrain.png"
$outBin = "c:\Users\0xwrdo\Documents\minecraft\assets\terrain_rgba.bin"

# 1. Load original terrain.png as base fallback so ASCII font and miscellaneous tiles are preserved
$baseBmp = [System.Drawing.Bitmap]::FromFile($outPng)
$atlasBmp = New-Object System.Drawing.Bitmap $atlasSize, $atlasSize, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
$g = [System.Drawing.Graphics]::FromImage($atlasBmp)
$g.DrawImage($baseBmp, 0, 0, $atlasSize, $atlasSize)
$baseBmp.Dispose()

# Helper to place a tile from modern_textures
function Place-Tile([int]$tx, [int]$ty, [string]$filename, [int[]]$tint = $null) {
    $path = Join-Path $modernDir $filename
    if (-not (Test-Path $path)) {
        Write-Warning "Missing $filename"
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
            } else {
                # transparent
                $atlasBmp.SetPixel($startX + $x, $startY + $y, [System.Drawing.Color]::FromArgb(0, 0, 0, 0))
            }
        }
    }
    $srcBmp.Dispose()
}

# Helper to composite grass side with overlay
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

Write-Host "Stitching exact modern Minecraft block textures..."

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

$g.Dispose()

# Save PNG
$atlasBmp.Save($outPng, [System.Drawing.Imaging.ImageFormat]::Png)
Write-Host "Saved modern atlas PNG to $outPng"

# Export RGBA bytes to terrain_rgba.bin
$rawBytes = New-Object byte[] (256 * 256 * 4)
$idx = 0
for ($y = 0; $y -lt 256; $y++) {
    for ($x = 0; $x -lt 256; $x++) {
        $p = $atlasBmp.GetPixel($x, $y)
        $rawBytes[$idx]     = $p.R
        $rawBytes[$idx + 1] = $p.G
        $rawBytes[$idx + 2] = $p.B
        $rawBytes[$idx + 3] = $p.A
        $idx += 4
    }
}
$atlasBmp.Dispose()

[System.IO.File]::WriteAllBytes($outBin, $rawBytes)
Write-Host "Exported $($rawBytes.Length) bytes to $outBin successfully!"

