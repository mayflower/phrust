<?php
function label(int|string $value): string {
    return "ok";
}

echo label("php"), "|", label(85), "\n";
