<?php
try {
    throw new Exception("boom");
} finally {
    echo "finally\n";
}
