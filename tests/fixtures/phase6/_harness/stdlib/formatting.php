<?php
// phase6-diff: id=PHASE6_STDLIB_FORMATTING area=stdlib expect=pass
echo sprintf("%04d|%-5s|%.2f|%08x|%X|%o|%c|%%", 7, "x", 1.25, 255, 255, 8, 65), "\n";
echo sprintf("%'_5s|%+d|% d", "x", 7, 7), "\n";
$count = printf("[%04d]", 7);
echo "|", $count, "\n";
echo vsprintf("%s:%d", ["id", 9]), "\n";
$vcount = vprintf("%s:%d", ["id", 9]);
echo "|", $vcount, "\n";
