--TEST--
fileinfo: SVG MIME detection
--DESCRIPTION--
Generated SVG MIME coverage for deterministic upload sniffing without a host
libmagic database.
--SKIPIF--
<?php
if (!extension_loaded("fileinfo")) die("skip fileinfo extension not available");
?>
--FILE--
<?php
$finfo = finfo_open(FILEINFO_MIME_TYPE);
var_dump(finfo_buffer($finfo, '<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"></svg>'));
var_dump(finfo_buffer($finfo, '<?xml version="1.0"?><svg xmlns="http://www.w3.org/2000/svg"></svg>'));
var_dump(finfo_buffer($finfo, '<?xml version="1.0"?><root></root>'));
var_dump(image_type_to_mime_type(IMAGETYPE_SVG));
?>
--EXPECT--
string(13) "image/svg+xml"
string(13) "image/svg+xml"
string(8) "text/xml"
string(13) "image/svg+xml"
