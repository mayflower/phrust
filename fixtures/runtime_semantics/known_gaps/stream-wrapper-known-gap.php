<?php
// runtime-semantics: category=known_gaps expect=known_gap known_gap=E_PHP_RUNTIME_UNSUPPORTED_STREAM_WRAPPER
// PHP reference: php://memory behaves like an in-memory stream resource.
$handle = fopen("php://memory", "w+");
fwrite($handle, "abc");
rewind($handle);
echo fread($handle, 3), "\n";
