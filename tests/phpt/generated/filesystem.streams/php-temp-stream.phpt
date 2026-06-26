--TEST--
filesystem.streams: php temp stream metadata
--DESCRIPTION--
Generated stream baseline covering php://temp resources, read/write seek state,
and deterministic stream metadata.
--FILE--
<?php
$stream = fopen("php://temp", "w+");
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
string(4) "TEMP"
string(3) "w+b"
string(10) "php://temp"
bool(true)
