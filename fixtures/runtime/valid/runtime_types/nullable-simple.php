<?php
function label(?string $value): string {
    if ($value === null) {
        return "none";
    }
    return $value;
}

echo label(null), "|", label("ok"), "\n";
