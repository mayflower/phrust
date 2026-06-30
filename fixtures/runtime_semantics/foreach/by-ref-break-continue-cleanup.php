<?php
// runtime-semantics: expect=pass
$items = [1, 2, 3];
foreach ($items as &$value) {
    echo $value, ";";
    if ($value === 2) {
        break;
    }
}
$value = 9;
echo "|", $items[1], "|";
foreach ($items as $key => $seen) {
    echo $key, ":", $seen, ";";
}
echo "\n";
unset($value);

$more = [4, 5, 6];
foreach ($more as &$next) {
    if ($next === 4) {
        continue;
    }
    echo $next, ";";
}
$next = 8;
echo "|", $more[2], "|";
foreach ($more as $key => $seen) {
    echo $key, ":", $seen, ";";
}
echo "\n";
