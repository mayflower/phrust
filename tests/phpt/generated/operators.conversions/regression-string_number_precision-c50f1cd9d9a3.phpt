--TEST--
Phase 9 generated regression: numeric-string comparison precision
--DESCRIPTION--
original php-src path: Zend/tests/string_to_number_comparison.phpt
original source hash: c50f1cd9d9a36635662220516c3734e73e2d9957e7c4587a4cf60686b455a60b
generated timestamp: 20260624T000000Z
generator version: phase9-operators-conversions-v1
reason: reduced precision-sensitive numeric-string comparison regression generated from reference output
--FILE--
<?php
function format($val) {
    if (is_float($val)) {
        if (is_nan($val)) return "NAN";
        if ($val == INF) return "INF";
        if ($val == -INF) return "-INF";
    }
    return json_encode($val);
}

function compare_3way($val1, $val2) {
    echo format($val1), " <=> ", format($val2), ": ", format($val1 <=> $val2), "\n";
}

ini_set("precision", 14);
compare_3way(1.75, "1.75abc");
compare_3way((string) 1.75, "1.75abc");
ini_set("precision", 0);
compare_3way(1.75, "1.75abc");
compare_3way((string) 1.75, "1.75abc");
--EXPECT--
1.75 <=> "1.75abc": -1
"1.75" <=> "1.75abc": -1
1.75 <=> "1.75abc": 1
"2" <=> "1.75abc": 1
