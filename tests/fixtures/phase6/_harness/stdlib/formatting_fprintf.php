<?php
// phase6-diff: id=PHASE6_STDLIB_FORMATTING_FPRINTF area=stdlib expect=known_gap known_gap=PHASE6-GAP-FPRINTF-STREAM-RESOURCE
$stream = fopen("php://memory", "w+");
echo fprintf($stream, "%04d", 7), "\n";
