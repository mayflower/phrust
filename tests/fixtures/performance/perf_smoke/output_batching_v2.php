<?php
echo "<section>", "\n", "row:", 1, true, false, null, ":end", "\n";

ob_start();
echo "buffer:", "inner", "-", 22, true;
$captured = ob_get_clean();
echo "captured=", $captured, "\n";

class OutputBatchString {
    public function __toString(): string {
        echo "side|";
        return "object";
    }
}

echo "object:", new OutputBatchString(), "\n";

$large = "";
for ($i = 0; $i < 8; $i++) {
    $large = $large . "x" . $i;
}
echo "large:", $large, "\n";
