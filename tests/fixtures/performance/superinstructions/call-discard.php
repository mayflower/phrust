<?php
// A/B probe for the fused known-call + discard: statement calls whose
// results are dropped, including one returning an object with state.
function emit($n) {
    echo "call:", $n, "\n";
    return $n * 2;
}
function make_row($n) {
    return array("id" => $n);
}
for ($i = 0; $i < 3; $i++) {
    emit($i);
    make_row($i);
}
echo "done\n";
