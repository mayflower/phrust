<?php
// stdlib-diff: id=STDLIB_SPL_ARRAY_ITERATOR area=spl-reflection expect=pass
$it = new ArrayIterator(["a" => 10, "b" => 20]);
foreach ($it as $key => $value) {
    echo $key, "=", $value, "\n";
}
echo count($it), "\n";
