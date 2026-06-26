--TEST--
filesystem.streams: include path scope
--DESCRIPTION--
Generated include baseline covering request-local include_path resolution and
include return values.
--FILE--
<?php
$dir = __DIR__ . "/include-path-scope-lib";
@mkdir($dir);
file_put_contents($dir . "/inc.php", "<?php echo \"INC|\"; return 7; ?>");
ini_set("include_path", $dir);
$value = include "inc.php";
echo "RET=$value\n";
?>
--CLEAN--
<?php
$dir = __DIR__ . "/include-path-scope-lib";
@unlink($dir . "/inc.php");
@rmdir($dir);
?>
--EXPECT--
INC|RET=7
