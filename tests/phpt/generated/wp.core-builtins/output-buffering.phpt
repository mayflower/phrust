--TEST--
wp.core-builtins: output buffering helpers
--DESCRIPTION--
Generated WordPress-oriented output buffering smoke coverage for template-style
capture and cleanup flows.
--FILE--
<?php
var_dump(ob_get_level());
ob_start();
echo "alpha";
$contents = ob_get_contents();
$length = ob_get_length();
$level = ob_get_level();
$captured = ob_get_clean();
var_dump($contents);
var_dump($length);
var_dump($level);
var_dump($captured);
var_dump(ob_get_level());
ob_start();
echo "beta";
ob_end_flush();
?>
--EXPECT--
int(0)
string(5) "alpha"
int(5)
int(1)
string(5) "alpha"
int(0)
beta
