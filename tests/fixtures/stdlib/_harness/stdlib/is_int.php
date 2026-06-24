<?php
// stdlib-diff: id=STDLIB_IS_INT area=stdlib expect=pass
echo is_int(7) ? "yes" : "no";
echo "|";
echo is_int("7") ? "yes" : "no";
echo "\n";
