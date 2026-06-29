--TEST--
wp.core-builtins: HTTP response function surface
--DESCRIPTION--
Generated CLI-comparable coverage for HTTP response builtin availability and
initial response state.
--FILE--
<?php
$sent = headers_sent();
$headers = headers_list();
header_remove();

foreach ([
    "header",
    "header_remove",
    "headers_list",
    "headers_sent",
    "http_response_code",
    "setcookie",
    "setrawcookie",
] as $name) {
    echo $name, "=", function_exists($name) ? "yes" : "no", "\n";
}

var_dump($sent);
var_dump($headers);
?>
--EXPECT--
header=yes
header_remove=yes
headers_list=yes
headers_sent=yes
http_response_code=yes
setcookie=yes
setrawcookie=yes
bool(false)
array(0) {
}
