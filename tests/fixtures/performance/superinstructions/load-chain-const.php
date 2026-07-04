<?php
// A/B probe for the fused local-load + constant-load chain: scalar loop
// math and string building keep emitting load_local/load_const pairs.
$sum = 0;
$label = "";
for ($i = 1; $i <= 5; $i++) {
    $sum = $sum + 3;
    $sum = $sum * 2;
    $label = $label . "#";
}
echo $sum, "|", $label, "\n";
$undefined_read = $missing + 1;
echo $undefined_read, "\n";
