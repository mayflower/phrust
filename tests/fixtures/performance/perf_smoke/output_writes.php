<?php
for ($i = 0; $i < 6; $i++) {
    echo "out:", $i, ":", true, "\n";
}

ob_start();
echo "buffer:", "xy";
ob_end_flush();
echo "\n";
