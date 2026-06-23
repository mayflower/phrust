<?php
$value = 0;
try {
    for ($i = 0; $i < 5; $i++) {
        $value += $i;
    }
} catch (Exception $e) {
    $value = -1;
}
echo "try-hot:", $value, "\n";
