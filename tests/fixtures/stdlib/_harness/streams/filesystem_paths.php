<?php
// stdlib-diff: id=STDLIB_STREAM_FILESYSTEM_PATHS area=streams expect=pass
$path = "/tmp/stdlib/example.txt";
echo basename($path), "\n";
echo dirname($path), "\n";
$info = pathinfo($path);
echo $info["dirname"], "|", $info["basename"], "|", $info["extension"], "|", $info["filename"], "\n";
