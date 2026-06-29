--TEST--
wp.request-filesystem: multipart transport upload registry
--DESCRIPTION--
Generated marker for integrated-server multipart upload parsing. The PHP CLI
PHPT oracle cannot populate the request-local upload registry, so this behavior
is validated by php_server tests and server-smoke.
--SKIPIF--
<?php die("skip multipart upload registry is covered by php_server tests and server-smoke\n"); ?>
--FILE--
<?php
echo "multipart transport only\n";
?>
--EXPECT--
multipart transport only
