<?php
try {
    throw new Exception("boom");
} catch (Exception $e) {
    echo "catch\n";
    throw $e;
}
