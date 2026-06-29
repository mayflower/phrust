<?php
class BytecodeMixedFallbackBox {
    public $value = 7;
}

function dense_supported_value() {
    $items = [1, 2, 4];
    return $items[2] + 3;
}

function rich_fallback_value() {
    $box = new BytecodeMixedFallbackBox();
    return $box->value;
}

echo dense_supported_value(), "\n", rich_fallback_value(), "\n";
