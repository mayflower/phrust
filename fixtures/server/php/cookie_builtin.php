<?php
setcookie("login", "hello world", [
    "path" => "/",
    "secure" => true,
    "httponly" => true,
    "samesite" => "Lax",
]);
setrawcookie("raw", "a=b", 0, "/raw");
foreach (headers_list() as $header) {
    echo $header, "\n";
}
