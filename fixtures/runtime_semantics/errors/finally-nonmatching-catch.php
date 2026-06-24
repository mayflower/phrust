<?php
// runtime-semantics: category=errors expect=pass
try {
    try {
        throw new TypeError("bad");
    } catch (Exception $e) {
        echo "wrong";
    } finally {
        echo "finally|";
    }
} catch (Throwable $e) {
    echo "outer:", $e->getMessage(), "\n";
}
