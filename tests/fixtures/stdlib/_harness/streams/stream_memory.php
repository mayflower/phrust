<?php
// stdlib-diff: id=STDLIB_STREAM_MEMORY area=streams expect=pass
$wrappers = stream_get_wrappers();
echo in_array("php", $wrappers, true) ? "php-wrapper\n" : "missing-php\n";
echo in_array("file", $wrappers, true) ? "file-wrapper\n" : "missing-file\n";

$stream = fopen("php://memory", "w+");
echo get_resource_type($stream), "\n";
echo fwrite($stream, "abc"), "\n";
rewind($stream);
echo stream_get_contents($stream), "\n";
echo fseek($stream, -1, SEEK_END), "|", ftell($stream), "|", stream_get_contents($stream), "\n";
echo fseek($stream, -1, SEEK_CUR), "|", ftell($stream), "\n";
echo fseek($stream, -1, SEEK_SET), "|", ftell($stream), "\n";
$meta = stream_get_meta_data($stream);
echo $meta["wrapper_type"], "|", $meta["stream_type"], "|", ($meta["seekable"] ? "seek" : "noseek"), "\n";
fclose($stream);
