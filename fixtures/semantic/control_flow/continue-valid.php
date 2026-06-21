<?php
namespace App\ControlFlow;

for ($i = 0; $i < 3; $i++) {
    switch ($i) {
        case 1:
            continue 2;
        default:
            break;
    }
}
