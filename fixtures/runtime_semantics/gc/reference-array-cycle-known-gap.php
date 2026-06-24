<?php
// runtime-semantics: category=gc expect=known_gap known_gap=E_PHP_RUNTIME_GC_PUBLIC_API_GAP
$array = [];
$array["self"] =& $array;
unset($array);
echo gc_collect_cycles(), "\n";
