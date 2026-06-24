<?php
$text = "";
for ($i = 0; $i < 12; $i++) {
    $text = $text . "ab";
}
echo "strings:", $text, ":", strlen($text), "\n";
