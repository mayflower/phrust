<?php
$fiber = new Fiber(function () {
    $sum = 0;
    for ($i = 0; $i < 12; $i++) {
        $sum = $sum + $i;
        if ($i === 5) {
            echo 'resume:', Fiber::suspend($sum), '|';
        }
    }
    echo 'sum:', $sum, '|';
});

echo 'start:', $fiber->start(), '|';
$fiber->resume('R');
echo 'done';
echo "\n";
