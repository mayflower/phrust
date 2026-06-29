--TEST--
wp.core-language: assignment and isset bootstrap shapes
--DESCRIPTION--
WordPress bootstrap code uses compound array-dimension assignment, list
destructuring, and multi-operand isset checks while preparing request globals.
--FILE--
<?php
$_SERVER["REQUEST_URI"] = "index.php";
$_SERVER["QUERY_STRING"] = "a=1";
$_SERVER["REQUEST_URI"] .= "?" . $_SERVER["QUERY_STRING"];
echo $_SERVER["REQUEST_URI"], "\n";

list($user, $pass) = explode(":", "demo:secret", 2);
list($single) = explode(":", "solo", 2);
echo $user, "|", $pass, "|", $single, "\n";

$_REQUEST = array("a" => 1, "b" => 0);
echo isset($_REQUEST["a"], $_REQUEST["b"]) ? "both\n" : "missing\n";
unset($_REQUEST["b"]);
echo isset($_REQUEST["a"], $_REQUEST["b"]) ? "both\n" : "missing\n";
?>
--EXPECT--
index.php?a=1
demo|secret|solo
both
missing
