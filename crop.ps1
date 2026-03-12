Add-Type -AssemblyName System.Drawing
$imgPath = 'd:\scripts\Peekoo项目\peekoo-ai\apps\desktop-ui\public\sprites\dark-cat\sprite.png'
$outPath = 'd:\scripts\Peekoo项目\peekoo-ai\apps\desktop-ui\public\sprites\dark-cat\sprite_cropped.png'

$img = [System.Drawing.Image]::FromFile($imgPath)
$bmp = New-Object System.Drawing.Bitmap(5368, 6174)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$rect = New-Object System.Drawing.Rectangle(0, 0, 5368, 6174)

# DrawImage preserves alpha channel usually if pixel format is ARGB. Let's make sure.
$g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::NearestNeighbor
$g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::Half
$g.DrawImage($img, $rect, 0, 0, 5368, 6174, [System.Drawing.GraphicsUnit]::Pixel)

$g.Dispose()
$img.Dispose()

$bmp.Save($outPath, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()
Write-Host 'Done Cropping!'
