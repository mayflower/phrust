<?php
// runtime-semantics: expect=pass
$left = ["a", 2 => "b", "name" => "old"];
$right = ["x", "name" => "new", 5 => "y"];
$merged = [...$left, ...$right, "tail"];
foreach ($merged as $key => $value) {
    echo gettype($key), ":", $key, "=", $value, ";";
}
echo "\n";
