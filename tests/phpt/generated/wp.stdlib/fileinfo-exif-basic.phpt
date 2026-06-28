--TEST--
wp.stdlib: fileinfo and EXIF lightweight media helpers
--DESCRIPTION--
Generated media helper coverage for common WordPress MIME and image-size
detection without a full libmagic or EXIF tag database.
--SKIPIF--
<?php
if (!extension_loaded("fileinfo")) die("skip fileinfo extension not available");
if (!extension_loaded("exif")) die("skip exif extension not available");
?>
--FILE--
<?php
$dir = __DIR__ . "/wp-stdlib-media";
$png = $dir . "/tiny.png";
$pdf = $dir . "/doc.pdf";
@unlink($png);
@unlink($pdf);
@rmdir($dir);
mkdir($dir);
file_put_contents($png, "\x89PNG\r\n\x1a\n\x00\x00\x00\x0dIHDR\x00\x00\x00\x02\x00\x00\x00\x03");
file_put_contents($pdf, "%PDF-1.7\n");
$finfo = finfo_open(FILEINFO_MIME_TYPE);
var_dump(is_resource($finfo));
var_dump(finfo_file($finfo, $png));
var_dump(finfo_buffer($finfo, "%PDF-1.7\n"));
var_dump(mime_content_type($pdf));
var_dump(exif_imagetype($png));
$size = getimagesize($png);
echo $size[0], "x", $size[1], "|", $size[2], "|", $size["mime"], "\n";
?>
--CLEAN--
<?php
$dir = __DIR__ . "/wp-stdlib-media";
@unlink($dir . "/tiny.png");
@unlink($dir . "/doc.pdf");
@rmdir($dir);
?>
--EXPECT--
bool(true)
string(9) "image/png"
string(15) "application/pdf"
string(15) "application/pdf"
int(3)
2x3|3|image/png
