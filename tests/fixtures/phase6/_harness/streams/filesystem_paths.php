<?php
// phase6-diff: id=PHASE6_STREAM_FILESYSTEM_PATHS area=streams expect=pass
$path = "/tmp/phase6/example.txt";
echo basename($path), "\n";
echo dirname($path), "\n";
$info = pathinfo($path);
echo $info["dirname"], "|", $info["basename"], "|", $info["extension"], "|", $info["filename"], "\n";
