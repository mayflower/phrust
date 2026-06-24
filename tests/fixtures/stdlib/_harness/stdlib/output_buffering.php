<?php
// stdlib-diff: id=STDLIB_OUTPUT_BUFFERING area=stdlib expect=pass
echo "root|";
ob_start();
echo "a";
ob_start();
echo "b";
$level = ob_get_level();
$inner_length = ob_get_length();
$contents = ob_get_contents();
$inner = ob_get_clean();
echo "c";
$outer_length = ob_get_length();
ob_end_flush();
echo "|", $level, ":", $inner_length, ":", $contents, ":", $inner, ":", $outer_length, "\n";
echo ob_get_level(), "\n";
