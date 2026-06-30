--TEST--
filesystem.streams: stream seek contents and eof
--DESCRIPTION--
Generated stream baseline covering php://memory resource identity, fseek
SEEK_SET/SEEK_CUR/SEEK_END behavior, ftell, stream_get_contents, feof, rewind,
fread, and fclose.
--FILE--
<?php
$stream = fopen("php://memory", "w+");
var_dump(is_resource($stream));
var_dump(fwrite($stream, "abcdef"));
var_dump(ftell($stream));
var_dump(fseek($stream, 2));
var_dump(ftell($stream));
var_dump(stream_get_contents($stream, 2));
var_dump(fseek($stream, -1, SEEK_CUR));
var_dump(ftell($stream));
var_dump(stream_get_contents($stream, 1));
var_dump(fseek($stream, -1, SEEK_END));
var_dump(ftell($stream));
var_dump(stream_get_contents($stream));
var_dump(fseek($stream, -1, SEEK_SET));
var_dump(ftell($stream));
var_dump(fseek($stream, 99, 99));
var_dump(ftell($stream));
var_dump(feof($stream));
var_dump(stream_get_contents($stream));
var_dump(feof($stream));
rewind($stream);
var_dump(fread($stream, 3));
var_dump(fclose($stream));
?>
--EXPECT--
bool(true)
int(6)
int(6)
int(0)
int(2)
string(2) "cd"
int(0)
int(3)
string(1) "d"
int(0)
int(5)
string(1) "f"
int(-1)
int(6)
int(-1)
int(6)
bool(true)
string(0) ""
bool(true)
string(3) "abc"
bool(true)
