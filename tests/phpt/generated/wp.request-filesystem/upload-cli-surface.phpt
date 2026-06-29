--TEST--
wp.request-filesystem: upload CLI-comparable surface
--DESCRIPTION--
Generated upload surface check for empty CLI $_FILES, upload constants, and
non-upload file rejection. Actual multipart upload metadata and move success are
covered by php_server tests and server-smoke.
--FILE--
<?php
$path = __DIR__ . "/wp-request-filesystem-plain-upload.txt";
file_put_contents($path, "plain");
var_dump($_FILES);
var_dump(is_uploaded_file($path));
var_dump(move_uploaded_file($path, __DIR__ . "/wp-request-filesystem-moved.txt"));
var_dump(file_exists($path));
var_dump(UPLOAD_ERR_OK);
var_dump(UPLOAD_ERR_NO_FILE);
unlink($path);
?>
--CLEAN--
<?php
@unlink(__DIR__ . "/wp-request-filesystem-plain-upload.txt");
@unlink(__DIR__ . "/wp-request-filesystem-moved.txt");
?>
--EXPECT--
array(0) {
}
bool(false)
bool(false)
bool(true)
int(0)
int(4)
