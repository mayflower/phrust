<?php
try {
    throw new Exception("boom");
} catch (Exception $e) {
    echo "caught\n";
}
