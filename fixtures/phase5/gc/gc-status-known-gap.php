<?php
// phase5-runtime: category=gc expect=known_gap known_gap=E_PHP_RUNTIME_GC_PUBLIC_API_GAP
$status = gc_status();
echo is_array($status) ? "array\n" : "not-array\n";
