<?php
// runtime-semantics: category=errors expect=pass
try {
    throw new TypeError("bad");
} catch (Error $e) {
    echo "error:", $e->getMessage(), "\n";
}
