--TEST--
gd: bounded GdImage load, resize, and save MVP
--DESCRIPTION--
Generated GD-compatible coverage for WordPress fallback image workflows.
--SKIPIF--
<?php
if (!extension_loaded("gd")) die("skip gd extension not available");
?>
--FILE--
<?php
$dir = __DIR__ . "/gd-image-basic";
$pngPath = $dir . "/out.png";
$jpgPath = $dir . "/out.jpg";
@unlink($pngPath);
@unlink($jpgPath);
@rmdir($dir);
mkdir($dir);
$png = base64_decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAAEElEQVR4AQEFAPr/AAAAAAAABQABZHiVOAAAAABJRU5ErkJggg==");
$img = imagecreatefromstring($png);
var_dump($img instanceof GdImage);
echo imagesx($img), "x", imagesy($img), "\n";
$dst = imagecreatetruecolor(4, 2);
var_dump($dst instanceof GdImage);
var_dump(imagecopyresampled($dst, $img, 0, 0, 0, 0, 4, 2, 1, 1));
var_dump(imagepng($dst, $pngPath));
var_dump(imagejpeg($dst, $jpgPath, 80));
$pngSize = getimagesize($pngPath);
$jpgSize = getimagesize($jpgPath);
echo $pngSize[0], "x", $pngSize[1], "|", $pngSize["mime"], "\n";
echo $jpgSize[0], "x", $jpgSize[1], "|", $jpgSize["mime"], "\n";
$info = gd_info();
var_dump($info["PNG Support"]);
var_dump($info["JPEG Support"]);
var_dump(imagedestroy($dst));
?>
--CLEAN--
<?php
$dir = __DIR__ . "/gd-image-basic";
@unlink($dir . "/out.png");
@unlink($dir . "/out.jpg");
@rmdir($dir);
?>
--EXPECT--
bool(true)
1x1
bool(true)
bool(true)
bool(true)
bool(true)
4x2|image/png
4x2|image/jpeg
bool(true)
bool(true)
bool(true)
