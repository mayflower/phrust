<?php
echo "start\n";
ob_start();
echo "captured";
$captured = ob_get_clean();
echo "clean=", $captured, "\n";

ob_start();
echo "flush";
ob_end_flush();
echo "\n";

ob_start();
echo "outer";
ob_start();
echo "inner";
flush();
echo "tail";
$level = ob_get_level();
ob_end_flush();
ob_end_flush();
echo "\nlevel=", $level, "\n";
