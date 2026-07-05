--TEST--
zlib: gzip stream helper coverage
--DESCRIPTION--
Generated gzip file-handle coverage for WordPress archive reads and writes.
--SKIPIF--
<?php
if (!extension_loaded("zlib")) die("skip zlib extension not available");
?>
--FILE--
<?php
$path = __DIR__ . "/zlib-gzip-stream-helpers.gz";
@unlink($path);
$handle = gzopen($path, "wb");
var_dump(gzwrite($handle, "alpha\nbeta\nomega"));
var_dump(gzclose($handle));

$handle = gzopen($path, "rb");
var_dump(gzgetc($handle));
var_dump(gztell($handle));
var_dump(str_replace("\n", "\\n", gzgets($handle)));
var_dump(gzeof($handle));
var_dump(gzseek($handle, 0));
var_dump(gzread($handle, 5));
var_dump(gzrewind($handle));
ob_start();
$result = gzpassthru($handle);
$passthru = ob_get_clean();
var_dump($result);
var_dump($passthru);
var_dump(gzeof($handle));
var_dump(gzclose($handle));

$lines = gzfile($path);
var_dump(count($lines));
echo str_replace("\n", "\\n", $lines[0]), "|", str_replace("\n", "\\n", $lines[1]), "|", $lines[2], "\n";
ob_start();
$result = readgzfile($path);
$read = ob_get_clean();
var_dump($result);
var_dump($read);
unlink($path);
?>
--CLEAN--
<?php
@unlink(__DIR__ . "/zlib-gzip-stream-helpers.gz");
?>
--EXPECT--
int(16)
bool(true)
string(1) "a"
int(1)
string(6) "lpha\n"
bool(false)
int(0)
string(5) "alpha"
bool(true)
int(16)
string(16) "alpha
beta
omega"
bool(true)
bool(true)
int(3)
alpha\n|beta\n|omega
int(16)
string(16) "alpha
beta
omega"
