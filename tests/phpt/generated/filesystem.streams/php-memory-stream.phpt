--TEST--
filesystem.streams: php memory stream metadata
--DESCRIPTION--
Generated stream baseline covering php://memory resources, read/write seek
state, and deterministic stream metadata.
--FILE--
<?php
$stream = fopen("php://memory", "w+");
var_dump(fwrite($stream, "abcdef"));
rewind($stream);
var_dump(fread($stream, 3));
$metadata = stream_get_meta_data($stream);
var_dump($metadata["wrapper_type"]);
var_dump($metadata["stream_type"]);
var_dump($metadata["mode"]);
var_dump($metadata["uri"]);
var_dump(fclose($stream));
?>
--EXPECT--
int(6)
string(3) "abc"
string(3) "PHP"
string(6) "MEMORY"
string(3) "w+b"
string(12) "php://memory"
bool(true)
