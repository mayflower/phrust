<?php
function value() {
    try {
        return "body";
    } finally {
        echo "finally|";
    }
}

echo value(), "\n";
