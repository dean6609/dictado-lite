# Rebuild the owned vector mark as PNG-backed Windows ICO frames. Build-only GDI+.
$ErrorActionPreference='Stop'
Add-Type -AssemblyName System.Drawing
$dictadoFrames = [Collections.Generic.List[byte[]]]::new()
$dictadoSizes = @(16,24,32,48,64,128,256)
foreach ($dictadoSize in $dictadoSizes) {
    $dictadoBitmap = [Drawing.Bitmap]::new($dictadoSize,$dictadoSize)
    $dictadoGraphics = [Drawing.Graphics]::FromImage($dictadoBitmap)
    $dictadoGraphics.SmoothingMode = 'AntiAlias'
    $dictadoGraphics.ScaleTransform($dictadoSize/64.0,$dictadoSize/64.0)
    $dictadoPath = [Drawing.Drawing2D.GraphicsPath]::new()
    foreach ($dictadoCorner in @(@(1,1,180),@(31,1,270),@(31,31,0),@(1,31,90))) { $dictadoPath.AddArc($dictadoCorner[0],$dictadoCorner[1],32,32,$dictadoCorner[2],90) }
    $dictadoPath.CloseFigure()
    $dictadoGradient=[Drawing.Drawing2D.LinearGradientBrush]::new([Drawing.RectangleF]::new(1,1,62,62),[Drawing.Color]::FromArgb(37,39,50),[Drawing.Color]::FromArgb(14,15,21),90.0)
    $dictadoGraphics.FillPath($dictadoGradient,$dictadoPath)
    $dictadoRim=[Drawing.Pen]::new([Drawing.Color]::FromArgb(56,59,72),0.7)
    $dictadoGraphics.DrawPath($dictadoRim,$dictadoPath)
    $dictadoCapsule=[Drawing.Drawing2D.GraphicsPath]::new()
    $dictadoCapsule.AddArc(7,22,20,20,90,180);$dictadoCapsule.AddArc(37,22,20,20,270,180);$dictadoCapsule.CloseFigure()
    $dictadoFill=[Drawing.SolidBrush]::new([Drawing.Color]::FromArgb(17,19,27))
    $dictadoGraphics.FillPath($dictadoFill,$dictadoCapsule);$dictadoGraphics.DrawPath($dictadoRim,$dictadoCapsule)
    for ($dictadoRadius=10;$dictadoRadius -ge 4;$dictadoRadius--) {
        $dictadoGlow=[Drawing.SolidBrush]::new([Drawing.Color]::FromArgb(10,155,174,244))
        $dictadoGraphics.FillEllipse($dictadoGlow,18-$dictadoRadius,32-$dictadoRadius,2*$dictadoRadius,2*$dictadoRadius);$dictadoGlow.Dispose()
    }
    $dictadoPearl=[Drawing.SolidBrush]::new([Drawing.Color]::FromArgb(231,241,255))
    $dictadoGraphics.FillEllipse($dictadoPearl,15,29,6,6)
    $dictadoWave=[Drawing.Pen]::new([Drawing.Color]::FromArgb(213,221,239),1.8);$dictadoWave.StartCap='Round';$dictadoWave.EndCap='Round'
    $dictadoHeights=@(5,10,16,12,7)
    for($dictadoIndex=0;$dictadoIndex -lt 5;$dictadoIndex++) {$dictadoX=30+4.5*$dictadoIndex;$dictadoHalf=$dictadoHeights[$dictadoIndex]/2.0;$dictadoGraphics.DrawLine($dictadoWave,$dictadoX,32-$dictadoHalf,$dictadoX,32+$dictadoHalf)}
    $dictadoStream=[IO.MemoryStream]::new();$dictadoBitmap.Save($dictadoStream,[Drawing.Imaging.ImageFormat]::Png);$dictadoFrames.Add($dictadoStream.ToArray())
    foreach($dictadoResource in @($dictadoWave,$dictadoPearl,$dictadoFill,$dictadoCapsule,$dictadoRim,$dictadoGradient,$dictadoPath,$dictadoGraphics,$dictadoBitmap,$dictadoStream)) {$dictadoResource.Dispose()}
}
$dictadoOutput=Join-Path (Split-Path $PSScriptRoot -Parent) 'assets/dictado.ico'
$dictadoFile=[IO.File]::Create($dictadoOutput);$dictadoWriter=[IO.BinaryWriter]::new($dictadoFile)
$dictadoWriter.Write([uint16]0);$dictadoWriter.Write([uint16]1);$dictadoWriter.Write([uint16]$dictadoFrames.Count)
$dictadoOffset=6+16*$dictadoFrames.Count
for($dictadoIndex=0;$dictadoIndex -lt $dictadoFrames.Count;$dictadoIndex++) {
    $dictadoDimension=if($dictadoSizes[$dictadoIndex] -eq 256){0}else{$dictadoSizes[$dictadoIndex]}
    $dictadoWriter.Write([byte]$dictadoDimension);$dictadoWriter.Write([byte]$dictadoDimension);$dictadoWriter.Write([uint16]0);$dictadoWriter.Write([uint16]1);$dictadoWriter.Write([uint16]32)
    $dictadoWriter.Write([uint32]$dictadoFrames[$dictadoIndex].Length);$dictadoWriter.Write([uint32]$dictadoOffset);$dictadoOffset+=$dictadoFrames[$dictadoIndex].Length
}
foreach($dictadoFrame in $dictadoFrames){$dictadoWriter.Write($dictadoFrame)}
$dictadoWriter.Dispose();$dictadoFile.Dispose()
