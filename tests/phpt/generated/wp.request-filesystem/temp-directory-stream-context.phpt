--TEST--
wp.request-filesystem: temp files, directories, and stream contexts
--DESCRIPTION--
Generated coverage for sys_get_temp_dir, tempnam, tmpfile, directory resources,
scandir, stream context options/defaults, and stream_set_timeout.
--FILE--
<?php
$dir = __DIR__ . "/wp-request-filesystem-streams";
$a = $dir . "/a.txt";
$b = $dir . "/b.txt";
@unlink($a);
@unlink($b);
@rmdir($dir);
mkdir($dir);
$tmpRoot = sys_get_temp_dir();
var_dump(is_string($tmpRoot) && $tmpRoot !== "");
$temp = tempnam($dir, "wp");
var_dump(is_file($temp));
file_put_contents($temp, "tempnam");
var_dump(file_get_contents($temp));
unlink($temp);
$tmp = tmpfile();
var_dump(is_resource($tmp));
var_dump(fwrite($tmp, "tmpfile"));
rewind($tmp);
var_dump(fread($tmp, 20));
var_dump(fclose($tmp));
file_put_contents($a, "a");
file_put_contents($b, "b");
$scan = scandir($dir);
sort($scan);
echo implode(",", $scan), "\n";
$handle = opendir($dir);
$entries = [];
while (($entry = readdir($handle)) !== false) {
    $entries[] = $entry;
}
sort($entries);
echo implode(",", $entries), "\n";
rewinddir($handle);
var_dump(readdir($handle));
var_dump(closedir($handle));
$ctx = stream_context_create(["http" => ["timeout" => 3]]);
$options = stream_context_get_options($ctx);
var_dump($options["http"]["timeout"]);
var_dump(stream_context_set_option($ctx, "ssl", "verify_peer", false));
$options = stream_context_get_options($ctx);
var_dump($options["ssl"]["verify_peer"]);
$default = stream_context_set_default(["http" => ["method" => "GET"]]);
var_dump(is_resource($default));
$defaultOptions = stream_context_get_options(stream_context_get_default());
var_dump($defaultOptions["http"]["method"]);
$memory = fopen("php://memory", "w+");
var_dump(stream_set_timeout($memory, 1, 200));
fclose($memory);
unlink($a);
unlink($b);
rmdir($dir);
?>
--CLEAN--
<?php
$dir = __DIR__ . "/wp-request-filesystem-streams";
foreach (glob($dir . "/*") ?: [] as $path) {
    @unlink($path);
}
@rmdir($dir);
?>
--EXPECT--
bool(true)
bool(true)
string(7) "tempnam"
bool(true)
int(7)
string(7) "tmpfile"
bool(true)
.,..,a.txt,b.txt
.,..,a.txt,b.txt
string(1) "."
NULL
int(3)
bool(true)
bool(false)
bool(true)
string(3) "GET"
bool(false)
