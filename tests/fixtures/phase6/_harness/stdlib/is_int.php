<?php
// phase6-diff: id=PHASE6_STDLIB_IS_INT area=stdlib expect=pass
echo is_int(7) ? "yes" : "no";
echo "|";
echo is_int("7") ? "yes" : "no";
echo "\n";
