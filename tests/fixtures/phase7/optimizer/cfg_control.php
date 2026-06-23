<?php
if (true) {
    echo "if:then\n";
} else {
    echo "if:else\n";
}

$w = 0;
while ($w < 1) {
    echo "while:", $w, "\n";
    $w++;
}

for ($i = 0; $i < 2; $i++) {
    echo "for:", $i, "\n";
}

echo "match:", match (2) {
    1 => "one",
    2 => "two",
    default => "other",
}, "\n";

try {
    throw new Exception("phase7-cfg");
} catch (Exception $e) {
    echo "catch\n";
} finally {
    echo "finally\n";
}
