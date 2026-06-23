<?php
for ($i = 0; $i < 10; $i++) {
    try {
        if ($i === 5) {
            echo str_repeat('x', -1), '|';
        } else {
            echo str_repeat('x', 2), '|';
        }
    } catch (ValueError $e) {
        echo 'value|';
    }
}
echo "\n";
