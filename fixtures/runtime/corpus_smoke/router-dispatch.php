<?php
// phase4-runtime: corpus=pass
function corpus_home(): string
{
    return "home:index";
}

function corpus_users(): string
{
    return "users:index";
}

$method = "GET";
$path = "/users";
$handler = "not-found";

if ($method === "GET") {
    switch ($path) {
        case "/":
            $handler = "home";
            break;
        case "/users":
            $handler = "users";
            break;
    }
}

if ($handler === "home") {
    echo corpus_home(), "\n";
} elseif ($handler === "users") {
    echo corpus_users(), "\n";
} else {
    echo $handler, "\n";
}
