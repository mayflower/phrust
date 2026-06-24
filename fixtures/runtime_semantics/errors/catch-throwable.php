<?php
// runtime-semantics: category=errors expect=pass
try {
    throw new Exception("boom");
} catch (Throwable $e) {
    echo "throwable:", $e->getMessage(), "\n";
}
