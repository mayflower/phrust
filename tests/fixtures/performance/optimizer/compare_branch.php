<?php
if (3 < 5) {
    echo "compare:lt\n";
} else {
    echo "compare:bad\n";
}

if (7 === 7) {
    echo "compare:strict\n";
}

echo "compare:spaceship:", 2 <=> 3, "\n";
