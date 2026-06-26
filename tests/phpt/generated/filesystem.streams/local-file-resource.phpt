--TEST--
filesystem.streams: local file resource metadata
--DESCRIPTION--
Generated file resource baseline covering fopen, fwrite, ftell, rewind, fread,
stream metadata, fclose, persisted contents, and unlink behavior.
--FILE--
<?php
$path = __DIR__ . "/local-file-resource.tmp";
@unlink($path);
$stream = fopen($path, "w+");
var_dump(is_resource($stream));
var_dump(get_resource_type($stream));
var_dump(fwrite($stream, "abc"));
var_dump(ftell($stream));
rewind($stream);
var_dump(fread($stream, 2));
$metadata = stream_get_meta_data($stream);
var_dump($metadata["wrapper_type"]);
var_dump($metadata["stream_type"]);
var_dump($metadata["mode"]);
var_dump(is_string($metadata["uri"]));
var_dump(fclose($stream));
var_dump(file_get_contents($path));
var_dump(unlink($path));
?>
--EXPECT--
bool(true)
string(6) "stream"
int(3)
int(3)
string(2) "ab"
string(9) "plainfile"
string(5) "STDIO"
string(2) "w+"
bool(true)
bool(true)
string(3) "abc"
bool(true)
