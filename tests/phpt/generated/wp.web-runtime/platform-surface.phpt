--TEST--
wp.web-runtime: platform surface
--DESCRIPTION--
Generated Branch 1 web-runtime coverage for CLI-comparable request and response function availability.
--FILE--
<?php
foreach ([
    "header",
    "headers_list",
    "headers_sent",
    "http_response_code",
    "setcookie",
    "setrawcookie",
] as $name) {
    echo $name, "=", function_exists($name) ? "yes" : "no", "\n";
}
echo "_GET=", is_array($_GET) ? "array" : "missing", "\n";
echo "_POST=", is_array($_POST) ? "array" : "missing", "\n";
echo "_COOKIE=", is_array($_COOKIE) ? "array" : "missing", "\n";
echo "_FILES=", is_array($_FILES) ? "array" : "missing", "\n";
echo "_REQUEST=", is_array($_REQUEST) ? "array" : "missing", "\n";
echo "_SERVER=", is_array($_SERVER) ? "array" : "missing", "\n";
?>
--EXPECT--
header=yes
headers_list=yes
headers_sent=yes
http_response_code=yes
setcookie=yes
setrawcookie=yes
_GET=array
_POST=array
_COOKIE=array
_FILES=array
_REQUEST=array
_SERVER=array
