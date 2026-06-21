<?php
// phase4-runtime: expect=known_gap known_gap=E_PHP_RUNTIME_WARNING_CHANNEL_COMPAT
$a = [];
echo $a["missing"], "x\n";
