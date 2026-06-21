<?php
try {
    throw new Exception("boom");
} catch (Exception $e) {
    echo "catch|";
} finally {
    echo "finally\n";
}
