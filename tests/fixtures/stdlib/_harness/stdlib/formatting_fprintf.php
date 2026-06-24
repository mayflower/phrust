<?php
// stdlib-diff: id=STDLIB_FORMATTING_FPRINTF area=stdlib expect=known_gap known_gap=STDLIB-GAP-FPRINTF-STREAM-RESOURCE
$stream = fopen("php://memory", "w+");
echo fprintf($stream, "%04d", 7), "\n";
