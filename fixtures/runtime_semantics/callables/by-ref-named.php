<?php
function bump(&$value, $step = 1) {
    $value = $value + $step;
}

$count = 2;
bump(step: 3, value: $count);
echo $count;
