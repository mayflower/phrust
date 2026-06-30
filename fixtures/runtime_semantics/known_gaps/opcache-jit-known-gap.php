<?php
// runtime-semantics: category=known_gaps expect=known_gap known_gap=E_PHP_RUNTIME_UNSUPPORTED_JIT
// PHP reference: opcache/JIT capability is surfaced through the opcache API when loaded.
echo function_exists("opcache_get_status") ? "opcache-api\n" : "no-opcache-api\n";
