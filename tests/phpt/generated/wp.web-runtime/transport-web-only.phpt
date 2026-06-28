--TEST--
wp.web-runtime: web transport request state
--DESCRIPTION--
Generated Branch 1 web-runtime placeholder. HTTP request superglobals, php://input, cookies, multipart uploads, and response headers are covered by php_server tests and server-smoke because the PHP CLI oracle does not populate web transport state.
--SKIPIF--
<?php die("skip web transport request state is covered by server-smoke and php_server tests\n"); ?>
--FILE--
<?php
echo "web transport only\n";
?>
--EXPECT--
web transport only
